//! Native Seerr user-session API, independent of Jellyfin's client and token.
//!
//! Persist only [`ConnectionConfig`] in app configuration. Remember-me passwords
//! are stored separately in the system keyring; cookies, Quick Connect secrets
//! and release tokens stay in memory. Never log
//! request/response bodies. The client deliberately rejects **all** redirects,
//! including same-origin redirects, so credentials cannot escape the configured
//! endpoint. Supply the final server base URL, not `/api/v1`.
//!
//! # UI security contract
//! Check [`ConnectionConfig::transport_security`] before login. For
//! [`TransportSecurity::CleartextHttp`], warn that passwords, sessions and viewing
//! activity are exposed to the network and require explicit user acknowledgement.
//! The remember-me opt-in applies only to the exact confirmed URL and account;
//! passwords never enter app configuration. Do not treat an existing
//! Jellyfin access token as a Seerr session. A `401` clears the in-memory session;
//! `403` means permission denied and does not imply logout. Do not automatically
//! retry mutations: a timeout can mean the server already accepted the request.
//!
//! Methods take `&mut self` to serialize cookie updates/login/logout. Keep one
//! client per connection/user in a service task rather than cloning sessions.
mod discovery;
mod discovery_card;
mod dto;
mod keyring;
mod removal;
pub(crate) use removal::*;
#[cfg(test)]
mod tests;
pub(crate) mod ui;
mod ui_service;
mod ui_state;
pub use dto::*;

use reqwest::{header, redirect::Policy, Method, RequestBuilder, Response, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt, net::IpAddr, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginMethod {
    Local,
    Jellyfin,
    JellyfinQuickConnect,
}

/// The only persistable Seerr state; intentionally has no secret fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredConnection")]
pub struct ConnectionConfig {
    url: String,
    pub login_method: LoginMethod,
    pub username: Option<String>,
    /// Opt-in for this exact server and account. The password itself is in Secret Service.
    #[serde(default, rename = "remember_me")]
    pub remember_password: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredConnection {
    url: String,
    login_method: LoginMethod,
    username: Option<String>,
    #[serde(default, rename = "remember_me")]
    remember_password: bool,
}
impl TryFrom<StoredConnection> for ConnectionConfig {
    type Error = Error;
    fn try_from(value: StoredConnection) -> Result<Self, Error> {
        let mut config = Self::new(&value.url, value.login_method, value.username)?;
        config.remember_password = value.remember_password;
        Ok(config)
    }
}
impl ConnectionConfig {
    pub fn new(
        url: &str,
        login_method: LoginMethod,
        username: Option<String>,
    ) -> Result<Self, Error> {
        let mut parsed = Url::parse(url).map_err(|_| Error::InvalidUrl)?;
        // Also reject empty userinfo (`https://@host`) that Url otherwise drops.
        let authority = url
            .split_once("://")
            .map(|(_, tail)| tail.split(['/', '?', '#']).next().unwrap_or(""));
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || authority.is_some_and(|a| a.contains('@'))
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(Error::InvalidUrl);
        }
        if !parsed.path().ends_with('/') {
            parsed.set_path(&format!("{}/", parsed.path()));
        }
        Ok(Self {
            url: parsed.to_string(),
            login_method,
            username,
            remember_password: false,
        })
    }
    pub fn url(&self) -> &str {
        &self.url
    }
    /// HTTP on nonloopback endpoints exposes credentials and requires a UI warning.
    pub fn transport_security(&self) -> TransportSecurity {
        let url = Url::parse(&self.url).expect("validated connection URL");
        if url.scheme() == "https" {
            TransportSecurity::Encrypted
        } else {
            let host = url.host_str().unwrap_or("");
            let loopback = host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback());
            if loopback {
                TransportSecurity::LoopbackHttp
            } else {
                TransportSecurity::CleartextHttp
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportSecurity {
    Encrypted,
    LoopbackHttp,
    CleartextHttp,
}

/// Sanitized failures: no URLs, bodies, passwords, cookies or reqwest errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidUrl,
    InvalidSelection,
    ClientSetup,
    NotAuthenticated,
    Unauthorized,
    Forbidden,
    RedirectDenied,
    Timeout,
    Transport,
    Http(u16),
    InvalidResponse,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidUrl => "Use an HTTP(S) base URL without credentials, query or fragment.",
            Self::InvalidSelection => "Invalid selection or pagination.",
            Self::ClientSetup => "Could not create the Seerr connection.",
            Self::NotAuthenticated => "Sign in to Seerr first.",
            Self::Unauthorized => "Seerr session is invalid or expired. Sign in again.",
            Self::Forbidden => "Seerr denied permission for this operation.",
            Self::RedirectDenied => "Seerr redirected the request. Configure the final server URL.",
            Self::Timeout => "Seerr timed out. A submitted operation may have succeeded; refresh before retrying.",
            Self::Transport => "Could not reach Seerr. A submitted operation may have succeeded; refresh before retrying.",
            Self::Http(_) => "Seerr returned an unsuccessful HTTP status.",
            Self::InvalidResponse => "Seerr returned an invalid response.",
        })
    }
}
impl std::error::Error for Error {}
fn transport_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout
    } else {
        Error::Transport
    }
}

