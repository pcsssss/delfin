use std::{cell::RefCell, sync::Arc};

use relm4::gtk::glib;
use serde::{Deserialize, Serialize};

use crate::{
    tr,
    utils::round::round_one_place,
    video_player::backends::{
        mpv::VideoPlayerBackendMpv, VideoPlayerBackend, VideoPlayerSubtitleFont,
    },
};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(default)]
pub struct VideoPlayerConfig {
    pub position_update_frequency: usize,
    pub volume: f64,
    pub muted: bool,

    pub skip_backwards_amount: usize,
    pub skip_forwards_amount: usize,
    pub on_left_click: VideoPlayerOnLeftClick,
    pub duration_display: DurationDisplay,

    pub subtitles: VideoPlayerSubtitlesConfig,

    pub intro_skipper: bool,
    pub intro_skipper_auto_skip: bool,
    pub jellyscrub: bool,

    pub backend: VideoPlayerBackendPreference,
    pub hls_playback: bool,
    pub deinterlace: Deinterlace,

    pub mpv_prefs: MpvPrefs,
}

impl Default for VideoPlayerConfig {
    fn default() -> Self {
        Self {
            position_update_frequency: 10,
            volume: 1.0,
            muted: false,

            skip_backwards_amount: 10,
            skip_forwards_amount: 30,
            on_left_click: VideoPlayerOnLeftClick::default(),
            duration_display: DurationDisplay::default(),

            subtitles: VideoPlayerSubtitlesConfig::default(),

            intro_skipper: true,
            intro_skipper_auto_skip: true,
            jellyscrub: true,

            backend: VideoPlayerBackendPreference::default(),
            hls_playback: false,
            deinterlace: Deinterlace::default(),

            mpv_prefs: MpvPrefs::default(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Clone, Copy)]
pub enum VideoPlayerOnLeftClick {
    #[default]
    PlayPause,
    ToggleControls,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct VideoPlayerSubtitlesConfig {
    #[serde(serialize_with = "round_one_place")]
    pub scale: f64,
    pub colour: String,
    pub background_colour: String,
    pub position: u32,
    pub font: VideoPlayerSubtitleFont,
}

impl Default for VideoPlayerSubtitlesConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            colour: "#FFFFFFFF".into(),
            background_colour: "#00000000".into(),
            position: 100,
            font: VideoPlayerSubtitleFont::default(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum VideoPlayerBackendPreference {
    #[default]
    Mpv,
    Gst,
}

impl From<VideoPlayerBackendPreference> for Arc<RefCell<dyn VideoPlayerBackend>> {
    fn from(val: VideoPlayerBackendPreference) -> Self {
        match val {
            VideoPlayerBackendPreference::Mpv => Arc::<RefCell<VideoPlayerBackendMpv>>::default(),
            VideoPlayerBackendPreference::Gst => {
                #[cfg(feature = "gst")]
                {
                    Arc::<RefCell<crate::video_player::backends::gst::VideoPlayerBackendGst>>::default()
                }

                #[cfg(not(feature = "gst"))]
                {
                    tracing::warn!("GStreamer backend not available, falling back to MPV backend");
                    Arc::<RefCell<VideoPlayerBackendMpv>>::default()
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub enum DurationDisplay {
    #[default]
    Total,
    Remaining,
}

impl DurationDisplay {
    pub fn toggle(&self) -> Self {
        match self {
            Self::Total => Self::Remaining,
            Self::Remaining => Self::Total,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, glib::Variant)]
pub enum Deinterlace {
    Auto,
    On,
    #[default]
    Off,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
pub struct MpvPrefs {
    pub hardware_decoding: MpvHardwareDecoding,
    // We store the custom string separately so the user doesn't lose it when swapping between the
    // available options
    pub hardware_decoding_custom: String,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, glib::Variant)]
pub enum MpvHardwareDecoding {
    Auto = 0,
    AutoCopy,
    #[default]
    AutoSafe,
    Custom,
    Off,
}

impl MpvHardwareDecoding {
    pub fn label(self) -> String {
        match self {
            Self::Auto => tr!("prefs-vp-mpv-hwdec-options.auto").into(),
            Self::AutoCopy => tr!("prefs-vp-mpv-hwdec-options.auto-copy").into(),
            Self::AutoSafe => tr!("prefs-vp-mpv-hwdec-options.auto-safe").into(),
            Self::Custom => tr!("prefs-vp-mpv-hwdec-options.custom").into(),
            Self::Off => tr!("prefs-vp-mpv-hwdec-options.off").into(),
        }
    }
}

impl From<u32> for MpvHardwareDecoding {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Auto,
            1 => Self::AutoCopy,
            2 => Self::AutoSafe,
            3 => Self::Custom,
            4 => Self::Off,
            _ => unreachable!("Hardare decoding option {value} does not exist"),
        }
    }
}

impl From<MpvHardwareDecoding> for u32 {
    fn from(value: MpvHardwareDecoding) -> Self {
        value as u32
    }
}

impl MpvPrefs {
    pub fn get_hardware_decoding(&self) -> String {
        match self.hardware_decoding {
            MpvHardwareDecoding::Auto => "auto".into(),
            MpvHardwareDecoding::AutoCopy => "auto-copy".into(),
            MpvHardwareDecoding::AutoSafe => "auto-safe".into(),
            MpvHardwareDecoding::Custom => self.hardware_decoding_custom.to_owned(),
            MpvHardwareDecoding::Off => "no".into(),
        }
    }
}
