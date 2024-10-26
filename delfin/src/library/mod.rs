pub mod collection;
pub mod collections;
mod home;
mod home_sections;
mod library_container;
mod media_button;
mod media_carousel;
mod media_fetcher;
mod media_grid;
mod media_list;
pub mod media_page;
mod media_tile;
mod search;

use jellyfin_api::types::BaseItemDto;
use relm4::{
    actions::{AccelsPlus, RelmAction, RelmActionGroup},
    binding::BoolBinding,
    ComponentController, RelmObjectExt, SharedState,
};
use std::sync::{Arc, RwLock};
use tracing::error;

use adw::prelude::*;
use gtk::glib;
use relm4::{adw, gtk, prelude::*, Component, Controller};

use crate::{
    app::{AppInput, AppPage, APP_BROKER},
    borgar::borgar_menu::{BorgarMenu, BorgarMenuAuth},
    config::{Account, Server},
    jellyfin_api::{
        api_client::ApiClient,
        models::{display_preferences::DisplayPreferences, user_view::UserView},
    },
    media_details::MEDIA_DETAILS_REFRESH_QUEUED,
    tr,
    utils::{
        constants::WIDGET_NONE, main_window::get_main_window,
        message_broker::ResettableMessageBroker,
    },
};

use self::{
    collections::Collections,
    home::{Home, HomeInit},
    search::{
        search_bar::SearchBar,
        search_results::{SearchResults, SearchResultsInput},
    },
};

pub static LIBRARY_BROKER: ResettableMessageBroker<LibraryInput> = ResettableMessageBroker::new();
pub static LIBRARY_REFRESH_QUEUED: SharedState<bool> = SharedState::new();

#[derive(Debug)]
pub enum LibraryState {
    Loading,
    Offline,
    Error,
    Ready,
}

pub struct Library {
    borgar_menu: Controller<BorgarMenu>,
    api_client: Arc<ApiClient>,
    state: LibraryState,
    search_results: Controller<SearchResults>,
    home: Option<Controller<Home>>,
    collections: Option<Controller<Collections>>,
    searching: BoolBinding,
    // Store previous view stack child so we can go back from search
    previous_stack_child: Arc<RwLock<String>>,
}

#[derive(Debug)]
pub enum LibraryInput {
    SetLibraryState(LibraryState),
    Refresh,
    Shown,
    Hidden,
    ViewStackChildVisible(String),
    SearchChanged(String),
    SearchingChanged(bool),
    ShowSearch,
    ShowCollections,
}

#[derive(Debug)]
pub enum LibraryOutput {
    PlayVideo(Box<BaseItemDto>),
}

#[derive(Debug)]
pub enum LibraryCommandOutput {
    LibraryLoaded(Vec<UserView>, DisplayPreferences),
    SetLibraryState(LibraryState),
}

#[relm4::component(pub)]
impl Component for Library {
    type Init = (Server, Account, Arc<ApiClient>);
    type Input = LibraryInput;
    type Output = LibraryOutput;
    type CommandOutput = LibraryCommandOutput;

