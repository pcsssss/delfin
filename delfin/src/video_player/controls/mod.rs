mod audio_tracks;
pub(super) mod fullscreen;
pub(super) mod next_prev_episode;
pub(super) mod play_pause;
pub(super) mod playback_speed;
pub(super) mod scrubber;
pub(super) mod skip_forwards_backwards;
pub(super) mod subtitles;
pub(super) mod video_player_menu;
pub(super) mod volume;

use std::{
    cell::{OnceCell, RefCell},
    sync::Arc,
};

use crate::{
    app::APP_BROKER,
    jellyfin_api::api_client::ApiClient,
    video_player::{
        backends::VideoPlayerBackend,
        controls::{
            fullscreen::FULLSCREEN_BROKER,
            next_prev_episode::{
                NextPrevEpisodeDirection, NextPrevEpisodeOutput, NEXT_EPISODE_BROKER,
                PREV_EPISODE_BROKER,
            },
            play_pause::PLAY_PAUSE_BROKER,
            scrubber::SCRUBBER_BROKER,
            skip_forwards_backwards::{
                SkipForwardsBackwardsDirection, SKIP_BACKWARDS_BROKER, SKIP_FORWARDS_BROKER,
            },
            subtitles::SUBTITLES_BROKER,
            volume::VOLUME_BROKER,
        },
        next_up::NEXT_UP_VISIBILE,
    },
};
use gtk::prelude::*;
use jellyfin_api::types::BaseItemDto;
use playback_speed::PLAYBACK_SPEED_BROKER;
use relm4::{gtk, Component, ComponentController, ComponentParts, Controller, SimpleComponent};
use video_player_menu::{VideoPlayerMenu, VIDEO_PLAYER_MENU_BROKER};

use self::{
    audio_tracks::{AudioTracks, AudioTracksInput},
    fullscreen::Fullscreen,
    next_prev_episode::{NextPrevEpisode, NextPrevEpisodeInput},
    play_pause::PlayPause,
    playback_speed::PlaybackSpeed,
    scrubber::Scrubber,
    skip_forwards_backwards::SkipForwardsBackwards,
    subtitles::{Subtitles, SubtitlesInput},
    volume::Volume,
};

pub struct VideoPlayerControls {
    show_controls: bool,
    next_prev_episodes: (Option<BaseItemDto>, Option<BaseItemDto>),
    // We need to keep these controllers around, even if we don't read them
    scrubber: Option<Controller<Scrubber>>,
    play_pause: Option<Controller<PlayPause>>,
    skip_forwards_backwards: OnceCell<(
        Controller<SkipForwardsBackwards>,
        Controller<SkipForwardsBackwards>,
    )>,
    next_prev_episode_controls:
        OnceCell<(Controller<NextPrevEpisode>, Controller<NextPrevEpisode>)>,
    volume: Option<Controller<Volume>>,
    subtitles: OnceCell<Controller<Subtitles>>,
    audio_tracks: OnceCell<Controller<AudioTracks>>,
    fullscreen: Option<Controller<Fullscreen>>,
    playback_speed: OnceCell<Controller<PlaybackSpeed>>,
    menu: OnceCell<Controller<VideoPlayerMenu>>,
}

pub struct VideoPlayerControlsInit {
    pub player: Arc<RefCell<dyn VideoPlayerBackend>>,
    pub default_show_controls: bool,
}

#[derive(Debug)]
pub enum VideoPlayerControlsInput {
    Noop,
    SetShowControls(bool),
    SetPlaying {
        api_client: Arc<ApiClient>,
        item: Box<BaseItemDto>,
    },
    SetNextPreviousEpisodes(Box<Option<BaseItemDto>>, Box<Option<BaseItemDto>>),
    PlayPreviousEpisode,
    PlayNextEpisode,
    RevealerClicked,
}

#[derive(Debug)]
pub enum VideoPlayerControlsOutput {
    ShowControls,
}

#[relm4::component(pub)]
impl SimpleComponent for VideoPlayerControls {
    type Init = VideoPlayerControlsInit;
    type Input = VideoPlayerControlsInput;
    type Output = VideoPlayerControlsOutput;

