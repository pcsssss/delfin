//! Native GTK Seerr surfaces. Jellyfin navigation and authentication remain separate.
use super::{
    discovery::{probe_seerr_url, suggest_seerr_url},
    discovery_card::DiscoveryCardInfo,
    ui_service::{CredentialStatus, Data, Job, Operation, Reply, Service},
    ui_state::*,
    *,
};
use crate::{
    globals::CONFIG,
    tr,
    utils::{
        constants::{
            MAX_LIBRARY_WIDTH, PAGE_MARGIN, POSTER_COLUMNS, POSTER_COLUMN_SPACING, POSTER_HEIGHT,
            POSTER_ROW_SPACING, POSTER_TITLE_HEIGHT, POSTER_WIDTH,
        },
        file_size::format_file_size,
        main_window::get_main_window,
    },
};
use relm4::adw::prelude::*;
use relm4::{
    adw,
    gtk::{self, glib},
    RelmWidgetExt,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

mod detail_view;
mod removal_ui;
mod request_view;
type Shared = Rc<RefCell<Hub>>;
struct SavedGrid {
    widget: gtk::Widget,
    posters: HashMap<u64, gtk::Picture>,
    scroll: f64,
    images: Vec<(u64, String)>,
}
fn scroll_for(body: &gtk::Box) -> Option<gtk::ScrolledWindow> {
    body.ancestor(gtk::ScrolledWindow::static_type())
        .and_downcast::<gtk::ScrolledWindow>()
}
fn back_to_grid(hub: &Shared) {
    let saved = hub.borrow_mut().saved_grid.take();
    let Some(saved) = saved else {
        refresh(hub);
        return;
    };
    replace_view(hub);
    let body = hub.borrow().bodies[0].clone();
    body.append(&saved.widget);
    let images: Vec<_> = saved
        .images
        .into_iter()
        .filter(|(id, _)| {
            saved
                .posters
                .get(id)
                .is_some_and(|picture| picture.paintable().is_none())
        })
        .collect();
    let (account, view) = {
        let mut model = hub.borrow_mut();
        model.posters = saved.posters;
        model.back.set_visible(false);
        model.request_focus = false;
        model.controls[0].set_visible(true);
        model.previous.set_sensitive(model.page > 1);
        model.next.set_sensitive(model.page < model.total_pages);
        (model.state.account, model.state.view)
    };
    if !images.is_empty() {
        let _ = hub.borrow().service.sender.send(Job {
            account,
            view,
            operation: Operation::Artwork(images),
        });
    }
    let weak = Rc::downgrade(hub);
    glib::timeout_add_local_once(Duration::from_millis(50), move || {
        if let Some(hub) = weak.upgrade() {
            if hub.borrow().state.accepts(account, view) {
                if let Some(scroll) = scroll_for(&body) {
                    scroll.vadjustment().set_value(saved.scroll);
                }
            }
        }
    });
    status(hub, tr!("seerr-connected"));
}

pub(crate) struct SeerrHub {
    inner: Shared,
    source: Option<glib::SourceId>,
}
struct Hub {
    removal: Option<Rc<RefCell<removal_ui::Wizard>>>,
    removal_sequence: u64,
    saved_grid: Option<SavedGrid>,
    grid_images: Vec<(u64, String)>,
    back: gtk::Button,
    request_focus: bool,
    roots: [gtk::Box; 3],
    bodies: [gtk::Box; 3],
    statuses: [gtk::Label; 3],
    spinners: [gtk::Spinner; 3],
    account_buttons: [gtk::Button; 3],
    controls: [gtk::Box; 3],
    service: Service,
    state: ViewState,
    user: Option<User>,
    active: Option<usize>,
    page: u32,
    total_pages: u32,
    request_skip: u32,
    request_total: u64,
    category: gtk::DropDown,
    media_toggle: [gtk::ToggleButton; 2],
    search: gtk::SearchEntry,
    last_search: String,
    previous: gtk::Button,
    next: gtk::Button,
    request_tabs: [gtk::ToggleButton; 2],
    request_previous: gtk::Button,
    request_next: gtk::Button,
    page_label: gtk::Label,
    last_poll: Instant,
    poll_allowed: bool,
    posters: HashMap<u64, gtk::Picture>,
    settings: Option<gtk::Window>,
    settings_error: Option<gtk::Label>,
    login_button: Option<gtk::Button>,
    suggested_url: Option<String>,
}

fn column(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, spacing)
}
fn row(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, spacing)
}
fn label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    label.set_selectable(true);
    label
}
fn button(text: &str) -> gtk::Button {
    let button = gtk::Button::with_label(text);
    button.set_valign(gtk::Align::Center);
    button
}
fn check_button(text: &str) -> gtk::CheckButton {
    let check = gtk::CheckButton::with_label(text);
    if let Some(child) = check.child().and_downcast::<gtk::Label>() {
        child.set_wrap(true);
        child.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        child.set_xalign(0.0);
    }
    check
}
fn clear(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}
fn on_click(widget: &gtk::Button, hub: &Shared, action: impl Fn(&Shared) + 'static) {
    let weak = Rc::downgrade(hub);
    widget.connect_clicked(move |_| {
        if let Some(hub) = weak.upgrade() {
            action(&hub);
        }
    });
}
fn replace_view(hub: &Shared) {
    let mut model = hub.borrow_mut();
    model.state.navigate();
    model
        .service
        .view
        .store(model.state.view, Ordering::Relaxed);
    model.posters.clear();
    model.back.set_sensitive(true);
    if let Some(active) = model.active.filter(|active| *active != 2) {
        clear(&model.bodies[active]);
    }
    for body in &model.bodies {
        body.set_sensitive(true);
    }
    for control in &model.controls {
        control.set_sensitive(model.user.is_some());
    }
    for spinner in &model.spinners {
        spinner.set_spinning(false);
    }
}
fn status(hub: &Shared, text: &str) {
    for item in &hub.borrow().statuses {
        item.set_label(text);
    }
}
fn send(hub: &Shared, operation: Operation) {
    let mut model = hub.borrow_mut();
    if model.state.busy {
        return;
    }
    let mutating = matches!(&operation, Operation::Request(..) | Operation::Grab(..));
    let job = Job {
        account: model.state.account,
        view: model.state.view,
        operation,
    };
    if model.service.sender.send(job).is_err() {
        drop(model);
        status(hub, tr!("seerr-error-setup"));
        return;
    }
    model.state.busy = true;
    // Keep escape available during reads, but retain the mutation result so a
    // confirmed request can invalidate the cached grid before going back.
    model.back.set_sensitive(!mutating);
    for spinner in &model.spinners {
        spinner.set_spinning(true);
    }
    for (index, control) in model.controls.iter().enumerate() {
        // Discovery inputs stay editable while an obsolete search is in flight.
        control.set_sensitive(index == 0 && model.user.is_some());
    }
    for body in &model.bodies {
        body.set_sensitive(false);
    }
    if let Some(button) = &model.login_button {
        button.set_sensitive(false);
    }
    if model.active != Some(2) || model.bodies[2].first_child().is_none() {
        for item in &model.statuses {
            item.set_label(tr!("seerr-loading"));
        }
    }
}
fn discovery_kind(hub: &Shared) -> MediaType {
    if hub.borrow().media_toggle[1].is_active() {
        MediaType::Tv
    } else {
        MediaType::Movie
    }
}
fn discovery_changed(hub: &Shared) {
    hub.borrow_mut().page = 1;
    let kind = discovery_kind(hub);
    let placeholder = match kind {
        MediaType::Movie => tr!("seerr-search-movies").to_string(),
        MediaType::Tv => tr!("seerr-search-tv").to_string(),
    };
    hub.borrow().search.set_placeholder_text(Some(&placeholder));
    hub.borrow().previous.set_sensitive(false);
    hub.borrow().next.set_sensitive(false);
    hub.borrow().page_label.set_label("");
    if hub.borrow().active == Some(0) {
        replace_view(hub); // Invalidate stale replies immediately, even offline.
        if hub.borrow().user.is_some() {
            discover(hub);
        }
    }
}
fn discover(hub: &Shared) {
    {
        let mut model = hub.borrow_mut();
        model.saved_grid = None;
        model.request_focus = false;
        model.back.set_visible(false);
        model.controls[0].set_visible(true);
    }
    if hub.borrow().active != Some(0) {
        return;
    }
    replace_view(hub);
    let mut model = hub.borrow_mut();
    let kind = if model.media_toggle[1].is_active() {
        MediaType::Tv
    } else {
        MediaType::Movie
    };
    let feed = DiscoverFeed::for_selection(model.category.selected(), kind);
    let query = model.search.text().trim().to_string();
    model.last_search = query.clone();
    let operation = Operation::Discover(feed, kind, query, model.page);
    drop(model);
    send(hub, operation);
}
fn refresh(hub: &Shared) {
    let model = hub.borrow();
    if model.user.is_none() {
        drop(model);
        status(hub, tr!("seerr-sign-in-hint"));
        return;
    }
    let active = model.active;
    if active == Some(2) && !model.user.as_ref().is_some_and(User::can_manage_requests) {
        drop(model);
        hub.borrow_mut().poll_allowed = false;
        status(hub, tr!("seerr-manager-required"));
        return;
    }
    let skip = model.request_skip;
    let finished = model.request_tabs[0].is_active();
    drop(model);
    match active {
        Some(0) => discover(hub),
        Some(1) => {
            replace_view(hub);
            send(hub, Operation::Requests(skip, finished));
        }
        Some(2) => {
            hub.borrow_mut().poll_allowed = true;
            replace_view(hub);
            send(hub, Operation::Torrents);
        }
        _ => {}
    }
}
impl SeerrHub {
    pub fn new(jellyfin: Option<&reqwest::Url>) -> Self {
        let (service, receiver) = Service::new();
        let roots = std::array::from_fn(|_| column(0));
        let bodies = std::array::from_fn(|_| column(18));
        let statuses = std::array::from_fn(|_| label(tr!("seerr-sign-in-hint")));
        let spinners = std::array::from_fn(|_| gtk::Spinner::new());
        let account_buttons = std::array::from_fn(|_| button(tr!("seerr-account")));
        let controls = std::array::from_fn(|_| row(8));
        let back = button(tr!("seerr-back-discovery"));
        back.set_icon_name("left");
        // A labelled child, rather than icon-name alone, keeps navigation explicit.
        let back_content = row(6);
        back_content.append(&gtk::Image::from_icon_name("left"));
        let back_text = label(tr!("seerr-back-discovery"));
        back_text.set_selectable(false);
        back_content.append(&back_text);
        back.set_child(Some(&back_content));
        back.set_halign(gtk::Align::Start);
        back.set_margin_start(12);
        back.set_margin_bottom(8);
        back.set_visible(false);
        for i in 0..3 {
            let toolbar = row(12);
            toolbar.set_margin_start(18);
            toolbar.set_margin_end(18);
            toolbar.set_margin_top(if i == 0 { 8 } else { 12 });
            toolbar.set_margin_bottom(if i == 0 { 8 } else { 12 });
            statuses[i].set_hexpand(true);
            statuses[i].set_selectable(false);
            toolbar.append(&spinners[i]);
            toolbar.append(&statuses[i]);
            toolbar.append(&account_buttons[i]);
            roots[i].append(&toolbar);
            if i == 0 {
                roots[i].append(&back);
            }
            controls[i].set_margin_bottom(8);
            if i == 0 {
                let clamp = adw::Clamp::new();
                clamp.set_maximum_size(MAX_LIBRARY_WIDTH);
                clamp.set_tightening_threshold(MAX_LIBRARY_WIDTH);
                clamp.set_margin_start(PAGE_MARGIN);
                clamp.set_margin_end(PAGE_MARGIN);
                clamp.set_child(Some(&controls[i]));
                roots[i].append(&clamp);
            } else {
                controls[i].set_margin_start(18);
                controls[i].set_margin_end(18);
                roots[i].append(&controls[i]);
            }
            let scroll = gtk::ScrolledWindow::new();
            scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
            scroll.set_vexpand(true);
            let clamp = adw::Clamp::new();
            if i == 0 {
                // Match the Movies LibraryContainer, including where padding is
                // applied; an extra inner margin would change the column count.
                clamp.set_maximum_size(MAX_LIBRARY_WIDTH);
                clamp.set_tightening_threshold(MAX_LIBRARY_WIDTH);
                clamp.set_margin_all(PAGE_MARGIN);
            } else {
                clamp.set_maximum_size(1100);
                bodies[i].set_margin_start(24);
                bodies[i].set_margin_end(24);
                bodies[i].set_margin_top(12);
                bodies[i].set_margin_bottom(32);
            }
            clamp.set_child(Some(&bodies[i]));
            scroll.set_child(Some(&clamp));
            roots[i].append(&scroll);
        }
        let category = gtk::DropDown::from_strings(&[
            tr!("seerr-trending"),
            tr!("seerr-feed-popular"),
            tr!("seerr-feed-upcoming"),
        ]);
        category.set_tooltip_text(Some(tr!("seerr-category")));
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(tr!("seerr-search-movies")));
        search.set_search_delay(300);
        search.set_hexpand(true);
        search.set_width_chars(14);
        let movies = gtk::ToggleButton::with_label(tr!("seerr-discover-movies"));
        let tv = gtk::ToggleButton::with_label(tr!("seerr-discover-tv"));
        tv.set_group(Some(&movies));
        movies.set_active(true);
        let media_buttons = row(0);
        media_buttons.add_css_class("linked");
        media_buttons.append(&movies);
        media_buttons.append(&tv);
        let discover_filters = row(8);
        discover_filters.append(&search);
        let settings = row(6);
        settings.append(&media_buttons);
        settings.append(&category);
        discover_filters.append(&settings);
        let responsive = adw::BreakpointBin::new();
        responsive.set_hexpand(true);
        responsive.set_size_request(200, 34);
        responsive.set_child(Some(&discover_filters));
        let narrow =
            adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 760px").unwrap());
        narrow.add_setter(
            &discover_filters,
            "orientation",
            &gtk::Orientation::Vertical.to_value(),
        );
        narrow.add_setter(&responsive, "height-request", &76i32.to_value());
        responsive.add_breakpoint(narrow);
        controls[0].append(&responsive);
        let previous = gtk::Button::from_icon_name("left");
        previous.set_tooltip_text(Some(tr!("seerr-previous")));
        previous.update_property(&[gtk::accessible::Property::Label(tr!("seerr-previous"))]);
        let next = gtk::Button::from_icon_name("right");
        next.set_tooltip_text(Some(tr!("seerr-next")));
        next.update_property(&[gtk::accessible::Property::Label(tr!("seerr-next"))]);
        let page_label = label("");
        page_label.set_wrap(false);
        page_label.set_selectable(false);
        let pages = row(4);
        pages.append(&previous);
        pages.append(&page_label);
        pages.append(&next);
        settings.append(&pages);
        let finished = gtk::ToggleButton::with_label(tr!("seerr-requests-finished"));
        let unfinished = gtk::ToggleButton::with_label(tr!("seerr-requests-unfinished"));
        unfinished.set_group(Some(&finished));
        unfinished.set_active(true);
        let tabs = row(0);
        tabs.add_css_class("linked");
        tabs.append(&finished);
        tabs.append(&unfinished);
        controls[1].append(&tabs);
        let request_previous = gtk::Button::from_icon_name("left");
        request_previous.set_tooltip_text(Some(tr!("seerr-previous")));
        request_previous
            .update_property(&[gtk::accessible::Property::Label(tr!("seerr-previous"))]);
        let request_next = gtk::Button::from_icon_name("right");
        request_next.set_tooltip_text(Some(tr!("seerr-next")));
        request_next.update_property(&[gtk::accessible::Property::Label(tr!("seerr-next"))]);
        controls[1].append(&request_previous);
        controls[1].append(&request_next);
        let refresh_buttons: [gtk::Button; 3] =
            std::array::from_fn(|_| button(tr!("seerr-refresh")));
        refresh_buttons[0].set_label("");
        refresh_buttons[0].set_icon_name("refresh");
        refresh_buttons[0].set_tooltip_text(Some(tr!("seerr-refresh")));
        refresh_buttons[0]
            .update_property(&[gtk::accessible::Property::Label(tr!("seerr-refresh"))]);
        settings.append(&refresh_buttons[0]);
        refresh_buttons[1].set_label("");
        refresh_buttons[1].set_icon_name("refresh");
        refresh_buttons[1].set_tooltip_text(Some(tr!("seerr-refresh")));
        refresh_buttons[1]
            .update_property(&[gtk::accessible::Property::Label(tr!("seerr-refresh"))]);
        for i in 1..3 {
            controls[i].append(&refresh_buttons[i]);
        }
        let inner = Rc::new(RefCell::new(Hub {
            roots,
            removal: None,
            removal_sequence: 0,
            saved_grid: None,
            grid_images: Vec::new(),
            back: back.clone(),
            request_focus: false,
            bodies,
            statuses,
            spinners,
            account_buttons,
            controls,
            service,
            state: ViewState::default(),
            user: None,
            active: None,
            page: 1,
            total_pages: 1,
            request_skip: 0,
            request_total: 0,
            category,
            media_toggle: [movies, tv],
            search,
            last_search: String::new(),
            previous,
            next,
            request_tabs: [finished, unfinished],
            request_previous,
            request_next,
            page_label,
            last_poll: Instant::now(),
            poll_allowed: true,
            posters: HashMap::new(),
            settings: None,
            settings_error: None,
            login_button: None,
            suggested_url: jellyfin.and_then(suggest_seerr_url),
        }));
        {
            let model = inner.borrow();
            let history = button(tr!("seerr-removal-history"));
            model.controls[2].append(&history);
            on_click(&history, &inner, |hub| removal_ui::open(hub, None));
            for b in &model.account_buttons {
                on_click(b, &inner, show_account);
            }
            for b in refresh_buttons {
                on_click(&b, &inner, refresh);
            }
            on_click(&model.previous, &inner, |hub| {
                let page = hub.borrow().page;
                if page > 1 {
                    hub.borrow_mut().page -= 1;
                    discover(hub);
                }
            });
            on_click(&model.next, &inner, |hub| {
                if hub.borrow().page < hub.borrow().total_pages {
                    hub.borrow_mut().page += 1;
                    discover(hub);
                }
            });
            let weak = Rc::downgrade(&inner);
            model.request_tabs[1].connect_toggled(move |_| {
                if let Some(hub) = weak.upgrade() {
                    hub.borrow_mut().request_skip = 0;
                    if hub.borrow().active == Some(1) {
                        replace_view(&hub);
                        if hub.borrow().user.is_some() {
                            refresh(&hub);
                        }
                    }
                }
            });
            on_click(&model.request_previous, &inner, |hub| {
                let skip = hub.borrow().request_skip.saturating_sub(20);
                hub.borrow_mut().request_skip = skip;
                refresh(hub);
            });
            on_click(&model.request_next, &inner, |hub| {
                hub.borrow_mut().request_skip += 20;
                refresh(hub);
            });
            let weak = Rc::downgrade(&inner);
            on_click(&back, &inner, back_to_grid);
            model.category.connect_selected_notify(move |_| {
                if let Some(hub) = weak.upgrade() {
                    discovery_changed(&hub);
                }
            });
            let weak = Rc::downgrade(&inner);
            model.media_toggle[1].connect_toggled(move |_| {
                if let Some(hub) = weak.upgrade() {
                    discovery_changed(&hub);
                }
            });
            let weak = Rc::downgrade(&inner);
            model.search.connect_search_changed(move |entry| {
                if let Some(hub) = weak.upgrade() {
                    if entry.text().trim() == hub.borrow().last_search {
                        return;
                    }
                    hub.borrow_mut().page = 1;
                    if hub.borrow().active == Some(0) && hub.borrow().user.is_some() {
                        discover(&hub);
                    }
                }
            });
            // Invalidate immediately, before SearchEntry's debounce timeout fires.
            let weak = Rc::downgrade(&inner);
            model.search.connect_changed(move |entry| {
                if let Some(hub) = weak.upgrade() {
                    if hub.borrow().active == Some(0) {
                        if entry.text().trim() != hub.borrow().last_search {
                            // Old page arrows must not dispatch the new query at
                            // an old offset during the debounce window.
                            hub.borrow_mut().page = 1;
                            hub.borrow().previous.set_sensitive(false);
                            hub.borrow().next.set_sensitive(false);
                            hub.borrow().page_label.set_label("");
                            replace_view(&hub);
                        }
                        let model = hub.borrow();
                        model
                            .category
                            .set_sensitive(model.search.text().trim().is_empty());
                    }
                }
            });
        }
        let weak = Rc::downgrade(&inner);
        let source = glib::timeout_add_local(Duration::from_millis(100), move || {
            let Some(hub) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            while let Ok(reply) = receiver.try_recv() {
                receive(&hub, reply);
            }
            let model = hub.borrow();
            let poll = model.active == Some(2)
                && model.roots[2].is_mapped()
                && model.user.as_ref().is_some_and(User::can_manage_requests)
                && model.poll_allowed
                && !model.state.busy
                && model.last_poll.elapsed() >= Duration::from_secs(5);
            drop(model);
            if poll {
                send(&hub, Operation::Torrents);
            }
            glib::ControlFlow::Continue
        });
        let remembered = CONFIG.read().seerr.clone().filter(|config| {
            config.remember_password
                && config
                    .username
                    .as_deref()
                    .is_some_and(|name| !name.is_empty())
                && config.login_method != LoginMethod::JellyfinQuickConnect
        });
        if let Some(config) = remembered {
            send(&inner, Operation::Restore(config));
        }
        Self {
            inner,
            source: Some(source),
        }
    }
    pub fn refresh_current(&self) -> bool {
        if self.inner.borrow().active.is_some() {
            refresh(&self.inner);
            true
        } else {
            false
        }
    }
    pub(crate) fn remove_item(&self, id: String, jellyfin_server_id: String) {
        if jellyfin_server_id.is_empty() {
            crate::app::APP_BROKER.send(crate::app::AppInput::Toast(
                tr!("seerr-removal-origin-required").to_string(),
                None,
            ));
            return;
        }
        removal_ui::open(
            &self.inner,
            Some(RemovalTarget::Jellyfin {
                jellyfin_item_id: id,
                jellyfin_server_id,
            }),
        );
    }
    pub fn pages(&self) -> [gtk::Box; 3] {
        self.inner.borrow().roots.clone()
    }
    pub fn set_visible(&self, name: Option<&str>) {
        let active = match name {
            Some("discover") => Some(0),
            Some("requests") => Some(1),
            Some("downloads") => Some(2),
            _ => None,
        };
        if self.inner.borrow().active != active {
            replace_view(&self.inner);
            self.inner.borrow_mut().active = active;
            refresh(&self.inner);
        }
    }
}
impl Drop for SeerrHub {
    fn drop(&mut self) {
        if let Some(source) = self.source.take() {
            source.remove();
        }
        let window = self.inner.borrow().settings.clone();
        if let Some(window) = window {
            window.close();
        }
    }
}

