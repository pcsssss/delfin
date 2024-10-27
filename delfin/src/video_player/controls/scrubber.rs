use chrono::TimeDelta;
use jellyfin_api::types::ChapterInfo;
use std::{cell::RefCell, sync::Arc};
use tracing::warn;

use gdk::{Rectangle, Texture};
use gtk::{gdk, glib, prelude::*};
use relm4::prelude::*;

use crate::{
    config::video_player_config::DurationDisplay,
    globals::CONFIG,
    tr,
    utils::{bif::Thumbnail, message_broker::ResettableMessageBroker, ticks::ticks_to_seconds},
    video_player::backends::VideoPlayerBackend,
};

const TIMESTAMP_WIDTH: i32 = 80;

pub(crate) static SCRUBBER_BROKER: ResettableMessageBroker<ScrubberInput> =
    ResettableMessageBroker::new();

struct ScrubberPopover {
    position: f32,
    timestamp: usize,
    thumbnail: Option<Texture>,
    chapter: Option<Chapter>,
}

#[derive(Clone, Debug)]
struct Chapter {
    timestamp: usize,
    name: Option<String>,
}

pub(crate) struct Scrubber {
    video_player: Arc<RefCell<dyn VideoPlayerBackend>>,

    loading: bool,

    position: usize,
    duration: usize,
    duration_display: DurationDisplay,

    scrubbing: bool,
    popover: Option<ScrubberPopover>,
    thumbnails: Option<Vec<Thumbnail>>,
    chapters: Vec<Chapter>,
}

#[derive(Debug)]
pub enum ScrubberInput {
    Reset,
    SetPosition(usize),
    SetDuration(usize),
    SetPlaying,
    ToggleDurationDisplay,
    SetScrubbing(bool),
    ScrubberMouseHover(f64),
    ScrubberMouseLeave,
    LoadedThumbnails(Option<Vec<Thumbnail>>),
    DurationDisplayUpdated(DurationDisplay),
    DisplayChapters(Vec<ChapterInfo>),
}

#[relm4::component(pub(crate))]
impl Component for Scrubber {
    type Init = Arc<RefCell<dyn VideoPlayerBackend>>;
    type Input = ScrubberInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_spacing: 8,

            #[name = "position"]
            gtk::Label {
                #[watch]
                set_label: &seconds_to_timestamp(model.position),
                add_css_class: "scrubber-position-label",
                set_width_request: TIMESTAMP_WIDTH,
            },

            #[name = "scrubber"]
            gtk::Scale {
                set_hexpand: true,
                set_focus_on_click: false,

                #[watch]
                set_value: model.position as f64,
                #[watch]
                set_range: (0.0, model.duration as f64),
                #[watch]
                set_sensitive: !model.loading,

                add_controller = gtk::GestureClick {
                    connect_pressed[sender] => move |_, _, x, _| {
                        sender.input(ScrubberInput::SetScrubbing(true));
                        // Update position so we seek correctly if user immediately releases click
                        sender.input(ScrubberInput::ScrubberMouseHover(x));
                    },
                    connect_unpaired_release[sender] => move |_, _, _, _, _| {
                        sender.input(ScrubberInput::SetScrubbing(false));
                    },
                    connect_stopped[sender] => move |gesture| {
                        if gesture.current_button() == 0 {
                            sender.input(ScrubberInput::SetScrubbing(false));
                        }
                    },
                },

                add_controller = gtk::EventControllerMotion {
                    connect_motion[sender] => move |_, x, _| {
                        sender.input(ScrubberInput::ScrubberMouseHover(x));
                    },
                    connect_leave[sender] => move |_| {
                        sender.input(ScrubberInput::ScrubberMouseLeave);
                    },
                },
            },

