use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use glib::SignalHandlerId;
use relm4::gtk::{self, glib, prelude::*};
use tracing::{debug, error};
use uuid::Uuid;
use video_player_mpv::{Track, TrackType, VideoPlayerMpv};

use crate::{tr, utils::rgba::RGBA};

use super::{
    AudioTrack, Deinterlace, PlayerState, PlayerStateChangedCallback, SubtitleTrack,
    VideoPlayerBackend, VideoPlayerSubtitleFont,
};

fn uuid() -> Uuid {
    Uuid::new_v4()
}

macro_rules! set_player_state {
    ($state:expr, $player_state_changed_callbacks:expr, $new_state:expr$(,)?) => {
        $state.replace($new_state);
        for callback in $player_state_changed_callbacks.borrow().values() {
            callback($new_state);
        }
    };
}

pub struct VideoPlayerBackendMpv {
    widget: VideoPlayerMpv,
    state: Rc<Cell<PlayerState>>,
    player_state_changed_callbacks: Rc<RefCell<HashMap<Uuid, PlayerStateChangedCallback>>>,
    signal_handler_ids: HashMap<Uuid, SignalHandlerId>,
}

impl std::fmt::Debug for VideoPlayerBackendMpv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoPlayerBackendMpv")
            .field("widget", &self.widget)
            .field("state", &self.state)
            .field("signal_handler_ids", &self.signal_handler_ids)
            .finish_non_exhaustive()
    }
}

impl Default for VideoPlayerBackendMpv {
    fn default() -> Self {
        let widget = VideoPlayerMpv::new();
        let state = Rc::new(Cell::new(PlayerState::Loading));
        let player_state_changed_callbacks = Rc::default();
        let mpv = Self {
            widget,
            state,
            player_state_changed_callbacks,
            signal_handler_ids: HashMap::default(),
        };

        mpv.widget.connect_core_idle({
            let state = mpv.state.clone();
            let player_state_changed_callbacks = mpv.player_state_changed_callbacks.clone();
            move |_, core_idle| {
                if !core_idle && matches!(state.get(), PlayerState::Loading) {
                    set_player_state!(
                        state,
                        player_state_changed_callbacks,
                        PlayerState::Playing { paused: false },
                    );
                }
            }
        });

        mpv.widget.connect_pause({
            let state = mpv.state.clone();
            let player_state_changed_callbacks = mpv.player_state_changed_callbacks.clone();
            move |_, cur_paused| {
                match state.get() {
                    PlayerState::Playing { paused } if paused != cur_paused => {
                        set_player_state!(
                            state,
                            player_state_changed_callbacks,
                            PlayerState::Playing { paused: cur_paused },
                        );
                    }
                    _ => {}
                };
            }
        });

        mpv.widget.connect_seeking({
            let state = mpv.state.clone();
            let player_state_changed_callbacks = mpv.player_state_changed_callbacks.clone();
            move |_, seeking| {
                let new_state = match (state.get(), seeking) {
                    (PlayerState::Playing { paused: _ }, true) => Some(PlayerState::Buffering),
                    (PlayerState::Buffering, false) => Some(PlayerState::Playing { paused: false }),
                    _ => None,
                };
                if let Some(new_state) = new_state {
                    set_player_state!(state, player_state_changed_callbacks, new_state,);
                }
            }
        });

        mpv
    }
}

impl VideoPlayerBackendMpv {
    fn set_player_state(&mut self, state: PlayerState) {
        set_player_state!(self.state, self.player_state_changed_callbacks, state);
    }

    pub fn set_property_string(&self, name: &str, value: &str) {
        let res = self.widget.set_property_string(name, value);
        if res != 0 {
            error!("Error setting property {name} to {value}: {res}");
        }
    }

    pub fn connect_current_hwdec_updated(
        &self,
        callback: Box<dyn Fn(&str) + Send + Sync + 'static>,
    ) {
        self.widget.connect_current_hwdec(move |_, val| {
            callback(val);
        });
    }
}

impl VideoPlayerBackend for VideoPlayerBackendMpv {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn widget(&self) -> &gtk::Widget {
        self.widget.upcast_ref()
    }

    fn connect_player_state_changed(&mut self, callback: PlayerStateChangedCallback) -> Uuid {
        let id = Uuid::new_v4();
        self.player_state_changed_callbacks
            .borrow_mut()
            .insert(id, callback);
        id
    }

