use super::*;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

#[tokio::test]
async fn same_host_probe_is_read_only_and_sends_no_authentication() {
    let mock = Mock::new(vec![Reply::json(
        200,
        json!({"version":"3.0.0", "commitTag":"test"}),
    )]);
    assert!(super::discovery::probe_seerr_url(&mock.url).await);
    let requests = mock.finish();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].target, "/seerr/api/v1/status");
    for header in [
        "cookie",
        "authorization",
        "x-api-key",
        "x-emby-authorization",
    ] {
        assert!(requests[0].header(header).is_none());
    }
}

#[tokio::test]
async fn same_host_probe_rejects_redirects_unrecognized_and_oversized_responses() {
    for reply in [
        Reply::json(302, json!({})).header("Location", "http://example.invalid/"),
        Reply::json(200, json!({"version":"a different service"})),
        Reply::json(
            200,
            json!({"version":"3.0.0", "commitTag":"x".repeat(17000)}),
        ),
        Reply::json(403, json!({"message":"denied"})),
    ] {
        let mock = Mock::new(vec![reply]);
        assert!(!super::discovery::probe_seerr_url(&mock.url).await);
        assert_eq!(mock.finish().len(), 1);
    }
    assert!(!super::discovery::probe_seerr_url("file:///tmp/server").await);
}

const COOKIE: &str = "connect.sid=private-session";
fn user() -> Value {
    json!({"id":7,"email":"user@example.test","username":"user","permissions":16,
        "jellyfinAuthToken":"upstream-secret", "plexToken":"another-secret"})
}
struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
    delay: Duration,
}
impl Reply {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            headers: vec![],
            body: body.to_string(),
            delay: Duration::ZERO,
        }
    }
    fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }
}
fn login_reply() -> Reply {
    Reply::json(200, user()).header(
        "Set-Cookie",
        &format!("{COOKIE}; Path=/; HttpOnly; SameSite=Lax"),
    )
}
#[derive(Debug)]
struct Captured {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Value,
}
impl Captured {
    fn header(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}
struct Mock {
    url: String,
    requests: Arc<Mutex<Vec<Captured>>>,
    handle: thread::JoinHandle<()>,
}
impl Mock {
    fn new(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/seerr/", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(vec![]));
        let captured = requests.clone();
        let handle = thread::spawn(move || {
            for reply in replies {
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(
                                std::time::Instant::now() < deadline,
                                "mock request never arrived"
                            );
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(error) => panic!("mock accept: {error}"),
                    }
                };
                captured.lock().unwrap().push(read_request(&mut stream));
                thread::sleep(reply.delay);
                let mut response = format!("HTTP/1.1 {} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n", reply.status, reply.body.len());
                for (key, value) in reply.headers {
                    response.push_str(&format!("{key}: {value}\r\n"));
                }
                response.push_str("\r\n");
                response.push_str(&reply.body);
                // A timeout test may already have closed the socket.
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Self {
            url,
            requests,
            handle,
        }
    }
    fn client(&self) -> SeerrClient {
        SeerrClient::new(&ConnectionConfig::new(&self.url, LoginMethod::Local, None).unwrap())
            .unwrap()
    }
    fn finish(self) -> Vec<Captured> {
        self.handle.join().unwrap();
        Arc::try_unwrap(self.requests)
            .unwrap()
            .into_inner()
            .unwrap()
    }
}
fn read_request(stream: &mut TcpStream) -> Captured {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = vec![];
    let end = loop {
        let mut chunk = [0; 4096];
        let count = stream.read(&mut chunk).unwrap();
        assert_ne!(count, 0);
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break end + 4;
        }
    };
    let head = String::from_utf8(bytes[..end].to_vec()).unwrap();
    let mut lines = head.split("\r\n");
    let mut start = lines.next().unwrap().split_whitespace();
    let method = start.next().unwrap().to_string();
    let target = start.next().unwrap().to_string();
    let headers: Vec<_> = lines
        .filter_map(|l| {
            l.split_once(':')
                .map(|(k, v)| (k.to_string(), v.trim().to_string()))
        })
        .collect();
    let length = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .map(|(_, v)| v.parse::<usize>().unwrap())
        .unwrap_or(0);
    while bytes.len() < end + length {
        let mut chunk = [0; 4096];
        let count = stream.read(&mut chunk).unwrap();
        assert_ne!(count, 0);
        bytes.extend_from_slice(&chunk[..count]);
    }
    let body = if length == 0 {
        Value::Null
    } else {
        serde_json::from_slice(&bytes[end..end + length]).unwrap()
    };
    Captured {
        method,
        target,
        headers,
        body,
    }
}
fn query(request: &Captured) -> std::collections::HashMap<String, String> {
    Url::parse(&format!("http://localhost{}", request.target))
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect()
}

#[test]
fn connection_config_validates_urls_preserves_subpaths_and_exposes_cleartext_risk() {
    let cfg = ConnectionConfig::new(
        "https://EXAMPLE.test/seerr/nested",
        LoginMethod::Jellyfin,
        Some("user".into()),
    )
    .unwrap();
    assert_eq!(cfg.url(), "https://example.test/seerr/nested/");
    assert_eq!(cfg.transport_security(), TransportSecurity::Encrypted);
    for url in [
        "http://localhost:5055",
        "http://127.1.2.3/path",
        "http://[::1]:5055/",
    ] {
        assert_eq!(
            ConnectionConfig::new(url, LoginMethod::Local, None)
                .unwrap()
                .transport_security(),
            TransportSecurity::LoopbackHttp
        );
    }
    assert_eq!(
        ConnectionConfig::new("http://192.168.1.2/seerr", LoginMethod::Local, None)
            .unwrap()
            .transport_security(),
        TransportSecurity::CleartextHttp
    );
    for url in [
        "not a url",
        "ftp://host/",
        "https://user:password@host/",
        "https://@host/",
        "https://host/?token=secret",
        "https://host/#secret",
        "https://host/?",
        "https://host/#",
    ] {
        let err = ConnectionConfig::new(url, LoginMethod::Local, None).unwrap_err();
        assert_eq!(err, Error::InvalidUrl);
        assert!(!format!("{err:?} {err}").contains("secret"));
    }
}

#[test]
fn connection_config_roundtrips_without_auth_and_old_configs_still_load() {
    let mut config = crate::config::Config::default();
    let old = config.to_toml().unwrap();
    assert!(toml::from_str::<crate::config::Config>(&old)
        .unwrap()
        .seerr
        .is_none());
    config.seerr = Some(
        ConnectionConfig::new(
            "https://host/seerr",
            LoginMethod::Local,
            Some("email@example.test".into()),
        )
        .unwrap(),
    );
    let text = config.to_toml().unwrap();
    assert_eq!(
        toml::from_str::<crate::config::Config>(&text).unwrap(),
        config
    );
    for forbidden in ["password", "session", "token", "secret"] {
        assert!(!text.contains(forbidden));
    }
    assert!(serde_json::from_value::<ConnectionConfig>(
        json!({"url":"https://host","login_method":"local","password":"secret"})
    )
    .is_err());
    assert!(serde_json::from_value::<ConnectionConfig>(
        json!({"url":"https://u:p@host","login_method":"local"})
    )
    .is_err());
}

#[tokio::test]
async fn local_auth_cookies_rotate_me_logout_and_never_expose_upstream_tokens() {
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(200, user()).header("Set-Cookie", "connect.sid=rotated; Path=/"),
        Reply::json(204, Value::Null),
    ]);
    let mut client = mock.client();
    assert_eq!(client.me().await.unwrap_err(), Error::NotAuthenticated);
    let user = client
        .login_local("user@example.test", "password-secret")
        .await
        .unwrap();
    assert!(user.can_manage_requests());
    assert!(!format!("{user:?} {client:?}").contains("upstream-secret"));
    assert!(!format!("{client:?}").contains("private-session"));
    assert!(client.has_session());
    client.me().await.unwrap();
    client.logout().await.unwrap();
    assert!(!client.has_session());
    assert_eq!(client.me().await.unwrap_err(), Error::NotAuthenticated);
    let calls = mock.finish();
    assert_eq!(calls[0].method, "POST");
    assert_eq!(calls[0].target, "/seerr/api/v1/auth/local");
    assert_eq!(
        calls[0].body,
        json!({"email":"user@example.test","password":"password-secret"})
    );
    assert!(calls[0].header("cookie").is_none());
    assert_eq!(calls[1].header("cookie"), Some(COOKIE));
    assert_eq!(calls[2].target, "/seerr/api/v1/auth/logout");
    assert_eq!(calls[2].header("cookie"), Some("connect.sid=rotated"));
    assert!(calls
        .iter()
        .all(|c| c.header("x-api-key").is_none() && c.header("authorization").is_none()));
}

