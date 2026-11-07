use std::sync::Arc;

use adw::prelude::*;
use gtk::glib;
use jellyfin_api::types::BaseItemDto;
use relm4::{prelude::*, ComponentParts};

use crate::{globals::SHIFT_STATE, jellyfin_api::api_client::ApiClient};

use super::{
    media_button::MediaButton,
    media_tile::{MediaTile, MediaTileDisplay},
};

const MIN_PADDING: i32 = 24;

pub(crate) enum MediaCarouselItem {
    Tile(AsyncController<MediaTile>),
    Button(Controller<MediaButton>),
}

impl MediaCarouselItem {
    fn widget(&self) -> &gtk::Widget {
        match self {
            MediaCarouselItem::Tile(media_tile) => media_tile.widget().upcast_ref(),
            MediaCarouselItem::Button(media_button) => media_button.widget().upcast_ref(),
        }
    }
}

pub(crate) enum MediaCarouselType {
    Tiles,
    Buttons,
}

pub(crate) struct MediaCarousel {
    media_tile_display: MediaTileDisplay,
    media_tiles: Vec<MediaCarouselItem>,
    pages: Vec<gtk::Box>,
}

pub(crate) struct MediaCarouselInit {
    pub(crate) media: Vec<BaseItemDto>,
    pub(crate) media_tile_display: MediaTileDisplay,
    pub(crate) carousel_type: MediaCarouselType,
    pub(crate) api_client: Arc<ApiClient>,
    pub(crate) label: String,
    pub(crate) label_clickable: bool,
}

#[derive(Debug)]
pub(crate) enum MediaCarouselInput {
    Resize(i32),
    Left,
    Right,
}

#[derive(Debug)]
pub(crate) enum MediaCarouselOutput {
    LabelClicked,
}

impl MediaTileDisplay {
    fn min_height(&self, pages: &[gtk::Box]) -> i32 {
        // BreakpointBin ignores the child's natural minimum: reserve the new
        // separate removal action (34px button + 8px spacing) on media tiles.
        let action_height = if matches!(self, Self::Cover | Self::CoverLarge | Self::Wide) {
            42
        } else {
            0
        };
        match pages.len() {
            1 => self.height() + 50 + action_height,
            _ => self.height() + 80 + action_height,
        }
    }
}

#[relm4::component(pub(crate))]
impl Component for MediaCarousel {
    type Init = MediaCarouselInit;
    type Input = MediaCarouselInput;
    type Output = MediaCarouselOutput;
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,

            add_css_class: "media-carousel",

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,

