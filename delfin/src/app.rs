use adw::prelude::*;
use gtk::glib;
use jellyfin_api::types::BaseItemDto;
use relm4::{
    actions::{AccelsPlus, RelmAction, RelmActionGroup},
    adw::Toast,
    prelude::*,
    MessageBroker,
};
use std::{
    cell::OnceCell,
    fmt::Display,
    sync::{Arc, RwLock},
};

use crate::{
    accounts::account_list::{AccountList, AccountListInput, AccountListOutput},
    borgar::borgar_menu::{BorgarMenuInput, BORGAR_MENU_SENDER},
    config::{self, general::MostRecentLogin},
    globals::CONFIG,
    jellyfin_api::api_client::ApiClient,
    library::{collection::Collection, Library, LibraryOutput, LIBRARY_BROKER},
    locales::tera_tr,
    media_details::{MediaDetails, MEDIA_DETAILS_BROKER},
    meson_config::APP_ID,
    preferences::Preferences,
    servers::server_list::{ServerList, ServerListOutput},
    tr,
    utils::{main_window::MAIN_APP_WINDOW_NAME, shift_state::shift_state_controller},
    video_player::{VideoPlayer, VideoPlayerInput, VideoPlayerOutput, VIDEO_PLAYER_BROKER},
};

pub static APP_BROKER: MessageBroker<AppInput> = MessageBroker::new();

#[derive(Clone, Copy, Debug)]
pub enum AppPage {
    Servers,
    Accounts,
    Library,
    Collection,
    MediaDetails,
    VideoPlayer,
}

impl Display for AppPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AppPage::Servers => "servers",
                AppPage::Accounts => "accounts",
                AppPage::Library => "library",
                AppPage::Collection => "collection",
                AppPage::MediaDetails => "media_details",
                AppPage::VideoPlayer => "video_player",
            }
        )
    }
}

pub struct App {
    api_client: Option<Arc<ApiClient>>,
    servers: Controller<ServerList>,
    account_list: Controller<AccountList>,
    library: Option<Controller<Library>>,
    media_details: Option<Controller<MediaDetails>>,
    collection: Option<Controller<Collection>>,
    video_player: OnceCell<Controller<VideoPlayer>>,
    server: Option<config::Server>,
    account: Option<config::Account>,
    preferences: Option<Controller<Preferences>>,
}

#[derive(Debug)]
pub enum AppInput {
    Present,
    NavigateBack,
    PopToPage(AppPage),
    ServerSelected(config::Server),
    AccountSelected(config::Server, config::Account),
    ShowDetails(BaseItemDto),
    RemoveMedia(BaseItemDto, String),
    RemovalFinished,
    ShowCollection(BaseItemDto),
    PlayVideo(BaseItemDto),
    SignOut,
    SetThemeDark(bool),
    PagePopped(Option<String>),
    ShowPreferences,
    Toast(String, Option<u32>),
}