#[tokio::test]
async fn jellyfin_login_is_username_password_and_invalid_session_clears_cookie() {
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(401, json!({"error":"password-secret https://private/"})),
        Reply::json(401, json!({"error":"bad password"})),
    ]);
    let mut client = mock.client();
    client
        .login_jellyfin("jf-user", "jf-password")
        .await
        .unwrap();
    let error = client.me().await.unwrap_err();
    assert_eq!(error, Error::Unauthorized);
    assert!(!format!("{error:?} {error}").contains("password-secret"));
    assert!(!client.has_session());
    assert_eq!(
        client.login_local("bad", "bad").await.unwrap_err(),
        Error::Unauthorized
    );
    assert!(!client.has_session());
    let calls = mock.finish();
    assert_eq!(calls[0].target, "/seerr/api/v1/auth/jellyfin");
    assert_eq!(
        calls[0].body,
        json!({"username":"jf-user", "password":"jf-password"})
    );
    assert!(calls[2].header("cookie").is_none());
}

#[tokio::test]
async fn failed_logout_forgets_cookie_and_forbidden_does_not_log_out() {
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(403, json!({"error":"secret"})),
        Reply::json(502, json!({"error":"secret"})),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    assert_eq!(
        client.torrent_downloads().await.unwrap_err(),
        Error::Forbidden
    );
    assert!(client.has_session());
    assert_eq!(client.logout().await.unwrap_err(), Error::Http(502));
    assert!(!client.has_session());
    mock.finish();
}

#[tokio::test]
async fn rejects_cross_origin_and_same_origin_redirects_without_forwarding_credentials() {
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    let location = format!("http://{}/steal", target.local_addr().unwrap());
    let mock = Mock::new(vec![
        Reply::json(307, Value::Null)
            .header("Location", &location)
            .header("Set-Cookie", "connect.sid=malicious; Path=/"),
        login_reply(),
        Reply::json(302, Value::Null).header("Location", &location),
        Reply::json(302, Value::Null).header("Location", "/seerr/other"),
    ]);
    let mut client = mock.client();
    assert_eq!(
        client
            .login_local("secret-user", "secret-password")
            .await
            .unwrap_err(),
        Error::RedirectDenied
    );
    assert!(!client.has_session());
    client.login_local("a", "b").await.unwrap();
    assert_eq!(client.me().await.unwrap_err(), Error::RedirectDenied);
    assert_eq!(client.me().await.unwrap_err(), Error::RedirectDenied);
    assert!(
        target.accept().is_err(),
        "redirect destination received a request"
    );
    let calls = mock.finish();
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[2].header("cookie"), Some(COOKIE));
}

