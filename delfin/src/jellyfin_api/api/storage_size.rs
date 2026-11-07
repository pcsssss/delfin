//! Server-reported media file sizes, not filesystem block usage or sidecar files.
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::{ensure, Context, Result};
use jellyfin_api::types::{BaseItemDto, BaseItemKind};
use serde::Deserialize;
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

use crate::jellyfin_api::api_client::ApiClient;

const PAGE_SIZE: usize = 200;
const CACHE_TTL: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub(crate) struct StorageSizeState {
    cache: Mutex<HashMap<Uuid, (Instant, Option<u64>)>>,
    requests: Semaphore,
}

impl Default for StorageSizeState {
    fn default() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            requests: Semaphore::new(4),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SizeItem {
    id: Option<String>,
    media_sources: Option<Vec<SizeSource>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SizeSource {
    id: Option<String>,
    path: Option<String>,
    size: Option<i64>,
    is_remote: Option<bool>,
    protocol: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SizePage {
    items: Vec<SizeItem>,
    total_record_count: usize,
}

#[derive(Default)]
struct FileSizes {
    files: HashMap<String, u64>,
    incomplete: bool,
}

impl FileSizes {
    fn add(&mut self, item: SizeItem) {
        let Some(sources) = item.media_sources.filter(|sources| !sources.is_empty()) else {
            self.incomplete = true;
            return;
        };
        for (index, source) in sources.into_iter().enumerate() {
            if source.is_remote == Some(true)
                || source
                    .protocol
                    .as_deref()
                    .is_some_and(|p| !p.eq_ignore_ascii_case("File"))
            {
                continue;
            }
            let Some(size) = source.size.and_then(|size| u64::try_from(size).ok()) else {
                self.incomplete = true;
                continue;
            };
            // Multi-episode files can appear once per episode. Count each path once,
            // but count distinct versions/editions as separate files.
            let key = source
                .path
                .filter(|path| !path.is_empty())
                .map(|p| format!("path:{p}"))
                .or_else(|| {
                    source
                        .id
                        .filter(|id| !id.is_empty())
                        .map(|id| format!("source:{id}"))
                })
                .or_else(|| item.id.as_ref().map(|id| format!("item:{id}:{index}")));
            if let Some(key) = key {
                self.files
                    .entry(key)
                    .and_modify(|old| *old = (*old).max(size))
                    .or_insert(size);
            } else {
                self.incomplete = true;
            }
        }
    }

    fn total(&self) -> Option<u64> {
        if self.incomplete || self.files.is_empty() {
            return None;
        }
        self.files
            .values()
            .try_fold(0_u64, |sum, size| sum.checked_add(*size))
    }
}

impl ApiClient {
    pub async fn get_storage_size(&self, item: &BaseItemDto) -> Result<Option<u64>> {
        if !matches!(item.type_, Some(BaseItemKind::Movie | BaseItemKind::Series)) {
            return Ok(None);
        }
        let id = item.id.context("Media item has no ID")?;
        let _permit = self.media_sizes.requests.acquire().await?;
        if let Some((saved, size)) = self.media_sizes.cache.lock().await.get(&id) {
            if saved.elapsed() < CACHE_TTL {
                return Ok(*size);
            }
        }

        let mut files = FileSizes::default();
        if item.type_ == Some(BaseItemKind::Movie) {
            let mut url = self
                .root
                .join(&format!("Users/{}/Items/{id}", self.account.id))?;
            url.query_pairs_mut()
                .append_pair("Fields", "MediaSources,Path");
            let item = self
                .client
                .get(url)
                .timeout(Duration::from_secs(20))
                .send()
                .await?
                .error_for_status()?
                .json::<SizeItem>()
                .await?;
            files.add(item);
        } else {
            // Request all episodes, across seasons, with explicit pagination. The
            // collection's 24-item page must never limit the show total.
            let mut start = 0;
            loop {
                let mut url = self
                    .root
                    .join(&format!("Users/{}/Items", self.account.id))?;
                url.query_pairs_mut()
                    .append_pair("ParentId", &id.to_string())
                    .append_pair("Recursive", "true")
                    .append_pair("IncludeItemTypes", "Episode")
                    .append_pair("IsMissing", "false")
                    .append_pair("Fields", "MediaSources,Path")
                    .append_pair("SortBy", "SortName")
                    .append_pair("SortOrder", "Ascending")
                    .append_pair("StartIndex", &start.to_string())
                    .append_pair("Limit", &PAGE_SIZE.to_string());
                let page = self
                    .client
                    .get(url)
                    .timeout(Duration::from_secs(20))
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<SizePage>()
                    .await?;
                let count = page.items.len();
                ensure!(
                    count > 0 || start >= page.total_record_count,
                    "Incomplete episode size response"
                );
                for item in page.items {
                    files.add(item);
                }
                start += count;
                if start >= page.total_record_count {
                    break;
                }
            }
        }
        let size = files.total();
        let mut cache = self.media_sizes.cache.lock().await;
        cache.retain(|_, (saved, _)| saved.elapsed() < CACHE_TTL);
        if cache.len() >= 512 {
            if let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, (saved, _))| *saved)
                .map(|(id, _)| *id)
            {
                cache.remove(&oldest);
            }
        }
        cache.insert(id, (Instant::now(), size));
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(value: serde_json::Value) -> SizeItem {
        serde_json::from_value(value).unwrap()
    }

    async fn mock_server(
        responses: Vec<(u16, serde_json::Value)>,
    ) -> (ApiClient, tokio::task::JoinHandle<Vec<String>>) {
        use crate::config::{Account, Config, Server};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut socket, _) =
                    tokio::time::timeout(Duration::from_secs(5), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut request = Vec::new();
                loop {
                    let mut buf = [0; 4096];
                    let count = socket.read(&mut buf).await.unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&buf[..count]);
                    if request.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                    assert!(request.len() < 65536);
                }
                requests.push(
                    String::from_utf8(request)
                        .unwrap()
                        .lines()
                        .next()
                        .unwrap()
                        .to_string(),
                );
                let body = body.to_string();
                let response = format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        let account = Account {
            id: Uuid::nil(),
            username: "test".into(),
            access_token: "test-token".into(),
            device_id: Uuid::nil(),
        };
        let server = Server {
            url: format!("http://{address}/jellyfin/"),
            ..Default::default()
        };
        let api = ApiClient::new(
            std::sync::Arc::new(std::sync::RwLock::new(Config::default())),
            &server,
            &account,
        );
        (api, task)
    }

    fn media(kind: &str) -> BaseItemDto {
        serde_json::from_value(json!({"Id":Uuid::nil(), "Type":kind})).unwrap()
    }

    #[tokio::test]
    async fn fetches_all_episode_pages_and_caches_show_total() {
        let (api, requests) = mock_server(vec![
            (200, json!({"Items":[{"Id":"ep1", "MediaSources":[{"Path":"/show/a", "Size":100}]}], "TotalRecordCount":2})),
            (200, json!({"Items":[{"Id":"ep2", "MediaSources":[{"Path":"/show/b", "Size":200}]}], "TotalRecordCount":2})),
        ]).await;
        assert_eq!(
            api.get_storage_size(&media("Series")).await.unwrap(),
            Some(300)
        );
        assert_eq!(
            api.get_storage_size(&media("Series")).await.unwrap(),
            Some(300)
        );
        let requests = requests.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /jellyfin/Users/"));
        assert!(requests[0].contains("IncludeItemTypes=Episode"));
        assert!(requests[0].contains("Recursive=true"));
        assert!(requests[0].contains("Fields=MediaSources%2CPath"));
        assert!(requests[0].contains("StartIndex=0"));
        assert!(requests[1].contains("StartIndex=1"));
    }

    #[tokio::test]
    async fn movie_http_errors_are_not_cached_and_unsupported_types_do_not_fetch() {
        let (api, requests) = mock_server(vec![
            (403, json!({"error":"forbidden"})),
            (
                200,
                json!({"Id":"movie", "MediaSources":[{"Path":"/movie.mkv", "Size":123}]}),
            ),
        ])
        .await;
        assert_eq!(api.get_storage_size(&media("Episode")).await.unwrap(), None);
        assert!(api.get_storage_size(&media("Movie")).await.is_err());
        assert_eq!(
            api.get_storage_size(&media("Movie")).await.unwrap(),
            Some(123)
        );
        assert_eq!(
            api.get_storage_size(&media("Movie")).await.unwrap(),
            Some(123)
        );
        let requests = requests.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].contains(&format!("/Items/{}?Fields=", Uuid::nil())));
    }

    #[tokio::test]
    async fn truncated_show_response_is_not_reported_as_complete() {
        let (api, requests) =
            mock_server(vec![(200, json!({"Items":[], "TotalRecordCount":5}))]).await;
        assert!(api.get_storage_size(&media("Series")).await.is_err());
        requests.await.unwrap();
    }

    #[test]
    fn sums_versions_and_deduplicates_multi_episode_files() {
        let mut files = FileSizes::default();
        files.add(item(json!({"Id":"ep1", "MediaSources":[
            {"Path":"/show/double.mkv", "Size":100},
            {"Path":"/show/double-4k.mkv", "Size":300}]})));
        files.add(item(json!({"Id":"ep2", "MediaSources":[
            {"Path":"/show/double.mkv", "Size":100}]})));
        files.add(item(json!({"Id":"special", "MediaSources":[
            {"Path":"/show/special.mkv", "Size":50}]})));
        assert_eq!(files.total(), Some(450));
    }

    #[test]
    fn incomplete_sizes_are_not_shown_as_a_partial_total() {
        for source in [json!({"Path":"/a"}), json!({"Path":"/a", "Size":-1})] {
            let mut files = FileSizes::default();
            files.add(item(
                json!({"Id":"known", "MediaSources":[{"Path":"/known", "Size":123}]}),
            ));
            files.add(item(json!({"Id":"unknown", "MediaSources":[source]})));
            assert_eq!(files.total(), None);
        }
        let mut files = FileSizes::default();
        files.add(item(json!({"Id":"empty"})));
        assert_eq!(files.total(), None);
        assert_eq!(FileSizes::default().total(), None);
    }

    #[test]
    fn skips_remote_sources_and_handles_zero_sizes_and_missing_paths() {
        let mut files = FileSizes::default();
        files.add(item(json!({"Id":"movie", "MediaSources":[
            {"Id":"local", "Size":0},
            {"Path":"https://remote/a", "Size":900, "IsRemote":true},
            {"Path":"https://remote/b", "Size":900, "Protocol":"Http"}]})));
        assert_eq!(files.total(), Some(0));
        files.add(item(json!({"Id":"other", "MediaSources":[{"Size":100}]})));
        assert_eq!(files.total(), Some(100));
    }
}
