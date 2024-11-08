use std::{any::Any, cell::RefCell, fmt, sync::Arc};

use mpv::VideoPlayerBackendMpv;
use relm4::gtk;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{config::video_player_config::Deinterlace, utils::rgba::RGBA};

#[cfg(feature = "gst")]
pub mod gst;

pub mod mpv;

#[derive(Clone, Debug)]
pub struct SubtitleTrack {
    pub id: usize,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct AudioTrack {
    pub id: usize,
    pub name: String,
}

#[derive(Clone, Copy, Debug)]
pub enum PlayerState {
    Loading,
    Playing { paused: bool },
    Buffering,
}

type PlayerStateChangedCallback = Box<dyn Fn(PlayerState)>;

pub trait VideoPlayerBackend: Any + fmt::Debug {
    fn as_any(&self) -> &dyn Any;

    /// Returns widget that renders the video.
    fn widget(&self) -> &gtk::Widget;

    /// Get notified when the player state changes.
    fn connect_player_state_changed(&mut self, callback: PlayerStateChangedCallback) -> Uuid;

    /// Disconnect a player state changed callback.
    fn disconnect_player_state_changed(&mut self, id: Uuid);

    /// Stream video from the given URI.
    fn play_uri(&mut self, uri: &str);

    /// Play the current video.
    fn play(&self);

    /// Pause the current video.
    fn pause(&self);

    /// Stops the current video and performs clean up.
    fn stop(&mut self);

    /// Seek to timestamp in seconds.
    fn seek_to(&self, seconds: usize);

    /// Seek by seconds relative to current timestamp.
    fn seek_by(&self, seconds: isize);

    /// Step forwards by one frame.
    fn frame_step_forwards(&self);

    /// Step backwards by one frame.
    fn frame_step_backwards(&self);

    /// Check if the player is currently muted.
    fn muted(&self) -> bool;

    /// Set if audio is muted.
    fn set_muted(&self, muted: bool);

    /// Get the current volume level.
    fn volume(&self) -> f64;

    /// Set volume in percentage (0 - 1).
    fn set_volume(&self, volume: f64);

    /// Get current video position in seconds.
    fn position(&self) -> usize;

    /// Get the current playback speed.
    fn playback_speed(&self) -> f64;

    /// Set the playback speed.
    fn set_playback_speed(&self, playback_speed: f64);

    /// Set deinterlacing.
    fn set_deinterlace(&self, deinterlace: Deinterlace);

    /// Get the current subtitle track ID.
    fn current_subtitle_track(&self) -> Option<usize>;

    /// Set the current subtitle track.
    fn set_subtitle_track(&self, subtitle_track_id: Option<usize>);

    /// Add a subtitle track.
    fn add_subtitle_track(&self, url: &str, title: &str);

    /// Get the current audio track ID.
    fn current_audio_track(&self) -> Option<usize>;

    /// Set the current audio track.
    fn set_audio_track(&self, audio_track_id: Option<usize>);

    /// Set Subtitle scale.
    fn set_subtitle_scale(&self, subtitle_scale: f64);

    /// Set subtitle text colour.
    fn set_subtitle_colour(&self, colour: RGBA);

    /// Set subtitle background colour.
    fn set_subtitle_background_colour(&self, colour: RGBA);

    /// Set subtitle position from 0 - 150, where 0 is the top of the screen and 100 is the bottom.
    fn set_subtitle_position(&self, position: u32);

    /// Set subtitle font.
    fn set_subtitle_font(&self, font: &VideoPlayerSubtitleFont);

    /// Get notified when the video player has been initialized.
    fn connect_initialized(&mut self, callback: Box<dyn Fn() + Send + 'static>) -> Uuid;

    /// Get notified when video player reaches the end of the current video.
    fn connect_end_of_stream(&mut self, callback: Box<dyn Fn() + Send + 'static>) -> Uuid;

    /// Get notified when the playback position changes.
    fn connect_position_updated(
        &mut self,
        callback: Box<dyn Fn(usize) + Send + Sync + 'static>,
    ) -> Uuid;

    /// Get notified when the media duration changes.
    fn connect_duration_updated(
        &mut self,
        callback: Box<dyn Fn(usize) + Send + Sync + 'static>,
    ) -> Uuid;

    /// Get notified when the player is muted or unmuted.
    fn connect_mute_updated(&mut self, callback: Box<dyn Fn(bool) + Send + Sync + 'static>)
        -> Uuid;

    /// Get notified when the player volume changes.
    fn connect_volume_updated(
        &mut self,
        callback: Box<dyn Fn(f64) + Send + Sync + 'static>,
    ) -> Uuid;

    /// Get notified when the list of avilable subtitle tracks changes.
    fn connect_subtitle_tracks_updated(
        &mut self,
        callback: Box<dyn Fn(Vec<SubtitleTrack>) + Send + Sync + 'static>,
    ) -> Uuid;

    /// Get notified when the list of avilable audio tracks changes.
    fn connect_audio_tracks_updated(
        &mut self,
        callback: Box<dyn Fn(Vec<AudioTrack>) + Send + Sync + 'static>,
    ) -> Uuid;

    /// Get notified when deinterlacing is activated or deactivated.
    fn connect_deinterlace_active_updated(
        &mut self,
        callback: Box<dyn Fn(bool) + Send + Sync + 'static>,
    ) -> Uuid;

    // Disconnects the signal handler for the given ID.
    fn disconnect_signal_handler(&mut self, id: &Uuid);
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct VideoPlayerSubtitleFont {
    pub family: String,
    pub size: usize,
    pub bold: bool,
    pub italic: bool,
}

impl Default for VideoPlayerSubtitleFont {
    fn default() -> Self {
        Self {
            family: "Sans".into(),
            size: 55,
            bold: false,
            italic: false,
        }
    }
}

impl From<VideoPlayerSubtitleFont> for relm4::gtk::pango::FontDescription {
    fn from(font: VideoPlayerSubtitleFont) -> Self {
        use relm4::gtk::pango::{FontDescription, Style, Weight};

        let mut desc = FontDescription::new();
        desc.set_family(&font.family);
        desc.set_size((font.size * 1024) as i32);
        desc.set_weight(if font.bold {
            Weight::Bold
        } else {
            Weight::Normal
        });
        desc.set_style(if font.italic {
            Style::Italic
        } else {
            Style::Normal
        });
        desc
    }
}

pub fn with_backend_mpv<F, T>(
    backend: &Arc<RefCell<dyn VideoPlayerBackend>>,
    callback: F,
) -> Option<T>
where
    F: FnOnce(&VideoPlayerBackendMpv) -> T,
{
    backend.borrow().as_any().downcast_ref().map(callback)
}