fn show_account(hub: &Shared) {
    if let Some(window) = hub.borrow().settings.as_ref() {
        window.present();
        return;
    }
    let window = gtk::Window::builder()
        .title(tr!("seerr-account"))
        .default_width(480)
        .modal(true)
        .build();
    if let Some(parent) = get_main_window() {
        window.set_transient_for(Some(&parent));
    }
    let content = column(18);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroll.set_child(Some(&content));
    window.set_default_height(720);
    window.set_child(Some(&scroll));
    content.append(&label(tr!("seerr-session-memory")));
    let url = gtk::Entry::new();
    url.set_placeholder_text(Some("https://seerr.example.org/"));
    let username = gtk::Entry::new();
    let method = gtk::DropDown::from_strings(&[tr!("seerr-local"), tr!("seerr-jellyfin")]);
    let saved = CONFIG.read().seerr.clone();
    let suggested = hub
        .borrow()
        .suggested_url
        .clone()
        .filter(|_| saved.is_none());
    if let Some(saved) = &saved {
        url.set_text(saved.url());
        username.set_text(saved.username.as_deref().unwrap_or(""));
        method.set_selected(if saved.login_method == LoginMethod::Local {
            0
        } else {
            1
        });
    } else if let Some(suggested) = &suggested {
        url.set_text(suggested);
    }
    let connection_hint = label("");
    connection_hint.set_visible(false);
    let password = gtk::PasswordEntry::new();
    password.set_show_peek_icon(true);
    for (title, widget) in [
        (tr!("seerr-url"), url.clone().upcast::<gtk::Widget>()),
        (tr!("seerr-method"), method.clone().upcast()),
        (tr!("seerr-username"), username.clone().upcast()),
        (tr!("seerr-password"), password.clone().upcast()),
    ] {
        let group = column(6);
        group.append(&label(title));
        widget.update_property(&[gtk::accessible::Property::Label(title)]);
        group.append(&widget);
        content.append(&group);
    }
    let remember = check_button(tr!("seerr-remember"));
    remember.set_active(
        saved
            .as_ref()
            .is_some_and(|config| config.remember_password),
    );
    content.append(&remember);
    content.append(&label(tr!("seerr-remember-description")));
    content.append(&connection_hint);
    let warning = label(tr!("seerr-http-warning"));
    let acknowledge = check_button(tr!("seerr-http-acknowledge"));
    if let Some(child) = acknowledge.child().and_downcast::<gtk::Label>() {
        child.set_wrap(true);
    }
    content.append(&warning);
    content.append(&acknowledge);
    let update_warning = {
        let warning = warning.clone();
        let acknowledge = acknowledge.clone();
        move |url: &gtk::Entry| {
            acknowledge.set_active(false);
            let cleartext = ConnectionConfig::new(url.text().as_str(), LoginMethod::Local, None)
                .is_ok_and(|c| c.transport_security() == TransportSecurity::CleartextHttp);
            warning.set_visible(cleartext);
            acknowledge.set_visible(cleartext);
        }
    };
    update_warning(&url);
    url.connect_changed(update_warning);
    {
        let hint = connection_hint.downgrade();
        url.connect_changed(move |_| {
            if let Some(hint) = hint.upgrade() {
                hint.set_visible(false);
            }
        });
    }
    if let Some(candidate) = suggested {
        connection_hint.set_label(tr!("seerr-auto-checking"));
        connection_hint.set_visible(true);
        let (tx, rx) = tokio::sync::oneshot::channel();
        let probe_url = candidate.clone();
        relm4::spawn(async move {
            let _ = tx.send(probe_seerr_url(&probe_url).await);
        });
        let weak_url = url.downgrade();
        let weak_hint = connection_hint.downgrade();
        let weak_window = window.downgrade();
        glib::spawn_future_local(async move {
            let Ok(found) = rx.await else {
                return;
            };
            if weak_window.upgrade().is_none() {
                return;
            }
            let (Some(url), Some(hint)) = (weak_url.upgrade(), weak_hint.upgrade()) else {
                return;
            };
            // Never overwrite a saved preference or a manually edited address.
            if url.text().as_str() == candidate {
                hint.set_label(tr!(if found {
                    "seerr-auto-found"
                } else {
                    "seerr-auto-unconfirmed"
                }));
                hint.set_visible(true);
            }
        });
    }
    let error = label("");
    error.add_css_class("error");
    content.append(&error);
    let actions = row(12);
    let login = button(tr!("seerr-sign-in"));
    login.add_css_class("suggested-action");
    let logout = button(tr!("seerr-sign-out"));
    logout.set_sensitive(hub.borrow().user.is_some());
    actions.append(&login);
    actions.append(&logout);
    content.append(&actions);
    let weak = Rc::downgrade(hub);
    let login_error = error.clone();
    login.connect_clicked(move |_| {
        let Some(hub) = weak.upgrade() else {
            return;
        };
        let mut config = match ConnectionConfig::new(
            url.text().trim(),
            if method.selected() == 0 {
                LoginMethod::Local
            } else {
                LoginMethod::Jellyfin
            },
            Some(username.text().trim().to_string()),
        ) {
            Ok(config) => config,
            Err(e) => {
                login_error.set_label(&error_text(e));
                return;
            }
        };
        if config.transport_security() == TransportSecurity::CleartextHttp
            && !acknowledge.is_active()
        {
            login_error.set_label(tr!("seerr-http-required"));
            return;
        }
        if username.text().trim().is_empty() || password.text().is_empty() {
            login_error.set_label(tr!("seerr-credentials-required"));
            return;
        }
        config.remember_password = remember.is_active();
        {
            let mut stored = CONFIG.write();
            stored.seerr = Some(config.clone());
            if stored.save().is_err() {
                login_error.set_label(tr!("seerr-save-error"));
                return;
            }
        }
        let secret = password.text().to_string();
        password.set_text("");
        login_error.set_label("");
        reset_account(&hub);
        send(&hub, Operation::Login(config, secret));
    });
    on_click(&logout, hub, |hub| {
        {
            let mut stored = CONFIG.write();
            if let Some(config) = stored.seerr.as_mut() {
                config.remember_password = false;
            }
            let _ = stored.save();
        }
        reset_account(hub);
        send(hub, Operation::Logout);
        let window = hub.borrow().settings.clone();
        if let Some(window) = window {
            window.close();
        }
    });
    let weak = Rc::downgrade(hub);
    window.connect_close_request(move |_| {
        if let Some(hub) = weak.upgrade() {
            let mut model = hub.borrow_mut();
            model.settings = None;
            model.settings_error = None;
            model.login_button = None;
        }
        glib::Propagation::Proceed
    });
    let mut model = hub.borrow_mut();
    model.settings = Some(window.clone());
    model.settings_error = Some(error);
    model.login_button = Some(login);
    drop(model);
    window.present();
}
fn reset_account(hub: &Shared) {
    let mut model = hub.borrow_mut();
    model.state.account_changed();
    model
        .service
        .account
        .store(model.state.account, Ordering::Relaxed);
    model
        .service
        .view
        .store(model.state.view, Ordering::Relaxed);
    let removal = model.removal.take().map(|w| w.borrow().window.clone());
    model.user = None;
    model.saved_grid = None;
    model.grid_images.clear();
    model.request_focus = false;
    model.back.set_visible(false);
    model.controls[0].set_visible(true);
    for button in &model.account_buttons {
        button.set_tooltip_text(Some(tr!("seerr-session-memory")));
    }
    model.poll_allowed = false;
    for spinner in &model.spinners {
        spinner.set_spinning(false);
    }
    if let Some(button) = &model.login_button {
        button.set_sensitive(true);
    }
    model.posters.clear();
    model.page = 1;
    model.request_skip = 0;
    for body in &model.bodies {
        clear(body);
        body.set_sensitive(true);
    }
    for control in &model.controls {
        control.set_sensitive(false);
    }
    drop(model);
    if let Some(window) = removal {
        window.close();
    }
}
fn receive(hub: &Shared, reply: Reply) {
    if hub.borrow().state.account != reply.account {
        return;
    }
    if matches!(&reply.result, Ok(Data::Removal(..))) {
        if let Ok(Data::Removal(id, result)) = reply.result {
            if matches!(&result, Err(Error::Unauthorized | Error::NotAuthenticated)) {
                reset_account(hub);
                crate::app::APP_BROKER.send(crate::app::AppInput::Toast(
                    tr!("seerr-error-auth").to_string(),
                    None,
                ));
            } else {
                removal_ui::receive(hub, id, result);
            }
        }
        return;
    }
    if matches!(&reply.result, Ok(Data::Artwork(..))) {
        return;
    }
    let auth = reply.auth;
    let expired = matches!(
        &reply.result,
        Err(Error::Unauthorized | Error::NotAuthenticated)
    ) || (auth && matches!(&reply.result, Err(Error::Forbidden)));
    if expired {
        reset_account(hub);
        let text = if auth {
            tr!("seerr-error-login").to_string()
        } else {
            tr!("seerr-error-auth").to_string()
        };
        status(hub, &text);
        if let Some(label) = &hub.borrow().settings_error {
            label.set_label(&text);
        }
        return;
    }
    if !auth && !hub.borrow().state.accepts(reply.account, reply.view) {
        return;
    }
    if let Ok(Data::Poster(id, bytes)) = &reply.result {
        if let Some(picture) = hub.borrow().posters.get(id) {
            if let Ok(texture) = gtk::gdk::Texture::from_bytes(&glib::Bytes::from(bytes.as_slice()))
            {
                picture.set_paintable(Some(&texture));
            }
        }
        return;
    }
    {
        let mut model = hub.borrow_mut();
        model.state.busy = false;
        model.back.set_sensitive(true);
        model.last_poll = Instant::now();
        for spinner in &model.spinners {
            spinner.set_spinning(false);
        }
        for control in &model.controls {
            control.set_sensitive(model.user.is_some());
        }
        for body in &model.bodies {
            body.set_sensitive(true);
        }
        if let Some(button) = &model.login_button {
            button.set_sensitive(true);
        }
    }
    match reply.result {
        Err(error) => {
            if matches!(error, Error::Forbidden | Error::Http(404 | 503)) {
                hub.borrow_mut().poll_allowed = false;
            }
            let mut text = error_text(error);
            if hub.borrow().active == Some(2) {
                text = format!("{} — {text}", tr!("seerr-stale"));
            }
            status(hub, &text);
            if let Some(label) = &hub.borrow().settings_error {
                label.set_label(&text);
            }
        }
        Ok(Data::Login(user, persistence)) => {
            let identity = user
                .username
                .as_deref()
                .or(user.email.as_deref())
                .or(user.jellyfin_username.as_deref())
                .unwrap_or(tr!("seerr-account"))
                .to_string();
            for button in &hub.borrow().account_buttons {
                button.set_tooltip_text(Some(&identity));
            }
            hub.borrow_mut().user = Some(user);
            hub.borrow_mut().poll_allowed = true;
            let warning = match persistence {
                CredentialStatus::SaveFailed => Some(tr!("seerr-keyring-save-failed")),
                CredentialStatus::ClearFailed => Some(tr!("seerr-keyring-clear-failed")),
                _ => None,
            };
            if let Some(warning) = warning {
                {
                    let mut stored = CONFIG.write();
                    if let Some(config) = stored.seerr.as_mut() {
                        config.remember_password = false;
                    }
                    let _ = stored.save();
                }
                if let Some(label) = &hub.borrow().settings_error {
                    label.set_label(warning);
                }
                status(hub, warning);
            } else {
                let window = hub.borrow().settings.clone();
                if let Some(window) = window {
                    window.close();
                }
                status(hub, tr!("seerr-connected"));
            }
            refresh(hub);
        }
        Ok(Data::RestoreUnavailable(failed)) => {
            status(
                hub,
                tr!(if failed {
                    "seerr-keyring-unavailable"
                } else {
                    "seerr-keyring-missing"
                }),
            );
        }
        Ok(Data::Logout(result, clear_failed)) => {
            let mut text = match result {
                Ok(()) => tr!("seerr-signed-out").to_string(),
                Err(error) => format!("{} {}", tr!("seerr-logout-local-only"), error_text(error)),
            };
            if clear_failed {
                text.push_str(&format!(" {}", tr!("seerr-keyring-clear-failed")));
            }
            status(hub, &text);
        }
        Ok(Data::Discover(page)) => render_discover(hub, page),
        Ok(Data::Movie(details, request, is4k)) => render_movie(hub, details, request, is4k),
        Ok(Data::Tv(details, request, is4k)) => render_tv(hub, details, request, is4k),
        Ok(Data::Requested(request)) => {
            hub.borrow_mut().saved_grid = None; // Refresh availability after a mutation.
            status(
                hub,
                &format!(
                    "{} #{} — {}",
                    tr!("seerr-request-created"),
                    request.id,
                    request_status(request.status)
                ),
            );
        }
        Ok(Data::Requests(page)) => render_requests(hub, page),
        Ok(Data::Diagnostics(id, request, diagnostics)) => {
            render_diagnostics(hub, id, request, diagnostics)
        }
        Ok(Data::Releases(id, selection, releases)) => {
            render_releases(hub, id, selection, releases)
        }
        Ok(Data::Grabbed(result)) => {
            hub.borrow_mut().saved_grid = None; // Source selection also changes request availability.
            let active = hub.borrow().active.unwrap_or(0);
            clear(&hub.borrow().bodies[active]);
            status(hub, &result.message);
        }
        Ok(Data::Torrents(downloads)) => render_torrents(hub, downloads),
        Ok(Data::Poster(..) | Data::Artwork(..) | Data::Removal(..)) => {}
    }
}
fn active_body(hub: &Shared) -> gtk::Box {
    hub.borrow().bodies[hub.borrow().active.unwrap_or(0)].clone()
}
fn prepare_details(hub: &Shared) {
    {
        let mut model = hub.borrow_mut();
        if model.active == Some(0) {
            if model.saved_grid.is_none() {
                if let Some(widget) = model.bodies[0].first_child() {
                    if widget.is::<gtk::FlowBox>() {
                        let scroll = scroll_for(&model.bodies[0])
                            .map(|s| s.vadjustment().value())
                            .unwrap_or(0.0);
                        model.bodies[0].remove(&widget);
                        let posters = std::mem::take(&mut model.posters);
                        model.saved_grid = Some(SavedGrid {
                            widget,
                            posters,
                            scroll,
                            images: model.grid_images.clone(),
                        });
                    }
                }
            }
            model.controls[0].set_visible(false);
            model.back.set_visible(true);
        }
    }
    replace_view(hub);
}
fn details(hub: &Shared, kind: MediaType, id: u64, request: Option<u64>, is4k: bool) {
    prepare_details(hub);
    send(hub, Operation::Details(kind, id, request, is4k));
}

