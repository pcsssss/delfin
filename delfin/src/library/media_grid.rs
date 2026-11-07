use std::sync::Arc;

use jellyfin_api::types::BaseItemDto;
use relm4::{gtk::prelude::*, prelude::*};

use crate::{
    jellyfin_api::api_client::ApiClient,
    utils::constants::{POSTER_COLUMNS, POSTER_COLUMN_SPACING, POSTER_ROW_SPACING},
};

use super::media_tile::{MediaTile, MediaTileDisplay};

pub(crate) struct MediaGrid {
    media_tiles: Vec<(BaseItemDto, AsyncController<MediaTile>)>,
    seen: std::collections::HashSet<uuid::Uuid>,
    api_client: Arc<ApiClient>,
    display: MediaTileDisplay,
}

#[derive(Debug)]
pub(crate) enum MediaGridInput {
    Append(Vec<BaseItemDto>),
    Reconcile(Vec<BaseItemDto>),
}

pub(crate) struct MediaGridInit {
    pub(crate) media: Vec<BaseItemDto>,
    pub(crate) media_tile_display: MediaTileDisplay,
    pub(crate) api_client: Arc<ApiClient>,
}

#[relm4::component(pub(crate))]
impl Component for MediaGrid {
    type Init = MediaGridInit;
    type Input = MediaGridInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::FlowBox {
            set_column_spacing: POSTER_COLUMN_SPACING,
            set_row_spacing: POSTER_ROW_SPACING,
            set_homogeneous: true,
            set_selection_mode: gtk::SelectionMode::None,
            set_max_children_per_line: POSTER_COLUMNS,
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let MediaGridInit {
            api_client,
            media,
            media_tile_display,
        } = init;

        let mut model = MediaGrid {
            media_tiles: vec![],
            seen: Default::default(),
            api_client,
            display: media_tile_display,
        };
        model.append(&root, media);

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            MediaGridInput::Append(items) => self.append(root, items),
            MediaGridInput::Reconcile(items) => self.reconcile(root, items),
        }
    }
}