#[tokio::test]
async fn cookie_scope_secure_expiry_and_domain_are_respected() {
    for cookie in [
        "connect.sid=secret; Secure; Path=/",
        "connect.sid=secret; Path=/unrelated",
        "connect.sid=secret; Path=/; Domain=evil.test",
        "connect.sid=secret; Path=/; Max-Age=0",
        "connect.sid=secret; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
        "other-cookie=secret; Path=/",
    ] {
        let mock = Mock::new(vec![Reply::json(200, user()).header("Set-Cookie", cookie)]);
        let mut client = mock.client();
        assert_eq!(
            client.login_local("a", "b").await.unwrap_err(),
            Error::InvalidResponse,
            "{cookie}"
        );
        assert!(!client.has_session());
        mock.finish();
    }
    let mock = Mock::new(vec![
        Reply::json(200, user()).header("Set-Cookie", "connect.sid=secret; Path=/seerr"),
        Reply::json(200, user()).header("Set-Cookie", "connect.sid=; Path=/; Max-Age=0"),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    client.me().await.unwrap();
    assert!(!client.has_session());
    mock.finish();
}

#[tokio::test]
async fn quick_connect_uses_seerr_endpoints_and_redacts_capabilities() {
    let mock = Mock::new(vec![
        Reply::json(200, json!({"code":"123456","secret":"abcdef0123456789"})),
        Reply::json(200, json!({"authenticated":true})),
        login_reply(),
    ]);
    let mut client = mock.client();
    let qc = client.quick_connect_initiate().await.unwrap();
    assert_eq!(qc.code, "123456");
    assert!(!format!("{qc:?}").contains("123456"));
    assert!(!format!("{:?}", qc.secret).contains("abcdef"));
    assert!(
        client
            .quick_connect_check(&qc.secret)
            .await
            .unwrap()
            .authenticated
    );
    client.quick_connect_authenticate(&qc.secret).await.unwrap();
    let calls = mock.finish();
    assert_eq!(query(&calls[1])["secret"], "abcdef0123456789");
    assert_eq!(calls[2].body, json!({"secret":"abcdef0123456789"}));
    assert!(calls.iter().all(|c| c.header("cookie").is_none()));
}

#[test]
fn discovery_feed_selection_respects_the_media_toggle() {
    for (kind, popular, upcoming) in [
        (
            MediaType::Movie,
            DiscoverFeed::PopularMovies,
            DiscoverFeed::UpcomingMovies,
        ),
        (
            MediaType::Tv,
            DiscoverFeed::PopularTv,
            DiscoverFeed::UpcomingTv,
        ),
    ] {
        assert_eq!(DiscoverFeed::for_selection(0, kind), DiscoverFeed::Trending);
        assert_eq!(DiscoverFeed::for_selection(1, kind), popular);
        assert_eq!(DiscoverFeed::for_selection(2, kind), upcoming);
        assert_eq!(
            DiscoverFeed::for_selection(99, kind),
            DiscoverFeed::Trending
        );
        assert!(!ResultType::Person.matches(kind));
        assert!(!ResultType::Other.matches(kind));
    }
}
#[tokio::test]
async fn discovery_scopes_feeds_and_search_without_faking_pagination_totals() {
    let page = json!({"page":2,"totalPages":9,"totalResults":180,"results":[{"id":1,"mediaType":"movie","title":"Movie"},{"id":2,"mediaType":"tv","name":"TV"},{"id":3,"mediaType":"person","name":"Person"}]});
    let mut replies = vec![login_reply()];
    for _ in 0..8 {
        replies.push(Reply::json(200, page.clone()));
    }
    replies.push(Reply::json(200,json!({"page":2,"totalPages":9,"totalResults":180,"results":[{"id":2,"mediaType":"tv","name":"TV only"}]})));
    let mock = Mock::new(replies);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    assert_eq!(
        client
            .discover_filtered(DiscoverFeed::Trending, MediaType::Movie, "", 0)
            .await
            .unwrap_err(),
        Error::InvalidSelection
    );
    for kind in [MediaType::Movie, MediaType::Tv] {
        for selection in 0..3 {
            let page = client
                .discover_filtered(DiscoverFeed::for_selection(selection, kind), kind, "", 2)
                .await
                .unwrap();
            assert_eq!(page.results.len(), 1);
            assert!(page.results[0].media_type.matches(kind));
            assert_eq!(page.total_pages, 9);
        }
        let page = client
            .discover_filtered(DiscoverFeed::Trending, kind, "  title & /?  ", 2)
            .await
            .unwrap();
        assert_eq!(page.results.len(), 1);
        assert!(page.results[0].media_type.matches(kind));
        assert_eq!(page.total_results, 180);
    }
    let empty = client
        .discover_filtered(DiscoverFeed::Trending, MediaType::Movie, "empty", 2)
        .await
        .unwrap();
    assert!(empty.results.is_empty());
    assert_eq!(empty.page, 2);
    assert_eq!(empty.total_pages, 9);
    let calls = mock.finish();
    for (offset, kind) in [(1, "movie"), (5, "tv")] {
        for index in 0..3 {
            let call = &calls[offset + index];
            let params = query(call);
            if index == 0 {
                assert_eq!(params["mediaType"], kind);
            } else {
                assert!(!params.contains_key("mediaType"));
            }
            assert_eq!(params["page"], "2");
            let path = DiscoverFeed::for_selection(
                index as u32,
                if kind == "movie" {
                    MediaType::Movie
                } else {
                    MediaType::Tv
                },
            )
            .path();
            assert!(call.target.starts_with(&format!("/seerr/api/v1/{path}?")));
        }
        let search = &calls[offset + 3];
        assert!(search.target.starts_with("/seerr/api/v1/search?"));
        assert_eq!(query(search)["query"], "title & /?");
    }
}

#[tokio::test]
async fn discovery_endpoints_search_pagination_and_typed_details() {
    let page = json!({"page":2,"totalPages":3,"totalResults":41,"results":[{"id":1,"mediaType":"movie","title":"Movie"},{"id":2,"mediaType":"tv","name":"TV"},{"id":3,"mediaType":"person","name":"Person"}]});
    let info = json!({"id":8,"tmdbId":2,"status":4,"status4k":1,"seasons":[{"seasonNumber":1,"status":5,"status4k":1}],"requests":[{"id":4,"status":2,"seasons":[{"seasonNumber":1,"status":2}]}]});
    let mut replies = vec![login_reply()];
    for _ in 0..6 {
        replies.push(Reply::json(200, page.clone()));
    }
    replies.push(Reply::json(
        200,
        json!({"id":1,"title":"Movie","mediaInfo":null}),
    ));
    replies.push(Reply::json(200, json!({"id":2,"name":"TV","seasons":[{"id":11,"seasonNumber":0,"episodeCount":2}],"mediaInfo":info})));
    replies.push(Reply::json(
        200,
        json!({"id":11,"seasonNumber":0,"episodes":[{"id":22,"seasonNumber":0,"episodeNumber":1}]}),
    ));
    let mock = Mock::new(replies);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    assert_eq!(
        client
            .discover(DiscoverFeed::Trending, 0, None)
            .await
            .unwrap_err(),
        Error::InvalidSelection
    );
    for feed in [
        DiscoverFeed::Trending,
        DiscoverFeed::PopularMovies,
        DiscoverFeed::PopularTv,
        DiscoverFeed::UpcomingMovies,
        DiscoverFeed::UpcomingTv,
    ] {
        let result = client.discover(feed, 2, Some("en-US")).await.unwrap();
        assert_eq!(result.total_pages, 3);
        assert_eq!(result.results.len(), 3);
        assert!(matches!(result.results[2].media_type, ResultType::Person));
    }
    client.search("TV & movie/?", 2, None).await.unwrap();
    assert!(client.movie_details(1).await.unwrap().media_info.is_none());
    let tv = client.tv_details(2).await.unwrap();
    assert_eq!(tv.seasons[0].season_number, 0);
    let media = tv.media_info.unwrap();
    assert_eq!(media.status, MediaStatus::PARTIALLY_AVAILABLE);
    assert_eq!(media.requests[0].status, RequestStatus::APPROVED);
    assert_eq!(media.seasons[0].status, MediaStatus::AVAILABLE);
    assert_eq!(
        client.tv_season(2, 0).await.unwrap().episodes[0].episode_number,
        1
    );
    let calls = mock.finish();
    for (call, path) in calls[1..6].iter().zip([
        "discover/trending",
        "discover/movies",
        "discover/tv",
        "discover/movies/upcoming",
        "discover/tv/upcoming",
    ]) {
        assert!(call.target.starts_with(&format!("/seerr/api/v1/{path}?")));
        assert_eq!(query(call)["page"], "2");
        assert_eq!(query(call)["language"], "en-US");
    }
    assert_eq!(query(&calls[6])["query"], "TV & movie/?");
    assert_eq!(calls[9].target, "/seerr/api/v1/tv/2/season/0");
}

#[tokio::test]
async fn request_payload_mapping_explicit_seasons_listing_and_statuses() {
    let request = json!({"id":4,"status":1,"is4k":false,"seasons":[{"seasonNumber":2,"status":1}],"media":{"id":8,"tmdbId":2,"mediaType":"tv","status":2}});
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(201, request.clone()),
        Reply::json(201, request.clone()),
        Reply::json(
            200,
            json!({"pageInfo":{"pages":1,"pageSize":20,"results":1,"page":1},"results":[request]}),
        ),
        Reply::json(200, json!({"id":4,"status":99})),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    for seasons in [vec![], vec![1, 1]] {
        assert_eq!(
            client
                .create_request(&RequestSelection::Tv {
                    tmdb_id: 2,
                    seasons,
                    is4k: false
                })
                .await
                .unwrap_err(),
            Error::InvalidSelection
        );
    }
    client
        .create_request(&RequestSelection::Movie {
            tmdb_id: 1,
            is4k: true,
        })
        .await
        .unwrap();
    assert_eq!(
        client
            .create_request(&RequestSelection::Tv {
                tmdb_id: 2,
                seasons: vec![0, 2],
                is4k: false
            })
            .await
            .unwrap()
            .status,
        RequestStatus::PENDING
    );
    let options = RequestListOptions {
        skip: 20,
        filter: RequestFilter::Pending,
        requested_by: Some(7),
        ..Default::default()
    };
    assert_eq!(client.requests(&options).await.unwrap().results.len(), 1);
    assert_eq!(
        client.request_status(4).await.unwrap().status,
        RequestStatus(99)
    );
    let calls = mock.finish();
    assert_eq!(
        calls[1].body,
        json!({"mediaType":"movie","mediaId":1,"is4k":true})
    );
    assert_eq!(
        calls[2].body,
        json!({"mediaType":"tv","mediaId":2,"is4k":false,"seasons":[0,2]})
    );
    assert_eq!(query(&calls[3])["requestedBy"], "7");
    assert_eq!(query(&calls[3])["filter"], "pending");
    assert_eq!(query(&calls[3])["skip"], "20");
    assert_eq!(calls[4].target, "/seerr/api/v1/request/4");
    assert_eq!(MediaStatus::BLOCKLISTED.0, 6);
    assert_eq!(MediaStatus::DELETED.0, 7);
}

#[tokio::test]
async fn source_picker_diagnostics_token_and_confirmation_contract() {
    let token = "a".repeat(48);
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(
            200,
            json!({"serverName":"Sonarr","is4k":true,"seriesMonitored":true,"queueTruncated":true,"historyTruncated":false,"episodes":[{"id":66,"seasonNumber":2,"episodeNumber":1,"title":"Episode","hasFile":false,"monitored":true,"state":"queued","queue":[{"title":"Release","status":"downloading","trackedStatus":"ok","trackedState":"downloading","progress":25.0,"messages":[]}],"history":[{"date":"2026-01-01","eventType":"grabbed","title":"Release"}]}]}),
        ),
        Reply::json(
            200,
            json!({"expiresAt":"2026-01-01T00:05:00Z","truncated":false,"releases":[{"token":token,"title":"Release","indexer":"Indexer","quality":"1080p","size":123456789,"seeders":null,"protocol":"torrent","fullSeason":false,"episodeNumbers":[1],"rejections":["quality"],"rejected":true,"duplicate":true,"downloadUrl":"https://private/secret"}]}),
        ),
        Reply::json(409, json!({"message":"secret token URL"})),
        Reply::json(409, json!({"message":"confirmation required"})),
        Reply::json(409, json!({"message":"confirmation required"})),
        Reply::json(202, json!({"message":"Release sent to Sonarr."})),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    let diagnostics = client.tv_downloads(2, true, Some(4)).await.unwrap();
    assert!(diagnostics.queue_truncated);
    assert_eq!(diagnostics.episodes[0].queue[0].progress, Some(25.0));
    let selection = DownloadSelection {
        is4k: true,
        request_id: Some(4),
        season_number: 2,
        episode_id: Some(66),
    };
    let search = client.search_tv_releases(2, &selection).await.unwrap();
    let release = &search.releases[0];
    assert!(release.rejected && release.duplicate);
    assert!(!format!("{search:?}").contains(&token));
    assert!(!format!("{search:?}").contains("https://private"));
    let token = release.token.as_ref().unwrap();
    for (rejected, duplicate) in [(false, false), (true, false), (false, true)] {
        assert_eq!(
            client
                .grab_tv_release(2, &selection, token, rejected, duplicate, true)
                .await
                .unwrap_err(),
            Error::Http(409)
        );
    }
    client
        .grab_tv_release(2, &selection, token, true, true, true)
        .await
        .unwrap();
    let calls = mock.finish();
    assert_eq!(query(&calls[1])["requestId"], "4");
    assert_eq!(query(&calls[1])["is4k"], "true");
    let base = json!({"is4k":true,"requestId":4,"seasonNumber":2,"episodeId":66});
    assert_eq!(calls[2].body, base);
    for (call, (rejected, duplicate)) in
        calls[3..]
            .iter()
            .zip([(false, false), (true, false), (false, true), (true, true)])
    {
        let mut expected = base.clone();
        expected["token"] = json!("a".repeat(48));
        expected["confirmRejected"] = json!(rejected);
        expected["confirmDuplicate"] = json!(duplicate);
        expected["confirmReplacement"] = json!(true);
        assert_eq!(call.body, expected);
        assert_eq!(call.target, "/seerr/api/v1/tv/2/downloads/grab");
    }
    assert_eq!(
        serde_json::to_value(DownloadSelection {
            is4k: false,
            request_id: None,
            season_number: 0,
            episode_id: None
        })
        .unwrap(),
        json!({"is4k":false,"seasonNumber":0})
    );
}

#[tokio::test]
async fn movie_sources_keep_request_identity_and_show_partial_replacement() {
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(
            200,
            json!({"expiresAt":"2099-01-01T00:00:00Z","truncated":false,"releases":[{"token":"a".repeat(48),"title":"Movie","indexer":"Indexer","quality":"1080p","size":1,"seeders":null,"protocol":"torrent","fullSeason":false,"episodeNumbers":[],"rejections":[],"rejected":false,"duplicate":true,"replacements":["Old movie"]}]}),
        ),
        Reply::json(
            202,
            json!({"message":"Replacement incomplete; files kept.","partial":true,"replaced":0}),
        ),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    let selection = MovieDownloadSelection {
        is4k: true,
        request_id: Some(4),
    };
    let search = client.search_movie_releases(2, &selection).await.unwrap();
    assert_eq!(search.releases[0].replacements, vec!["Old movie"]);
    let token = search.releases[0].token.as_ref().unwrap();
    let result = client
        .grab_movie_release(2, &selection, token, false, true, true)
        .await
        .unwrap();
    assert!(result.partial);
    assert!(result.message.contains("files kept"));
    let calls = mock.finish();
    assert_eq!(calls[1].target, "/seerr/api/v1/movie/2/downloads/search");
    assert_eq!(calls[1].body, json!({"is4k":true,"requestId":4}));
    assert_eq!(calls[2].target, "/seerr/api/v1/movie/2/downloads/grab");
    assert_eq!(
        calls[2].body,
        json!({"is4k":true,"requestId":4,"token":"a".repeat(48),"confirmRejected":false,"confirmDuplicate":true,"confirmReplacement":true})
    );
}

#[tokio::test]
async fn partial_local_login_resolves_actual_permissions_before_loading_downloads() {
    for (permissions, missing) in [(0, true), (2, false), (16, true)] {
        let login = if missing {
            json!({"id":7,"email":"user@example.test"})
        } else {
            json!({"id":7,"email":"user@example.test","permissions":0})
        };
        let partial =
            Reply::json(200, login).header("Set-Cookie", &format!("{COOKIE}; Path=/; HttpOnly"));
        let allowed = permissions != 0;
        let mock = Mock::new(vec![
            partial,
            Reply::json(
                200,
                json!({"id":7,"username":"Full profile","permissions":permissions}),
            ),
            if allowed {
                Reply::json(200, json!({"torrents":[torrent(0.5)],"updatedAt":"now"}))
            } else {
                Reply::json(403, json!({"message":"denied"}))
            },
        ]);
        let mut client = mock.client();
        let user = client.login_local("a", "b").await.unwrap();
        assert_eq!(user.permissions, permissions);
        assert_eq!(user.can_manage_requests(), allowed);
        assert_eq!(user.username.as_deref(), Some("Full profile"));
        if allowed {
            assert_eq!(client.torrent_downloads().await.unwrap().torrents.len(), 1);
        } else {
            assert_eq!(
                client.torrent_downloads().await.unwrap_err(),
                Error::Forbidden
            );
        }
        let calls = mock.finish();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].target, "/seerr/api/v1/auth/local");
        assert_eq!(calls[1].method, "GET");
        assert_eq!(calls[1].target, "/seerr/api/v1/auth/me");
        assert_eq!(calls[1].header("cookie"), Some(COOKIE));
        assert_eq!(calls[2].target, "/seerr/api/v1/downloads/torrents");
        assert!(calls[1].body.is_null());
    }
}
#[tokio::test]
async fn failed_or_mismatched_profile_resolution_forgets_the_partial_session() {
    for response in [
        Reply::json(503, json!({})),
        Reply::json(401, json!({})),
        Reply::json(200, json!({"id":99,"permissions":2})),
    ] {
        let partial = Reply::json(200, json!({"id":7}))
            .header("Set-Cookie", &format!("{COOKIE}; Path=/; HttpOnly"));
        let mock = Mock::new(vec![partial, response]);
        let mut client = mock.client();
        assert!(client.login_local("a", "b").await.is_err());
        assert!(!client.has_session());
        assert_eq!(mock.finish().len(), 2);
    }
}