struct SessionCookie {
    value: header::HeaderValue,
    path: String,
    secure: bool,
    expires_at: Option<i64>,
}
impl SessionCookie {
    fn applies(&self, url: &Url) -> bool {
        let path = url.path();
        (!self.secure || url.scheme() == "https")
            && self
                .expires_at
                .map_or(true, |expiry| expiry > chrono::Utc::now().timestamp())
            && path.starts_with(&self.path)
            && (path.len() == self.path.len()
                || self.path.ends_with('/')
                || path.as_bytes().get(self.path.len()) == Some(&b'/'))
    }
}

pub struct SeerrClient {
    client: reqwest::Client,
    root: Url,
    session: Option<SessionCookie>,
}
impl fmt::Debug for SeerrClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SeerrClient { [session redacted] }")
    }
}
impl SeerrClient {
    /// Uses a 10s connect / 45s total timeout; release search allows 90s for
    /// Seerr's series mapping plus its bounded 45s upstream indexer search.
    pub fn new(config: &ConnectionConfig) -> Result<Self, Error> {
        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(45))
            .user_agent(crate::jellyfin_api::api_client::APP_USER_AGENT)
            .build()
            .map_err(|_| Error::ClientSetup)?;
        let root = Url::parse(config.url())
            .map_err(|_| Error::InvalidUrl)?
            .join("api/v1/")
            .map_err(|_| Error::InvalidUrl)?;
        Ok(Self {
            client,
            root,
            session: None,
        })
    }

    /// Indicates a usable in-memory cookie, not proof that the server still accepts it.
    pub fn has_session(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.applies(&self.root))
    }
    pub fn forget_session(&mut self) {
        self.session = None;
    }

    fn request(
        &mut self,
        method: Method,
        path: &str,
        authenticated: bool,
    ) -> Result<RequestBuilder, Error> {
        let url = self.root.join(path).map_err(|_| Error::InvalidUrl)?;
        let mut request = self.client.request(method, url.clone());
        if authenticated {
            if let Some(cookie) = self.session.as_ref().filter(|s| s.applies(&url)) {
                request = request.header(header::COOKIE, cookie.value.clone());
            } else {
                self.session = None;
                return Err(Error::NotAuthenticated);
            }
        }
        Ok(request)
    }

    // Only connect.sid from this exact configured host is retained; no general jar,
    // export, persistence or external URL request API exists.
    fn update_cookie(&mut self, response: &Response) {
        for value in response.headers().get_all(header::SET_COOKIE) {
            let Ok(value) = value.to_str() else { continue };
            let mut fields = value.split(';');
            let Some((name, value)) = fields.next().and_then(|s| s.trim().split_once('=')) else {
                continue;
            };
            if name != "connect.sid" {
                continue;
            }
            let default_path = response
                .url()
                .path()
                .rsplit_once('/')
                .map(|(p, _)| if p.is_empty() { "/" } else { p })
                .unwrap_or("/");
            let mut path = default_path.to_string();
            let mut secure = false;
            let mut expires_at = None;
            let mut max_age = None;
            let mut valid_domain = true;
            for field in fields {
                let (key, val) = field.trim().split_once('=').unwrap_or((field.trim(), ""));
                match key.to_ascii_lowercase().as_str() {
                    "path" if val.starts_with('/') => path = val.to_string(),
                    "secure" => secure = true,
                    "domain" => {
                        valid_domain = val
                            .trim_start_matches('.')
                            .eq_ignore_ascii_case(response.url().host_str().unwrap_or(""))
                    }
                    "max-age" => max_age = val.parse::<i64>().ok(),
                    "expires" => {
                        expires_at = chrono::DateTime::parse_from_rfc2822(val)
                            .ok()
                            .map(|d| d.timestamp())
                    }
                    _ => {}
                }
            }
            if !valid_domain {
                continue;
            }
            if let Some(seconds) = max_age {
                expires_at = Some(
                    chrono::Utc::now()
                        .timestamp()
                        .saturating_add(seconds.max(0)),
                );
            }
            if value.is_empty() || expires_at.is_some_and(|e| e <= chrono::Utc::now().timestamp()) {
                self.session = None;
                continue;
            }
            if !value
                .bytes()
                .all(|b| b.is_ascii_graphic() && !matches!(b, b'"' | b',' | b';' | b'\\'))
            {
                continue;
            }
            if let Ok(mut value) = header::HeaderValue::from_str(&format!("connect.sid={value}")) {
                value.set_sensitive(true);
                self.session = Some(SessionCookie {
                    value,
                    path,
                    secure,
                    expires_at,
                });
            }
        }
    }
    async fn send(&mut self, request: RequestBuilder) -> Result<Response, Error> {
        let response = request.send().await.map_err(transport_error)?;
        let status = response.status();
        // Never process cookies from redirect responses.
        if status.is_redirection() {
            return Err(Error::RedirectDenied);
        }
        if status == StatusCode::UNAUTHORIZED {
            self.session = None;
            return Err(Error::Unauthorized);
        }
        self.update_cookie(&response);
        if status == StatusCode::FORBIDDEN {
            return Err(Error::Forbidden);
        }
        if !status.is_success() {
            return Err(Error::Http(status.as_u16()));
        }
        Ok(response)
    }
    async fn json<T: DeserializeOwned>(&mut self, request: RequestBuilder) -> Result<T, Error> {
        self.send(request).await?.json().await.map_err(|error| {
            if error.is_timeout() {
                Error::Timeout
            } else {
                Error::InvalidResponse
            }
        })
    }
    async fn login(&mut self, path: &str, body: serde_json::Value) -> Result<User, Error> {
        self.forget_session();
        let request = self.request(Method::POST, path, false)?.json(&body);
        let result = self.json::<User>(request).await;
        // Seerr's local-login endpoint selects only id/email/password/plexId.
        // Its entity default (or an omitted field) yields zero, not an
        // authoritative denial of the authenticated account's persisted rights.
        // Resolve the session profile in that case (also safe for a truly
        // zero-permission account); never grant permissions locally.
        let result = match result {
            Ok(user) if self.has_session() && user.permissions == 0 => match self.me().await {
                Ok(profile) if profile.id == user.id => Ok(profile),
                Ok(_) => Err(Error::InvalidResponse),
                Err(error) => Err(error),
            },
            other => other,
        };
        match result {
            Ok(user) if self.has_session() => Ok(user),
            Ok(_) => {
                self.forget_session();
                Err(Error::InvalidResponse)
            }
            Err(error) => {
                self.forget_session();
                Err(error)
            }
        }
    }
    /// Local login takes email (not the display username).
    pub async fn login_local(&mut self, email: &str, password: &str) -> Result<User, Error> {
        self.login(
            "auth/local",
            serde_json::json!({"email": email, "password": password}),
        )
        .await
    }
    /// Uses Seerr's already-configured Jellyfin server; never sends a Jellyfin token.
    pub async fn login_jellyfin(&mut self, username: &str, password: &str) -> Result<User, Error> {
        self.login(
            "auth/jellyfin",
            serde_json::json!({"username": username, "password": password}),
        )
        .await
    }
    pub async fn me(&mut self) -> Result<User, Error> {
        let request = self.request(Method::GET, "auth/me", true)?;
        self.json(request).await
    }
    /// Always forgets the local session, including when remote logout fails.
    pub async fn logout(&mut self) -> Result<(), Error> {
        let request = self.request(Method::POST, "auth/logout", true);
        self.forget_session();
        let result = match request {
            Ok(request) => self.send(request).await.map(|_| ()),
            Err(error) => Err(error),
        };
        self.forget_session();
        result
    }
    pub async fn quick_connect_initiate(&mut self) -> Result<QuickConnect, Error> {
        self.forget_session();
        let request = self.request(Method::POST, "auth/jellyfin/quickconnect/initiate", false)?;
        self.json(request).await
    }
    pub async fn quick_connect_check(
        &mut self,
        secret: &QuickConnectSecret,
    ) -> Result<QuickConnectStatus, Error> {
        let request = self
            .request(Method::GET, "auth/jellyfin/quickconnect/check", false)?
            .query(&[("secret", &secret.0)]);
        self.json(request).await
    }
    pub async fn quick_connect_authenticate(
        &mut self,
        secret: &QuickConnectSecret,
    ) -> Result<User, Error> {
        self.login(
            "auth/jellyfin/quickconnect/authenticate",
            serde_json::json!({"secret": secret.0}),
        )
        .await
    }

    /// Type-specific feeds; Seerr's search endpoint is multi-type, so preserve
    /// its page cursor while filtering before artwork jobs or rendering.
    pub async fn discover_filtered(
        &mut self,
        feed: DiscoverFeed,
        kind: MediaType,
        query: &str,
        page: u32,
    ) -> Result<Page<DiscoveryResult>, Error> {
        let request = if query.trim().is_empty() {
            let mut request = self.paged_request(feed.path(), page, None)?;
            if feed == DiscoverFeed::Trending {
                request = request.query(&[(
                    "mediaType",
                    match kind {
                        MediaType::Movie => "movie",
                        MediaType::Tv => "tv",
                    },
                )]);
            }
            request
        } else {
            self.paged_request("search", page, None)?
                .query(&[("query", query.trim())])
        };
        let mut result: Page<DiscoveryResult> = self.json(request).await?;
        result.results.retain(|item| item.media_type.matches(kind));
        Ok(result)
    }

    pub async fn discover(
        &mut self,
        feed: DiscoverFeed,
        page: u32,
        language: Option<&str>,
    ) -> Result<Page<DiscoveryResult>, Error> {
        let request = self.paged_request(feed.path(), page, language)?;
        self.json(request).await
    }
    pub async fn search(
        &mut self,
        query: &str,
        page: u32,
        language: Option<&str>,
    ) -> Result<Page<DiscoveryResult>, Error> {
        let request = self
            .paged_request("search", page, language)?
            .query(&[("query", query)]);
        self.json(request).await
    }
    fn paged_request(
        &mut self,
        path: &str,
        page: u32,
        language: Option<&str>,
    ) -> Result<RequestBuilder, Error> {
        if page == 0 {
            return Err(Error::InvalidSelection);
        }
        let mut request = self
            .request(Method::GET, path, true)?
            .query(&[("page", page)]);
        if let Some(language) = language {
            request = request.query(&[("language", language)]);
        }
        Ok(request)
    }
    pub async fn movie_details(&mut self, tmdb_id: u64) -> Result<MovieDetails, Error> {
        positive_id(tmdb_id)?;
        let request = self.request(Method::GET, &format!("movie/{tmdb_id}"), true)?;
        self.json(request).await
    }
    pub async fn tv_details(&mut self, tmdb_id: u64) -> Result<TvDetails, Error> {
        positive_id(tmdb_id)?;
        let request = self.request(Method::GET, &format!("tv/{tmdb_id}"), true)?;
        self.json(request).await
    }
    pub async fn tv_season(&mut self, tmdb_id: u64, season_number: u32) -> Result<Season, Error> {
        positive_id(tmdb_id)?;
        let request = self.request(
            Method::GET,
            &format!("tv/{tmdb_id}/season/{season_number}"),
            true,
        )?;
        self.json(request).await
    }
    pub async fn create_request(
        &mut self,
        selection: &RequestSelection,
    ) -> Result<MediaRequest, Error> {
        let body = match selection {
            RequestSelection::Movie { tmdb_id, is4k } => {
                positive_id(*tmdb_id)?;
                serde_json::json!({"mediaType":"movie", "mediaId":tmdb_id, "is4k":is4k})
            }
            RequestSelection::Tv {
                tmdb_id,
                seasons,
                is4k,
            } => {
                positive_id(*tmdb_id)?;
                if seasons.is_empty()
                    || seasons
                        .iter()
                        .enumerate()
                        .any(|(i, s)| seasons[..i].contains(s))
                {
                    return Err(Error::InvalidSelection);
                }
                serde_json::json!({"mediaType":"tv", "mediaId":tmdb_id, "seasons":seasons, "is4k":is4k})
            }
        };
        let request = self.request(Method::POST, "request", true)?.json(&body);
        self.json(request).await
    }
    pub async fn requests(&mut self, options: &RequestListOptions) -> Result<RequestPage, Error> {
        if options.take == 0 {
            return Err(Error::InvalidSelection);
        }
        if let Some(id) = options.requested_by {
            positive_id(id)?;
        }
        let request = self.request(Method::GET, "request", true)?.query(options);
        self.json(request).await
    }
    pub async fn request_status(&mut self, request_id: u64) -> Result<MediaRequest, Error> {
        positive_id(request_id)?;
        let request = self.request(Method::GET, &format!("request/{request_id}"), true)?;
        self.json(request).await
    }

    /// Requires MANAGE_REQUESTS. Progress here uses 0..100, not torrent fractions.
    pub async fn tv_downloads(
        &mut self,
        tmdb_id: u64,
        is4k: bool,
        request_id: Option<u64>,
    ) -> Result<TvDownloadDiagnostics, Error> {
        positive_id(tmdb_id)?;
        let mut request = self
            .request(Method::GET, &format!("tv/{tmdb_id}/downloads"), true)?
            .query(&[("is4k", is4k)]);
        if let Some(id) = request_id {
            positive_id(id)?;
            request = request.query(&[("requestId", id)]);
        }
        self.json(request).await
    }
    /// Requires MANAGE_REQUESTS. Tokens expire after five minutes and bind to this
    /// user and exact selection. Show rejection reasons/duplicate warnings first.
    pub async fn search_tv_releases(
        &mut self,
        tmdb_id: u64,
        selection: &DownloadSelection,
    ) -> Result<TvDownloadSearch, Error> {
        validate_download_selection(tmdb_id, selection)?;
        let request = self
            .request(
                Method::POST,
                &format!("tv/{tmdb_id}/downloads/search"),
                true,
            )?
            .timeout(Duration::from_secs(90))
            .json(selection);
        self.json(request).await
    }
    /// Both confirmations must come from explicit UI decisions, not defaults based
    /// on warning flags. A 409 requires refreshing/re-searching or confirmation.
    /// A 202 means accepted, NOT completed. Never automatically retry a grab.
    pub async fn grab_tv_release(
        &mut self,
        tmdb_id: u64,
        selection: &DownloadSelection,
        token: &ReleaseToken,
        confirm_rejected: bool,
        confirm_duplicate: bool,
        confirm_replacement: bool,
    ) -> Result<GrabAccepted, Error> {
        validate_download_selection(tmdb_id, selection)?;
        if token.0.len() != 48
            || !token
                .0
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::InvalidSelection);
        }
        let mut body = serde_json::to_value(selection).map_err(|_| Error::InvalidSelection)?;
        body["token"] = serde_json::json!(token.0);
        body["confirmRejected"] = serde_json::json!(confirm_rejected);
        body["confirmDuplicate"] = serde_json::json!(confirm_duplicate);
        body["confirmReplacement"] = serde_json::json!(confirm_replacement);
        let request = self
            .request(Method::POST, &format!("tv/{tmdb_id}/downloads/grab"), true)?
            .timeout(Duration::from_secs(120))
            .json(&body);
        self.json(request).await
    }
    /// Movie sources use the same user-bound capability and confirmation rules as TV.
    pub async fn search_movie_releases(
        &mut self,
        tmdb_id: u64,
        selection: &MovieDownloadSelection,
    ) -> Result<TvDownloadSearch, Error> {
        positive_id(tmdb_id)?;
        if let Some(id) = selection.request_id {
            positive_id(id)?;
        }
        let request = self
            .request(
                Method::POST,
                &format!("movie/{tmdb_id}/downloads/search"),
                true,
            )?
            .timeout(Duration::from_secs(90))
            .json(selection);
        self.json(request).await
    }
    pub async fn grab_movie_release(
        &mut self,
        tmdb_id: u64,
        selection: &MovieDownloadSelection,
        token: &ReleaseToken,
        confirm_rejected: bool,
        confirm_duplicate: bool,
        confirm_replacement: bool,
    ) -> Result<GrabAccepted, Error> {
        positive_id(tmdb_id)?;
        if let Some(id) = selection.request_id {
            positive_id(id)?;
        }
        if token.0.len() != 48
            || !token
                .0
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::InvalidSelection);
        }
        let mut body = serde_json::to_value(selection).map_err(|_| Error::InvalidSelection)?;
        body["token"] = serde_json::json!(token.0);
        body["confirmRejected"] = serde_json::json!(confirm_rejected);
        body["confirmDuplicate"] = serde_json::json!(confirm_duplicate);
        body["confirmReplacement"] = serde_json::json!(confirm_replacement);
        let request = self
            .request(
                Method::POST,
                &format!("movie/{tmdb_id}/downloads/grab"),
                true,
            )?
            .timeout(Duration::from_secs(120))
            .json(&body);
        self.json(request).await
    }
    /// Requires MANAGE_REQUESTS. Read-only global torrent snapshot, no upstream URLs.
    pub async fn torrent_downloads(&mut self) -> Result<TorrentDownloads, Error> {
        let request = self.request(Method::GET, "downloads/torrents", true)?;
        let response: TorrentDownloads = self.json(request).await?;
        if response
            .torrents
            .iter()
            .any(|t| !t.progress.is_finite() || !(0.0..=1.0).contains(&t.progress))
        {
            return Err(Error::InvalidResponse);
        }
        Ok(response)
    }
}
fn positive_id(id: u64) -> Result<(), Error> {
    if id == 0 || id > 9_007_199_254_740_991 {
        Err(Error::InvalidSelection)
    } else {
        Ok(())
    }
}
fn validate_download_selection(tmdb_id: u64, selection: &DownloadSelection) -> Result<(), Error> {
    positive_id(tmdb_id)?;
    if let Some(id) = selection.request_id {
        positive_id(id)?;
    }
    if let Some(id) = selection.episode_id {
        positive_id(id)?;
    }
    Ok(())
}
