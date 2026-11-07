use super::{
    media_fetcher::{Fetcher, FetcherCount, FetcherState, FetcherUpdate, MediaFetcher},
    media_grid::{MediaGrid, MediaGridInit, MediaGridInput},
    media_tile::MediaTileDisplay,
};
use crate::{
    jellyfin_api::api_client::ApiClient, library::library_container::LibraryContainer, tr,
    utils::constants::PAGE_MARGIN,
};
use gtk::{pango, prelude::*};
use relm4::{gtk, prelude::*};
use std::sync::Arc;
use tokio::sync::mpsc;
const ITEMS_PER_PAGE: usize = 24;

pub struct MediaPage<F: Fetcher + Send + Sync + 'static, EmptyComponent: Component>
where
    <EmptyComponent as Component>::Root: IsA<gtk::Widget>,
{
    api_client: Arc<ApiClient>,
    fetcher: MediaFetcher<F>,
    empty_component: Option<Controller<EmptyComponent>>,
    media_grid: Option<Controller<MediaGrid>>,
    media_tile_display: MediaTileDisplay,
    state: FetcherState,
    count: Option<FetcherCount>,
}
pub struct MediaPageInit<F, C: Component> {
    pub api_client: Arc<ApiClient>,
    pub fetcher: F,
    pub empty_component: Option<Controller<C>>,
    pub media_tile_display: Option<MediaTileDisplay>,
}
#[derive(Debug)]
pub enum MediaPageInput {
    FetcherState(FetcherUpdate),
    Refresh,
    NextPage,
    CheckScroll,
}

fn near_bottom(value: f64, upper: f64, page: f64) -> bool {
    page > 0.0 && upper - value - page <= (page * 0.6).max(400.0)
}

#[relm4::component(pub)]
impl<F: Fetcher + Send + Sync + 'static, EmptyComponent: Component> Component
    for MediaPage<F, EmptyComponent>