fn discovery_caption(info: &DiscoveryCardInfo) -> gtk::Box {
    let caption = column(5);
    caption.set_width_request(POSTER_WIDTH);
    caption.add_css_class("movie-card-caption");
    let text = |value: &str, class: &str| {
        let text = label(value);
        text.set_selectable(false);
        text.set_wrap(false);
        text.set_ellipsize(gtk::pango::EllipsizeMode::End);
        text.set_max_width_chars(1);
        text.set_hexpand(true);
        text.add_css_class(class);
        text
    };
    let title = text(&info.title, "movie-card-title");
    title.set_wrap(true);
    title.set_lines(2);
    title.set_height_request(POSTER_TITLE_HEIGHT);
    title.set_valign(gtk::Align::Start);
    title.set_yalign(0.0);
    caption.append(&title);
    caption.append(&text(&info.subtitle, "movie-card-subtitle"));
    let names = info.genres.join(" · ");
    let genre_text = if names.is_empty() && info.kind.is_some() {
        tr!("seerr-discover-genres-missing").to_string()
    } else {
        names.clone()
    };
    let genre = text(&genre_text, "movie-card-secondary");
    genre.set_height_request(17);
    if !names.is_empty() {
        genre.set_tooltip_text(Some(
            &tr!("seerr-discover-genres",{"genres"=>names.as_str()}),
        ));
    }
    caption.append(&genre);
    let ratings = row(5);
    ratings.add_css_class("movie-card-ratings");
    if let Some(rating) = &info.rating {
        ratings.set_tooltip_text(info.rating_tooltip.as_deref());
        let star = gtk::Image::from_icon_name("star-large");
        star.set_pixel_size(14);
        star.add_css_class("movie-rating-star");
        ratings.append(&star);
        let score = text(rating, "movie-card-score");
        score.set_max_width_chars(-1);
        score.set_hexpand(false);
        ratings.append(&score);
        let scale = text("/10", "movie-card-secondary");
        scale.set_max_width_chars(-1);
        scale.set_hexpand(false);
        ratings.append(&scale);
        let source = text("TMDB", "movie-card-secondary");
        source.set_hexpand(true);
        source.set_xalign(1.0);
        ratings.append(&source);
    } else if info.kind.is_some() {
        ratings.append(&text(tr!("seerr-discover-unrated"), "movie-card-secondary"));
    }
    caption.append(&ratings);
    let availability = column(5);
    availability.set_height_request(41); // Reserve both standard/4K lines across the row.
    for line in &info.availability {
        let status = row(5);
        status.add_css_class(line.class);
        let icon = gtk::Image::from_icon_name(line.icon);
        icon.set_pixel_size(14);
        status.append(&icon);
        let state = text(&line.text, "movie-card-watch-label");
        state.set_hexpand(true);
        status.append(&state);
        availability.append(&status);
    }
    caption.append(&availability);
    caption
}