#[tokio::test]
async fn request_tabs_use_server_buckets_and_receive_metadata_in_one_list_call() {
    let page = json!({"pageInfo":{"pages":2,"pageSize":20,"results":21,"page":1},"results":[{"id":17,"title":"A real title","posterPath":"/poster.jpg","overview":"Summary","releaseDate":"2021-03-01","profileName":"HD-1080p","status":2,"media":{"id":1,"tmdbId":2,"mediaType":"tv","status":3}}]});
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(200, page.clone()),
        Reply::json(200, page),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    for filter in [RequestFilter::Finished, RequestFilter::Unfinished] {
        let page = client
            .requests(&RequestListOptions {
                filter,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.page_info.results, 21);
        assert_eq!(page.results[0].title.as_deref(), Some("A real title"));
        assert_eq!(page.results[0].profile_name.as_deref(), Some("HD-1080p"));
        assert_eq!(page.results[0].poster_path.as_deref(), Some("/poster.jpg"));
    }
    let calls = mock.finish();
    assert_eq!(calls.len(), 3);
    assert_eq!(query(&calls[1])["filter"], "finished");
    assert_eq!(query(&calls[2])["filter"], "unfinished");
    assert!(calls[1..].iter().all(|r| r.method == "GET"));
}

#[tokio::test]
async fn removal_contract_previews_before_explicit_commit_and_recovers_by_receipt() {
    let receipt = json!({"operationId":"operation-1","title":"Selected show","status":"complete","steps":[{"name":"Unmonitor","status":"complete"},{"name":"Delete files","status":"complete"}]});
    let capability = "a".repeat(64);
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(
            200,
            json!({"target":{"mediaType":"tv","tmdbId":12,"title":"Selected show"},"episodes":[{"seasonNumber":1,"episodeNumber":2,"title":"Second episode","hasFile":true}],"warnings":[]}),
        ),
        Reply::json(
            200,
            json!({"title":"Selected show","scopeLabel":"S1 E2","expiresAt":"2099-01-01T00:00:00Z","token":capability,"canCommit":true,"effects":["Unmonitor and remove exact selected files"],"blockers":[],"warnings":[]}),
        ),
        Reply::json(200, receipt.clone()),
        Reply::json(200, json!({"operations":[receipt.clone()]})),
        Reply::json(200, receipt.clone()),
        Reply::json(200, receipt),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    let target = RemovalTarget::Jellyfin {
        jellyfin_item_id: "a".repeat(32),
        jellyfin_server_id: "b".repeat(32),
    };
    let options = client.removal_options(&target).await.unwrap();
    assert_eq!(options.target.title, "Selected show");
    let selection = RemovalSelection {
        target,
        scope: RemovalScope::Episodes {
            episodes: vec![RemovalEpisodeKey {
                season_number: 1,
                episode_number: 2,
            }],
        },
    };
    let mut preview = client.removal_preview(&selection).await.unwrap();
    assert!(preview.confirmable());
    let token = preview.token.take().unwrap();
    let receipt = client.removal_commit(&token).await.unwrap();
    assert_eq!(receipt.status, RemovalStatus::Complete);
    assert_eq!(client.removal_history().await.unwrap().operations.len(), 1);
    client.removal_receipt(&receipt.operation_id).await.unwrap();
    client.removal_retry(&receipt.operation_id).await.unwrap();
    let calls = mock.finish();
    assert_eq!(calls.len(), 7);
    assert_eq!(calls[1].target, "/seerr/api/v1/removals/options");
    assert_eq!(
        calls[1].body,
        json!({"jellyfinItemId":"a".repeat(32),"jellyfinServerId":"b".repeat(32)})
    );
    assert_eq!(calls[2].target, "/seerr/api/v1/removals/preview");
    assert_eq!(calls[2].body["scope"], "episodes");
    assert_eq!(
        calls[2].body["episodes"],
        json!([{"seasonNumber":1,"episodeNumber":2}])
    );
    assert_eq!(calls[3].body, json!({"token":capability,"confirmed":true}));
    assert_eq!(calls[4].method, "GET");
    assert_eq!(calls[5].target, "/seerr/api/v1/removals/operation-1");
    assert_eq!(calls[6].target, "/seerr/api/v1/removals/operation-1/retry");
    assert_eq!(calls[6].body, json!({"confirmed":true}));
}
#[tokio::test]
async fn removal_never_retries_mutation_or_accepts_untrusted_identifiers() {
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(503, json!({"message":"unknown outcome"})),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    assert_eq!(
        client
            .removal_options(&RemovalTarget::Torrent {
                torrent_hash: "all".into()
            })
            .await
            .unwrap_err(),
        Error::InvalidSelection
    );
    assert_eq!(
        client.removal_receipt("../commit").await.unwrap_err(),
        Error::InvalidSelection
    );
    let token: RemovalToken = serde_json::from_value(json!("a".repeat(64))).unwrap();
    assert!(client.removal_commit(&token).await.is_err());
    let calls = mock.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].target, "/seerr/api/v1/removals/commit");
}