where
    <EmptyComponent as Component>::Root: IsA<gtk::Widget>,
{
    type Init = MediaPageInit<F, EmptyComponent>;
    type Input = MediaPageInput;
    type Output = ();
    type CommandOutput = ();
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,
            set_vexpand: true,
            #[template]
            LibraryContainer {
                set_margin_top: 0,
                set_margin_bottom: 0,
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_margin_top: PAGE_MARGIN,
                    set_margin_bottom: PAGE_MARGIN,
                    gtk::Label {
                        #[watch]
                        set_label: &model.fetcher.title(),
                        add_css_class: "title-1",
                        set_margin_end: 8,
                        set_ellipsize: pango::EllipsizeMode::End,
                    },
                    gtk::Label {
                        set_halign: gtk::Align::End,
                        set_hexpand: true,
                        #[watch]
                        set_visible: model.count.is_some_and(|c| c.total > 0 || c.end > 0),
                        #[watch]
                        set_label?: &model.count.as_ref().map(|count| count.label()),
                    },
                },
            },
            #[template]
            LibraryContainer {
                #[watch]
                set_visible: matches!(model.state, FetcherState::Empty),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 32,
                    append?: model.empty_component.as_ref().map(|c| c.widget()),
                    gtk::Label {
                        set_visible: model.empty_component.is_none(),
                        set_label: tr!("library-no-items"),
                        add_css_class: "dim-label",
                    },
                },
            },
            #[name = "scroll"]
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,
                #[watch]
                set_visible: model.media_grid.is_some(),
                #[template]
                LibraryContainer {
                    #[name = "media_grid_container"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                    },
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Center,
                set_spacing: 8,
                set_margin_all: 12,
                #[watch]
                set_visible: matches!(model.state, FetcherState::Loading(_)),
                gtk::Spinner { set_spinning: true, set_width_request: 24, set_height_request: 24 },
                gtk::Label { set_label: tr!("library-loading-items") },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Center,
                set_spacing: 12,
                set_margin_all: 12,
                #[watch]
                set_visible: matches!(model.state, FetcherState::Error(_)),
                gtk::Label {
                    set_label: tr!("library-load-error"),
                    set_wrap: true,
                },
                gtk::Button {
                    set_label: tr!("library-load-retry"),
                    connect_clicked => MediaPageInput::NextPage,
                },
            },
            gtk::Button {
                set_halign: gtk::Align::Center,
                set_margin_all: 12,
                set_label: tr!("library-load-more"),
                #[watch]
                set_visible: matches!(model.state, FetcherState::Ready(_)) && model.fetcher.has_next(),
                connect_clicked => MediaPageInput::NextPage,
            },
        }
    }
    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let MediaPageInit {
            api_client,
            fetcher,
            empty_component,
            media_tile_display,
        } = init;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let fetcher = MediaFetcher::new(Arc::new(fetcher), tx, ITEMS_PER_PAGE);
        relm4::spawn({
            let sender = sender.clone();
            async move {
                while let Some(state) = rx.recv().await {
                    sender.input(MediaPageInput::FetcherState(state));
                }
            }
        });
        let model = MediaPage {
            api_client,
            fetcher,
            empty_component,
            media_grid: None,
            media_tile_display: media_tile_display.unwrap_or(MediaTileDisplay::Wide),
            state: FetcherState::Empty,
            count: None,
        };
        let widgets = view_output!();
        let adjustment = widgets.scroll.vadjustment();
        adjustment.connect_value_changed({
            let sender = sender.clone();
            move |_| sender.input(MediaPageInput::CheckScroll)
        });
        // Also fill a tall/resized viewport without requiring a scroll gesture.
        adjustment.connect_changed(move |_| sender.input(MediaPageInput::CheckScroll));
        ComponentParts { model, widgets }
    }
    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            MediaPageInput::Refresh => {
                self.fetcher.refresh_loaded();
            }
            MediaPageInput::FetcherState(update) => {
                if !self.fetcher.is_current(update.generation) {
                    return;
                }
                let state = update.state;
                match &state {
                    FetcherState::Loading(count) => self.count = Some(*count),
                    FetcherState::Ready(display) => {
                        self.count = Some(display.count);
                        if display.replace && display.items.is_empty() {
                            if let Some(grid) = self.media_grid.take() {
                                widgets.media_grid_container.remove(grid.widget());
                            }
                        } else if let Some(grid) = &self.media_grid {
                            grid.emit(if display.replace {
                                MediaGridInput::Reconcile(display.items.clone())
                            } else {
                                MediaGridInput::Append(display.items.clone())
                            });
                        } else if !display.items.is_empty() {
                            let grid = MediaGrid::builder()
                                .launch(MediaGridInit {
                                    media: display.items.clone(),
                                    media_tile_display: self.media_tile_display,
                                    api_client: self.api_client.clone(),
                                })
                                .detach();
                            widgets.media_grid_container.append(grid.widget());
                            self.media_grid = Some(grid);
                        }
                    }
                    FetcherState::Error(error) => {
                        // Keep the original error for debugging, never render a
                        // raw server URL/response in the desktop notification.
                        let _ = error;
                        tracing::warn!("Could not load library items; retry is available");
                    }
                    FetcherState::Empty => {}
                }
                self.state = if matches!(&state,FetcherState::Ready(d) if d.items.is_empty())
                    && self.media_grid.is_none()
                {
                    FetcherState::Empty
                } else {
                    state
                };
            }
            MediaPageInput::CheckScroll => {
                let a = widgets.scroll.vadjustment();
                if !matches!(self.state, FetcherState::Ready(_))
                    || !near_bottom(a.value(), a.upper(), a.page_size())
                {
                    return;
                }
                if self.fetcher.next_page() {
                    self.state = FetcherState::Loading(self.count.unwrap_or_default());
                }
            }
            MediaPageInput::NextPage => {
                if !matches!(self.state, FetcherState::Loading(_)) && self.fetcher.next_page() {
                    self.state = FetcherState::Loading(self.count.unwrap_or_default());
                }
            }
        }
        self.update_view(widgets, sender);
    }
}
#[cfg(test)]
mod tests {
    use super::near_bottom;
    #[test]
    fn prefetches_near_the_bottom_and_fills_tall_viewports_but_not_unallocated_ones() {
        assert!(!near_bottom(0.0, 0.0, 0.0));
        assert!(!near_bottom(0.0, 2400.0, 800.0));
        assert!(near_bottom(1200.0, 2400.0, 800.0));
        assert!(near_bottom(0.0, 800.0, 800.0));
        assert!(near_bottom(1599.0, 2400.0, 800.0));
    }
}