    fn disconnect_player_state_changed(&mut self, id: Uuid) {
        self.player_state_changed_callbacks.borrow_mut().remove(&id);
    }

    fn play_uri(&mut self, uri: &str) {
        self.widget.play_uri(uri);
        self.widget.play();
        self.set_player_state(PlayerState::Loading);
    }

    fn play(&self) {
        self.widget.play();
    }

    fn pause(&self) {
        self.widget.pause();
    }

    fn stop(&mut self) {
        self.widget.stop();
    }

    fn seek_to(&self, seconds: usize) {
        self.widget.seek_to(seconds as u32);
    }

    fn seek_by(&self, seconds: isize) {
        self.widget.seek_by(seconds as i32);
    }

    fn frame_step_forwards(&self) {
        self.widget.frame_step_forwards();
    }

    fn frame_step_backwards(&self) {
        self.widget.frame_step_backwards();
    }

    fn muted(&self) -> bool {
        self.widget.mute()
    }

    fn set_muted(&self, muted: bool) {
        self.widget.set_mute(muted);
    }

    fn volume(&self) -> f64 {
        self.widget.volume() / 100.0
    }

    fn set_volume(&self, volume: f64) {
        self.widget.set_volume(volume * 100.0);
    }

    fn position(&self) -> usize {
        self.widget.position() as usize
    }

    fn playback_speed(&self) -> f64 {
        self.widget.playback_speed()
    }

    fn set_playback_speed(&self, playback_speed: f64) {
        self.widget.set_playback_speed(playback_speed);
    }

    fn set_deinterlace(&self, deinterlace: Deinterlace) {
        self.widget.set_property_string(
            "deinterlace",
            match deinterlace {
                Deinterlace::Auto => "auto",
                Deinterlace::On => "yes",
                Deinterlace::Off => "no",
            },
        );
    }

    fn current_subtitle_track(&self) -> Option<usize> {
        match self.widget.current_subtitle_track() {
            id @ 0.. => Some(id as usize),
            _ => None,
        }
    }

    fn set_subtitle_track(&self, subtitle_track_id: Option<usize>) {
        self.widget
            .set_subtitle_track(subtitle_track_id.map_or(0, |id| id as u32));
    }

    fn add_subtitle_track(&self, url: &str, title: &str) {
        self.widget.add_subtitle_track(url, title);
    }

    fn current_audio_track(&self) -> Option<usize> {
        match self.widget.current_audio_track() {
            id @ 0.. => Some(id as usize),
            _ => None,
        }
    }

    fn set_audio_track(&self, audio_track_id: Option<usize>) {
        self.widget
            .set_audio_track(audio_track_id.map_or(0, |id| id as u32));
    }

    fn set_subtitle_scale(&self, subtitle_scale: f64) {
        self.widget.set_subtitle_scale(subtitle_scale);
    }

    fn set_subtitle_colour(&self, colour: RGBA) {
        self.widget.set_subtitle_colour(&colour.to_mpv_hex());
    }

    fn set_subtitle_background_colour(&self, colour: RGBA) {
        self.widget
            .set_subtitle_background_colour(&colour.to_mpv_hex());
    }

    fn set_subtitle_position(&self, position: u32) {
        assert!((0..150).contains(&position));
        self.widget.set_subtitle_position(position);
    }

    fn set_subtitle_font(&self, font: &VideoPlayerSubtitleFont) {
        let player = &self.widget;
        player.set_subtitle_font_family(&font.family);
        player.set_subtitle_font_size(font.size as u32);
        player.set_subtitle_font_bold(font.bold);
        player.set_subtitle_font_italic(font.italic);
    }

    fn connect_initialized(&mut self, callback: Box<dyn Fn() + Send + 'static>) -> Uuid {
        let id = uuid();
        self.signal_handler_ids.insert(
            id,
            self.widget.connect_initialized(move |_| {
                callback();
            }),
        );
        id
    }

    fn connect_end_of_stream(&mut self, callback: Box<dyn Fn() + Send + 'static>) -> Uuid {
        let id = uuid();
        self.signal_handler_ids.insert(
            id,
            self.widget.connect_end_of_file(move |_| {
                callback();
            }),
        );
        id
    }

    fn connect_position_updated(
        &mut self,
        callback: Box<dyn Fn(usize) + Send + Sync + 'static>,
    ) -> Uuid {
        let id = uuid();
        self.signal_handler_ids.insert(
            id,
            self.widget.connect_position_updated(move |_, val| {
                callback(val as usize);
            }),
        );
        id
    }