fn render_discover(hub: &Shared, mut page: Page<DiscoveryResult>) {
    let kind = discovery_kind(hub);
    page.results.retain(|item| item.media_type.matches(kind));
    hub.borrow_mut().grid_images = page
        .results
        .iter()
        .enumerate()
        .filter_map(|(i, item)| Some((i as u64, item.poster_path.clone()?)))
        .collect();
    let body = active_body(hub);
    clear(&body);
    {
        let mut model = hub.borrow_mut();
        model.page = page.page;
        model.total_pages = page.total_pages;
        model.previous.set_sensitive(page.page > 1);
        model.next.set_sensitive(page.page < page.total_pages);
        model
            .page_label
            .set_label(&format!("{}/{}", page.page, page.total_pages.max(1)));
    }
    status(hub, tr!("seerr-connected"));
    if page.results.is_empty() {
        let text = match kind {
            MediaType::Movie => tr!("seerr-empty-movies-page").to_string(),
            MediaType::Tv => tr!("seerr-empty-tv-page").to_string(),
        };
        body.append(&label(&text));
        return;
    }
    let grid = gtk::FlowBox::new();
    grid.set_selection_mode(gtk::SelectionMode::None);
    grid.set_max_children_per_line(POSTER_COLUMNS);
    grid.set_min_children_per_line(1);
    grid.set_row_spacing(POSTER_ROW_SPACING);
    grid.set_column_spacing(POSTER_COLUMN_SPACING);
    grid.set_homogeneous(true);
    body.append(&grid);
    for (index, item) in page.results.into_iter().enumerate() {
        let info = DiscoveryCardInfo::new(&item);
        let card = column(10);
        card.add_css_class("media-tile");
        card.add_css_class("movie-grid-card");
        card.add_css_class("discovery-card");
        card.set_halign(gtk::Align::Fill);
        card.set_valign(gtk::Align::Start);
        let picture = gtk::Picture::new();
        picture.set_size_request(POSTER_WIDTH, POSTER_HEIGHT);
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_alternative_text(Some(&info.title));
        let art = gtk::Overlay::new();
        art.set_size_request(POSTER_WIDTH, POSTER_HEIGHT);
        art.set_halign(gtk::Align::Center);
        art.set_overflow(gtk::Overflow::Hidden);
        art.add_css_class("discovery-art");
        let fallback = gtk::Image::from_icon_name("video-x-generic-symbolic");
        fallback.set_pixel_size(48);
        art.set_child(Some(&fallback));
        art.add_overlay(&picture);
        let caption = discovery_caption(&info);
        if let Some(kind) = info.kind {
            let open = button("");
            open.add_css_class("movie-card-details");
            open.add_css_class("discovery-poster");
            open.set_halign(gtk::Align::Center);
            open.set_child(Some(&art));
            open.set_tooltip_text(Some(&info.summary));
            open.update_property(&[gtk::accessible::Property::Label(&format!(
                "{}: {}",
                tr!("seerr-details"),
                info.title
            ))]);
            on_click(&open, hub, move |hub| {
                details(hub, kind, item.id, None, false)
            });
            card.append(&open);
            let open = button("");
            open.add_css_class("movie-card-details");
            open.set_halign(gtk::Align::Center);
            open.set_width_request(POSTER_WIDTH);
            open.set_child(Some(&caption));
            open.set_tooltip_text(Some(&info.summary));
            on_click(&open, hub, move |hub| {
                details(hub, kind, item.id, None, false)
            });
            card.append(&open);
        } else {
            card.append(&art);
            caption.set_halign(gtk::Align::Center);
            caption.set_tooltip_text(Some(&info.summary));
            card.append(&caption);
        }
        // Keep external navigation outside the details buttons. No video is
        // fetched/autoplayed, and this cannot submit a request or start a grab.
        if let Some(kind) = info.kind {
            let actions = column(6);
            actions.set_width_request(POSTER_WIDTH);
            actions.set_halign(gtk::Align::Center);
            if let Some(url) = &info.trailer_search {
                let trailer =
                    gtk::LinkButton::with_label(url, tr!("seerr-discover-trailer-search"));
                trailer.add_css_class("discovery-trailer");
                trailer.set_tooltip_text(Some(tr!("seerr-discover-trailer-hint")));
                trailer.update_property(&[gtk::accessible::Property::Label(
                    &tr!("seerr-discover-trailer-label",{"title"=>info.title.as_str()}),
                )]);
                actions.append(&trailer);
            }
            let text = if item.media_info.is_some() {
                tr!("seerr-request-options").to_string()
            } else {
                tr!("seerr-request").to_string()
            };
            let request = button(&text);
            request.add_css_class("suggested-action");
            request.update_property(&[gtk::accessible::Property::Label(&format!(
                "{}: {}",
                tr!("seerr-request"),
                info.title
            ))]);
            on_click(&request, hub, move |hub| {
                hub.borrow_mut().request_focus = true;
                details(hub, kind, item.id, None, false);
            });
            actions.append(&request);
            card.append(&actions);
        }
        hub.borrow_mut().posters.insert(index as u64, picture);
        grid.insert(&card, -1);
    }
}
fn detail_header(hub: &Shared) -> gtk::Box {
    let body = active_body(hub);
    clear(&body);
    if hub.borrow().active == Some(0) {
        hub.borrow().back.set_visible(true);
        hub.borrow().controls[0].set_visible(false);
    } else {
        let back = button(tr!("seerr-back-results"));
        back.set_halign(gtk::Align::Start);
        on_click(&back, hub, refresh);
        body.append(&back);
    }
    if let Some(scroll) = scroll_for(&body) {
        scroll.vadjustment().set_value(0.0);
    }
    body
}
fn confirm_request(hub: &Shared, selection: RequestSelection, title: &str) {
    let description = match &selection {
        RequestSelection::Movie { is4k, .. } => {
            format!("{title}\n{}: {}", tr!("seerr-4k"), yes_no(*is4k))
        }
        RequestSelection::Tv { seasons, is4k, .. } => format!(
            "{title}\n{}: {}\n{}: {}",
            tr!("seerr-seasons"),
            seasons
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            tr!("seerr-4k"),
            yes_no(*is4k)
        ),
    };
    confirm(
        hub,
        tr!("seerr-confirm-request"),
        &description,
        tr!("seerr-submit-request"),
        move |hub| {
            send(hub, Operation::Request(selection.clone()));
        },
    );
}
fn confirm(
    hub: &Shared,
    title: &str,
    description: &str,
    action: &str,
    callback: impl Fn(&Shared) + 'static,
) {
    let dialog =
        adw::MessageDialog::new(get_main_window().as_ref(), Some(title), Some(description));
    dialog.add_response("cancel", tr!("seerr-cancel"));
    dialog.add_response("confirm", action);
    dialog.set_close_response("cancel");
    dialog.set_default_response(Some("cancel"));
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);
    let weak = Rc::downgrade(hub);
    let account = hub.borrow().state.account;
    let view = hub.borrow().state.view;
    dialog.connect_response(None, move |_, response| {
        if response == "confirm" {
            if let Some(hub) = weak.upgrade() {
                if hub.borrow().state.accepts(account, view) {
                    callback(&hub);
                }
            }
        }
    });
    dialog.present();
}
fn render_movie(hub: &Shared, details: MovieDetails, request_id: Option<u64>, source_is4k: bool) {
    let body = detail_view::header(
        hub,
        &details.title,
        details.release_date.as_deref(),
        details.runtime,
        &details.genres,
        details.vote_average,
        details.poster_path.as_deref(),
        &details.metadata,
        details.media_info.as_ref(),
        MediaType::Movie,
        details.id,
        details.imdb_id.as_deref(),
    );
    let actions = row(12);
    let is4k = check_button(tr!("seerr-4k"));
    is4k.set_active(source_is4k);
    actions.append(&is4k);
    let request = button(tr!("seerr-request"));
    request.add_css_class("suggested-action");
    let tmdb_id = details.id;
    let title = details.title.clone();
    let request4k = is4k.clone();
    on_click(&request, hub, move |hub| {
        confirm_request(
            hub,
            RequestSelection::Movie {
                tmdb_id,
                is4k: request4k.is_active(),
            },
            &title,
        )
    });
    actions.append(&request);
    if hub
        .borrow()
        .user
        .as_ref()
        .is_some_and(User::can_manage_requests)
    {
        let source = button(tr!("seerr-custom-sources"));
        on_click(&source, hub, move |hub| {
            replace_view(hub);
            send(
                hub,
                Operation::Search(
                    tmdb_id,
                    SourceSelection::Movie(MovieDownloadSelection {
                        is4k: is4k.is_active(),
                        request_id,
                    }),
                ),
            );
        });
        actions.append(&source);
    }
    body.append(&actions);
    if hub.borrow().request_focus {
        request.grab_focus();
    }
    hub.borrow_mut().request_focus = false;
    detail_view::story(&body, details.overview.as_deref(), &details.metadata);
    detail_view::movie_facts(&body, &details);
    detail_view::credits(&body, &details.metadata);
}
fn render_tv(hub: &Shared, details: TvDetails, request_id: Option<u64>, source_is4k: bool) {
    let root = detail_view::header(
        hub,
        &details.name,
        details.first_air_date.as_deref(),
        details.episode_run_time.iter().copied().find(|n| *n > 0),
        &details.genres,
        details.vote_average,
        details.poster_path.as_deref(),
        &details.metadata,
        details.media_info.as_ref(),
        MediaType::Tv,
        details.id,
        None,
    );
    let body = column(10);
    body.set_margin_all(12);
    let chooser = adw::ExpanderRow::new();
    chooser.set_use_markup(false);
    chooser.set_title(tr!("seerr-detail-season-requests"));
    chooser.set_subtitle(tr!("seerr-select-seasons"));
    chooser.add_row(&body);
    chooser.set_expanded(hub.borrow().request_focus);
    hub.borrow_mut().request_focus = false;
    let group = adw::PreferencesGroup::new();
    group.add(&chooser);
    root.append(&group);
    detail_view::story(&root, details.overview.as_deref(), &details.metadata);
    detail_view::tv_facts(&root, &details);
    detail_view::credits(&root, &details.metadata);
    let availability: HashMap<_, _> = details
        .media_info
        .as_ref()
        .map(|info| {
            info.seasons
                .iter()
                .map(|s| (s.season_number, media_availability(s.status, s.status4k)))
                .collect()
        })
        .unwrap_or_default();
    let seasons: Vec<_> = details
        .seasons
        .into_iter()
        .map(|season| {
            let name = season
                .name
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!(" — {s}"))
                .unwrap_or_default();
            let count = season
                .episode_count
                .map(|n| format!(" ({n} {})", tr!("seerr-episodes")))
                .unwrap_or_default();
            let check = check_button(&format!(
                "{} {}{name}{count}",
                tr!("seerr-season"),
                season.season_number
            ));
            body.append(&check);
            if let Some(value) = availability.get(&season.season_number) {
                body.append(&label(value));
            }
            if let Some(date) = season.air_date.as_deref().filter(|s| !s.is_empty()) {
                body.append(&label(date));
            }
            (season.season_number, check)
        })
        .collect();
    let is4k = check_button(tr!("seerr-4k"));
    is4k.set_active(source_is4k);
    body.append(&is4k);
    let request = button(tr!("seerr-request"));
    request.set_sensitive(false);
    request.add_css_class("suggested-action");
    for (_, check) in &seasons {
        let request = request.downgrade();
        let checks: Vec<_> = seasons.iter().map(|(_, c)| c.downgrade()).collect();
        check.connect_toggled(move |_| {
            if let Some(request) = request.upgrade() {
                request.set_sensitive(
                    checks
                        .iter()
                        .filter_map(|c| c.upgrade())
                        .any(|c| c.is_active()),
                );
            }
        });
    }
    let request4k = is4k.clone();
    on_click(&request, hub, move |hub| {
        let selected: Vec<_> = seasons
            .iter()
            .filter(|(_, c)| c.is_active())
            .map(|(n, _)| *n)
            .collect();
        if !selected.is_empty() {
            confirm_request(
                hub,
                RequestSelection::Tv {
                    tmdb_id: details.id,
                    seasons: selected,
                    is4k: request4k.is_active(),
                },
                &details.name,
            );
        }
    });
    body.append(&request);
    if hub
        .borrow()
        .user
        .as_ref()
        .is_some_and(User::can_manage_requests)
    {
        let source = button(tr!("seerr-custom-sources"));
        on_click(&source, hub, move |hub| {
            replace_view(hub);
            send(
                hub,
                Operation::Diagnostics(details.id, is4k.is_active(), request_id),
            );
        });
        body.append(&source);
    } else {
        body.append(&label(tr!("seerr-manager-required")));
    }
}
fn render_requests(hub: &Shared, page: RequestPage) {
    request_view::render(hub, page);
}
fn render_diagnostics(
    hub: &Shared,
    id: u64,
    request: Option<u64>,
    diagnostics: TvDownloadDiagnostics,
) {
    let body = active_body(hub);
    clear(&body);
    let back = button(tr!("seerr-back-details"));
    let is4k = diagnostics.is4k;
    on_click(&back, hub, move |hub| {
        details(hub, MediaType::Tv, id, request, is4k)
    });
    body.append(&back);
    let heading = label(tr!("seerr-custom-sources"));
    heading.add_css_class("title-1");
    body.append(&heading);
    body.append(&label(&format!(
        "{} · {}: {}",
        diagnostics.server_name,
        tr!("seerr-monitored"),
        yes_no(diagnostics.series_monitored)
    )));
    if diagnostics.queue_truncated || diagnostics.history_truncated {
        body.append(&label(tr!("seerr-diagnostics-truncated")));
    }
    body.append(&label(tr!("seerr-source-selection-hint")));
    let selector = column(8);
    body.append(&selector);
    let mut selections = Vec::new();
    let mut names = Vec::new();
    let mut season_numbers: Vec<_> = diagnostics
        .episodes
        .iter()
        .map(|e| e.season_number)
        .collect();
    season_numbers.sort_unstable();
    season_numbers.dedup();
    for season in season_numbers {
        names.push(format!(
            "{} {} — {}",
            tr!("seerr-season"),
            season,
            tr!("seerr-full-season")
        ));
        selections.push(DownloadSelection {
            is4k,
            request_id: request,
            season_number: season,
            episode_id: None,
        });
    }
    for episode in diagnostics.episodes {
        names.push(format!(
            "S{} E{} — {}",
            episode.season_number, episode.episode_number, episode.title
        ));
        selections.push(DownloadSelection {
            is4k,
            request_id: request,
            season_number: episode.season_number,
            episode_id: Some(episode.id),
        });
        let episode_group = column(4);
        let title = label(&format!(
            "S{} E{} — {}",
            episode.season_number, episode.episode_number, episode.title
        ));
        title.add_css_class("heading");
        episode_group.append(&title);
        episode_group.append(&label(&format!(
            "{}: {} · {}: {} · {}: {}",
            tr!("seerr-monitored"),
            yes_no(episode.monitored),
            tr!("seerr-file"),
            yes_no(episode.has_file),
            tr!("seerr-state"),
            episode_state(episode.state)
        )));
        for queue in episode.queue {
            episode_group.append(&label(&format!(
                "{} · {} · {} · {} · {}",
                queue.title,
                queue.status,
                queue.tracked_status,
                queue.tracked_state,
                queue
                    .progress
                    .map_or_else(|| tr!("seerr-unknown").into(), |p| format!("{p:.1}%"))
            )));
            for message in queue.messages {
                episode_group.append(&label(&message));
            }
        }
        for history in episode.history {
            episode_group.append(&label(&format!(
                "{} · {} · {}\n{}",
                history.date,
                history.event_type,
                history.title,
                history.message.unwrap_or_default()
            )));
        }
        body.append(&episode_group);
    }
    if selections.is_empty() {
        body.append(&label(tr!("seerr-no-episodes")));
        return;
    }
    let chooser =
        gtk::DropDown::from_strings(&names.iter().map(String::as_str).collect::<Vec<_>>());
    selector.append(&chooser);
    let search = button(tr!("seerr-search-releases"));
    search.add_css_class("suggested-action");
    on_click(&search, hub, move |hub| {
        let selection = selections[chooser.selected() as usize].clone();
        replace_view(hub);
        send(hub, Operation::Search(id, SourceSelection::Tv(selection)));
    });
    selector.append(&search);
    status(hub, tr!("seerr-connected"));
}
fn render_releases(hub: &Shared, id: u64, selection: SourceSelection, results: TvDownloadSearch) {
    let body = active_body(hub);
    clear(&body);
    let back = button(tr!(if matches!(&selection, SourceSelection::Movie(_)) {
        "seerr-back-details"
    } else {
        "seerr-back-diagnostics"
    }));
    let back_selection = selection.clone();
    on_click(&back, hub, move |hub| match &back_selection {
        SourceSelection::Tv(value) => {
            replace_view(hub);
            send(
                hub,
                Operation::Diagnostics(id, value.is4k, value.request_id),
            );
        }
        SourceSelection::Movie(value) => {
            details(hub, MediaType::Movie, id, value.request_id, value.is4k)
        }
    });
    body.append(&back);
    body.append(&label(&format!(
        "{} {}\n{}",
        tr!("seerr-token-expires"),
        results.expires_at,
        tr!("seerr-token-hint")
    )));
    if results.truncated {
        body.append(&label(tr!("seerr-releases-truncated")));
    }
    if results.releases.is_empty() {
        body.append(&label(tr!("seerr-no-releases")));
    }
    for release in results.releases {
        let group = column(8);
        let title = label(&release.title);
        title.add_css_class("heading");
        group.append(&title);
        let mut metadata = format!(
            "{} · {} · {} · {}: {} · {}",
            release.quality,
            format_file_size(release.size),
            release.indexer,
            tr!("seerr-seeders"),
            release
                .seeders
                .filter(|s| *s >= 0)
                .map_or_else(|| tr!("seerr-unknown").into(), |s| s.to_string()),
            release.protocol
        );
        if !release.episode_numbers.is_empty() {
            metadata.push_str(&format!(" · {:?}", release.episode_numbers));
        }
        group.append(&label(&metadata));
        if release.full_season {
            group.append(&label(tr!("seerr-full-season")));
        }
        for reason in &release.rejections {
            group.append(&label(reason));
        }
        let rejected = check_button(tr!("seerr-confirm-rejected"));
        let duplicate = check_button(tr!("seerr-confirm-duplicate"));
        if release.rejected {
            group.append(&label(tr!("seerr-rejected-warning")));
            group.append(&rejected);
        }
        if release.duplicate {
            group.append(&label(tr!("seerr-duplicate-warning")));
            group.append(&duplicate);
        }
        if let Some(reason) = &release.replacement_blocked {
            group.append(&label(reason));
        }
        let grab = button(tr!("seerr-grab"));
        if let Some(token) = release.token {
            let token = Rc::new(RefCell::new(Some(token)));
            let selection = selection.clone();
            let expires_at = results.expires_at.clone();
            let weak_grab = grab.downgrade();
            on_click(&grab, hub, move |hub| {
                let expired = chrono::DateTime::parse_from_rfc3339(&expires_at).map_or(true, |t| {
                    t.with_timezone(&chrono::Utc) <= chrono::Utc::now()
                });
                if expired {
                    status(hub, tr!("seerr-token-expired"));
                    return;
                }
                if (release.rejected && !rejected.is_active())
                    || (release.duplicate && !duplicate.is_active())
                {
                    status(hub, tr!("seerr-confirm-warnings"));
                    return;
                }
                let token = token.clone();
                let selection = selection.clone();
                let weak_grab = weak_grab.clone();
                let rejected = rejected.is_active();
                let duplicate = duplicate.is_active();
                let expires_at = expires_at.clone();
                let description = if release.replacements.is_empty() {
                    format!("{}\n{}", release.title, tr!("seerr-source-keep-files"))
                } else {
                    format!(
                        "{}\n{}\n{}",
                        release.title,
                        tr!("seerr-source-replace-confirm"),
                        release.replacements.join("\n")
                    )
                };
                confirm(
                    hub,
                    tr!("seerr-confirm-grab"),
                    &description,
                    tr!("seerr-grab"),
                    move |hub| {
                        if chrono::DateTime::parse_from_rfc3339(&expires_at).map_or(true, |t| {
                            t.with_timezone(&chrono::Utc) <= chrono::Utc::now()
                        }) {
                            status(hub, tr!("seerr-token-expired"));
                            return;
                        }
                        if let Some(token) = token.borrow_mut().take() {
                            if let Some(grab) = weak_grab.upgrade() {
                                grab.set_sensitive(false);
                            }
                            send(
                                hub,
                                Operation::Grab(id, selection.clone(), token, rejected, duplicate),
                            );
                        }
                    },
                );
            });
        } else {
            grab.set_sensitive(false);
            grab.set_tooltip_text(Some(tr!("seerr-not-grabbable")));
        }
        group.append(&grab);
        group.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        body.append(&group);
    }
    status(hub, tr!("seerr-connected"));
}
fn render_torrents(hub: &Shared, downloads: TorrentDownloads) {
    let body = active_body(hub);
    clear(&body);
    status(
        hub,
        &format!(
            "{} {} · {}",
            tr!("seerr-updated"),
            downloads.updated_at,
            tr!("seerr-read-only")
        ),
    );
    if downloads.torrents.is_empty() {
        body.append(&label(tr!("seerr-no-downloads")));
    }
    for torrent in downloads.torrents {
        let group = column(8);
        let title = label(&torrent.name);
        title.add_css_class("heading");
        group.append(&title);
        let progress = gtk::ProgressBar::new();
        progress.set_fraction(torrent.progress);
        progress.set_show_text(true);
        progress.set_text(Some(&format!("{:.1}%", torrent.progress * 100.0)));
        group.append(&progress);
        group.append(&label(&format!(
            "{} · {} / {} · {}: {} · {}: {}\n{}: {} · {}: {} · {}: {} · {}",
            torrent.state,
            format_file_size(torrent.downloaded),
            format_file_size(torrent.size),
            tr!("seerr-download-speed"),
            speed(torrent.download_speed),
            tr!("seerr-upload-speed"),
            speed(torrent.upload_speed),
            tr!("seerr-eta"),
            eta(torrent.eta),
            tr!("seerr-seeders"),
            torrent.seeds,
            tr!("seerr-peers"),
            torrent.peers,
            torrent.category
        )));
        if let Some(error) = torrent.error {
            let text = label(&error);
            text.add_css_class("error");
            group.append(&text);
        }
        let cancel = button(tr!("seerr-download-cancel"));
        cancel.set_halign(gtk::Align::Start);
        let hash = torrent.hash.clone();
        let name = torrent.name.clone();
        on_click(&cancel, hub, move |hub| {
            removal_ui::open_download(hub, hash.clone(), &name, false)
        });
        group.append(&cancel);
        let delete = button(tr!("seerr-download-delete"));
        delete.set_halign(gtk::Align::Start);
        delete.add_css_class("destructive-action");
        let hash = torrent.hash.clone();
        let name = torrent.name.clone();
        on_click(&delete, hub, move |hub| {
            removal_ui::open_download(hub, hash.clone(), &name, true)
        });
        group.append(&delete);
        group.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        body.append(&group);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn walk(root: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut result = vec![root.clone()];
        let mut child = root.first_child();
        while let Some(widget) = child {
            result.extend(walk(&widget));
            child = widget.next_sibling();
        }
        result
    }
    fn fixture<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> T {
        serde_json::from_value(value).unwrap()
    }
    fn settle() {
        let context = glib::MainContext::default();
        for _ in 0..20 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn capture(window: &gtk::Window, name: &str) {
        settle();
        let snapshot = gtk::Snapshot::new();
        let paintable = gtk::WidgetPaintable::new(Some(window));
        paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
        let node = snapshot.to_node().expect("visible GTK surface");
        let texture = window.renderer().unwrap().render_texture(&node, None);
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("local-dist");
        texture
            .save_to_png(directory.join(format!("seerr-ui-{name}.png")))
            .unwrap();
    }
    #[test]
    #[ignore = "requires a native GTK display; uses only synthetic sources"]
    fn native_movie_source_confirmation_keeps_files() {
        adw::init().unwrap();
        let resources = gtk::gio::Resource::load(crate::meson_config::RESOURCES_FILE).unwrap();
        gtk::gio::resources_register(&resources);
        let hub = SeerrHub::new(None);
        hub.inner.borrow_mut().active = Some(0);
        let window = gtk::Window::builder()
            .default_width(600)
            .default_height(600)
            .build();
        window.set_child(Some(&hub.pages()[0]));
        window.present();
        render_releases(
            &hub.inner,
            1,
            SourceSelection::Movie(MovieDownloadSelection {
                is4k: false,
                request_id: Some(18),
            }),
            fixture(
                json!({"expiresAt":"2099-01-01T00:00:00Z","truncated":false,"releases":[{"title":"New movie source","indexer":"Fixture","quality":"1080p","size":10,"seeders":1,"protocol":"torrent","fullSeason":false,"episodeNumbers":[],"rejections":[],"rejected":false,"duplicate":false,"token":"a".repeat(48),"replacements":["Old movie download"]}]}),
            ),
        );
        let grab = walk(active_body(&hub.inner).upcast_ref())
            .into_iter()
            .filter_map(|widget| widget.downcast::<gtk::Button>().ok())
            .find(|button| button.label().as_deref() == Some(tr!("seerr-grab").as_str()))
            .expect("movie source button");
        grab.emit_clicked();
        settle();
        let dialog = gtk::Window::list_toplevels()
            .into_iter()
            .find_map(|item| item.downcast::<adw::MessageDialog>().ok())
            .expect("movie source confirmation");
        assert!(dialog.body().contains("Old movie download"));
        assert!(dialog.body().contains("Downloaded files are kept"));
        dialog.response("cancel");
        hub.inner.borrow_mut().saved_grid = Some(SavedGrid {
            widget: gtk::Box::new(gtk::Orientation::Vertical, 0).upcast(),
            posters: HashMap::new(),
            scroll: 10.0,
            images: Vec::new(),
        });
        let account = hub.inner.borrow().state.account;
        let view = hub.inner.borrow().state.view;
        receive(
            &hub.inner,
            Reply {
                account,
                view,
                auth: false,
                result: Ok(Data::Grabbed(GrabAccepted {
                    message: "Source submitted; check progress".into(),
                    partial: false,
                })),
            },
        );
        assert!(hub.inner.borrow().saved_grid.is_none());
        assert_eq!(
            hub.inner.borrow().statuses[0].text(),
            "Source submitted; check progress"
        );
        window.close();
    }
    /// Explicit display-only run; fixtures never contact Seerr, TMDB or Jellyfin.
    #[test]
    #[ignore = "requires a native GTK display; run separately with --ignored --test-threads=1"]
    fn native_view_fixtures_and_account_lifecycle() {
        adw::init().unwrap();
        let resources = gtk::gio::Resource::load(crate::meson_config::RESOURCES_FILE).unwrap();
        gtk::gio::resources_register(&resources);
        relm4_icons::initialize_icons();
        let css = gtk::CssProvider::new();
        css.load_from_string(include_str!(concat!(env!("OUT_DIR"), "/app.css")));
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let hub = SeerrHub::new(None);
        let window = gtk::Window::builder()
            .title("Delfin · Seerr fixture review")
            .default_width(1360)
            .default_height(900)
            .build();
        window.set_resizable(false);
        window.set_size_request(1360, 900);
        window.set_child(Some(&hub.pages()[0]));
        {
            let mut model = hub.inner.borrow_mut();
            model.active = Some(0);
            model.user = Some(fixture(
                json!({"id": 1, "username": "Fixture user", "permissions": 16}),
            ));
            model.poll_allowed = false;
        }
        window.present();
        let discovery_fixture = json!({"page": 1, "totalPages": 2, "totalResults": 18, "results": [
            {"id": 1, "mediaType": "movie", "title": "13 Hours: The Secret Soldiers of Benghazi", "releaseDate": "2016-01-15", "voteAverage": 7.3, "voteCount": 1200, "genreIds": [28, 18, 10752], "originalLanguage": "en", "overview": "Synthetic metadata and availability for a display-only fixture.", "mediaInfo": {"id": 1, "tmdbId": 1, "status": 5, "status4k": 3}},
            {"id": 2, "mediaType": "movie", "title": "2001: A Space Odyssey", "releaseDate": "1968-04-02", "voteAverage": 8.0, "voteCount": 42, "genreIds": [878, 12]},
            {"id": 3, "mediaType": "movie", "title": "Ad Astra", "releaseDate": "2019-09-20", "voteAverage": 6.1, "voteCount": 80, "genreIds": [878, 18], "mediaInfo": {"id": 3, "tmdbId": 3, "status": 4, "status4k": 5}},
            {"id": 4, "mediaType": "movie", "title": "A Film with a Longer Title", "releaseDate": "2026-02-01", "voteAverage": 8.2, "genreIds": [10759, 10765, 9648], "mediaInfo": {"id": 4, "tmdbId": 4, "status": 2}},
            {"id": 5, "mediaType": "movie", "title": "An Upcoming Movie — Not Rated Yet", "releaseDate": "2027-01-01", "voteAverage": 0, "voteCount": 0, "genreIds": [999999]},
            {"id": 6, "mediaType": "movie", "title": "One More Movie", "releaseDate": "2023-04-01", "genreIds": [18,35], "voteAverage": 7.2},
            {"id": 7, "mediaType": "tv", "name": "TV fixture one", "firstAirDate": "2021-03-01", "genreIds": [10765], "voteAverage": 8.1},
            {"id": 8, "mediaType": "tv", "name": "TV fixture two", "firstAirDate": "2024-10-01", "genreIds": [18], "voteAverage": 7.7},
            {"id": 9, "mediaType": "person", "name": "Not a movie or TV series"}
        ]});
        render_discover(&hub.inner, fixture(discovery_fixture.clone()));
        assert!(!hub.inner.borrow().previous.is_sensitive());
        assert!(hub.inner.borrow().next.is_sensitive());
        // Optional local assets are fetched separately by a read-only script;
        // this fixture never starts a network poster job or submits an action.
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("local-dist/discovery-fixture-posters");
        for (index, picture) in &hub.inner.borrow().posters {
            let path = directory.join(format!("{index}.jpg"));
            if path.exists() {
                let texture =
                    gtk::gdk::Texture::from_file(&gtk::gio::File::for_path(path)).unwrap();
                picture.set_paintable(Some(&texture));
            }
        }
        capture(&window, "discover-wide");
        let grid = active_body(&hub.inner)
            .first_child()
            .unwrap()
            .downcast::<gtk::FlowBox>()
            .unwrap();
        assert_eq!(grid.max_children_per_line(), POSTER_COLUMNS);
        assert_eq!(grid.column_spacing(), POSTER_COLUMN_SPACING);
        assert_eq!(grid.row_spacing(), POSTER_ROW_SPACING);
        let mut child = grid.first_child();
        let mut positions = Vec::new();
        while let Some(cell) = child {
            positions.push(cell.compute_bounds(&grid).unwrap().y());
            child = cell.next_sibling();
        }
        assert_eq!(positions.len(), 6);
        let mut links = Vec::new();
        let mut child = grid.first_child();
        while let Some(cell) = child {
            if let Some(link) = cell
                .clone()
                .downcast::<gtk::FlowBoxChild>()
                .ok()
                .and_then(|cell| cell.child())
                .and_then(|card| card.last_child())
                .and_then(|actions| actions.first_child())
                .and_downcast::<gtk::LinkButton>()
            {
                assert!(
                    link.parent()
                        .unwrap()
                        .ancestor(gtk::Button::static_type())
                        .is_none(),
                    "trailer link must not be nested in a details button"
                );
                assert_eq!(
                    reqwest::Url::parse(link.uri().as_str()).unwrap().host_str(),
                    Some("www.youtube.com")
                );
                links.push(link);
            }
            child = cell.next_sibling();
        }
        assert_eq!(
            links.len(),
            6,
            "only the six selected movie cards have trailer links"
        );
        let view = hub.inner.borrow().state.view;
        let activated = Rc::new(std::cell::Cell::new(false));
        links[0].connect_activate_link({
            let activated = activated.clone();
            move |_| {
                activated.set(true);
                glib::Propagation::Stop
            }
        });
        assert!(links[0].emit_by_name::<bool>("activate-link", &[]));
        assert!(activated.get());
        assert_eq!(
            hub.inner.borrow().state.view,
            view,
            "trailer navigation must not open details or submit any action"
        );
        assert!(
            positions.iter().all(|y| *y == positions[0]),
            "wide grid must fit six columns"
        );
        for picture in hub.inner.borrow().posters.values() {
            assert_eq!(
                (picture.width(), picture.height()),
                (POSTER_WIDTH, POSTER_HEIGHT)
            );
        }
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight);
        capture(&window, "discover-light");
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
        window.set_size_request(480, 900);
        window.set_default_size(480, 900);
        capture(&window, "discover-narrow");
        let first = grid.first_child().unwrap();
        let second = first.next_sibling().unwrap();
        let third = second.next_sibling().unwrap();
        assert_eq!(
            first.compute_bounds(&grid).unwrap().y(),
            second.compute_bounds(&grid).unwrap().y()
        );
        assert!(
            third.compute_bounds(&grid).unwrap().y() > first.compute_bounds(&grid).unwrap().y(),
            "narrow grid should use two columns"
        );
        assert_eq!(
            walk(grid.upcast_ref())
                .iter()
                .filter_map(|w| w.downcast_ref::<gtk::Button>())
                .filter(|b| matches!(b.label().as_deref(), Some("Request" | "Request options")))
                .count(),
            6
        );
        let scroll = scroll_for(&active_body(&hub.inner)).unwrap();
        scroll.vadjustment().set_value(250.0);
        settle();
        let saved_scroll = scroll.vadjustment().value();
        assert!(saved_scroll > 0.0);
        prepare_details(&hub.inner);
        assert!(hub.inner.borrow().back.is_visible());
        assert!(!hub.inner.borrow().controls[0].is_visible());
        render_movie(
            &hub.inner,
            fixture(
                json!({"id":1,"title":"13 Hours: The Secret Soldiers of Benghazi","posterPath":"/fixture.jpg","releaseDate":"2016-01-15","runtime":144,"voteAverage":7.3,"voteCount":1200,"tagline":"When everything went wrong, six men had the courage to do what was right.","status":"Released","originalLanguage":"en","overview":"Display-only fixture. A security team faces an overnight attack on an isolated diplomatic outpost. Metadata and availability shown here are synthetic; no requests are sent.","genres":[{"id":28,"name":"Action"},{"id":18,"name":"Drama"}],"productionCompanies":[{"name":"Paramount Pictures"}],"productionCountries":[{"name":"United States of America"}],"spokenLanguages":[{"name":"English"}],"budget":50000000,"revenue":69000000,"externalIds":{"imdbId":"tt4172430"},"relatedVideos":[{"site":"YouTube","type":"Trailer","key":"aB12_-34567"}],"credits":{"cast":[{"name":"John Krasinski","character":"Jack Silva"},{"name":"James Badge Dale","character":"Tyrone Woods"}],"crew":[{"name":"Michael Bay","job":"Director"}]},"mediaInfo":{"id":1,"tmdbId":1,"status":5,"status4k":3}}),
            ),
            None,
            false,
        );
        if let Some(picture) = hub.inner.borrow().posters.get(&detail_view::DETAIL_POSTER) {
            let path = directory.join("0.jpg");
            if path.exists() {
                picture.set_paintable(Some(
                    &gtk::gdk::Texture::from_file(&gtk::gio::File::for_path(path)).unwrap(),
                ));
            }
        }
        let texts = walk(active_body(&hub.inner).upcast_ref())
            .iter()
            .filter_map(|w| w.downcast_ref::<gtk::Label>().map(|l| l.text().to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        for expected in [
            "144 min",
            "TMDB 7.3/10",
            "1200 votes",
            "Paramount Pictures",
            "John Krasinski",
            "Michael Bay",
            "USD 50,000,000",
        ] {
            assert!(
                texts.contains(expected),
                "missing detail metadata: {expected}"
            );
        }
        window.set_size_request(1360, 900);
        window.set_default_size(1360, 900);
        capture(&window, "movie-detail-wide");
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight);
        capture(&window, "movie-detail-light");
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
        window.set_size_request(480, 900);
        window.set_default_size(480, 900);
        capture(&window, "movie-detail-narrow");
        let back = hub.inner.borrow().back.clone();
        back.emit_clicked();
        settle();
        assert_eq!(
            active_body(&hub.inner).first_child().unwrap(),
            grid.clone().upcast::<gtk::Widget>()
        );
        assert!(
            (scroll.vadjustment().value() - saved_scroll).abs() < 1.0,
            "back did not preserve grid scroll"
        );
        assert!(!hub.inner.borrow().back.is_visible());
        assert!(hub.inner.borrow().controls[0].is_visible());
        hub.inner.borrow_mut().user = None;
        hub.inner.borrow_mut().page = 2;
        let search = hub.inner.borrow().search.clone();
        search.set_text("a different query");
        assert_eq!(hub.inner.borrow().page, 1);
        assert!(!hub.inner.borrow().previous.is_sensitive());
        assert!(!hub.inner.borrow().next.is_sensitive());
        assert!(!hub.inner.borrow().category.property::<bool>("sensitive"));
        search.set_text("");
        assert!(hub.inner.borrow().category.property::<bool>("sensitive"));
        let old_account = hub.inner.borrow().state.account;
        let old_view = hub.inner.borrow().state.view;
        hub.inner.borrow_mut().user = None; // Scope changes must not send fixture jobs.
        hub.inner.borrow_mut().page = 2;
        let toggle = hub.inner.borrow().media_toggle[1].clone();
        toggle.set_active(true);
        assert_eq!(hub.inner.borrow().page, 1);
        assert_eq!(discovery_kind(&hub.inner), MediaType::Tv);
        assert!(active_body(&hub.inner).first_child().is_none());
        receive(
            &hub.inner,
            Reply {
                account: old_account,
                view: old_view,
                auth: false,
                result: Ok(Data::Discover(fixture(discovery_fixture.clone()))),
            },
        );
        assert!(
            active_body(&hub.inner).first_child().is_none(),
            "stale reply repopulated a changed scope"
        );
        render_discover(&hub.inner, fixture(discovery_fixture.clone()));
        assert_eq!(
            hub.inner.borrow().posters.len(),
            2,
            "TV scope leaked movies or people"
        );
        hub.inner.borrow().controls[0].set_sensitive(true);
        capture(&window, "discover-tv-narrow");
        let movies = hub.inner.borrow().media_toggle[0].clone();
        movies.set_active(true);
        assert_eq!(discovery_kind(&hub.inner), MediaType::Movie);
        render_discover(&hub.inner, fixture(discovery_fixture));
        assert_eq!(
            hub.inner.borrow().posters.len(),
            6,
            "movie scope leaked TV or people"
        );
        hub.inner.borrow_mut().user = Some(fixture(
            json!({"id":1,"username":"Fixture user","permissions":16}),
        ));
        hub.inner.borrow().controls[0].set_sensitive(true);
        render_tv(
            &hub.inner,
            fixture(
                json!({"id": 2, "name": "A Series with a Longer Title", "firstAirDate":"2021-03-01","lastAirDate":"2024-04-10","episodeRunTime":[48],"numberOfSeasons":1,"numberOfEpisodes":10,"status":"Returning Series","tagline":"Every episode brings a new perspective.","voteAverage":8.1,"voteCount":420,"originalLanguage":"en","networks":[{"name":"Fixture network"}],"createdBy":[{"name":"Fixture creator"}],"productionCompanies":[{"name":"Fixture studio"}],"credits":{"cast":[{"name":"Fixture performer","character":"The lead"}]},"overview": "An example overview for native layout validation. No requests or downloads are submitted by this fixture.", "seasons": [{"id": 10, "seasonNumber": 0, "name": "Specials", "episodeCount": 2}, {"id": 11, "seasonNumber": 1, "name": "Season One", "episodeCount": 8}]}),
            ),
            Some(17),
            false,
        );
        capture(&window, "tv-narrow");
        window.set_size_request(1360, 900);
        window.set_default_size(1360, 900);
        capture(&window, "tv-wide");
        window.set_size_request(480, 900);
        window.set_default_size(480, 900);
        let chooser = walk(active_body(&hub.inner).upcast_ref())
            .into_iter()
            .find_map(|w| w.downcast::<adw::ExpanderRow>().ok())
            .unwrap();
        chooser.set_expanded(true);
        capture(&window, "tv-request-narrow");
        drop(chooser);
        // A request callback owns the season controls; their toggle callbacks
        // must reference the request button weakly so navigation releases both.
        let (season_weak, request_weak) = {
            let body = active_body(&hub.inner);
            let all = walk(body.upcast_ref());
            let mut children = all.into_iter();
            let mut child = children.next();
            let mut season = None;
            let mut request = None;
            while let Some(widget) = child {
                if season.is_none() {
                    if let Some(check) = widget.downcast_ref::<gtk::CheckButton>() {
                        season = Some(check.clone());
                    }
                }
                if let Some(button) = widget.downcast_ref::<gtk::Button>() {
                    if button.label().as_deref() == Some(tr!("seerr-request").as_str()) {
                        request = Some(button.clone());
                    }
                }
                child = children.next();
            }
            let season = season.unwrap();
            let request = request.unwrap();
            season.set_active(true);
            assert!(request.is_sensitive());
            (season.downgrade(), request.downgrade())
        };
        render_diagnostics(
            &hub.inner,
            2,
            Some(17),
            fixture(
                json!({"serverName": "Sonarr fixture", "is4k": false, "seriesMonitored": true, "queueTruncated": true, "historyTruncated": false, "episodes": [{"id": 901, "seasonNumber": 1, "episodeNumber": 1, "title": "Pilot", "hasFile": false, "monitored": true, "state": "queued", "queue": [{"title": "A queued source", "status": "downloading", "trackedStatus": "ok", "trackedState": "downloading", "progress": 35.2, "messages": []}], "history": []}]}),
            ),
        );
        settle();
        assert!(
            season_weak.upgrade().is_none(),
            "season control retained after navigation"
        );
        assert!(
            request_weak.upgrade().is_none(),
            "request button retained after navigation"
        );
        capture(&window, "diagnostics-narrow");
        render_releases(
            &hub.inner,
            2,
            SourceSelection::Tv(DownloadSelection {
                is4k: false,
                request_id: Some(17),
                season_number: 1,
                episode_id: Some(901),
            }),
            fixture(
                json!({"expiresAt": "2099-01-01T00:00:00Z", "truncated": false, "releases": [{"title": "A Series S01E01 1080p WEB-DL", "indexer": "Fixture indexer", "quality": "WEBDL-1080p", "size": 2147483648_u64, "seeders": 12, "protocol": "torrent", "fullSeason": false, "episodeNumbers": [1], "rejections": ["Quality is below the configured cutoff"], "rejected": true, "duplicate": true, "token": "0123456789abcdef0123456789abcdef0123456789abcdef"}]}),
            ),
        );
        capture(&window, "sources-narrow");
        window.set_child(None::<&gtk::Widget>);
        window.set_child(Some(&hub.pages()[1]));
        hub.inner.borrow_mut().active = Some(1);
        render_requests(
            &hub.inner,
            fixture(
                json!({"pageInfo": {"pages": 1, "pageSize": 20, "results": 1, "page": 1}, "results": [{"id": 17,"title":"A Series with a Longer Title","overview":"An ongoing season request. Native fixture only; no downloads are started.","releaseDate":"2021-03-01","profileName":"HD-1080p","requestedBy":{"id":1,"username":"Fixture user"},"createdAt":"2026-10-23T14:00:00Z", "status": 2, "is4k": false, "seasons": [{"seasonNumber": 1, "status": 2}], "media": {"id": 1, "tmdbId": 2, "mediaType": "tv", "status": 3}}]}),
            ),
        );
        hub.inner.borrow().controls[1].set_sensitive(true);
        capture(&window, "requests-narrow");
        let text = walk(active_body(&hub.inner).upcast_ref())
            .iter()
            .filter_map(|w| w.downcast_ref::<gtk::Label>().map(|l| l.text().to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        for expected in [
            "A Series with a Longer Title",
            "HD-1080p",
            "Fixture user",
            "2026-10-23",
            "Season 1",
        ] {
            assert!(
                text.contains(expected),
                "missing request metadata {expected}"
            );
        }
        window.set_size_request(1360, 900);
        window.set_default_size(1360, 900);
        capture(&window, "requests-wide");
        hub.inner.borrow_mut().user = None;
        hub.inner.borrow_mut().request_skip = 20;
        let finished = hub.inner.borrow().request_tabs[0].clone();
        finished.set_active(true);
        assert_eq!(hub.inner.borrow().request_skip, 0);
        render_requests(
            &hub.inner,
            fixture(
                json!({"pageInfo":{"pages":1,"pageSize":20,"results":1,"page":1},"results":[{"id":18,"title":"13 Hours: The Secret Soldiers of Benghazi","releaseDate":"2016-01-15","status":5,"is4k":true,"media":{"id":1,"tmdbId":1,"mediaType":"movie","status":3,"status4k":5}}]}),
            ),
        );
        hub.inner.borrow().controls[1].set_sensitive(true);
        capture(&window, "requests-finished-wide");
        window.set_size_request(480, 900);
        window.set_default_size(480, 900);
        capture(&window, "requests-finished-narrow");
        hub.inner.borrow_mut().user = Some(fixture(
            json!({"id":1,"username":"Fixture user","permissions":16}),
        ));
        window.set_child(None::<&gtk::Widget>);
        window.set_child(Some(&hub.pages()[2]));
        hub.inner.borrow_mut().active = Some(2);
        render_torrents(
            &hub.inner,
            fixture(
                json!({"updatedAt": "2026-01-01T12:00:00Z", "torrents": [{"hash": "0123456789abcdef0123456789abcdef01234567", "name": "A Series S01E01", "state": "downloading", "progress": 0.352, "size": 2147483648_u64, "downloaded": 756000000, "downloadSpeed": 1048576, "uploadSpeed": 32768, "eta": null, "seeds": 12, "peers": 3, "category": "tv"}]}),
            ),
        );
        capture(&window, "downloads-narrow");
        removal_ui::open_download(&hub.inner, "a".repeat(40), "A Series S01E01", true);
        let confirmation = hub
            .inner
            .borrow()
            .removal
            .as_ref()
            .unwrap()
            .borrow()
            .window
            .clone();
        confirmation.set_default_size(360, -1);
        capture(&confirmation, "download-delete-narrow");
        confirmation.close();
        assert!(hub.inner.borrow().removal.is_none());
        // Errors retain the successful snapshot and mark it stale.
        let before = hub.inner.borrow().bodies[2].first_child().unwrap();
        let current_view = hub.inner.borrow().state.view;
        receive(
            &hub.inner,
            Reply {
                account: 0,
                view: current_view,
                auth: false,
                result: Err(Error::Http(503)),
            },
        );
        assert_eq!(hub.inner.borrow().bodies[2].first_child().unwrap(), before);
        assert!(hub.inner.borrow().statuses[2].text().contains("stale"));
        assert!(!hub.inner.borrow().poll_allowed);
        capture(&window, "downloads-stale-narrow");
        // Display-only confirmation fixtures: never invoke a real service.
        let dialog = removal_ui::build(
            &hub.inner,
            Some(RemovalTarget::Media {
                media_type: MediaType::Tv,
                tmdb_id: 2,
            }),
        );
        let removal_window = dialog.borrow().window.clone();
        removal_window.set_resizable(false);
        let old_view = hub.inner.borrow().state.view;
        hub.inner.borrow_mut().state.navigate();
        let epoch = hub.inner.borrow().state.account;
        let options: RemovalOptions = fixture(
            json!({"target":{"mediaType":"tv","tmdbId":2,"title":"Fixture series"},"episodes":[{"seasonNumber":1,"episodeNumber":1,"title":"Pilot","hasFile":true},{"seasonNumber":1,"episodeNumber":2,"title":"Second episode","hasFile":true},{"seasonNumber":2,"episodeNumber":1,"title":"Next season","hasFile":false}],"warnings":["All matched quality versions are included. Shared files and season packs will be checked before deletion."]}),
        );
        receive(
            &hub.inner,
            Reply {
                account: epoch,
                view: old_view,
                auth: false,
                result: Ok(Data::Removal(
                    0,
                    Ok(crate::seerr::ui_service::RemovalData::Options(options)),
                )),
            },
        );
        let preview = walk(removal_window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some(tr!("seerr-removal-delete").as_str()))
            .unwrap();
        assert!(
            !preview.is_sensitive(),
            "TV must not default to entire-series deletion"
        );
        let seasons = walk(removal_window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::CheckButton>().ok())
            .find(|b| b.label().as_deref() == Some(tr!("seerr-removal-seasons").as_str()))
            .unwrap();
        seasons.set_active(true);
        drop(seasons);
        let season = walk(removal_window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::CheckButton>().ok())
            .find(|b| b.label().as_deref() == Some("Season 1"))
            .unwrap();
        season.set_active(true);
        assert!(preview.is_sensitive());
        let season_weak = season.downgrade();
        drop(season);
        drop(preview);
        capture(&removal_window, "removal-scope-wide");
        removal_window.set_default_size(360, -1);
        capture(&removal_window, "removal-scope-narrow");
        removal_window.close();
        assert!(hub.inner.borrow().removal.is_none());
        drop(dialog);
        drop(removal_window);
        assert!(season_weak.upgrade().is_none());
        let dialog = removal_ui::build(
            &hub.inner,
            Some(RemovalTarget::Media {
                media_type: MediaType::Movie,
                tmdb_id: 1,
            }),
        );
        let removal_window = dialog.borrow().window.clone();
        removal_ui::render_options(
            &hub.inner,
            fixture(
                json!({"target":{"mediaType":"movie","tmdbId":1,"title":"Fixture movie"},"episodes":[],"warnings":[]}),
            ),
        );
        assert!(
            !walk(removal_window.upcast_ref())
                .iter()
                .any(|w| w.is::<gtk::Entry>()),
            "No typed confirmation"
        );
        let buttons: Vec<_> = walk(removal_window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .filter_map(|b| b.label())
            .collect();
        assert_eq!(buttons.len(), 2, "Only Cancel and Delete");
        capture(&removal_window, "removal-simple-wide");
        removal_window.set_default_size(360, -1);
        capture(&removal_window, "removal-simple-narrow");
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight);
        capture(&removal_window, "removal-simple-light");
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
        removal_window.close();
        drop(dialog);
        removal_ui::exercise_single_confirmation(&hub.inner);
        show_account(&hub.inner);
        settle();
        let settings = hub.inner.borrow().settings.clone().unwrap();
        fn find_url(widget: &gtk::Widget) -> Option<gtk::Entry> {
            if let Some(entry) = widget.downcast_ref::<gtk::Entry>() {
                if entry.placeholder_text().as_deref() == Some("https://seerr.example.org/") {
                    return Some(entry.clone());
                }
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                if let Some(entry) = find_url(&widget) {
                    return Some(entry);
                }
                child = widget.next_sibling();
            }
            None
        }
        // The cleartext warning blocks submission before any preference write/network.
        find_url(settings.upcast_ref())
            .unwrap()
            .set_text("http://192.0.2.1/seerr/");
        let login = hub.inner.borrow().login_button.clone().unwrap();
        login.emit_clicked();
        assert!(!hub.inner.borrow().state.busy);
        assert_eq!(
            hub.inner.borrow().settings_error.as_ref().unwrap().text(),
            tr!("seerr-http-required").as_str()
        );
        settings.close();
        assert!(hub.inner.borrow().settings.is_none());
        // A delayed search signal from a hidden Discover tab must not fetch or
        // overwrite Requests/Downloads, even when the account is signed in.
        let search = hub.inner.borrow().search.clone();
        let view = hub.inner.borrow().state.view;
        search.emit_by_name::<()>("search-changed", &[]);
        assert!(!hub.inner.borrow().state.busy);
        assert_eq!(hub.inner.borrow().state.view, view);
        // Invalidated navigation/account replies cannot replace the current body.
        replace_view(&hub.inner);
        receive(
            &hub.inner,
            Reply {
                account: 0,
                view: 0,
                auth: false,
                result: Ok(Data::Grabbed(GrabAccepted {
                    message: "Fixture".into(),
                    partial: false,
                })),
            },
        );
        assert_eq!(hub.inner.borrow().bodies[2].first_child().unwrap(), before);
        let current_view = hub.inner.borrow().state.view;
        receive(
            &hub.inner,
            Reply {
                account: 0,
                view: current_view,
                auth: false,
                result: Err(Error::Unauthorized),
            },
        );
        assert!(hub.inner.borrow().user.is_none());
        assert!(!hub.inner.borrow().state.busy);
        assert!(!hub.inner.borrow().spinners[2].is_spinning());
        assert!(hub.inner.borrow().bodies[2].first_child().is_none());
        window.close();
        drop(hub);
        settle();
    }
}