#[tokio::test]
async fn cancel_download_sends_only_the_confirmed_exact_hash_and_never_retries() {
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(200, json!({"cancelled":true,"filesDeleted":false})),
        Reply::json(200, json!({"cancelled":true,"filesDeleted":true})),
        Reply::json(503, json!({"message":"unknown outcome"})),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    assert_eq!(
        client.cancel_download("all", true).await.unwrap_err(),
        Error::InvalidSelection
    );
    let hash = "a".repeat(40);
    client.cancel_download(&hash, false).await.unwrap();
    client.cancel_download(&hash, true).await.unwrap();
    assert!(client.cancel_download(&hash, true).await.is_err());
    let calls = mock.finish();
    assert_eq!(calls.len(), 4);
    for call in &calls[1..] {
        assert_eq!(call.method, "POST");
        assert_eq!(
            call.target,
            format!("/seerr/api/v1/downloads/torrents/{hash}/cancel")
        );
    }
    assert_eq!(calls[1].body, json!({"confirmed":true,"deleteFiles":false}));
    assert_eq!(calls[2].body, json!({"confirmed":true,"deleteFiles":true}));
    assert_eq!(calls[3].body, json!({"confirmed":true,"deleteFiles":true}));
}
#[tokio::test]
async fn download_file_deletion_requires_the_server_to_confirm_the_requested_scope() {
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(200, json!({"cancelled":true,"filesDeleted":false})),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    assert_eq!(
        client
            .cancel_download(&"a".repeat(40), true)
            .await
            .unwrap_err(),
        Error::InvalidResponse
    );
    assert_eq!(mock.finish().len(), 2);
}
fn torrent(progress: f64) -> Value {
    json!({"hash":"abc123","name":"Torrent","state":"downloading","progress":progress,"size":5000000000_u64,"downloaded":2500000000_u64,"downloadSpeed":12000,"uploadSpeed":500,"eta":null,"seeds":3,"peers":4,"category":"tv"})
}
#[tokio::test]
async fn torrent_download_contract_units_nullable_eta_and_invalid_progress() {
    let mut finished = torrent(1.0);
    finished["eta"] = json!(0);
    finished["error"] = json!("paused by server");
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(
            200,
            json!({"torrents":[torrent(0.5), finished],"updatedAt":"2026-01-01T00:00:00Z"}),
        ),
        Reply::json(200, json!({"torrents":[torrent(50.0)],"updatedAt":"now"})),
    ]);
    let mut client = mock.client();
    client.login_local("a", "b").await.unwrap();
    let result = client.torrent_downloads().await.unwrap();
    let torrent = &result.torrents[0];
    assert_eq!(torrent.progress, 0.5);
    assert_eq!(torrent.size, 5_000_000_000);
    assert_eq!(torrent.downloaded, 2_500_000_000);
    assert_eq!(torrent.download_speed, 12000);
    assert_eq!(torrent.upload_speed, 500);
    assert_eq!(torrent.eta, None);
    assert_eq!(torrent.error, None);
    assert_eq!(result.torrents[1].eta, Some(0));
    assert_eq!(
        result.torrents[1].error.as_deref(),
        Some("paused by server")
    );
    assert_eq!(
        client.torrent_downloads().await.unwrap_err(),
        Error::InvalidResponse
    );
    let calls = mock.finish();
    assert_eq!(calls[1].target, "/seerr/api/v1/downloads/torrents");
}

