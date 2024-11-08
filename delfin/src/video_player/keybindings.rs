use glib::Propagation;
use gtk::{gdk, glib, EventControllerKey};
use relm4::gtk;

use super::controls::{
    fullscreen::{FullscreenInput, FULLSCREEN_BROKER},
    play_pause::{PlayPauseInput, PLAY_PAUSE_BROKER},
    playback_speed::{PlaybackSpeedInput, PLAYBACK_SPEED_BROKER},
    skip_forwards_backwards::{
        SkipForwardsBackwardsInput, SKIP_BACKWARDS_BROKER, SKIP_FORWARDS_BROKER,
    },
    subtitles::{SubtitlesInput, SUBTITLES_BROKER},
    video_player_menu::{VideoPlayerMenuInput, VIDEO_PLAYER_MENU_BROKER},
    volume::{VolumeInput, VOLUME_BROKER},
};

pub fn keybindings_controller() -> EventControllerKey {
    let controller = EventControllerKey::new();

    controller.connect_key_pressed(move |_, key, _, _| {
        match key {
            gdk::Key::space => {
                PLAY_PAUSE_BROKER.send(PlayPauseInput::TogglePlaying);
            }
            gdk::Key::Left => {
                SKIP_BACKWARDS_BROKER.send(SkipForwardsBackwardsInput::Skip);
            }
            gdk::Key::Right => {
                SKIP_FORWARDS_BROKER.send(SkipForwardsBackwardsInput::Skip);
            }
            gdk::Key::Up => {
                VOLUME_BROKER.send(VolumeInput::ChangeVolume(0.1));
            }
            gdk::Key::Down => {
                VOLUME_BROKER.send(VolumeInput::ChangeVolume(-0.1));
            }
            gdk::Key::m => {
                VOLUME_BROKER.send(VolumeInput::ToggleMute);
            }
            gdk::Key::f => {
                FULLSCREEN_BROKER.send(FullscreenInput::ToggleFullscreen);
            }
            gdk::Key::Escape => {
                FULLSCREEN_BROKER.send(FullscreenInput::ExitFullscreen);
            }
            gdk::Key::c => {
                SUBTITLES_BROKER
                    .read()
                    .send(SubtitlesInput::ToggleSubtitles);
            }
            gdk::Key::comma => {
                SKIP_BACKWARDS_BROKER.send(SkipForwardsBackwardsInput::FrameStep);
            }
            gdk::Key::period => {
                SKIP_FORWARDS_BROKER.send(SkipForwardsBackwardsInput::FrameStep);
            }
            // Left angle bracket/Shift + ,
            gdk::Key::less => {
                PLAYBACK_SPEED_BROKER.send(PlaybackSpeedInput::SlowDown);
            }
            // Right angle bracket/Shift + .
            gdk::Key::greater => {
                PLAYBACK_SPEED_BROKER.send(PlaybackSpeedInput::SpeedUp);
            }
            gdk::Key::d => {
                VIDEO_PLAYER_MENU_BROKER.send(VideoPlayerMenuInput::ToggleDeinterlacing);
            }
            _ => return Propagation::Proceed,
        };
        Propagation::Stop
    });

    controller
}
