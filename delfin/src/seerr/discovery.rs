//! A same-host suggestion and an unauthenticated, bounded Seerr identity probe.
use super::{ConnectionConfig, LoginMethod};
use reqwest::Url;
use std::time::Duration;

/// Port 5055 is Seerr's normal HTTP listener. Preserve the Jellyfin scheme:
/// never silently downgrade an HTTPS connection to plaintext for credentials.
/// Reverse proxies may need a manual URL; this is only an editable suggestion.
pub(super) fn suggest_seerr_url(jellyfin: &Url) -> Option<String> {
    if !matches!(jellyfin.scheme(), "http" | "https")
        || jellyfin.host_str().is_none()
        || !jellyfin.username().is_empty()
        || jellyfin.password().is_some()
    {
        return None;
    }
    let mut url = jellyfin.clone();
    url.set_port(Some(5055)).ok()?;
    url.set_path("/");
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

/// Do not reuse any Jellyfin or Seerr authentication, follow redirects, save a
/// discovered preference, or automatically log in as a side effect of probing.
pub(super) async fn probe_seerr_url(url: &str) -> bool {
    async fn probe(url: &str) -> Option<()> {
        let config = ConnectionConfig::new(url, LoginMethod::Local, None).ok()?;
        let endpoint = Url::parse(config.url()).ok()?.join("api/v1/status").ok()?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(3))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .ok()?;
        let mut response = client.get(endpoint).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            if body.len() + chunk.len() > 16 * 1024 {
                return None;
            }
            body.extend_from_slice(&chunk);
        }
        let status: serde_json::Value = serde_json::from_slice(&body).ok()?;
        if status.get("version")?.as_str()?.trim().is_empty()
            || status.get("commitTag")?.as_str()?.trim().is_empty()
        {
            return None;
        }
        Some(())
    }
    probe(url).await.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suggests_same_host_default_port_without_auth_query_or_jellyfin_path() {
        for (source, expected) in [
            ("http://192.0.2.37:8096/", "http://192.0.2.37:5055/"),
            (
                "https://media.example:8920/jellyfin/?api_key=secret#x",
                "https://media.example:5055/",
            ),
            ("http://[::1]:8096/jellyfin", "http://[::1]:5055/"),
            ("http://media.local/", "http://media.local:5055/"),
        ] {
            assert_eq!(
                suggest_seerr_url(&Url::parse(source).unwrap()).as_deref(),
                Some(expected)
            );
        }
        for source in ["file:///tmp/server", "http://user:password@host:8096/"] {
            assert!(suggest_seerr_url(&Url::parse(source).unwrap()).is_none());
        }
    }
}
