mod subtitles_preferences;

use adw::prelude::*;
use relm4::prelude::*;

use crate::{
    config::video_player_config::{
        MpvHardwareDecoding, VideoPlayerBackendPreference, VideoPlayerConfig,
        VideoPlayerOnLeftClick,
    },
    globals::CONFIG,
    tr,
};

use self::subtitles_preferences::SubtitlesPreferences;

pub struct VideoPlayerPreferences {
    video_player_config: VideoPlayerConfig,
    subtitles_preferences: Controller<SubtitlesPreferences>,
}

#[derive(Debug)]
pub enum VideoPlayerPreferencesInput {
    UpdateConfig(VideoPlayerConfig),

    SkipBackwardsAmount(usize),
    SkipForwardsAmount(usize),
    OnLeftClick(u32),

    IntroSkipper(bool),
    IntroSkipperAutoSkip(bool),
    Jellyscrub(bool),

    MpvHardwareDecoding(u32),
    MpvHardwareDecodingCustom(String),
    Backend(u32),
    HlsPlayback(bool),
}

#[relm4::component(pub)]
impl Component for VideoPlayerPreferences {
    type Init = ();
    type Input = VideoPlayerPreferencesInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        &adw::PreferencesPage {
            set_title: tr!("prefs-vp-page"),
            set_icon_name: Some("play-filled"),

            add = &adw::PreferencesGroup {
                set_title: tr!("prefs-vp-interface"),

                add = &adw::SpinRow::new(
                    Some(&gtk::Adjustment::new(
                        model.video_player_config.skip_backwards_amount as f64,
                        1.0, 60.0, 1.0, 10.0, 0.0,
                    )),
                    // Climb rate
                    1.0,
                    // Digits
                    0,
                ) {
                    set_title: tr!("prefs-vp-skip-backwards.title"),
                    set_subtitle: tr!("prefs-vp-skip-backwards.subtitle"),
                    connect_changed[sender] => move |spinrow| {
                        sender.input(VideoPlayerPreferencesInput::SkipBackwardsAmount(spinrow.value() as usize));
                    },
                },

                add = &adw::SpinRow::new(
                    Some(&gtk::Adjustment::new(
                        model.video_player_config.skip_forwards_amount as f64,
                        1.0, 60.0, 1.0, 10.0, 0.0,
                    )),
                    // Climb rate
                    1.0,
                    // Digits
                    0,
                ) {
                    set_title: tr!("prefs-vp-skip-forwards.title"),
                    set_subtitle: tr!("prefs-vp-skip-forwards.subtitle"),
                    connect_changed[sender] => move |spinrow| {
                        sender.input(VideoPlayerPreferencesInput::SkipForwardsAmount(spinrow.value() as usize));
                    },
                },

                add = &adw::ComboRow {
                    set_title: tr!("prefs-vp-on-left-click.title"),
                    set_subtitle: tr!("prefs-vp-on-left-click.subtitle"),
                    #[wrap(Some)]
                    set_model = &gtk::StringList::new(&[
                        tr!("prefs-vp-on-left-click-options.play-pause"),
                        tr!("prefs-vp-on-left-click-options.toggle-controls"),
                    ]),
                    set_selected: if let VideoPlayerOnLeftClick::PlayPause = video_player_config.on_left_click { 0 } else { 1 },
                    connect_selected_notify[sender] => move |cb| {
                        sender.input(VideoPlayerPreferencesInput::OnLeftClick(cb.selected()));
                    },
                },
            },

            add = model.subtitles_preferences.widget(),

            add = &adw::PreferencesGroup {
                set_title: tr!("prefs-vp-plugins"),

                #[name = "intro_skipper_expander"]
                add = &adw::ExpanderRow {
                    set_title: tr!("prefs-vp-intro-skipper.title"),
                    set_subtitle: tr!("prefs-vp-intro-skipper.subtitle", {
                        "introSkipperUrl" => "https://github.com/ConfusedPolarBear/intro-skipper/",
                    }),
                    set_show_enable_switch: false,
                    #[watch]
                    set_enable_expansion: model.video_player_config.intro_skipper,
                    #[watch]
                    set_expanded: model.video_player_config.intro_skipper,

                    add_suffix = &gtk::Switch {
                        set_valign: gtk::Align::Center,
                        set_active: video_player_config.intro_skipper,
                        connect_active_notify[sender] => move |switch| {
                            sender.input(VideoPlayerPreferencesInput::IntroSkipper(switch.is_active()));
                        },
                    },

                    add_row = &adw::SwitchRow {
                        set_title: tr!("prefs-vp-intro-skipper-auto-skip.title"),
                        set_subtitle: tr!("prefs-vp-intro-skipper-auto-skip.subtitle"),
                        set_active: video_player_config.intro_skipper_auto_skip,
                        connect_active_notify[sender] => move |sr| {
                            sender.input(VideoPlayerPreferencesInput::IntroSkipperAutoSkip(sr.is_active()));
                        },
                        #[watch]
                        set_sensitive: model.video_player_config.intro_skipper,
                    },
                },

                add = &adw::SwitchRow {
                    set_title: tr!("prefs-vp-jellyscrub.title"),
                    set_subtitle: tr!("prefs-vp-jellyscrub.subtitle", {
                        "jellyscrubUrl" => "https://github.com/nicknsy/jellyscrub/",
                    }),
                    set_active: video_player_config.jellyscrub,
                    connect_active_notify[sender] => move |sr| {
                        sender.input(VideoPlayerPreferencesInput::Jellyscrub(sr.is_active()));
                    },
                },
            },

            add = &adw::PreferencesGroup {
                set_title: tr!("prefs-vp-other"),

                add = &adw::ComboRow {
                    set_title: tr!("prefs-vp-mpv-hwdec.title"),
                    set_subtitle: tr!("prefs-vp-mpv-hwdec.subtitle", {
                        "mpvManualUrl" => "https://mpv.io/manual/master/#options-hwdec",
                    }),
                    set_use_markup: true,
                    #[wrap(Some)]
                    set_model = &gtk::StringList::new(&[
                        tr!("prefs-vp-mpv-hwdec-options.auto"),
                        tr!("prefs-vp-mpv-hwdec-options.auto-copy"),
                        tr!("prefs-vp-mpv-hwdec-options.auto-safe"),
                        tr!("prefs-vp-mpv-hwdec-options.custom"),
                        tr!("prefs-vp-mpv-hwdec-options.off"),
                    ]),
                    set_selected: video_player_config.mpv_prefs.hardware_decoding.clone().into(),
                    connect_selected_notify[sender] => move |cb| {
                        sender.input(VideoPlayerPreferencesInput::MpvHardwareDecoding(cb.selected()));
                    },
                },

                add = &adw::EntryRow {
                    set_title: "Custom hardware decoding option",
                    #[watch]
                    set_visible: matches!(model.video_player_config.mpv_prefs.hardware_decoding, MpvHardwareDecoding::Custom),
                    set_show_apply_button: true,
                    set_text: &model.video_player_config.mpv_prefs.hardware_decoding_custom,
                    connect_apply[sender] => move |entry| {
                        sender.input(VideoPlayerPreferencesInput::MpvHardwareDecodingCustom(entry.text().to_string()));
                    },
                },

                add = &adw::ExpanderRow {
                    set_title: tr!("prefs-vp-experimental.title"),
                    set_subtitle: tr!("prefs-vp-experimental.subtitle"),

                    add_row = &adw::ComboRow {
                        set_visible: cfg!(feature = "gst"),
                        set_title: tr!("prefs-vp-backend.title"),
                        set_subtitle: tr!("prefs-vp-backend.subtitle"),
                        #[wrap(Some)]
                        set_model = &gtk::StringList::new(&[
                            tr!("prefs-vp-backend.value-mpv"),
                            tr!("prefs-vp-backend.value-gstreamer"),
                        ]),
                        set_selected: if let VideoPlayerBackendPreference::Mpv = video_player_config.backend { 0 } else { 1 },
                        connect_selected_notify[sender] => move |cb| {
                            sender.input(VideoPlayerPreferencesInput::Backend(cb.selected()));
                        },
                    },

                    add_row = &adw::SwitchRow {
                        set_title: tr!("prefs-vp-hls-playback.title"),
                        set_subtitle: tr!("prefs-vp-hls-playback.subtitle"),
                        set_active: video_player_config.hls_playback,
                        connect_active_notify[sender] => move |sr| {
                            sender.input(VideoPlayerPreferencesInput::HlsPlayback(sr.is_active()));
                        },
                    },
                },
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = CONFIG.read();
        let video_player_config = &config.video_player;

        let model = VideoPlayerPreferences {
            video_player_config: video_player_config.clone(),
            subtitles_preferences: SubtitlesPreferences::builder().launch(()).detach(),
        };
        VideoPlayerPreferences::subscribe_to_config(&sender);

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if let VideoPlayerPreferencesInput::UpdateConfig(video_player_config) = message {
            self.video_player_config = video_player_config;
            self.update_view(widgets, sender);
            return;
        }

        let mut config = CONFIG.write();

        match message {
            VideoPlayerPreferencesInput::UpdateConfig(_) => {
                // Already handled above
                unreachable!();
            }
            VideoPlayerPreferencesInput::SkipBackwardsAmount(value) => {
                config.video_player.skip_backwards_amount = value;
            }
            VideoPlayerPreferencesInput::SkipForwardsAmount(value) => {
                config.video_player.skip_forwards_amount = value;
            }
            VideoPlayerPreferencesInput::OnLeftClick(index) => {
                config.video_player.on_left_click = match index {
                    0 => VideoPlayerOnLeftClick::PlayPause,
                    _ => VideoPlayerOnLeftClick::ToggleControls,
                };
            }

            VideoPlayerPreferencesInput::IntroSkipper(intro_skipper) => {
                config.video_player.intro_skipper = intro_skipper;
            }
            VideoPlayerPreferencesInput::IntroSkipperAutoSkip(intro_skipper_auto_skip) => {
                config.video_player.intro_skipper_auto_skip = intro_skipper_auto_skip;
            }
            VideoPlayerPreferencesInput::Jellyscrub(jellyscrub) => {
                config.video_player.jellyscrub = jellyscrub;
            }

            VideoPlayerPreferencesInput::MpvHardwareDecoding(mpv_hardware_decoding) => {
                config.video_player.mpv_prefs.hardware_decoding = mpv_hardware_decoding.into()
            }
            VideoPlayerPreferencesInput::MpvHardwareDecodingCustom(value) => {
                config.video_player.mpv_prefs.hardware_decoding_custom = value;
            }
            VideoPlayerPreferencesInput::Backend(index) => {
                config.video_player.backend = match index {
                    0 => VideoPlayerBackendPreference::Mpv,
                    _ => VideoPlayerBackendPreference::Gst,
                };
            }
            VideoPlayerPreferencesInput::HlsPlayback(active) => {
                config.video_player.hls_playback = active;
            }
        };

        config.save().expect("Error saving config");

        self.update_view(widgets, sender);
    }
}

impl VideoPlayerPreferences {
    fn subscribe_to_config(sender: &ComponentSender<Self>) {
        CONFIG.subscribe(sender.input_sender(), |config| {
            VideoPlayerPreferencesInput::UpdateConfig(config.video_player.clone())
        });
    }
}