            #[name = "popover"]
            gtk::Popover {
                set_autohide: false,
                set_position: gtk::PositionType::Top,

                #[watch]
                set_visible: model.popover.is_some(),
                #[watch]
                set_pointing_to: model.popover
                    .as_ref()
                    .map(|p| Rectangle::new(p.position as i32, -15, 0, 0))
                    .as_ref(),

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,

                    #[name = "thumbnail"]
                    gtk::Picture {
                        add_css_class: "scrubber-popover-thumbnail",
                        #[watch]
                        set_paintable: model.popover
                            .as_ref()
                            .and_then(|p| p.thumbnail.as_ref())
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &model.popover
                            .as_ref()
                            .and_then(|p| p.chapter.clone())
                            .and_then(|c| c.name)
                            .unwrap_or_default(),
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &model.popover
                            .as_ref()
                            .map_or(String::new(), |p| seconds_to_timestamp(p.timestamp)),
                    },
                },
            },

            #[name = "duration"]
            gtk::Button {
                set_focus_on_click: false,
                #[watch]
                set_label: &duration_to_timestamp(model.position, model.duration, model.duration_display),
                #[watch]
                set_tooltip_text: Some(&match model.duration_display {
                    DurationDisplay::Total => tr!("vp-duration-tooltip.total").to_string(),
                    DurationDisplay::Remaining => tr!("vp-duration-tooltip.remaining").to_string(),
                }),
                set_width_request: TIMESTAMP_WIDTH,
                add_css_class: "flat",
                add_css_class: "scrubber-duration-label",
                connect_clicked[sender] => move |_| {
                    sender.input(ScrubberInput::ToggleDurationDisplay);
                },
            },
        }
    }

    fn init(
        video_player: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        video_player.borrow_mut().connect_position_updated({
            let sender = sender.clone();
            Box::new(move |position| {
                sender.input(ScrubberInput::SetPosition(position));
            })
        });

        video_player.borrow_mut().connect_duration_updated({
            let sender = sender.clone();
            Box::new(move |duration| {
                sender.input(ScrubberInput::SetDuration(duration));
            })
        });

        let model = Scrubber {
            video_player,
            loading: true,
            position: 0,
            duration: 0,
            duration_display: CONFIG.read().video_player.duration_display,
            scrubbing: false,
            popover: None,
            thumbnails: None,
            chapters: vec![],
        };

        Scrubber::subscribe_to_config(&sender);

        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        let scrubber = &widgets.scrubber;

        match message {
            ScrubberInput::Reset => {
                self.loading = true;
                self.position = 0;
                self.duration = 0;
                self.thumbnails = None;
                self.chapters.clear();
                scrubber.clear_marks();
            }
            ScrubberInput::SetPosition(position) => {
                if !self.scrubbing {
                    self.position = position;
                }
            }
            ScrubberInput::SetDuration(duration) => {
                self.duration = duration;
                // Make sure marks are in the right place after scale range changes
                self.mark_chapters(scrubber);
            }
            ScrubberInput::SetPlaying => {
                self.loading = false;
            }
            ScrubberInput::ToggleDurationDisplay => {
                let mut config = CONFIG.write();
                config.video_player.duration_display =
                    config.video_player.duration_display.toggle();
                config.save().expect("Failed to save duration display");
            }
            ScrubberInput::DurationDisplayUpdated(duration_display) => {
                self.duration_display = duration_display;
            }
            ScrubberInput::SetScrubbing(scrubbing) => {
                self.scrubbing = scrubbing;

                if !scrubbing {
                    self.video_player.borrow().seek_to(self.position);
                }
            }
            ScrubberInput::ScrubberMouseHover(position) => {
                let percent = (position / scrubber.width() as f64).clamp(0.0, 1.0);
                let timestamp = (self.duration as f64 * percent) as usize;

                if self.scrubbing {
                    self.position = timestamp;
                }

                let popover_position = self.calculate_popover_position(root, scrubber, position);

                self.popover = Some(ScrubberPopover {
                    position: popover_position.unwrap_or(0.),
                    timestamp,
                    thumbnail: self.get_thumbnail(timestamp),
                    chapter: self.get_chapter(timestamp),
                });
                // Force thumbnail redraw or it won't update sometimes
                widgets.thumbnail.queue_draw();
            }
            ScrubberInput::ScrubberMouseLeave => {
                self.popover = None;
            }
            ScrubberInput::LoadedThumbnails(thumbnails) => {
                self.thumbnails = thumbnails;
            }
            ScrubberInput::DisplayChapters(chapters) => {
                self.chapters = chapters
                    .iter()
                    .filter_map(|c| {
                        let position = c.start_position_ticks?;
                        Some(Chapter {
                            timestamp: ticks_to_seconds(position) as usize,
                            name: c.name.clone(),
                        })
                    })
                    .collect();
                // Ensure chapters are in ascending timestamp order, so we can find the chapter for
                // a given timestamp when scrubbing
                self.chapters.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

                self.mark_chapters(scrubber);
            }
        }

        self.update_view(widgets, sender);
    }
}