#[tokio::test]
async fn timeout_and_malformed_response_are_sanitized() {
    let mut delayed = Reply::json(200, user());
    delayed.delay = Duration::from_millis(150);
    let mock = Mock::new(vec![delayed]);
    let mut client = mock.client();
    client.client = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_millis(30))
        .build()
        .unwrap();
    let error = client
        .login_local("secret-user", "secret-password")
        .await
        .unwrap_err();
    assert_eq!(error, Error::Timeout);
    assert!(!format!("{error:?} {error}").contains("secret"));
    assert!(!client.has_session());
    mock.finish();
    let mock = Mock::new(vec![Reply {
        body: "secret malformed json https://private/".into(),
        ..login_reply()
    }]);
    let mut client = mock.client();
    assert_eq!(
        client.login_local("a", "b").await.unwrap_err(),
        Error::InvalidResponse
    );
    assert!(!client.has_session());
    mock.finish();
}

#[derive(Default)]
struct FakeKeyring {
    entries: Mutex<std::collections::HashMap<String, String>>,
    fail_lookup: std::sync::atomic::AtomicBool,
    fail_store: std::sync::atomic::AtomicBool,
    fail_clear: std::sync::atomic::AtomicBool,
    delay_store: std::sync::atomic::AtomicBool,
    store_started: std::sync::atomic::AtomicBool,
    clear_calls: std::sync::atomic::AtomicU64,
}
impl FakeKeyring {
    fn key(config: &ConnectionConfig) -> String {
        format!(
            "{:?}",
            (config.url(), config.login_method, &config.username)
        )
    }
}
impl super::keyring::CredentialStore for FakeKeyring {
    fn lookup(&self, config: &ConnectionConfig) -> Result<Option<String>, ()> {
        if self.fail_lookup.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(());
        }
        Ok(self
            .entries
            .lock()
            .unwrap()
            .get(&Self::key(config))
            .cloned())
    }
    fn store(&self, config: &ConnectionConfig, password: &str) -> Result<(), ()> {
        self.store_started
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if self.delay_store.load(std::sync::atomic::Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(200));
        }
        if self.fail_store.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(());
        }
        self.entries
            .lock()
            .unwrap()
            .insert(Self::key(config), password.into());
        Ok(())
    }
    fn clear(&self, config: &ConnectionConfig) -> Result<(), ()> {
        if self.fail_clear.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(());
        }
        self.entries.lock().unwrap().remove(&Self::key(config));
        self.clear_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}
