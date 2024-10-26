use gst::{glib::SignalHandlerId, prelude::Cast};
use gstplay::{traits::PlayStreamInfoExt, PlayAudioInfo, PlaySubtitleInfo};
use uuid::Uuid;
use video_player_gst::{gst, gstplay, GstVideoPlayer};

use crate::tr;

use super::{AudioTrack, PlayerStateChangedCallback, SubtitleTrack, VideoPlayerBackend};

#[derive(Debug)]
pub struct VideoPlayerBackendGst {
    player: GstVideoPlayer,
    signal_handler_ids: Vec<SignalHandlerId>,
}

impl Default for VideoPlayerBackendGst {
    fn default() -> Self {
        let player = GstVideoPlayer::new();
        Self {
            player,
            signal_handler_ids: vec![],
        }
    }
}

impl VideoPlayerBackend for VideoPlayerBackendGst {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn widget(&self) -> &libadwaita::gtk::Widget {
        self.player.upcast_ref()
    }

    fn connect_player_state_changed(&mut self, _callback: PlayerStateChangedCallback) -> Uuid {
        todo!();
    }

    fn disconnect_player_state_changed(&mut self, _id: Uuid) {
        todo!();
    }

    fn play_uri(&mut self, uri: &str) {
        self.player.play_uri(uri);
    }

    fn play(&self) {
        for signal_handler in &self.signal_handler_ids {
            self.player.signal_adapter_unblock_signal(signal_handler);
        }

        self.player.play();
    }

    fn pause(&self) {
        self.player.pause();
    }

    fn stop(&mut self) {
        self.player.stop();

        for signal_handler in &self.signal_handler_ids {
            self.player.signal_adapter_block_signal(signal_handler);
        }
    }

    fn seek_to(&self, seconds: usize) {
        self.player
            .seek(gst::ClockTime::from_seconds(seconds as u64));
    }

    fn seek_by(&self, seconds: isize) {
        if let Some(position) = self.player.position() {
            let amount = gst::ClockTime::from_seconds(seconds.unsigned_abs() as u64);
            if seconds > 0 {
                self.player.seek(position.saturating_add(amount));
            } else {
                self.player.seek(position.saturating_sub(amount));
            }
        }
    }

    fn muted(&self) -> bool {
        todo!();
    }

    fn set_muted(&self, muted: bool) {
        self.player.set_mute(muted);
    }

    fn volume(&self) -> f64 {
        todo!();
    }

    fn set_volume(&self, volume: f64) {
        self.player.set_volume(volume);
    }

    fn position(&self) -> usize {
        self.player
            .position()
            .map(|t| t.seconds() as usize)
            .unwrap_or(0)
    }

    fn disconnect_signal_handler(&mut self, _id: &Uuid) {
        // TODO
    }

    fn current_subtitle_track(&self) -> Option<usize> {
        self.player
            .current_subtitle_track()
            .map(|t| t.index() as usize)
    }

    fn set_subtitle_track(&self, subtitle_track_id: Option<usize>) {
        match subtitle_track_id {
            Some(id) => {
                self.player
                    .set_subtitle_track(id as i32)
                    .expect("Failed to set subtitle track on Gst player.");
                self.player.set_audio_track_enabled(true);
            }
            None => {
                self.player.set_subtitle_track_enabled(false);
            }
        };
    }

    fn current_audio_track(&self) -> Option<usize> {
        self.player
            .current_audio_track()
            .map(|t| t.index() as usize)
    }

    fn set_audio_track(&self, audio_track_id: Option<usize>) {
        match audio_track_id {
            Some(id) => {
                self.player
                    .set_audio_track(id as i32)
                    .expect("Failed to set audio track on Gst player.");
                self.player.set_audio_track_enabled(true);
            }
            None => {
                self.player.set_audio_track_enabled(false);
            }
        };
    }

    fn connect_end_of_stream(&mut self, callback: Box<dyn Fn() + Send + 'static>) {
        self.signal_handler_ids
            .push(self.player.connect_end_of_stream(callback));
    }

    fn connect_position_updated(
        &mut self,
        callback: Box<dyn Fn(usize) + Send + Sync + 'static>,
    ) -> Uuid {
        self.signal_handler_ids
            .push(self.player.connect_position_updated(move |position| {
                callback(position.seconds() as usize);
            }));
        // TODO
        Uuid::new_v4()
    }

    fn connect_duration_updated(&mut self, callback: Box<dyn Fn(usize) + Send + Sync + 'static>) {
        self.signal_handler_ids
            .push(self.player.connect_duration_changed(move |duration| {
                callback(duration.seconds() as usize);
            }));
    }

    fn connect_mute_updated(&mut self, callback: Box<dyn Fn(bool) + Send + Sync + 'static>) {
        self.player.connect_mute_changed(callback);
    }

    fn connect_volume_updated(&mut self, callback: Box<dyn Fn(f64) + Send + Sync + 'static>) {
        self.player.connect_volume_changed(callback);
    }

    fn connect_subtitle_tracks_updated(
        &mut self,
        _callback: Box<dyn Fn(Vec<SubtitleTrack>) + Send + Sync + 'static>,
    ) {
        todo!();
    }

    fn connect_audio_tracks_updated(
        &mut self,
        _callback: Box<dyn Fn(Vec<AudioTrack>) + Send + Sync + 'static>,
    ) {
        todo!();
    }
}

impl From<PlaySubtitleInfo> for SubtitleTrack {
    fn from(value: PlaySubtitleInfo) -> Self {
        Self {
            name: value.display_name(),
            id: value.index() as usize,
        }
    }
}

trait PlaySubtitleInfoExt {
    fn display_name(&self) -> String;
}

impl PlaySubtitleInfoExt for PlaySubtitleInfo {
    fn display_name(&self) -> String {
        let mut display_name = self
            .language()
            .map(|l| l.to_string())
            .unwrap_or(self.index().to_string());

        let tags = self.tags();
        if let Some(tags) = tags {
            if let Some(title) = tags.get::<gst::tags::Title>() {
                let title = title.get();
                display_name = tr!("vp-backend-gst-track-name", {
                    "displayName "=> display_name,
                    "title" => title,
                })
                .to_string();
            }
        }

        display_name
    }
}

impl From<PlayAudioInfo> for AudioTrack {
    fn from(value: PlayAudioInfo) -> Self {
        Self {
            id: value.index() as usize,
            name: value.display_name(),
        }
    }
}

trait PlayAudioInfoExt {
    fn display_name(&self) -> String;
}

impl PlayAudioInfoExt for PlayAudioInfo {
    fn display_name(&self) -> String {
        let mut display_name = self
            .language()
            .map(|l| l.to_string())
            .unwrap_or(self.index().to_string());

        let tags = self.tags();
        if let Some(tags) = tags {
            if let Some(title) = tags.get::<gst::tags::Title>() {
                let title = title.get();
                display_name = tr!("vp-backend-gst-track-name", {
                    "displayName "=> display_name,
                    "title" => title,
                })
                .to_string();
            }
        }

        display_name
    }
}