    view! {
        gtk::Revealer {
            #[watch]
            set_reveal_child: model.show_controls,
            set_transition_type: gtk::RevealerTransitionType::Crossfade,
            set_valign: gtk::Align::End,

            add_controller = gtk::GestureClick {
                connect_released[sender] => move |_, _, _, _| {
                    sender.input(VideoPlayerControlsInput::RevealerClicked);
                },
            },

            #[name = "controls"]
            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "toolbar",
                add_css_class: "osd",
                add_css_class: "video-player-controls",

                #[name = "second_row"]
                gtk::Box {},
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let VideoPlayerControlsInit {
            player,
            default_show_controls,
        } = init;

        let mut model = VideoPlayerControls {
            show_controls: default_show_controls,
            skip_forwards_backwards: OnceCell::new(),
            next_prev_episodes: (None, None),
            scrubber: None,
            play_pause: None,
            next_prev_episode_controls: OnceCell::new(),
            volume: None,
            subtitles: OnceCell::new(),
            audio_tracks: OnceCell::new(),
            fullscreen: None,
            playback_speed: OnceCell::new(),
            menu: OnceCell::new(),
        };

        let widgets = view_output!();
        let controls = &widgets.controls;
        let second_row = &widgets.second_row;

        FULLSCREEN_BROKER.reset();
        PLAY_PAUSE_BROKER.reset();
        SCRUBBER_BROKER.reset();
        SKIP_BACKWARDS_BROKER.reset();
        SKIP_FORWARDS_BROKER.reset();
        PREV_EPISODE_BROKER.reset();
        NEXT_EPISODE_BROKER.reset();
        SUBTITLES_BROKER.reset();
        VOLUME_BROKER.reset();
        VIDEO_PLAYER_MENU_BROKER.reset();

        let scrubber = Scrubber::builder()
            .launch_with_broker(player.clone(), &SCRUBBER_BROKER.read())
            .detach();
        controls.prepend(scrubber.widget());
        model.scrubber = Some(scrubber);

        let prev_episode = NextPrevEpisode::builder()
            .launch_with_broker(
                NextPrevEpisodeDirection::Previous,
                &PREV_EPISODE_BROKER.read(),
            )
            .forward(sender.input_sender(), |msg| match msg {
                NextPrevEpisodeOutput::Play => VideoPlayerControlsInput::PlayPreviousEpisode,
            });
        second_row.append(prev_episode.widget());

        let skip_backwards = SkipForwardsBackwards::builder()
            .launch_with_broker(
                (SkipForwardsBackwardsDirection::Backwards, player.clone()),
                &SKIP_BACKWARDS_BROKER.read(),
            )
            .detach();
        second_row.append(skip_backwards.widget());

        let play_pause = PlayPause::builder()
            .launch_with_broker(player.clone(), &PLAY_PAUSE_BROKER.read())
            .detach();
        second_row.append(play_pause.widget());
        model.play_pause = Some(play_pause);

        let skip_forwards = SkipForwardsBackwards::builder()
            .launch_with_broker(
                (SkipForwardsBackwardsDirection::Forwards, player.clone()),
                &SKIP_FORWARDS_BROKER.read(),
            )
            .detach();
        second_row.append(skip_forwards.widget());
        model
            .skip_forwards_backwards
            .set((skip_backwards, skip_forwards))
            .unwrap();

        let next_episode = NextPrevEpisode::builder()
            .launch_with_broker(NextPrevEpisodeDirection::Next, &NEXT_EPISODE_BROKER.read())
            .forward(sender.input_sender(), |msg| match msg {
                NextPrevEpisodeOutput::Play => VideoPlayerControlsInput::PlayNextEpisode,
            });
        second_row.append(next_episode.widget());
        model
            .next_prev_episode_controls
            .set((prev_episode, next_episode))
            .unwrap();

        // Push remaining controls to end
        second_row.append(&gtk::Box::builder().hexpand(true).build());

        let menu = VideoPlayerMenu::builder()
            .launch_with_broker(player.clone(), &VIDEO_PLAYER_MENU_BROKER.read())
            .detach();
        second_row.append(menu.widget());
        model.menu.set(menu).unwrap();

        let playback_speed = PlaybackSpeed::builder()
            .launch_with_broker(player.clone(), &PLAYBACK_SPEED_BROKER.read())
            .detach();
        second_row.append(playback_speed.widget());
        model.playback_speed.set(playback_speed).unwrap();

        let subtitles = Subtitles::builder()
            .launch_with_broker(player.clone(), &SUBTITLES_BROKER.read())
            .detach();
        second_row.append(subtitles.widget());
        model.subtitles.set(subtitles).unwrap();

        let audio_tracks = AudioTracks::builder().launch(player.clone()).detach();
        second_row.append(audio_tracks.widget());
        model.audio_tracks.set(audio_tracks).unwrap();

        let volume = Volume::builder()
            .launch_with_broker(player.clone(), &VOLUME_BROKER.read())
            .detach();
        second_row.append(volume.widget());
        model.volume = Some(volume);

        let fullscreen = Fullscreen::builder()
            .launch_with_broker((), &FULLSCREEN_BROKER.read())
            .detach();
        second_row.append(fullscreen.widget());
        model.fullscreen = Some(fullscreen);

        NEXT_UP_VISIBILE.subscribe(sender.input_sender(), |visible| {
            if *visible {
                VideoPlayerControlsInput::SetShowControls(false)
            } else {
                VideoPlayerControlsInput::Noop
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: relm4::ComponentSender<Self>) {
        match message {
            VideoPlayerControlsInput::SetShowControls(show_controls) => {
                self.show_controls = show_controls;
            }
            VideoPlayerControlsInput::SetPlaying { api_client, item } => {
                if let Some(subtitles) = self.subtitles.get() {
                    subtitles.emit(SubtitlesInput::Reset { api_client, item });
                }
                if let Some(audio_tracks) = self.audio_tracks.get() {
                    audio_tracks.emit(AudioTracksInput::Reset);
                }
            }
            VideoPlayerControlsInput::SetNextPreviousEpisodes(prev, next) => {
                if let Some((prev_control, next_control)) = self.next_prev_episode_controls.get() {
                    prev_control.emit(if prev.is_some() {
                        NextPrevEpisodeInput::Show
                    } else {
                        NextPrevEpisodeInput::Hide
                    });
                    next_control.emit(if next.is_some() {
                        NextPrevEpisodeInput::Show
                    } else {
                        NextPrevEpisodeInput::Hide
                    });
                }

                self.next_prev_episodes = (*prev, *next);
            }
            VideoPlayerControlsInput::PlayPreviousEpisode => {
                if let (Some(previous), _) = &self.next_prev_episodes {
                    APP_BROKER.send(crate::app::AppInput::PlayVideo(previous.clone()));
                }
            }
            VideoPlayerControlsInput::PlayNextEpisode => {
                if let (_, Some(next)) = &self.next_prev_episodes {
                    APP_BROKER.send(crate::app::AppInput::PlayVideo(next.clone()));
                }
            }
            VideoPlayerControlsInput::Noop => {}
            VideoPlayerControlsInput::RevealerClicked => {
                if !self.show_controls {
                    sender
                        .output(VideoPlayerControlsOutput::ShowControls)
                        .unwrap();
                }
            }
        }
    }
}