fn remembered_config(url: &str) -> ConnectionConfig {
    let mut config =
        ConnectionConfig::new(url, LoginMethod::Local, Some("user@example.test".into())).unwrap();
    config.remember_password = true;
    config
}
fn auth_job(operation: super::ui_service::Operation, account: u64) -> super::ui_service::Job {
    super::ui_service::Job {
        account,
        view: 0,
        operation,
    }
}

#[test]
fn remembered_password_restores_a_new_session_and_logout_forgets_it_even_offline() {
    use super::{
        keyring::CredentialStore,
        ui_service::{CredentialStatus, Data, Operation, Service},
    };
    let mock = Mock::new(vec![
        login_reply(),
        login_reply(),
        Reply::json(500, json!({})),
    ]);
    let config = remembered_config(&mock.url);
    let store = Arc::new(FakeKeyring::default());
    let (first, replies) = Service::with_keyring(store.clone());
    first
        .sender
        .send(auth_job(
            Operation::Login(config.clone(), "remembered-password".into()),
            0,
        ))
        .unwrap();
    assert!(matches!(
        replies.recv_timeout(Duration::from_secs(5)).unwrap().result,
        Ok(Data::Login(_, CredentialStatus::Saved))
    ));
    assert_eq!(
        store.lookup(&config).unwrap().as_deref(),
        Some("remembered-password")
    );
    drop(first);
    let (second, replies) = Service::with_keyring(store.clone());
    second
        .sender
        .send(auth_job(Operation::Restore(config.clone()), 0))
        .unwrap();
    assert!(matches!(
        replies.recv_timeout(Duration::from_secs(5)).unwrap().result,
        Ok(Data::Login(_, CredentialStatus::Saved))
    ));
    second.sender.send(auth_job(Operation::Logout, 0)).unwrap();
    assert!(matches!(
        replies.recv_timeout(Duration::from_secs(5)).unwrap().result,
        Ok(Data::Logout(Err(Error::Http(500)), false))
    ));
    assert!(store.lookup(&config).unwrap().is_none());
    let requests = mock.finish();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].body["password"], "remembered-password");
    let serialized = serde_json::to_string(&config).unwrap();
    assert!(serialized.contains("remember_me"));
    assert!(!serialized.contains("remembered-password"));
    let legacy: ConnectionConfig = serde_json::from_value(
        json!({"url":"https://host", "login_method":"local", "username":"user"}),
    )
    .unwrap();
    assert!(!legacy.remember_password);
}

#[test]
fn unsuccessful_login_never_saves_an_unverified_password() {
    use super::ui_service::{Operation, Service};
    let mock = Mock::new(vec![Reply::json(401, json!({}))]);
    let store = Arc::new(FakeKeyring::default());
    let (service, replies) = Service::with_keyring(store.clone());
    service
        .sender
        .send(auth_job(
            Operation::Login(remembered_config(&mock.url), "wrong-password".into()),
            0,
        ))
        .unwrap();
    assert!(matches!(
        replies.recv_timeout(Duration::from_secs(5)).unwrap().result,
        Err(Error::Unauthorized)
    ));
    assert!(store.entries.lock().unwrap().is_empty());
    assert!(!store
        .store_started
        .load(std::sync::atomic::Ordering::SeqCst));
    mock.finish();
}

#[test]
fn keyring_save_failure_is_reported_without_rejecting_a_valid_login() {
    use super::ui_service::{CredentialStatus, Data, Operation, Service};
    let mock = Mock::new(vec![login_reply()]);
    let store = Arc::new(FakeKeyring::default());
    store
        .fail_store
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let (service, replies) = Service::with_keyring(store.clone());
    service
        .sender
        .send(auth_job(
            Operation::Login(remembered_config(&mock.url), "password".into()),
            0,
        ))
        .unwrap();
    assert!(matches!(
        replies.recv_timeout(Duration::from_secs(5)).unwrap().result,
        Ok(Data::Login(_, CredentialStatus::SaveFailed))
    ));
    assert!(store.entries.lock().unwrap().is_empty());
    mock.finish();
}