impl Scrubber {
    fn subscribe_to_config(sender: &ComponentSender<Self>) {
        CONFIG.subscribe(sender.input_sender(), |config| {
            ScrubberInput::DurationDisplayUpdated(config.video_player.duration_display)
        });
    }

    fn get_thumbnail(&self, timestamp: usize) -> Option<Texture> {
        let thumbnails = self.thumbnails.as_ref()?;

        let mut nearest_thumbnail_idx = 0;
        for (i, thumbnail) in thumbnails.iter().enumerate() {
            if thumbnail.timestamp < timestamp {
                nearest_thumbnail_idx = i;
            }
        }

        let image = if let Some(thumbnail) = thumbnails.get(nearest_thumbnail_idx) {
            &thumbnail.image
        } else {
            warn!("Error getting trickplay thumbnail");
            return None;
        };
        Texture::from_bytes(&glib::Bytes::from(image))
            .map_err(|err| {
                warn!(
                    "Error creating texture for scrubber thumbnail at timestamp {timestamp}: {err}"
                );
                err
            })
            .ok()
    }

    fn get_chapter(&self, timestamp: usize) -> Option<Chapter> {
        if self.chapters.is_empty() {
            return None;
        }

        let mut nearest_chapter_idx = 0;
        for (i, chapter) in self.chapters.iter().enumerate() {
            if chapter.timestamp < timestamp {
                nearest_chapter_idx = i;
            }
        }
        Some(self.chapters[nearest_chapter_idx].clone())
    }

    fn mark_chapters(&self, scrubber: &gtk::Scale) {
        scrubber.clear_marks();
        for chapter in &self.chapters {
            scrubber.add_mark(chapter.timestamp as f64, gtk::PositionType::Top, None);
        }
    }

    fn calculate_popover_position(
        &self,
        root: &gtk::Box,
        scrubber: &gtk::Scale,
        mouse_position: f64,
    ) -> Option<f32> {
        let scrubber_bounds = scrubber.compute_bounds(root)?;
        let range_rect = scrubber.range_rect();
        let range_offset = (scrubber_bounds.width() - range_rect.width() as f32) / 2.;
        Some(scrubber_bounds.x() + range_offset + mouse_position as f32)
    }
}

fn seconds_to_timestamp(seconds: usize) -> String {
    let Some(time) = TimeDelta::try_seconds(seconds as i64) else {
        warn!("Could not convert seconds to TimeDelta");
        return String::new();
    };

    let hours = time.num_hours();
    let minutes = time.num_minutes() - (60 * hours);
    let seconds = time.num_seconds() % 60;

    if hours > 0 {
        format!("{hours}:{minutes:0>2}:{seconds:0>2}")
    } else {
        format!("{minutes:0>2}:{seconds:0>2}")
    }
}

fn duration_to_timestamp(
    position: usize,
    duration: usize,
    duration_display: DurationDisplay,
) -> String {
    match duration_display {
        DurationDisplay::Total => seconds_to_timestamp(duration),
        DurationDisplay::Remaining => {
            format!("-{}", seconds_to_timestamp(duration.wrapping_sub(position)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seconds_to_timestamp() {
        assert_eq!(seconds_to_timestamp(8624), "2:23:44");
        assert_eq!(seconds_to_timestamp(345), "05:45");
    }

    #[test]
    fn test_duration_to_timestamp_total() {
        assert_eq!(
            duration_to_timestamp(345, 8624, DurationDisplay::Total),
            "2:23:44"
        );
        assert_eq!(
            duration_to_timestamp(345, 345, DurationDisplay::Total),
            "05:45"
        );
    }

    #[test]
    fn test_duration_to_timestamp_remaining() {
        assert_eq!(
            duration_to_timestamp(345, 8624, DurationDisplay::Remaining),
            "-2:17:59"
        );
        assert_eq!(
            duration_to_timestamp(24, 345, DurationDisplay::Remaining),
            "-05:21"
        );
    }
}
