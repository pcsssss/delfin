use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use crate::tr;
use anyhow::{Error, Result};
use jellyfin_api::types::BaseItemDto;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

#[derive(Debug, Default, Clone, Copy)]
pub struct FetcherCount {
    pub end: usize,
    pub total: usize,
}
impl FetcherCount {
    pub fn label(&self) -> String {
        tr!("library-loaded-count", {"loaded" => self.end, "total" => self.total}).to_string()
    }
}
#[derive(Debug)]
pub struct FetcherDisplay {
    pub items: Vec<BaseItemDto>,
    pub count: FetcherCount,
    pub replace: bool,
}
#[derive(Debug)]
pub struct FetcherUpdate {
    pub generation: u64,
    pub state: FetcherState,
}
#[derive(Debug)]
pub enum FetcherState {
    Empty,
    Loading(FetcherCount),
    Ready(FetcherDisplay),
    Error(Error),
}
#[derive(Default)]
struct Cursor {
    count: FetcherCount,
    exhausted: bool,
    pending: bool,
    generation: u64,
    refresh_retry: bool,
}
/// A single-flight cursor: normal loads append; reconciliation reloads only
/// the loaded prefix. Successful responses advance by actual response length.
pub struct MediaFetcher<F: Fetcher + Send + Sync + 'static> {
    fetcher: Arc<F>,
    sender: UnboundedSender<FetcherUpdate>,
    items_per_page: usize,
    cursor: Arc<Mutex<Cursor>>,
    fetch_handle: Option<JoinHandle<()>>,
}
impl<F: Fetcher + Send + Sync + 'static> MediaFetcher<F> {
    pub fn new(
        fetcher: Arc<F>,
        sender: UnboundedSender<FetcherUpdate>,
        items_per_page: usize,
    ) -> Self {
        assert!(items_per_page > 0);
        let _ = sender.send(FetcherUpdate {
            generation: 0,
            state: FetcherState::Empty,
        });
        Self {
            fetcher,
            sender,
            items_per_page,
            cursor: Arc::default(),
            fetch_handle: None,
        }
    }
    pub fn next_page(&mut self) -> bool {
        let retry_refresh = self.cursor.lock().unwrap().refresh_retry;
        if retry_refresh {
            return self.refresh_loaded();
        }
        let (count, generation) = {
            let mut cursor = self.cursor.lock().unwrap();
            if cursor.pending || cursor.exhausted {
                return false;
            }
            cursor.pending = true;
            (cursor.count, cursor.generation)
        };
        let _ = self.sender.send(FetcherUpdate {
            generation,
            state: FetcherState::Loading(count),
        });
        self.fetch_handle = Some(tokio::spawn({
            let fetcher = self.fetcher.clone();
            let sender = self.sender.clone();
            let cursor = self.cursor.clone();
            let limit = self.items_per_page;
            async move {
                let result = fetcher.fetch(count.end, limit).await;
                let mut cursor = cursor.lock().unwrap();
                if cursor.generation != generation {
                    return;
                }
                cursor.pending = false;
                match result {
                    Ok((items, total)) => {
                        cursor.count.end = count.end.saturating_add(items.len());
                        cursor.count.total = total;
                        // Empty responses terminate even if the server's total is
                        // stale; never loop forever on an inconsistent library.
                        cursor.exhausted = items.is_empty() || cursor.count.end >= total;
                        let _ = sender.send(FetcherUpdate {
                            generation,
                            state: FetcherState::Ready(FetcherDisplay {
                                items,
                                count: cursor.count,
                                replace: false,
                            }),
                        });
                    }
                    Err(error) => {
                        let _ = sender.send(FetcherUpdate {
                            generation,
                            state: FetcherState::Error(error),
                        });
                    }
                }
            }
        }));
        true
    }
    pub fn is_current(&self, generation: u64) -> bool {
        self.cursor.lock().unwrap().generation == generation
    }
    /// Reload the loaded prefix atomically. Cards and pagination survive errors;
    /// explicit Retry repeats the refresh rather than appending stale offsets.
    pub fn refresh_loaded(&mut self) -> bool {
        let (count, generation) = {
            let mut cursor = self.cursor.lock().unwrap();
            cursor.generation = cursor.generation.wrapping_add(1);
            cursor.pending = true;
            cursor.refresh_retry = false;
            (cursor.count, cursor.generation)
        };
        if let Some(handle) = self.fetch_handle.take() {
            handle.abort();
        }
        let _ = self.sender.send(FetcherUpdate {
            generation,
            state: FetcherState::Loading(count),
        });
        self.fetch_handle = Some(tokio::spawn({
            let fetcher = self.fetcher.clone();
            let sender = self.sender.clone();
            let cursor = self.cursor.clone();
            let page = self.items_per_page;
            async move {
                let result: Result<(Vec<BaseItemDto>, usize, bool)> = async {
                    let wanted = count.end.max(page);
                    let mut items = Vec::new();
                    loop {
                        let (batch, total) = fetcher
                            .fetch(items.len(), page.min(wanted - items.len()))
                            .await?;
                        let empty = batch.is_empty();
                        items.extend(batch);
                        let exhausted = empty || items.len() >= total;
                        if exhausted || items.len() >= wanted {
                            return Ok((items, total, exhausted));
                        }
                    }
                }
                .await;
                let mut cursor = cursor.lock().unwrap();
                if cursor.generation != generation {
                    return;
                }
                cursor.pending = false;
                let state = match result {
                    Ok((items, total, exhausted)) => {
                        cursor.count = FetcherCount {
                            end: items.len(),
                            total,
                        };
                        cursor.exhausted = exhausted;
                        FetcherState::Ready(FetcherDisplay {
                            items,
                            count: cursor.count,
                            replace: true,
                        })
                    }
                    Err(error) => {
                        cursor.refresh_retry = true;
                        FetcherState::Error(error)
                    }
                };
                let _ = sender.send(FetcherUpdate { generation, state });
            }
        }));
        true
    }
    pub fn has_next(&self) -> bool {
        !self.cursor.lock().unwrap().exhausted
    }
    pub fn title(&self) -> String {
        self.fetcher.title()
    }
}
impl<F: Fetcher + Send + Sync> Drop for MediaFetcher<F> {
    fn drop(&mut self) {
        if let Some(handle) = &self.fetch_handle {
            handle.abort();
        }
    }
}
pub trait Fetcher {
    fn fetch(
        &self,
        start_index: usize,
        limit: usize,
    ) -> impl Future<Output = Result<(Vec<BaseItemDto>, usize)>> + Send;
    fn title(&self) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use tokio::sync::mpsc;
    struct Mock {
        replies: Mutex<VecDeque<Result<(usize, usize)>>>,
        calls: Mutex<Vec<(usize, usize)>>,
    }
    impl Fetcher for Mock {
        async fn fetch(&self, start: usize, limit: usize) -> Result<(Vec<BaseItemDto>, usize)> {
            self.calls.lock().unwrap().push((start, limit));
            let (count, total) = self
                .replies
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected request")?;
            Ok((
                (0..count)
                    .map(|_| serde_json::from_value(serde_json::json!({})).unwrap())
                    .collect(),
                total,
            ))
        }
        fn title(&self) -> String {
            "Movies".into()
        }
    }
    async fn terminal(rx: &mut mpsc::UnboundedReceiver<FetcherUpdate>) -> FetcherState {
        loop {
            let state = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
                .await
                .unwrap()
                .unwrap()
                .state;
            if matches!(state, FetcherState::Ready(_) | FetcherState::Error(_)) {
                return state;
            }
        }
    }
    fn setup(
        replies: Vec<Result<(usize, usize)>>,
    ) -> (
        MediaFetcher<Mock>,
        Arc<Mock>,
        mpsc::UnboundedReceiver<FetcherUpdate>,
    ) {
        let mock = Arc::new(Mock {
            replies: Mutex::new(replies.into()),
            calls: Mutex::default(),
        });
        let (tx, rx) = mpsc::unbounded_channel();
        (MediaFetcher::new(mock.clone(), tx, 2), mock, rx)
    }
    #[tokio::test]
    async fn scrolling_is_single_flight_and_short_batches_do_not_skip_items() {
        let (mut fetcher, mock, mut rx) = setup(vec![Ok((2, 5)), Ok((1, 5)), Ok((2, 5))]);
        for end in [2, 3, 5] {
            assert!(fetcher.next_page());
            assert!(
                !fetcher.next_page(),
                "duplicate scroll event started another request"
            );
            let FetcherState::Ready(display) = terminal(&mut rx).await else {
                panic!("expected items")
            };
            assert_eq!(display.count.end, end);
        }
        assert!(!fetcher.has_next());
        assert!(!fetcher.next_page());
        assert_eq!(*mock.calls.lock().unwrap(), vec![(0, 2), (2, 2), (3, 2)]);
    }
    #[tokio::test]
    async fn failed_load_keeps_the_cursor_and_retry_requests_the_same_offset() {
        let (mut fetcher, mock, mut rx) = setup(vec![
            Ok((1, 3)),
            Err(anyhow::anyhow!("offline")),
            Ok((2, 3)),
        ]);
        fetcher.next_page();
        assert!(matches!(terminal(&mut rx).await, FetcherState::Ready(_)));
        fetcher.next_page();
        assert!(matches!(terminal(&mut rx).await, FetcherState::Error(_)));
        assert!(fetcher.has_next());
        fetcher.next_page();
        let FetcherState::Ready(display) = terminal(&mut rx).await else {
            panic!("retry failed")
        };
        assert_eq!(display.count.end, 3);
        assert_eq!(*mock.calls.lock().unwrap(), vec![(0, 2), (1, 2), (1, 2)]);
        assert!(!fetcher.has_next());
    }
    #[tokio::test]
    async fn empty_response_stops_even_with_an_incorrect_nonzero_total() {
        let (mut fetcher, mock, mut rx) = setup(vec![Ok((0, 999))]);
        fetcher.next_page();
        terminal(&mut rx).await;
        assert!(!fetcher.has_next());
        assert!(!fetcher.next_page());
        assert_eq!(mock.calls.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn refresh_reloads_the_loaded_prefix_and_keeps_the_next_offset_exact() {
        let (mut fetcher, mock, mut rx) = setup(vec![
            Ok((2, 8)),
            Ok((2, 8)),
            Ok((2, 7)),
            Ok((2, 7)),
            Ok((2, 7)),
        ]);
        for _ in 0..2 {
            fetcher.next_page();
            terminal(&mut rx).await;
        }
        fetcher.refresh_loaded();
        assert!(!fetcher.is_current(0));
        let FetcherState::Ready(display) = terminal(&mut rx).await else {
            panic!("refresh failed")
        };
        assert!(display.replace);
        assert_eq!(display.items.len(), 4);
        assert_eq!((display.count.end, display.count.total), (4, 7));
        fetcher.next_page();
        let FetcherState::Ready(display) = terminal(&mut rx).await else {
            panic!("append failed")
        };
        assert!(!display.replace);
        assert_eq!(display.count.end, 6);
        assert_eq!(
            *mock.calls.lock().unwrap(),
            vec![(0, 2), (2, 2), (0, 2), (2, 2), (4, 2)]
        );
    }
    #[tokio::test]
    async fn failed_prefix_refresh_is_atomic_and_retry_restarts_it_not_the_next_page() {
        let (mut fetcher, mock, mut rx) = setup(vec![
            Ok((2, 8)),
            Ok((2, 8)),
            Ok((1, 7)),
            Err(anyhow::anyhow!("offline")),
            Ok((2, 7)),
            Ok((2, 7)),
        ]);
        for _ in 0..2 {
            fetcher.next_page();
            terminal(&mut rx).await;
        }
        fetcher.refresh_loaded();
        assert!(matches!(terminal(&mut rx).await, FetcherState::Error(_)));
        assert_eq!(fetcher.cursor.lock().unwrap().count.end, 4);
        fetcher.next_page();
        let FetcherState::Ready(display) = terminal(&mut rx).await else {
            panic!("retry failed")
        };
        assert!(display.replace);
        assert_eq!(display.items.len(), 4);
        assert_eq!(
            *mock.calls.lock().unwrap(),
            vec![(0, 2), (2, 2), (0, 2), (1, 2), (0, 2), (2, 2)]
        );
    }
    #[tokio::test]
    async fn refreshing_the_last_removed_items_becomes_empty_without_another_page() {
        let (mut fetcher, _, mut rx) = setup(vec![Ok((2, 2)), Ok((0, 0))]);
        fetcher.next_page();
        terminal(&mut rx).await;
        assert!(!fetcher.has_next());
        fetcher.refresh_loaded();
        let FetcherState::Ready(display) = terminal(&mut rx).await else {
            panic!("refresh failed")
        };
        assert!(display.replace);
        assert!(display.items.is_empty());
        assert_eq!(display.count.end, 0);
        assert!(!fetcher.next_page());
    }
    struct InterruptedAppend {
        calls: std::sync::atomic::AtomicUsize,
        started: tokio::sync::Notify,
        completed: std::sync::atomic::AtomicBool,
    }
    impl Fetcher for InterruptedAppend {
        async fn fetch(&self, start: usize, _: usize) -> Result<(Vec<BaseItemDto>, usize)> {
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if call == 1 {
                assert_eq!(start, 2);
                self.started.notify_one();
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                self.completed
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            } else {
                assert_eq!(start, 0);
            }
            Ok((
                (0..2)
                    .map(|_| serde_json::from_value(serde_json::json!({})).unwrap())
                    .collect(),
                8,
            ))
        }
        fn title(&self) -> String {
            "Movies".into()
        }
    }
    #[tokio::test]
    async fn refresh_cancels_a_pending_append_and_invalidates_its_queued_updates() {
        let source = Arc::new(InterruptedAppend {
            calls: 0.into(),
            started: tokio::sync::Notify::new(),
            completed: false.into(),
        });
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut fetcher = MediaFetcher::new(source.clone(), tx, 2);
        fetcher.next_page();
        terminal(&mut rx).await;
        fetcher.next_page();
        source.started.notified().await;
        fetcher.refresh_loaded();
        assert!(!fetcher.is_current(0));
        let FetcherState::Ready(display) = terminal(&mut rx).await else {
            panic!("refresh failed")
        };
        assert!(display.replace);
        assert_eq!(display.count.end, 2);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(!source.completed.load(std::sync::atomic::Ordering::SeqCst));
    }
    struct Slow {
        started: tokio::sync::Notify,
        completed: std::sync::atomic::AtomicBool,
    }
    impl Fetcher for Slow {
        async fn fetch(&self, _: usize, _: usize) -> Result<(Vec<BaseItemDto>, usize)> {
            self.started.notify_one();
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            self.completed
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok((vec![], 0))
        }
        fn title(&self) -> String {
            String::new()
        }
    }
    #[tokio::test]
    async fn replacing_a_sorted_page_cancels_its_inflight_request() {
        let slow = Arc::new(Slow {
            started: tokio::sync::Notify::new(),
            completed: false.into(),
        });
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut fetcher = MediaFetcher::new(slow.clone(), tx, 24);
        fetcher.next_page();
        slow.started.notified().await;
        drop(fetcher);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(!slow.completed.load(std::sync::atomic::Ordering::SeqCst));
        while let Some(state) = rx.recv().await {
            assert!(!matches!(state.state, FetcherState::Ready(_)));
        }
    }
}
