use std::{
    cmp,
    future::Future,
    marker::Send,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::{Error, Result};
use jellyfin_api::types::BaseItemDto;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

use crate::tr;

#[derive(Debug, Default, Clone, Copy)]
pub struct FetcherCount {
    pub start: usize,
    pub end: usize,
    pub total: usize,
}

impl FetcherCount {
    fn new(page: usize, items_per_page: usize, total_item_count: usize) -> Self {
        Self {
            start: page * items_per_page,
            end: cmp::min(total_item_count, (page + 1) * items_per_page),
            total: total_item_count,
        }
    }

    pub fn label(&self) -> String {
        tr!("library-item-count", {
            "start" => self.start + 1,
            "end" => self.end,
            "total" => self.total,
        })
        .to_string()
    }
}

#[derive(Debug)]
pub struct FetcherDisplay {
    pub items: Vec<BaseItemDto>,
    pub count: FetcherCount,
}

#[derive(Debug)]
pub enum FetcherState {
    Empty,
    Loading(FetcherCount),
    Ready(FetcherDisplay),
    Error(Error),
}

pub struct MediaFetcher<F: Fetcher + Send + Sync + 'static> {
    fetcher: Arc<F>,
    sender: UnboundedSender<FetcherState>,
    items_per_page: usize,
    cur_page: Option<usize>,
    total_item_count: Arc<AtomicUsize>,
    fetch_handle: Option<JoinHandle<()>>,
}

impl<F: Fetcher + Send + Sync + 'static> MediaFetcher<F> {
    pub fn new(
        fetcher: Arc<F>,
        sender: UnboundedSender<FetcherState>,
        items_per_page: usize,
    ) -> Self {
        sender.send(FetcherState::Empty).unwrap();
        Self {
            fetcher,
            sender,
            items_per_page,
            cur_page: None,
            total_item_count: Arc::default(),
            fetch_handle: None,
        }
    }

    pub fn fetch_page(&mut self, page: usize) {
        self.sender.send(self.loading_state()).unwrap();

        if let Some(fetch_handle) = self.fetch_handle.take() {
            fetch_handle.abort();
        }

        self.fetch_handle = Some(tokio::spawn({
            let fetcher = self.fetcher.clone();
            let sender = self.sender.clone();
            let items_per_page = self.items_per_page;
            let self_total_item_count = self.total_item_count.clone();
            async move {
                match fetcher.fetch(page * items_per_page, items_per_page).await {
                    Ok((items, total_item_count)) => {
                        self_total_item_count.store(total_item_count, Ordering::Relaxed);
                        sender
                            .send(FetcherState::Ready(FetcherDisplay {
                                items,
                                count: FetcherCount::new(page, items_per_page, total_item_count),
                            }))
                            .unwrap();
                    }
                    Err(err) => {
                        sender.send(FetcherState::Error(err)).unwrap();
                    }
                }
            }
        }));
    }

    pub fn next_page(&mut self) {
        // Increment current page, default to first
        let cur_page = self.cur_page.map(|page| page + 1).unwrap_or(0);
        self.cur_page = Some(cur_page);
        self.fetch_page(cur_page);
    }

    pub fn prev_page(&mut self) {
        assert!(self.cur_page.is_some() && self.cur_page.unwrap() > 0);

        let cur_page = self.cur_page.map(|page| page - 1).unwrap();
        self.cur_page = Some(cur_page);
        self.fetch_page(cur_page);
    }

    pub fn has_prev(&self) -> bool {
        self.cur_page.unwrap_or(0) > 0
    }

    pub fn has_next(&self) -> bool {
        let cur_page = self.cur_page.unwrap_or(0);
        (cur_page as f32)
            < (self.total_item_count.load(Ordering::Relaxed) as f32 / self.items_per_page as f32)
                - 1.0
    }

    pub fn title(&self) -> String {
        self.fetcher.title()
    }

    fn loading_state(&self) -> FetcherState {
        match self.cur_page {
            Some(page) => FetcherState::Loading(FetcherCount::new(
                page,
                self.items_per_page,
                self.total_item_count.load(Ordering::Relaxed),
            )),
            _ => FetcherState::Loading(FetcherCount::default()),
        }
    }
}

impl<F: Fetcher + Send + Sync> Drop for MediaFetcher<F> {
    fn drop(&mut self) {
        if let Some(fetch_handle) = &self.fetch_handle {
            fetch_handle.abort();
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