impl MediaGrid {
    fn reconcile(&mut self, root: &gtk::FlowBox, items: Vec<BaseItemDto>) {
        let wanted: std::collections::HashSet<_> = items.iter().filter_map(|m| m.id).collect();
        let mut retained = std::collections::HashMap::new();
        for (media, tile) in self.media_tiles.drain(..) {
            if let Some(id) = media.id.filter(|id| wanted.contains(id)) {
                retained.insert(id, (media, tile));
            } else if let Some(parent) = tile.widget().parent() {
                root.remove(&parent);
            }
        }
        self.seen.clear();
        for media in items {
            let id = media.id;
            if id.is_some_and(|id| !self.seen.insert(id)) {
                continue;
            }
            let previous = id.and_then(|id| retained.remove(&id));
            let unchanged = previous.as_ref().is_some_and(|(old, _)| {
                serde_json::to_value(old)
                    .ok()
                    .is_some_and(|old| serde_json::to_value(&media).is_ok_and(|new| old == new))
            });
            let snapshot = media.clone();
            let tile = if unchanged {
                previous.unwrap().1
            } else {
                if let Some((_, tile)) = previous {
                    if let Some(parent) = tile.widget().parent() {
                        root.remove(&parent);
                    }
                }
                MediaTile::builder()
                    .launch((media, self.display, self.api_client.clone()))
                    .detach()
            };
            let position = self.media_tiles.len() as i32;
            let in_place = root
                .child_at_index(position)
                .is_some_and(|child| child.child().as_ref() == Some(tile.widget().upcast_ref()));
            if !in_place {
                if let Some(parent) = tile.widget().parent() {
                    root.remove(&parent);
                }
                root.insert(tile.widget(), position);
            }
            self.media_tiles.push((snapshot, tile));
        }
    }
    fn append(&mut self, root: &gtk::FlowBox, items: Vec<BaseItemDto>) {
        for media in items {
            if media.id.is_some_and(|id| !self.seen.insert(id)) {
                continue;
            }
            let snapshot = media.clone();
            let tile = MediaTile::builder()
                .launch((media, self.display, self.api_client.clone()))
                .detach();
            root.append(tile.widget());
            self.media_tiles.push((snapshot, tile));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use relm4::{
        adw,
        gtk::{self, gdk, gio, glib},
    };
    use std::{
        sync::RwLock,
        time::{Duration, Instant},
    };

    fn settle() {
        let context = glib::MainContext::default();
        while context.pending() {
            context.iteration(false);
        }
    }
    fn loaded_pictures(widget: &gtk::Widget) -> (usize, usize) {
        let mut loaded = 0;
        let mut total = 0;
        if let Some(picture) = widget.downcast_ref::<gtk::Picture>() {
            total += 1;
            loaded += usize::from(picture.paintable().is_some());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            let (l, t) = loaded_pictures(&widget);
            loaded += l;
            total += t;
            child = widget.next_sibling();
        }
        (loaded, total)
    }
    fn capture(window: &gtk::Window, directory: &std::path::Path, name: &str) {
        for _ in 0..12 {
            settle();
            std::thread::sleep(Duration::from_millis(20));
        }
        let snapshot = gtk::Snapshot::new();
        let paintable = gtk::WidgetPaintable::new(Some(window));
        paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
        let node = snapshot.to_node().expect("visible movie grid");
        let texture = window.renderer().unwrap().render_texture(&node, None);
        texture.save_to_png(directory.join(name)).unwrap();
    }

    fn descendants<
        T: glib::object::IsA<gtk::Widget>
            + glib::object::IsA<glib::Object>
            + Clone
            + glib::types::StaticType,
    >(
        widget: &gtk::Widget,
    ) -> Vec<T> {
        let mut found = Vec::new();
        if let Ok(value) = widget.clone().downcast::<T>() {
            found.push(value);
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            found.extend(descendants::<T>(&widget));
            child = widget.next_sibling();
        }
        found
    }
    fn wait_for(mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(25);
        loop {
            settle();
            if condition() {
                return;
            }
            assert!(Instant::now() < deadline, "native grid condition timed out");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    struct ScrollFixture {
        items: Arc<std::sync::Mutex<Vec<BaseItemDto>>>,
        calls: Arc<std::sync::Mutex<Vec<usize>>>,
        fail_refresh: Arc<std::sync::atomic::AtomicBool>,
    }
    impl crate::library::media_fetcher::Fetcher for ScrollFixture {
        async fn fetch(
            &self,
            start: usize,
            limit: usize,
        ) -> anyhow::Result<(Vec<BaseItemDto>, usize)> {
            let call = {
                let mut calls = self.calls.lock().unwrap();
                calls.push(start);
                calls.len()
            };
            if call == 2
                || self
                    .fail_refresh
                    .swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                return Err(anyhow::anyhow!("simulated offline batch"));
            }
            let items = self.items.lock().unwrap();
            Ok((
                items.iter().skip(start).take(limit).cloned().collect(),
                items.len(),
            ))
        }
        fn title(&self) -> String {
            "Movies · read-only scroll fixtures".into()
        }
    }
    #[test]
    #[ignore = "requires native display and 48 local movie fixtures; all server requests are read-only"]
    fn infinite_grid_native_scroll_retry_and_preserved_cards() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _runtime_context = runtime.enter();
        use crate::{
            library::media_page::{MediaPage, MediaPageInit, MediaPageInput},
            utils::empty_component::EmptyComponent,
        };
        adw::init().unwrap();
        let resource = gio::Resource::load(crate::meson_config::RESOURCES_FILE).unwrap();
        gio::resources_register(&resource);
        relm4_icons::initialize_icons();
        let css = gtk::CssProvider::new();
        css.load_from_string(include_str!(concat!(env!("OUT_DIR"), "/app.css")));
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("local-dist");
        let fixtures: serde_json::Value = serde_json::from_slice(
            &std::fs::read(directory.join("infinite-grid-fixtures.json")).unwrap(),
        )
        .unwrap();
        let items = crate::jellyfin_api::api::views::decode_collection_page(
            serde_json::json!({"Items":fixtures,"TotalRecordCount":48}),
        )
        .unwrap()
        .items
        .unwrap();
        assert_eq!(items.len(), 48);
        let config = Config::new().unwrap();
        let server = config.servers.first().unwrap().clone();
        let account = server.accounts.first().unwrap().clone();
        let api = Arc::new(ApiClient::new(
            Arc::new(RwLock::new(config)),
            &server,
            &account,
        ));
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let items = Arc::new(std::sync::Mutex::new(items));
        let fail_refresh = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let page = MediaPage::<ScrollFixture, EmptyComponent>::builder()
            .launch(MediaPageInit {
                api_client: api,
                fetcher: ScrollFixture {
                    items: items.clone(),
                    calls: calls.clone(),
                    fail_refresh: fail_refresh.clone(),
                },
                empty_component: None,
                media_tile_display: Some(MediaTileDisplay::CoverLarge),
            })
            .detach();
        let window = gtk::Window::builder()
            .title("Delfin · read-only scroll test")
            .default_width(1000)
            .default_height(850)
            .resizable(false)
            .build();
        window.set_size_request(1000, 850);
        window.set_child(Some(page.widget()));
        window.present();
        page.emit(MediaPageInput::NextPage);
        let scroll = descendants::<gtk::ScrolledWindow>(page.widget().upcast_ref()).remove(0);
        wait_for(|| {
            loaded_pictures(page.widget().upcast_ref()).0 == 24
                && scroll.vadjustment().upper() > scroll.vadjustment().page_size() + 500.0
        });
        assert_eq!(*calls.lock().unwrap(), vec![0]);
        let grid = descendants::<gtk::FlowBox>(page.widget().upcast_ref()).remove(0);
        let first = grid.first_child().unwrap();
        capture(&window, &directory, "infinite-grid-first.png");
        let a = scroll.vadjustment();
        a.set_value(a.upper() - a.page_size());
        wait_for(|| {
            descendants::<gtk::Button>(page.widget().upcast_ref())
                .iter()
                .any(|b| {
                    b.label().as_deref() == Some(crate::tr!("library-load-retry")) && b.is_mapped()
                })
        });
        assert_eq!(loaded_pictures(page.widget().upcast_ref()), (24, 24));
        assert_eq!(grid.first_child().unwrap(), first);
        capture(&window, &directory, "infinite-grid-retry.png");
        assert_eq!(
            *calls.lock().unwrap(),
            vec![0, 24],
            "error must not auto-retry in a loop"
        );
        let retry = descendants::<gtk::Button>(page.widget().upcast_ref())
            .into_iter()
            .find(|b| b.label().as_deref() == Some(crate::tr!("library-load-retry")))
            .unwrap();
        retry.emit_clicked();
        wait_for(|| loaded_pictures(page.widget().upcast_ref()) == (48, 48));
        assert_eq!(*calls.lock().unwrap(), vec![0, 24, 24]);
        assert_eq!(grid.first_child().unwrap(), first);
        assert!(
            scroll.vadjustment().value() > 1000.0,
            "appending reset the scroll position"
        );
        capture(&window, &directory, "infinite-grid-loaded.png");
        let position = scroll.vadjustment().value();
        // Only the in-memory catalog changes; no live deletion is performed.
        {
            let mut items = items.lock().unwrap();
            items.remove(15);
            items[5].name = Some("Updated movie fixture".into());
        }
        page.emit(MediaPageInput::Refresh);
        wait_for(|| loaded_pictures(page.widget().upcast_ref()) == (47, 47));
        capture(&window, &directory, "deletion-scroll-retained.png");
        assert!(descendants::<gtk::Label>(grid.upcast_ref())
            .iter()
            .any(|label| label.text().as_str() == "Updated movie fixture"));
        assert_eq!(grid.first_child().unwrap(), first);
        assert_eq!(
            descendants::<gtk::ScrolledWindow>(page.widget().upcast_ref())[0],
            scroll
        );
        assert!(
            (scroll.vadjustment().value() - position).abs() < 1.0,
            "deletion refresh reset scroll"
        );
        assert_eq!(*calls.lock().unwrap(), vec![0, 24, 24, 0, 24]);
        items.lock().unwrap().remove(20);
        fail_refresh.store(true, std::sync::atomic::Ordering::SeqCst);
        page.emit(MediaPageInput::Refresh);
        wait_for(|| retry.is_mapped());
        assert_eq!(loaded_pictures(page.widget().upcast_ref()), (47, 47));
        assert!((scroll.vadjustment().value() - position).abs() < 1.0);
        retry.emit_clicked();
        wait_for(|| loaded_pictures(page.widget().upcast_ref()) == (46, 46));
        assert_eq!(grid.first_child().unwrap(), first);
        assert!(
            (scroll.vadjustment().value() - position).abs() < 1.0,
            "refresh retry reset scroll"
        );
        assert_eq!(*calls.lock().unwrap(), vec![0, 24, 24, 0, 24, 0, 0, 24]);
        window.close();
        drop(page);
        settle();
    }

    /// Uses a local metadata snapshot and the saved Jellyfin account only for
    /// GET image/file-size requests. Watch states below are in-memory fixtures;
    /// no playback, credentials, preferences or server metadata are changed.
    #[test]
    #[ignore = "requires a native display, local movie fixtures and read-only access to the configured Jellyfin server"]
    fn rich_movie_grid_native_preview_read_only() {
        adw::init().unwrap();
        let resource = gio::Resource::load(crate::meson_config::RESOURCES_FILE).unwrap();
        gio::resources_register(&resource);
        relm4_icons::initialize_icons();
        let css = gtk::CssProvider::new();
        css.load_from_string(include_str!(concat!(env!("OUT_DIR"), "/app.css")));
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("local-dist");
        let mut fixtures: Vec<serde_json::Value> = serde_json::from_slice(
            &std::fs::read(directory.join("movie-grid-fixtures.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(fixtures.len(), 6);
        fixtures[0]["UserData"] =
            serde_json::json!({"Played":true,"PlayedPercentage":100,"PlaybackPositionTicks":0});
        fixtures[1]["UserData"] =
            serde_json::json!({"Played":false,"PlayedPercentage":35,"PlaybackPositionTicks":1});
        fixtures[2]["UserData"] = serde_json::json!({"Played":false,"PlaybackPositionTicks":0});
        fixtures[3]["CommunityRating"] = serde_json::Value::Null;
        fixtures[3]["CriticRating"] = serde_json::Value::Null;
        fixtures[3]["Width"] = serde_json::Value::Null;
        fixtures[3]["Height"] = serde_json::Value::Null;
        fixtures[3]["MediaStreams"] = serde_json::Value::Null;
        fixtures[4]["UserData"] = serde_json::Value::Null;
        fixtures[5]["UserData"] =
            serde_json::json!({"Played":true,"PlayedPercentage":47,"PlaybackPositionTicks":1});
        let media = crate::jellyfin_api::api::views::decode_collection_page(
            serde_json::json!({"Items": fixtures, "TotalRecordCount": 6}),
        )
        .unwrap()
        .items
        .unwrap();
        let duplicates = media.clone();
        let config = Config::new().unwrap();
        let server = config.servers.first().unwrap().clone();
        let account = server.accounts.first().unwrap().clone();
        let api = Arc::new(ApiClient::new(
            Arc::new(RwLock::new(config)),
            &server,
            &account,
        ));
        let grid = MediaGrid::builder()
            .launch(MediaGridInit {
                media,
                media_tile_display: MediaTileDisplay::CoverLarge,
                api_client: api,
            })
            .detach();
        grid.widget().set_margin_all(24);
        let scroll = gtk::ScrolledWindow::new();
        scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
        scroll.set_child(Some(grid.widget()));
        let window = gtk::Window::builder()
            .title("Delfin movie grid · read-only preview fixtures")
            .default_width(1360)
            .default_height(850)
            .build();
        // Fixed-size preview windows must not be tiled into an arbitrary half
        // screen: capture the actual wide/narrow allocations, not just defaults.
        window.set_resizable(false);
        window.set_size_request(1360, 850);
        window.set_child(Some(&scroll));
        window.present();
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            settle();
            let (loaded, total) = loaded_pictures(grid.widget().upcast_ref());
            if loaded == 6 && total == 6 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "movie thumbnails did not finish loading"
            );
            std::thread::sleep(Duration::from_millis(30));
        }
        let first = grid.widget().first_child().unwrap();
        grid.emit(MediaGridInput::Append(duplicates));
        settle();
        assert_eq!(loaded_pictures(grid.widget().upcast_ref()), (6, 6));
        assert_eq!(grid.widget().first_child().unwrap(), first);
        let links = descendants::<gtk::LinkButton>(grid.widget().upcast_ref());
        assert_eq!(links.len(), 6);
        let activated = std::rc::Rc::new(std::cell::Cell::new(false));
        links[0].connect_activate_link({
            let activated = activated.clone();
            move |_| {
                activated.set(true);
                glib::Propagation::Stop // Never open a browser in tests.
            }
        });
        assert!(links[0].uri().starts_with("https://www.imdb.com/title/tt"));
        assert!(links[0].emit_by_name::<bool>("activate-link", &[]));
        assert!(activated.get());
        capture(&window, &directory, "movie-grid-wide.png");
        assert!(
            window.width() >= 1300,
            "wide preview was constrained by the compositor"
        );
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight);
        capture(&window, &directory, "movie-grid-light.png");
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
        window.set_size_request(500, 850);
        window.set_default_size(500, 850);
        capture(&window, &directory, "movie-grid-narrow.png");
        assert!(window.width() <= 540, "narrow preview did not resize");
        assert_eq!(loaded_pictures(grid.widget().upcast_ref()), (6, 6));
        window.close();
        drop(grid);
        settle();
    }
}