    fn connect_duration_updated(
        &mut self,
        callback: Box<dyn Fn(usize) + Send + Sync + 'static>,
    ) -> Uuid {
        let id = uuid();
        self.signal_handler_ids.insert(
            id,
            self.widget.connect_duration_updated(move |_, val| {
                callback(val as usize);
            }),
        );
        id
    }

    fn connect_mute_updated(
        &mut self,
        callback: Box<dyn Fn(bool) + Send + Sync + 'static>,
    ) -> Uuid {
        let id = uuid();
        self.signal_handler_ids.insert(
            id,
            self.widget.connect_mute_updated(move |_, muted| {
                callback(muted);
            }),
        );
        id
    }

    fn connect_volume_updated(
        &mut self,
        callback: Box<dyn Fn(f64) + Send + Sync + 'static>,
    ) -> Uuid {
        let id = uuid();
        self.signal_handler_ids.insert(
            id,
            self.widget.connect_volume_updated(move |_, volume| {
                callback(volume / 100.0);
            }),
        );
        id
    }

    fn connect_subtitle_tracks_updated(
        &mut self,
        callback: Box<dyn Fn(Vec<SubtitleTrack>) + Send + Sync + 'static>,
    ) -> Uuid {
        let id = uuid();
        self.signal_handler_ids.insert(
            id,
            self.widget
                .connect_tracks_updated(move |_player, track_list| {
                    let mut subtitle_tracks: Vec<SubtitleTrack> = vec![];
                    for i in 0..track_list.len() {
                        if let Some(track) = track_list.track(i) {
                            if let TrackType::Subtitle = track.type_() {
                                subtitle_tracks.push(track.into());
                            }
                        }
                    }

                    callback(subtitle_tracks);
                }),
        );
        id
    }

    fn connect_audio_tracks_updated(
        &mut self,
        callback: Box<dyn Fn(Vec<AudioTrack>) + Send + Sync + 'static>,
    ) -> Uuid {
        let id = uuid();
        self.signal_handler_ids.insert(
            id,
            self.widget
                .connect_tracks_updated(move |_player, track_list| {
                    let mut audio_tracks: Vec<AudioTrack> = vec![];
                    for i in 0..track_list.len() {
                        if let Some(track) = track_list.track(i) {
                            if let TrackType::Audio = track.type_() {
                                audio_tracks.push(track.into());
                            }
                        }
                    }

                    callback(audio_tracks);
                }),
        );
        id
    }

    fn connect_deinterlace_active_updated(
        &mut self,
        callback: Box<dyn Fn(bool) + Send + Sync + 'static>,
    ) -> Uuid {
        let id = uuid();
        self.signal_handler_ids.insert(
            id,
            self.widget
                .connect_deinterlace_active(move |_, deinterlace_active| {
                    callback(deinterlace_active);
                }),
        );
        id
    }

    fn disconnect_signal_handler(&mut self, id: &Uuid) {
        match self.signal_handler_ids.remove(id) {
            Some(signal_handler_id) => {
                self.widget.disconnect(signal_handler_id);
            }
            None => {
                debug!("Signal handler not found when trying to disconnect: {id}");
            }
        };
    }
}

fn get_track_name(track: &Track) -> String {
    let id = track.id() as usize;
    let title = track.title().map(|s| s.to_string());
    let language = track.language().map(|s| s.to_string());

    match (title, language) {
        (Some(title), Some(language)) => tr!(
            "vp-backend-mpv-track-name.title-and-language",
            {
                "title" => title,
                "language" => language,
            },
        )
        .to_string(),
        (Some(title), None) => title,
        (None, Some(language)) => tr!(
            "vp-backend-mpv-track-name.id-and-language",
            {
                "id" => id,
                "language" => language,
            },
        )
        .to_string(),
        _ => tr!("vp-backend-mpv-track-name.id", {"id" => id}).to_string(),
    }
}

impl From<Track> for SubtitleTrack {
    fn from(value: Track) -> Self {
        assert!(value.type_() == TrackType::Subtitle);
        Self {
            id: value.id() as usize,
            name: get_track_name(&value),
        }
    }
}

impl From<Track> for AudioTrack {
    fn from(value: Track) -> Self {
        assert!(value.type_() == TrackType::Audio);
        Self {
            id: value.id() as usize,
            name: get_track_name(&value),
        }
    }
}