#[test]
fn restore_with_missing_locked_or_disabled_keyring_never_attempts_a_login() {
    use super::ui_service::{Data, Operation, Service};
    for (locked, enabled) in [(false, true), (true, true), (true, false)] {
        let mock = Mock::new(vec![]);
        let store = Arc::new(FakeKeyring::default());
        store
            .fail_lookup
            .store(locked, std::sync::atomic::Ordering::SeqCst);
        let (service, replies) = Service::with_keyring(store);
        let mut config = remembered_config(&mock.url);
        config.remember_password = enabled;
        service
            .sender
            .send(auth_job(Operation::Restore(config), 0))
            .unwrap();
        let reply = replies.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            matches!(reply.result, Ok(Data::RestoreUnavailable(failed)) if failed == (locked && enabled))
        );
        assert!(mock.finish().is_empty());
    }
}

#[test]
fn unchecking_remember_me_clears_previous_credentials_and_discloses_clear_failures() {
    use super::{
        keyring::CredentialStore,
        ui_service::{CredentialStatus, Data, Operation, Service},
    };
    for fail in [false, true] {
        let mock = Mock::new(vec![login_reply()]);
        let mut config = remembered_config(&mock.url);
        config.remember_password = false;
        let store = Arc::new(FakeKeyring::default());
        store.store(&config, "old-password").unwrap();
        store
            .fail_clear
            .store(fail, std::sync::atomic::Ordering::SeqCst);
        let (service, replies) = Service::with_keyring(store.clone());
        service
            .sender
            .send(auth_job(
                Operation::Login(config.clone(), "new-password".into()),
                0,
            ))
            .unwrap();
        let result = replies.recv_timeout(Duration::from_secs(5)).unwrap().result;
        assert!(
            matches!(result, Ok(Data::Login(_, state)) if state == if fail { CredentialStatus::ClearFailed } else { CredentialStatus::MemoryOnly })
        );
        assert_eq!(store.lookup(&config).unwrap().is_some(), fail);
        mock.finish();
    }
}

#[test]
fn logout_during_keyring_save_cannot_leave_a_new_remembered_password() {
    use super::ui_service::{Data, Operation, Service};
    use std::sync::atomic::Ordering;
    let mock = Mock::new(vec![login_reply()]);
    let store = Arc::new(FakeKeyring::default());
    store.delay_store.store(true, Ordering::SeqCst);
    let (service, replies) = Service::with_keyring(store.clone());
    service
        .sender
        .send(auth_job(
            Operation::Login(remembered_config(&mock.url), "password".into()),
            0,
        ))
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(4);
    while !store.store_started.load(Ordering::SeqCst) {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    service.account.store(1, Ordering::SeqCst);
    service.sender.send(auth_job(Operation::Logout, 1)).unwrap();
    assert!(matches!(
        replies.recv_timeout(Duration::from_secs(5)).unwrap().result,
        Ok(Data::Logout(Ok(()), false))
    ));
    assert!(store.entries.lock().unwrap().is_empty());
    assert_eq!(mock.finish().len(), 1);
}

#[test]
fn service_disposal_during_save_still_revokes_the_new_credential() {
    use super::ui_service::{Operation, Service};
    use std::sync::atomic::Ordering;
    let mock = Mock::new(vec![login_reply()]);
    let store = Arc::new(FakeKeyring::default());
    store.delay_store.store(true, Ordering::SeqCst);
    let (service, _replies) = Service::with_keyring(store.clone());
    service
        .sender
        .send(auth_job(
            Operation::Login(remembered_config(&mock.url), "password".into()),
            0,
        ))
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(4);
    while !store.store_started.load(Ordering::SeqCst) {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    drop(service);
    while store.clear_calls.load(Ordering::SeqCst) == 0 {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    assert!(store.entries.lock().unwrap().is_empty());
    mock.finish();
}

#[test]
fn rejected_saved_password_is_forgotten_instead_of_repeatedly_retried() {
    use super::{
        keyring::CredentialStore,
        ui_service::{Operation, Service},
    };
    for status in [401, 403] {
        let mock = Mock::new(vec![Reply::json(status, json!({}))]);
        let config = remembered_config(&mock.url);
        let store = Arc::new(FakeKeyring::default());
        store.store(&config, "old-password").unwrap();
        let (service, replies) = Service::with_keyring(store.clone());
        service
            .sender
            .send(auth_job(Operation::Restore(config.clone()), 0))
            .unwrap();
        assert!(matches!(
            replies.recv_timeout(Duration::from_secs(5)).unwrap().result,
            Err(Error::Unauthorized | Error::Forbidden)
        ));
        assert!(store.lookup(&config).unwrap().is_none());
        assert_eq!(mock.finish().len(), 1);
    }
}

#[test]
fn ui_service_serializes_session_operations_and_skips_stale_jobs() {
    use super::ui_service::{Data, Job, Operation, Service};
    use std::sync::atomic::Ordering;
    let mock = Mock::new(vec![
        login_reply(),
        Reply::json(403, json!({})),
        Reply::json(200, json!({})),
    ]);
    let (service, replies) = Service::with_keyring(Arc::new(FakeKeyring::default()));
    service.account.store(1, Ordering::Relaxed);
    service.view.store(2, Ordering::Relaxed);
    let config = ConnectionConfig::new(
        &mock.url,
        LoginMethod::Local,
        Some("user@example.test".into()),
    )
    .unwrap();
    let submit = |account, view, operation| {
        service
            .sender
            .send(Job {
                account,
                view,
                operation,
            })
            .unwrap()
    };
    submit(
        0,
        2,
        Operation::Login(config.clone(), "stale-password".into()),
    );
    submit(1, 1, Operation::Torrents);
    submit(1, 2, Operation::Login(config, "password".into()));
    submit(1, 2, Operation::Torrents);
    // Auth lifecycle commands run even if navigation has changed.
    submit(1, 1, Operation::Logout);
    submit(1, 2, Operation::Torrents);
    let login = replies.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(login.auth);
    assert!(matches!(login.result, Ok(Data::Login(..))));
    assert!(matches!(
        replies.recv_timeout(Duration::from_secs(5)).unwrap().result,
        Err(Error::Forbidden)
    ));
    let logout = replies.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(logout.auth);
    assert!(matches!(logout.result, Ok(Data::Logout(Ok(()), false))));
    assert!(matches!(
        replies.recv_timeout(Duration::from_secs(5)).unwrap().result,
        Err(Error::NotAuthenticated)
    ));
    let captured = mock.finish();
    assert_eq!(captured.len(), 3);
    assert_eq!(captured[0].body["password"], "password");
    assert!(captured[1].target.ends_with("/downloads/torrents"));
    assert!(captured[2].target.ends_with("/auth/logout"));
}
