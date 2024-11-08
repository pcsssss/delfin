use std::{
    cell::RefCell,
    sync::{Arc, RwLock},
};

use gtk::prelude::*;
use relm4::{
    actions::{ActionName, RelmAction, RelmActionGroup},
    gtk::{self, gdk},
    Component, ComponentParts,
};

use crate::{
    app::{AppInput, APP_BROKER},
    config::video_player_config::{MpvHardwareDecoding, VideoPlayerConfig},
    globals::CONFIG,
    tr,
    utils::{main_window::get_main_window, message_broker::ResettableMessageBroker},
    video_player::backends::{with_backend_mpv, VideoPlayerBackend},
};

pub static VIDEO_PLAYER_MENU_BROKER: ResettableMessageBroker<VideoPlayerMenuInput> =
    ResettableMessageBroker::new();

#[derive(Debug)]
pub struct VideoPlayerMenu {
    debug_menu: gtk::gio::Menu,
    mpv_props_section_pos: Option<i32>,
    stream_url: Arc<RwLock<String>>,
}

#[derive(Debug)]
pub enum VideoPlayerMenuInput {
    ConfigUpdated(VideoPlayerConfig),
    CurrentHwdecUpdated(String),
    SetStreamURL(String),
}

relm4::new_action_group!(VideoPlayerMenuGroup, "video_player_menu");
relm4::new_stateful_action!(
    HardwareDecodingAction,
    VideoPlayerMenuGroup,
    "hardware_decoding",
    MpvHardwareDecoding,
    MpvHardwareDecoding
);
relm4::new_stateless_action!(
    KeyboardShortcutsAction,
    VideoPlayerMenuGroup,
    "keyboard_shortcuts"
);
relm4::new_stateless_action!(CopyStreamURLAction, VideoPlayerMenuGroup, "copy_stream_url");

#[relm4::component(pub)]
impl Component for VideoPlayerMenu {
    type Init = Arc<RefCell<dyn VideoPlayerBackend>>;
    type Input = VideoPlayerMenuInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Box {
            gtk::MenuButton {
                set_icon_name: "view-more-symbolic",
                set_menu_model: Some(&menu),
                set_focus_on_click: false,
                set_direction: gtk::ArrowType::Up,
            },
            gtk::Separator { add_css_class: "spacer" },
        }
    }

    fn init(
        video_player: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        relm4::menu! {
            menu: {
                &*tr!("vp-menu-hwdec") {
                    &MpvHardwareDecoding::Auto.label() => HardwareDecodingAction(MpvHardwareDecoding::Auto),
                    &MpvHardwareDecoding::AutoCopy.label() => HardwareDecodingAction(MpvHardwareDecoding::AutoCopy),
                    &MpvHardwareDecoding::AutoSafe.label() => HardwareDecodingAction(MpvHardwareDecoding::AutoSafe),
                    &MpvHardwareDecoding::Custom.label() => HardwareDecodingAction(MpvHardwareDecoding::Custom),
                    &MpvHardwareDecoding::Off.label() => HardwareDecodingAction(MpvHardwareDecoding::Off),
                },
                section! {
                    &*tr!("borgar-keyboard-shortcuts") => KeyboardShortcutsAction,
                },
            }
        }

        relm4::menu! {
            debug_menu: {
                &*tr!("vp-menu-debug-copy-stream-url") => CopyStreamURLAction,
            }
        }

        menu.append_submenu(Some(tr!("vp-menu-debug")), &debug_menu);

        with_backend_mpv(&video_player, {
            let sender = sender.clone();
            |mpv| {
                mpv.connect_current_hwdec_updated(Box::new(move |hwdec| {
                    sender.input(VideoPlayerMenuInput::CurrentHwdecUpdated(hwdec.into()));
                }));
            }
        });

        let model = VideoPlayerMenu {
            debug_menu,
            mpv_props_section_pos: None,
            stream_url: Arc::default(),
        };

        let widgets = view_output!();

        let mut group = RelmActionGroup::<VideoPlayerMenuGroup>::new();

        let hardware_decoding_action: RelmAction<HardwareDecodingAction> =
            RelmAction::new_stateful_with_target_value(
                &CONFIG.read().video_player.mpv_prefs.hardware_decoding,
                |_, state, value| {
                    if *state == value {
                        return;
                    }
                    *state = value;
                    CONFIG.write().video_player.mpv_prefs.hardware_decoding = value;
                },
            );

        let keyboard_shortcuts_action: RelmAction<KeyboardShortcutsAction> =
            RelmAction::new_stateless(move |_| {
                get_main_window()
                    .and_then(|win| win.lookup_action("show-help-overlay"))
                    .expect("Error getting show-help-overlay action")
                    .activate(None);
            });

        let copy_stream_url_action: RelmAction<CopyStreamURLAction> = RelmAction::new_stateless({
            let stream_url = model.stream_url.clone();
            move |_| {
                if let (Some(display), Ok(stream_url)) =
                    (gdk::Display::default(), stream_url.read())
                {
                    display.clipboard().set_text(&stream_url);
                    APP_BROKER.send(AppInput::Toast(
                        tr!("vp-menu-debug-copy-stream-url-toast.success").into(),
                        None,
                    ));
                } else {
                    APP_BROKER.send(AppInput::Toast(
                        tr!("vp-menu-debug-copy-stream-url-toast.fail").into(),
                        None,
                    ));
                }
            }
        });

        group.add_action(hardware_decoding_action);
        group.add_action(keyboard_shortcuts_action);
        group.add_action(copy_stream_url_action);
        group.register_for_widget(root);

        CONFIG.subscribe(sender.input_sender(), |config| {
            VideoPlayerMenuInput::ConfigUpdated(config.video_player.clone())
        });

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: relm4::ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            VideoPlayerMenuInput::ConfigUpdated(video_player_config) => {
                let _ = root.activate_action(
                    &HardwareDecodingAction::action_name(),
                    Some(&video_player_config.mpv_prefs.hardware_decoding.into()),
                );
            }
            VideoPlayerMenuInput::CurrentHwdecUpdated(hwdec) => {
                if let Some(mpv_props_section_pos) = self.mpv_props_section_pos {
                    self.debug_menu.remove(mpv_props_section_pos);
                }

                let mpv_props = gtk::gio::Menu::new();
                mpv_props.append(Some(&format!("hwdec-current: {hwdec}")), None);

                self.mpv_props_section_pos = Some(self.debug_menu.n_items());
                self.debug_menu
                    .append_section(Some(tr!("vp-menu-debug-mpv-props")), &mpv_props);
            }
            VideoPlayerMenuInput::SetStreamURL(stream_url) => {
                *self.stream_url.write().expect("Failed to set stream URL") = stream_url;
            }
        }
    }
}