    view! {
        adw::NavigationPage {
            set_tag: Some(&AppPage::Library.to_string()),
            set_title: tr!("library-page-title"),

            #[wrap(Some)]
            set_child = &adw::BreakpointBin {
                set_size_request: (150, 150),

                #[wrap(Some)]
                set_child = &adw::ToolbarView {
                    add_top_bar = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        #[name = "header_bar"]
                        adw::HeaderBar {
                            #[name = "view_switcher"]
                            #[wrap(Some)]
                            set_title_widget = &adw::ViewSwitcher {
                                set_policy: adw::ViewSwitcherPolicy::Wide,
                                set_stack: Some(&view_stack),
                            },

                            pack_start = &gtk::ToggleButton {
                                set_icon_name: "loupe",
                                set_tooltip: tr!("library-search-button"),

                                #[watch]
                                set_visible: matches!(model.state, LibraryState::Ready),
                                add_binding: (&model.searching, "active"),
                            },

                            pack_end = model.borgar_menu.widget(),
                            pack_end = &gtk::Button::from_icon_name("refresh") {
                                set_tooltip: tr!("library-refresh-button"),
                                connect_clicked[sender] => move |_| {
                                    sender.input(LibraryInput::Refresh);
                                },
                            },
                        },

                        #[name = "search"]
                        SearchBar {
                            add_binding: (&model.searching, "searching"),
                            connect: ("search", false, glib::clone!(@strong sender => move |values| {
                                    let text: String = values[1].get().expect("Failed to get search text");
                                    sender.input(LibraryInput::SearchChanged(text));
                                    None
                                })
                            ),
                        },
                    },

                    #[wrap(Some)]
                    set_content = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        #[transition = "Crossfade"]
                        append = match model.state {
                            LibraryState::Loading => {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,

                                    adw::Clamp {
                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_hexpand: true,
                                            set_vexpand: true,
                                            set_halign: gtk::Align::Center,
                                            set_valign: gtk::Align::Center,
                                            set_spacing: 20,

                                            gtk::Spinner {
                                                set_spinning: true,
                                                set_size_request: (64, 64),
                                            },

                                            gtk::Label {
                                                set_label: tr!("library-loading"),
                                                add_css_class: "title-2",
                                            },
                                        }
                                    }
                                }
                            }
                            LibraryState::Ready => {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,

                                    #[name = "view_stack"]
                                    adw::ViewStack {
                                        set_valign: gtk::Align::Fill,

                                        add_named: (model.search_results.widget(), Some("search")),

                                        connect_visible_child_notify[sender] => move |stack| {
                                            if let Some(name) = stack.visible_child_name() {
                                                sender.input(LibraryInput::ViewStackChildVisible(name.into()));
                                            }
                                        },
                                    },

                                    #[name = "view_switcher_bar"]
                                    adw::ViewSwitcherBar {
                                        set_stack: Some(&view_stack),
                                    },
                                }
                            }
                            LibraryState::Offline => {
                                adw::StatusPage {
                                    set_title: tr!("library-offline.title"),
                                    set_description: Some(tr!("library-offline.description")),

                                    set_icon_name: Some("warning"),
                                    #[wrap(Some)]
                                    set_child = &gtk::Button {
                                        set_label: tr!("library-status-refresh-button"),
                                        set_halign: gtk::Align::Center,
                                        set_css_classes: &["pill", "suggested-action"],
                                        connect_clicked[sender] => move |_| {
                                            sender.input(LibraryInput::Refresh);
                                        },
                                    }
                                }
                            }
                            LibraryState::Error => {
                                adw::StatusPage {
                                    set_title: tr!("library-error.title"),
                                    set_description: Some(tr!("library-error.description")),

                                    set_icon_name: Some("warning"),
                                    #[wrap(Some)]
                                    set_child = &gtk::Button {
                                        set_label: tr!("library-status-refresh-button"),
                                        set_halign: gtk::Align::Center,
                                        set_css_classes: &["pill", "suggested-action"],
                                        connect_clicked[sender] => move |_| {
                                            sender.input(LibraryInput::Refresh);
                                        },
                                    }
                                }
                            }
                        },
                    },
                },

                add_breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
                    adw::BreakpointConditionLengthType::MaxWidth,
                    550.0,
                    adw::LengthUnit::Sp
                )) {
                    add_setter: (&header_bar, "title-widget", &WIDGET_NONE.into()),
                    add_setter: (&view_switcher_bar, "reveal", &true.into()),
                },
            },

            connect_shown[sender] => move |_| {
                sender.input(LibraryInput::Shown);
            },

            connect_hidden[sender] => move |_| {
                sender.input(LibraryInput::Hidden);
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let (server, account, api_client) = init;

        let model = Library {
            borgar_menu: BorgarMenu::builder()
                .launch(Some(BorgarMenuAuth {
                    api_client: api_client.clone(),
                    server,
                    account,
                }))
                .detach(),
            api_client: Arc::clone(&api_client),
            state: LibraryState::Loading,
            search_results: SearchResults::builder().launch(api_client).detach(),
            home: None,
            collections: None,
            searching: BoolBinding::default(),
            previous_stack_child: Arc::new(RwLock::new("home".into())),
        };

        model.searching.connect_value_notify({
            let sender = sender.clone();
            move |searching| {
                sender.input(LibraryInput::SearchingChanged(searching.value()));
            }
        });

        let widgets = view_output!();
        let view_stack = &widgets.view_stack;

        view_stack.connect_visible_child_name_notify({
            let previous_stack_child = model.previous_stack_child.clone();
            move |view_stack| {
                if let Some(visible_child_name) = view_stack.visible_child_name() {
                    let visible_child_name = visible_child_name.to_string();
                    if visible_child_name != "search"
                        && visible_child_name != *previous_stack_child.read().unwrap()
                    {
                        *previous_stack_child.write().unwrap() = visible_child_name;
                    }
                }
            }
        });

        model.initial_fetch(&sender);

        let app = relm4::main_application();
        app.set_accelerators_for_action::<SearchAction>(&["<Ctrl>f"]);

        let search_action: RelmAction<SearchAction> = RelmAction::new_stateless({
            let sender = sender.clone();
            move |_| {
                APP_BROKER.send(AppInput::PopToPage(AppPage::Library));
                sender.input(LibraryInput::ShowSearch);
            }
        });

        let refresh_action: RelmAction<RefreshAction> = RelmAction::new_stateless({
            let sender = sender.clone();
            move |_| {
                sender.input(LibraryInput::Refresh);
            }
        });

        let mut group = RelmActionGroup::<LibraryActionGroup>::new();
        group.add_action(search_action);
        group.add_action(refresh_action);
        if let Some(main_window) = get_main_window() {
            group.register_for_widget(main_window);
        }

        relm4::ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            LibraryInput::SetLibraryState(state) => {
                self.state = state;

                let search = &widgets.search;
                match self.state {
                    LibraryState::Ready => {
                        search.set_key_capture_widget(Some(root));
                    }
                    _ => {
                        search.set_key_capture_widget(None::<&gtk::Widget>);
                    }
                };
            }
            LibraryInput::Refresh => {
                if root.is_child_visible() {
                    let view_stack = &widgets.view_stack;

                    sender.input(LibraryInput::SetLibraryState(LibraryState::Loading));
                    self.searching.set_value(false);

                    // Clear the current set of pages before loading a new one
                    if let Some(home) = self.home.take() {
                        view_stack.remove(home.widget());
                    }
                    if let Some(collections) = self.collections.take() {
                        view_stack.remove(collections.widget());
                    }

                    self.initial_fetch(&sender);
                }
            }
            LibraryInput::Shown => {
                if *LIBRARY_REFRESH_QUEUED.read() {
                    sender.input(LibraryInput::Refresh);
                    *MEDIA_DETAILS_REFRESH_QUEUED.write() = false;
                }
                *LIBRARY_REFRESH_QUEUED.write() = false;

                relm4::main_application()
                    .set_accelerators_for_action::<RefreshAction>(&["<Ctrl>r"]);
            }
            LibraryInput::Hidden => {
                relm4::main_application().set_accelerators_for_action::<RefreshAction>(&[]);
            }
            LibraryInput::ViewStackChildVisible(name) => {
                if name != "search" {
                    self.searching.set_value(false);
                }
            }
            LibraryInput::SearchChanged(search_text) => {
                self.search_results
                    .emit(SearchResultsInput::SearchChanged(search_text));
            }
            LibraryInput::SearchingChanged(searching) => {
                if searching {
                    widgets.view_stack.set_visible_child_name("search");
                } else {
                    let previous_stack_child = self.previous_stack_child.read().unwrap();
                    if let Some(previous_stack_child) =
                        widgets.view_stack.child_by_name(&previous_stack_child)
                    {
                        widgets.view_stack.set_visible_child(&previous_stack_child);
                    }
                }
            }
            LibraryInput::ShowSearch => {
                if let LibraryState::Ready = self.state {
                    self.searching.set_value(true);
                }
            }
            LibraryInput::ShowCollections => {
                if let Some(collections) = &self.collections {
                    widgets.view_stack.set_visible_child(collections.widget());
                }
            }
        }

        self.update_view(widgets, sender);
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            LibraryCommandOutput::LibraryLoaded(user_views, display_preferences) => {
                self.display_user_views(widgets, &sender, user_views, display_preferences);
            }
            LibraryCommandOutput::SetLibraryState(state) => {
                sender.input(LibraryInput::SetLibraryState(state));
            }
        }

        self.update_view(widgets, sender);
    }
}