#[relm4::component(pub)]
impl Component for App {
    type Init = ();
    type Input = AppInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::ApplicationWindow {
            set_widget_name: MAIN_APP_WINDOW_NAME,
            set_title: Some(tr!("app-name")),

            set_default_width: config.window.width as i32,
            set_default_height: config.window.height as i32,
            set_maximized: config.window.maximized,

            connect_close_request => move |window| {
                let mut config = CONFIG.write();
                config.window.width = window.width() as usize;
                config.window.height = window.height() as usize;
                config.window.maximized = window.is_maximized();
                config.save().expect("Failed to save window state");
                glib::Propagation::Proceed
            },

            add_controller: shift_state_controller(),

            #[wrap(Some)]
            set_help_overlay = &keyboard_shortcuts(),


            #[name = "toaster"]
            #[wrap(Some)]
            set_content = &adw::ToastOverlay {
                #[name = "navigation"]
                #[wrap(Some)]
                set_child = &adw::NavigationView {
                    add = model.servers.widget() {
                        set_tag: Some(&AppPage::Servers.to_string()),
                    },
                    add = model.account_list.widget() {
                        set_tag: Some(&AppPage::Accounts.to_string()),
                    },

                    connect_popped[sender] => move |_, page| {
                        sender.input(AppInput::PagePopped(page.tag().map(|s| s.to_string())));
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        gtk::Window::set_default_icon_name(APP_ID);

        let config = CONFIG.read();

        let style_manager = adw::StyleManager::default();
        style_manager.set_color_scheme(config.general.theme().into());
        style_manager.connect_dark_notify({
            let sender = sender.clone();
            move |style_manager| {
                sender.input(AppInput::SetThemeDark(style_manager.is_dark()));
            }
        });
        root.add_css_class(if style_manager.is_dark() {
            "dark"
        } else {
            "light"
        });

        // Use development styles when running debug build
        #[cfg(debug_assertions)]
        root.add_css_class("devel");

        let servers = ServerList::builder()
            .launch(())
            .forward(sender.input_sender(), convert_server_list_output);

        let account_list = AccountList::builder()
            .launch(())
            .forward(sender.input_sender(), convert_account_list_output);

        let model = App {
            api_client: None,
            servers,
            account_list,
            library: None,
            media_details: None,
            collection: None,
            video_player: OnceCell::new(),
            server: None,
            account: None,
            preferences: None,
        };

        let widgets = view_output!();

        model.register_actions(&sender);

        if config.general.restore_most_recent_login {
            if let Some(MostRecentLogin {
                server_id,
                account_id,
            }) = config.general.most_recent_login
            {
                if let Some(server) = config.servers.iter().find(|server| server.id == server_id) {
                    sender.input(AppInput::ServerSelected(server.clone()));

                    if let Some(account) = server
                        .accounts
                        .iter()
                        .find(|account| account.id == account_id)
                    {
                        sender.input(AppInput::AccountSelected(server.clone(), account.clone()));
                    }
                }
            }
        }

        // Disable back on escape in video player
        let navigation = &widgets.navigation;
        navigation
            .bind_property("visible-page", navigation, "pop-on-escape")
            .transform_to(|_, page: Option<adw::NavigationPage>| {
                let Some(page) = page else {
                    return Some(true);
                };
                let Some(tag) = page.tag().map(|str| str.to_string()) else {
                    return Some(true);
                };
                Some(tag != AppPage::VideoPlayer.to_string())
            })
            .build();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        let navigation = &widgets.navigation;

        match message {
            AppInput::Present => {
                root.present();
            }
            AppInput::NavigateBack => {
                navigation.pop();
            }
            AppInput::PopToPage(page) => {
                navigation.pop_to_tag(&page.to_string());
            }
            AppInput::ServerSelected(server) => {
                self.server = Some(server.clone());
                self.account_list.emit(AccountListInput::SetServer(server));
                navigation.push_by_tag(&AppPage::Accounts.to_string());
            }
            AppInput::AccountSelected(server, account) => {
                self.account = Some(account.clone());

                let api_client = ApiClient::new(
                    // TODO
                    Arc::new(RwLock::new(CONFIG.read().clone())),
                    &server,
                    &account,
                );
                let api_client = Arc::new(api_client);
                self.api_client = Some(api_client.clone());

                LIBRARY_BROKER.reset();
                let library = Library::builder()
                    .launch_with_broker((server, account, api_client), &LIBRARY_BROKER.read())
                    .forward(sender.input_sender(), convert_library_output);
                navigation.push(library.widget());
                self.library = Some(library);
            }
            AppInput::RemoveMedia(media, origin) => {
                if let (Some(library), Some(id)) = (&self.library, media.id) {
                    library.emit(crate::library::LibraryInput::RemoveMedia(
                        id.to_string().replace('-', ""),
                        origin,
                    ));
                }
            }
            AppInput::RemovalFinished => {
                *crate::library::LIBRARY_REFRESH_QUEUED.write() = true;
                *crate::media_details::MEDIA_DETAILS_REFRESH_QUEUED.write() = true;
                if let Some(library) = &self.library {
                    library.emit(crate::library::LibraryInput::Refresh);
                }
                if let Some(collection) = &self.collection {
                    collection.emit(crate::library::collection::CollectionInput::AfterRemoval);
                }
            }
            AppInput::ShowDetails(media) => {
                if let (Some(api_client), Some(server), Some(account)) =
                    (&self.api_client, &self.server, &self.account)
                {
                    MEDIA_DETAILS_BROKER.reset();
                    let media_details = MediaDetails::builder()
                        .launch_with_broker(
                            (api_client.clone(), media, server.clone(), account.clone()),
                            &MEDIA_DETAILS_BROKER.read(),
                        )
                        .detach();
                    media_details
                        .widget()
                        .set_tag(Some(&AppPage::MediaDetails.to_string()));
                    navigation.push(media_details.widget());
                    self.media_details = Some(media_details);
                }
            }
            AppInput::ShowCollection(collection) => {
                if let (Some(api_client), Some(server), Some(account)) =
                    (&self.api_client, &self.server, &self.account)
                {
                    let collection = Collection::builder()
                        .launch((
                            api_client.clone(),
                            collection,
                            server.clone(),
                            account.clone(),
                        ))
                        .detach();
                    collection
                        .widget()
                        .set_tag(Some(&AppPage::Collection.to_string()));
                    navigation.push(collection.widget());
                    self.collection = Some(collection);
                }
            }
            AppInput::PlayVideo(item) => {
                if self.video_player.get().is_none() {
                    let video_player = VideoPlayer::builder()
                        .launch_with_broker((), &VIDEO_PLAYER_BROKER)
                        .forward(sender.input_sender(), convert_video_player_output);
                    let video_player_widget = video_player.widget();
                    video_player_widget.set_tag(Some(&AppPage::VideoPlayer.to_string()));
                    widgets.navigation.add(video_player_widget);
                    // We already checked that video_player is unset, ignore result
                    let _ = self.video_player.set(video_player);
                }

                if let Some(api_client) = &self.api_client {
                    self.video_player
                        .get()
                        .unwrap()
                        .emit(VideoPlayerInput::PlayVideo(
                            api_client.clone(),
                            Box::new(item),
                        ));
                    navigation.push_by_tag(&AppPage::VideoPlayer.to_string());
                }
            }
            AppInput::SignOut => {
                navigation.pop_to_tag(&AppPage::Servers.to_string());
            }
            AppInput::SetThemeDark(dark) => {
                if dark {
                    root.remove_css_class("light");
                    root.add_css_class("dark");
                } else {
                    root.remove_css_class("dark");
                    root.add_css_class("light");
                }
            }
            AppInput::PagePopped(tag) => {
                match tag {
                    Some(tag) if tag == AppPage::MediaDetails.to_string() => {
                        self.media_details = None;
                    }
                    Some(tag) if tag == AppPage::Collection.to_string() => {
                        self.collection = None;
                    }
                    _ => {}
                };
            }
            AppInput::ShowPreferences => {
                self.preferences = Some(
                    Preferences::builder()
                        .transient_for(root)
                        .launch(())
                        .detach(),
                );
            }
            AppInput::Toast(toast, timeout) => {
                widgets.toaster.add_toast(
                    Toast::builder()
                        .title(toast)
                        .timeout(timeout.unwrap_or(3))
                        .build(),
                );
            }
        }

        self.update_view(widgets, sender);
    }
}

impl App {
    fn register_actions(&self, sender: &ComponentSender<Self>) {
        let app = relm4::main_application();
        app.set_accels_for_action("win.show-help-overlay", &["<Ctrl>question"]);
        app.set_accels_for_action("window.close", &["<Ctrl>w"]);

        let mut group = RelmActionGroup::<AppActionGroup>::new();

        let menu_action: RelmAction<MenuAction> = RelmAction::new_stateless(|_| {
            BORGAR_MENU_SENDER.send(BorgarMenuInput::OpenMenu).unwrap();
        });
        app.set_accelerators_for_action::<MenuAction>(&["F10"]);

        let preferences_action: RelmAction<PreferencesAction> = RelmAction::new_stateless({
            let sender = sender.clone();
            move |_| {
                sender.input(AppInput::ShowPreferences);
            }
        });
        app.set_accelerators_for_action::<PreferencesAction>(&["<Ctrl>comma"]);

        let quit_action: RelmAction<QuitAction> = RelmAction::new_stateless(|_| {
            relm4::main_application().quit();
        });
        app.set_accelerators_for_action::<QuitAction>(&["<Ctrl>q"]);

        group.add_action(menu_action);
        group.add_action(preferences_action);
        group.add_action(quit_action);
        group.register_for_main_application();
    }
}

fn keyboard_shortcuts() -> gtk::ShortcutsWindow {
    gtk::Builder::from_string(&tera_tr(include_str!("../../data/ui/shortcuts.ui")).unwrap())
        .object::<gtk::ShortcutsWindow>("shortcuts")
        .unwrap()
}

fn convert_server_list_output(output: ServerListOutput) -> AppInput {
    match output {
        ServerListOutput::ServerSelected(server) => AppInput::ServerSelected(server),
    }
}

fn convert_account_list_output(output: AccountListOutput) -> AppInput {
    match output {
        AccountListOutput::AccountSelected(server, account) => {
            AppInput::AccountSelected(server, account)
        }
    }
}

fn convert_video_player_output(output: VideoPlayerOutput) -> AppInput {
    match output {
        VideoPlayerOutput::NavigateBack => AppInput::NavigateBack,
    }
}

fn convert_library_output(output: LibraryOutput) -> AppInput {
    match output {
        LibraryOutput::PlayVideo(media) => AppInput::PlayVideo(*media),
    }
}

relm4::new_action_group!(AppActionGroup, "app");
relm4::new_stateless_action!(MenuAction, AppActionGroup, "menu");
relm4::new_stateless_action!(PreferencesAction, AppActionGroup, "preferences");
relm4::new_stateless_action!(QuitAction, AppActionGroup, "quit");