                gtk::Box {
                    set_spacing: 4,
                    set_cursor_from_name: if label_clickable { Some("pointer") } else { None },
                    add_css_class?: label_clickable.then_some("clickable-title"),

                    gtk::Label {
                        set_label: label.as_str(),
                        add_css_class: "title-2",
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Image::from_icon_name("right") {
                        set_visible: label_clickable,
                    },

                    add_controller?: if label_clickable {
                        let controller = gtk::GestureClick::new();
                        controller.connect_released({
                            let sender = sender.clone();
                            move |_, _, _, _| {
                                sender.output(MediaCarouselOutput::LabelClicked).unwrap();
                            }
                        });
                        Some(controller)
                    } else {
                        None
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::End,
                    set_hexpand: true,
                    add_css_class: "linked",
                    #[watch]
                    set_visible: carousel.n_pages() > 1,

                    gtk::Button {
                        set_icon_name: "left",
                        add_css_class: "flat",
                        connect_clicked[sender] => move |_| {
                            sender.input(MediaCarouselInput::Left);
                        },
                    },

                    gtk::Button {
                        set_icon_name: "right",
                        add_css_class: "flat",
                        connect_clicked[sender] => move |_| {
                            sender.input(MediaCarouselInput::Right);
                        },
                    },
                },
            },

            #[name = "breakpoints"]
            adw::BreakpointBin {
                #[watch]
                set_size_request: (
                    model.media_tile_display.width() + MIN_PADDING,
                    model.media_tile_display.min_height(&model.pages),
                ),
                set_hexpand: true,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    #[name = "carousel"]
                    adw::Carousel {
                        set_allow_scroll_wheel: false,

                        connect_page_changed => move |carousel, page_index| {
                            for i in 0..carousel.n_pages() {
                                carousel.nth_page(i).set_can_focus(i == page_index);
                            }
                        },

                        add_controller = gtk::EventControllerScroll {
                            // TODO: Might need a separate controller for Horizontal scrolling that doesn't check if shift is pressed
                            set_flags: gtk::EventControllerScrollFlags::VERTICAL,
                            connect_scroll[sender] => move |_, _dx, dy| {
                                let shift_state = SHIFT_STATE.read();
                                if shift_state.pressed() {
                                    sender.input(
                                        if dy > 0.0 {
                                            MediaCarouselInput::Right
                                        } else {
                                            MediaCarouselInput::Left
                                        }
                                    );
                                    return glib::Propagation::Stop;
                                }
                                glib::Propagation::Proceed
                            },
                        },
                    },
                    #[name = "carousel_indicator"]
                    adw::CarouselIndicatorLines {},
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let MediaCarouselInit {
            api_client,
            media,
            media_tile_display,
            carousel_type,
            label,
            label_clickable,
        } = init;

        let media_tiles = media
            .iter()
            .map(|media| match carousel_type {
                MediaCarouselType::Tiles => MediaCarouselItem::Tile(
                    MediaTile::builder()
                        .launch((media.clone(), media_tile_display, api_client.clone()))
                        .detach(),
                ),
                MediaCarouselType::Buttons => MediaCarouselItem::Button(
                    MediaButton::builder()
                        .launch((media.clone(), media_tile_display))
                        .detach(),
                ),
            })
            .collect();

        let model = MediaCarousel {
            media_tile_display,
            media_tiles,
            pages: vec![],
        };

        let widgets = view_output!();
        let breakpoints = &widgets.breakpoints;
        let carousel = &widgets.carousel;
        let carousel_indicator = &widgets.carousel_indicator;

        carousel_indicator.set_carousel(Some(carousel));

        add_breakpoints(breakpoints, &sender, media_tile_display);

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let carousel: &adw::Carousel = &widgets.carousel;

        match message {
            MediaCarouselInput::Resize(tiles_per_page) => {
                // Remove existing pages
                for page in &self.pages {
                    while page.first_child().is_some() {
                        page.remove(&page.first_child().unwrap());
                    }
                    carousel.remove(page);
                }
                self.pages.clear();

                let media_tile_chunks: Vec<&[MediaCarouselItem]> =
                    self.media_tiles.chunks(tiles_per_page as usize).collect();

                for (i, &chunk) in media_tile_chunks.iter().enumerate() {
                    let page = gtk::Box::builder()
                        .orientation(gtk::Orientation::Horizontal)
                        .homogeneous(true)
                        .hexpand(true)
                        .spacing(MIN_PADDING)
                        .can_focus(i == 0)
                        .build();

                    // Not a full page, we don't want the tiles to be spaced out across the screen
                    if chunk.len() < tiles_per_page as usize {
                        page.set_halign(gtk::Align::Start);
                    }

                    for tile in chunk {
                        page.append(tile.widget());
                    }

                    carousel.append(&page);
                    self.pages.push(page);
                }

                self.update_view(widgets, sender);
            }
            MediaCarouselInput::Left => {
                let cur_pos = carousel.position() as i32;
                let mut pos = cur_pos - 1;
                if pos < 0 {
                    pos = carousel.n_pages() as i32 - 1;
                }

                let next_page = carousel.nth_page(pos as u32);
                carousel.scroll_to(&next_page, true);
            }
            MediaCarouselInput::Right => {
                let cur_pos = carousel.position() as i32;
                let mut pos = cur_pos + 1;
                if pos > (carousel.n_pages() as i32) - 1 {
                    pos = 0;
                }

                let next_page = carousel.nth_page(pos as u32);
                carousel.scroll_to(&next_page, true);
            }
        }
    }
}

fn add_breakpoints(
    breakpoints: &adw::BreakpointBin,
    sender: &ComponentSender<MediaCarousel>,
    media_tile_display: MediaTileDisplay,
) {
    for tiles_per_page in 1..=8 {
        let required =
            (media_tile_display.width() * tiles_per_page) + (MIN_PADDING * (tiles_per_page - 1));

        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MinWidth,
            required as f64,
            adw::LengthUnit::Px,
        ));
        breakpoint.connect_apply({
            let sender = sender.clone();
            move |_| {
                sender.input(MediaCarouselInput::Resize(tiles_per_page));
            }
        });

        breakpoints.add_breakpoint(breakpoint);
    }
}