impl Library {
    fn initial_fetch(&self, sender: &relm4::ComponentSender<Self>) {
        let api_client = Arc::clone(&self.api_client);
        sender.oneshot_command(async move {
            match api_client.ping().await {
                Ok(_) => {}
                Err(err) => {
                    error!("Failed to ping server: {err}");
                    return LibraryCommandOutput::SetLibraryState(LibraryState::Offline);
                }
            }

            match tokio::try_join!(
                async { api_client.get_user_views(None, None).await },
                async {
                    api_client
                        // We might eventually want client-specific settings, but for
                        // now use the Jellyfin ("emby") client settings
                        .get_user_display_preferences("emby")
                        .await
                }
            ) {
                Ok((user_views, display_preferences)) => {
                    LibraryCommandOutput::LibraryLoaded(user_views.0, display_preferences)
                }
                Err(err) => {
                    error!("Failed to load library: {err}");
                    LibraryCommandOutput::SetLibraryState(LibraryState::Error)
                }
            }
        });
    }

    fn display_user_views(
        &mut self,
        widgets: &mut LibraryWidgets,
        sender: &relm4::ComponentSender<Self>,
        user_views: Vec<UserView>,
        display_preferences: DisplayPreferences,
    ) {
        let view_stack = &widgets.view_stack;

        sender.input(LibraryInput::SetLibraryState(LibraryState::Ready));

        let home = Home::builder()
            .launch(HomeInit {
                api_client: self.api_client.clone(),
                display_preferences,
                user_views: user_views.clone(),
            })
            .detach();
        if let Some(prev_home) = self.home.take() {
            view_stack.remove(prev_home.widget());
        }
        view_stack.add_titled_with_icon(
            home.widget(),
            Some("home"),
            tr!("library-page-home-title"),
            "home-filled",
        );
        self.home = Some(home);

        view_stack.set_visible_child_name("home");

        let collections = Collections::builder()
            .launch(self.api_client.clone())
            .detach();
        if let Some(prev_collections) = self.collections.take() {
            view_stack.remove(prev_collections.widget());
        }
        view_stack.add_titled_with_icon(
            collections.widget(),
            Some("collections"),
            tr!("library-page-collections-title"),
            "library",
        );
        self.collections = Some(collections);
    }
}

relm4::new_action_group!(LibraryActionGroup, "library_actions");
relm4::new_stateless_action!(SearchAction, LibraryActionGroup, "search");
relm4::new_stateless_action!(RefreshAction, LibraryActionGroup, "refresh");
