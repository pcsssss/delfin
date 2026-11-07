use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use unic_langid::LanguageIdentifier;
use uuid::Uuid;

use crate::{
    config::{self, migrate::Migrate, Config},
    utils::round::round_one_place,
    video_player::backends::VideoPlayerSubtitleFont,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ConfigV1 {
    language: Option<LanguageIdentifier>,
    #[serde(default)]
    window: Window,

    device_id: String,
    servers: Vec<Server>,

    #[serde(default)]
    general: GeneralConfig,

    #[serde(default)]
    video_player: VideoPlayerConfig,
}

impl Default for ConfigV1 {
    fn default() -> Self {
        Self {
            language: None,
            window: Window::default(),
            device_id: Uuid::new_v4().to_string(),
            servers: Vec::default(),
            general: GeneralConfig::default(),
            video_player: VideoPlayerConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Server {
    id: String,
    url: String,
    name: String,
    accounts: Vec<Account>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Account {
    id: String,
    username: String,
    access_token: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Window {
    width: usize,
    height: usize,
    maximized: bool,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            width: 960,
            height: 540,
            maximized: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
enum Theme {
    #[default]
    Default,
    Light,
    Dark,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct GeneralConfig {
    #[serde(default)]
    theme: Theme,
}

#[derive(Debug, Deserialize_repr, Serialize_repr, PartialEq, Clone, Copy)]
#[repr(u8)]
enum VideoPlayerSkipAmount {
    Ten = 10,
    Thirty = 30,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, Copy)]
enum VideoPlayerOnLeftClick {
    PlayPause,
    ToggleControls,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
enum VideoPlayerBackendPreference {
    Mpv,
    Gst,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
struct VideoPlayerConfig {
    position_update_frequency: usize,
    volume: f64,
    muted: bool,

    skip_backwards_amount: VideoPlayerSkipAmount,
    skip_forwards_amount: VideoPlayerSkipAmount,
    on_left_click: VideoPlayerOnLeftClick,

    #[serde(serialize_with = "round_one_place")]
    subtitle_scale: f64,
    subtitle_colour: String,
    subtitle_background_colour: String,
    subtitle_position: u32,
    subtitle_font: VideoPlayerSubtitleFont,

    intro_skipper: bool,
    intro_skipper_auto_skip: bool,
    jellyscrub: bool,

    backend: VideoPlayerBackendPreference,
    hls_playback: bool,
}

impl Default for VideoPlayerConfig {
    fn default() -> Self {
        Self {
            position_update_frequency: 10,
            volume: 1.0,
            muted: false,

            skip_backwards_amount: VideoPlayerSkipAmount::Ten,
            skip_forwards_amount: VideoPlayerSkipAmount::Thirty,
            on_left_click: VideoPlayerOnLeftClick::PlayPause,

            subtitle_scale: 1.0,
            subtitle_colour: "#FFFFFFFF".into(),
            subtitle_background_colour: "#00000000".into(),
            subtitle_position: 100,
            subtitle_font: VideoPlayerSubtitleFont::default(),

            backend: VideoPlayerBackendPreference::Mpv,
            hls_playback: false,
            intro_skipper: true,
            intro_skipper_auto_skip: true,
            jellyscrub: true,
        }
    }
}

impl Migrate<Config> for ConfigV1 {
    fn migrate(self) -> Config {
        use config::{
            video_player_config::VideoPlayerSubtitlesConfig, Account, GeneralConfig, Server,
            VideoPlayerConfig, Window,
        };

        let servers = self
            .servers
            .into_iter()
            .map(|server| Server {
                id: Uuid::parse_str(&server.id).expect("Failed to migrate server ID"),
                url: server.url,
                name: server.name,
                accounts: server
                    .accounts
                    .into_iter()
                    .map(|account| Account {
                        id: Uuid::parse_str(&account.id).expect("Failed to migrate account ID"),
                        username: account.username,
                        access_token: account.access_token,
                        device_id: Uuid::parse_str(&self.device_id)
                            .expect("Failed to migrate device ID"),
                    })
                    .collect(),
            })
            .collect();

        Config {
            version: 2,
            window: Window {
                width: self.window.width,
                height: self.window.height,
                maximized: self.window.maximized,
            },
            servers,
            seerr: None,
            general: GeneralConfig {
                language: self.language,
                theme: match self.general.theme {
                    Theme::Light => config::general::THEME_LIGHT,
                    Theme::Dark => config::general::THEME_DARK,
                    Theme::Default => config::general::Theme::default(),
                },
                ..Default::default()
            },
            video_player: VideoPlayerConfig {
                position_update_frequency: self.video_player.position_update_frequency,
                volume: self.video_player.volume,
                muted: self.video_player.muted,
                skip_backwards_amount: match self.video_player.skip_backwards_amount {
                    VideoPlayerSkipAmount::Ten => 10,
                    VideoPlayerSkipAmount::Thirty => 30,
                },
                skip_forwards_amount: match self.video_player.skip_forwards_amount {
                    VideoPlayerSkipAmount::Ten => 10,
                    VideoPlayerSkipAmount::Thirty => 30,
                },
                on_left_click: match self.video_player.on_left_click {
                    VideoPlayerOnLeftClick::PlayPause => {
                        config::video_player_config::VideoPlayerOnLeftClick::PlayPause
                    }
                    VideoPlayerOnLeftClick::ToggleControls => {
                        config::video_player_config::VideoPlayerOnLeftClick::ToggleControls
                    }
                },
                subtitles: VideoPlayerSubtitlesConfig {
                    scale: self.video_player.subtitle_scale,
                    colour: self.video_player.subtitle_colour,
                    background_colour: self.video_player.subtitle_background_colour,
                    position: self.video_player.subtitle_position,
                    font: self.video_player.subtitle_font,
                },
                backend: match self.video_player.backend {
                    VideoPlayerBackendPreference::Mpv => {
                        config::video_player_config::VideoPlayerBackendPreference::Mpv
                    }
                    VideoPlayerBackendPreference::Gst => {
                        config::video_player_config::VideoPlayerBackendPreference::Gst
                    }
                },
                hls_playback: self.video_player.hls_playback,
                intro_skipper: self.video_player.intro_skipper,
                intro_skipper_auto_skip: self.video_player.intro_skipper_auto_skip,
                jellyscrub: self.video_player.jellyscrub,
                ..Default::default()
            },
        }
    }
}
