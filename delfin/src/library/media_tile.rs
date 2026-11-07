use std::{collections::VecDeque, sync::Arc};

use gdk::Texture;
use jellyfin_api::types::{BaseItemDto, BaseItemKind};
use relm4::{
    gtk::{self, gdk, gdk_pixbuf, glib::markup_escape_text, prelude::*},
    prelude::{AsyncComponent, AsyncComponentParts},
    AsyncComponentSender,
};
use tracing::error;

use super::movie_card::{self, MovieCardInfo};

use crate::{
    app::{AppInput, APP_BROKER},
    globals::CONFIG,
    jellyfin_api::api_client::ApiClient,
    tr,
    utils::{
        display_years::DisplayYears, file_size::format_file_size, item_name::ItemName,
        playable::get_next_playable_media,
    },
};

#[derive(Clone, Copy)]
pub enum MediaTileDisplay {
    Cover,
    CoverLarge,
    Wide,
    CollectionWide,
    Buttons,
}

impl MediaTileDisplay {
    fn rich_movie(&self, media: &BaseItemDto) -> bool {
        matches!(self, Self::CoverLarge) && media.type_ == Some(BaseItemKind::Movie)
    }
    fn dimensions(&self, media: &BaseItemDto) -> (i32, i32) {
        if self.rich_movie(media) {
            (
                crate::utils::constants::POSTER_WIDTH,
                crate::utils::constants::POSTER_HEIGHT,
            )
        } else {
            (self.width(), self.height())
        }
    }
    pub fn width(&self) -> i32 {
        match self {
            Self::Cover => 133,
            Self::CoverLarge => 175,
            Self::Wide => 300,
            Self::CollectionWide => 300,
            Self::Buttons => 300,
        }
    }

    pub fn height(&self) -> i32 {
        match self {
            Self::Cover => 200,
            Self::CoverLarge => 262,
            Self::Wide => 175,
            Self::CollectionWide => 175,
            Self::Buttons => 0,
        }
    }
}

pub struct MediaTile {
    media: BaseItemDto,
    api_client: Arc<ApiClient>,
    thumbnail: Option<Texture>,
    storage_size: Option<String>,
    movie_info: Option<MovieCardInfo>,
}

#[derive(Debug)]
pub enum MediaTileInput {
    Play,
    ShowDetails,
    Remove,
}

#[derive(Debug)]
pub enum MediaTileCommandOutput {
    ThumbnailLoaded(Option<Texture>),
    StorageSizeLoaded(Option<u64>),
}

#[relm4::component(pub async)]
impl AsyncComponent for MediaTile {
    type Init = (BaseItemDto, MediaTileDisplay, Arc<ApiClient>);
    type Input = MediaTileInput;
    type Output = ();
    type CommandOutput = MediaTileCommandOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_halign: gtk::Align::Fill,
            set_valign: gtk::Align::Start,
            set_spacing: if rich_movie { 10 } else { 8 },
            add_css_class: "media-tile",
            add_css_class: if rich_movie { "movie-grid-card" } else { "regular-media-tile" },

            #[name = "overlay"]
            gtk::Overlay {
                set_halign: gtk::Align::Center,
                set_cursor_from_name: Some("pointer"),
                #[watch]
                set_tooltip_text: model.poster_tooltip().as_deref(),

                set_accessible_role: gtk::AccessibleRole::Button,
                #[watch]
                update_property: &[gtk::accessible::Property::Label(&model.poster_accessible_label())],
                set_focusable: true,
                connect_has_focus_notify[root, media] => move |overlay| {
                    if overlay.has_focus() && !matches!(media.type_, Some(BaseItemKind::CollectionFolder)) {
                        root.add_css_class("hover");
                    } else {
                        root.remove_css_class("hover");
                    }
                },

                // TODO: progress bar overflows media tile. Hiding the overflow is a workaround, but it makes the
                // progress bar percentage look wrong.
                set_overflow: gtk::Overflow::Hidden,


                add_controller = gtk::GestureClick {
                    connect_released[sender] => move |gesture, _, x, y| {
                        // The IMDb link is a separate action, never playback.
                        if gesture.widget().pick(x, y, gtk::PickFlags::DEFAULT)
                            .is_some_and(|w| w.is::<gtk::LinkButton>() || w.ancestor(gtk::LinkButton::static_type()).is_some()) { return; }
                        sender.input(MediaTileInput::Play);
                    },
                },

                add_controller = gtk::EventControllerKey {
                    connect_key_released[sender] => move |controller, key, _, _| {
                        if controller.widget().has_focus() && matches!(key, gdk::Key::Return | gdk::Key::space) {
                            sender.input(MediaTileInput::Play);
                        }
                    },
                },

                add_controller = gtk::EventControllerMotion {
                    connect_enter[root, media] => move |_, _, _| {
                        if !matches!(media.type_, Some(BaseItemKind::CollectionFolder)) {
                            root.add_css_class("hover");
                        }
                    },
                    connect_leave[root] => move |_| {
                        root.remove_css_class("hover");
                    },
                },

                #[name = "image"]
                gtk::Picture {
                    #[watch]
                    set_paintable: model.thumbnail.as_ref(),

                    set_content_fit: gtk::ContentFit::Cover,
                    set_width_request: tile_width,
                    set_height_request: tile_height,
                },

                add_overlay = &gtk::Spinner {
                    #[watch]
                    set_visible: model.thumbnail.is_none(),
                    start: (),
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_width_request: 32,
                    set_height_request: 32,
                },

                add_overlay = &gtk::CenterBox {
                    add_css_class: "hover-overlay",

                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,

                    #[wrap(Some)]
                    set_center_widget = &gtk::Image {
                        set_from_icon_name: Some("play-filled"),

                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                    }
                },

                add_overlay = &gtk::LinkButton {
                    set_label: "IMDb",
                    set_uri: imdb_url.as_deref().unwrap_or("https://www.imdb.com/"),
                    set_visible: tile_display.height() > 0 && imdb_url.is_some(),
                    add_css_class: "imdb-link",
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Start,
                    set_margin_top: 8,
                    set_margin_start: 8,
                    set_tooltip_text: Some(tr!("movie-card-imdb-tooltip")),
                    update_property: &[gtk::accessible::Property::Label(tr!("movie-card-imdb-tooltip"))],
                },

                add_overlay = &gtk::Label {
                    add_css_class: "storage-size",
                    set_halign: gtk::Align::End,
                    set_valign: gtk::Align::Start,
                    set_margin_top: 8,
                    set_margin_end: 8,
                    // Keep clicks/keyboard focus on the existing thumbnail button.
                    set_can_target: false,
                    #[watch]
                    set_visible: model.storage_size.is_some(),
                    #[watch]
                    set_label: model.storage_size.as_deref().unwrap_or_default(),
                },

                add_overlay = &gtk::Label {
                    add_css_class: "movie-resolution",
                    set_visible: model.movie_info.as_ref().and_then(|i| i.resolution.as_ref()).is_some(),
                    set_label: model.movie_info.as_ref().and_then(|i| i.resolution.as_deref()).unwrap_or_default(),
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::End,
                    set_margin_start: 8,
                    set_margin_bottom: 10,
                    set_can_target: false,
                },

                add_overlay = &gtk::ProgressBar {
                    set_valign: gtk::Align::End,
                    set_visible: model.progress_fraction().is_some(),
                    set_fraction: model.progress_fraction().unwrap_or_default(),
                    set_overflow: gtk::Overflow::Hidden,
                    set_width_request: tile_width,
                },
            },

            gtk::Button {
                set_visible: rich_movie,
                add_css_class: "movie-card-details",
                set_has_frame: false,
                set_halign: gtk::Align::Center,
                set_width_request: tile_width,
                set_tooltip_text: details_tooltip.as_deref(),
                set_child: Some(&movie_caption),
                connect_clicked[sender] => move |_| {
                    sender.input(MediaTileInput::ShowDetails);
                },
            },

            gtk::Label {
                set_visible: !rich_movie,
                set_halign: gtk::Align::Center,
                set_justify: gtk::Justification::Center,
                set_cursor_from_name: Some("pointer"),
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                set_max_width_chars: 1,
                set_width_request: tile_width,
                #[watch]
                set_markup: &model.get_item_label(),

                set_has_tooltip: true,
                connect_query_tooltip[item_label] => move |_, _, _, _, tooltip| {
                    let label = gtk::Label::builder()
                        .use_markup(true)
                        .label(&item_label)
                        .justify(gtk::Justification::Center)
                        .build();
                    tooltip.set_custom(Some(&label));
                    true
                },

                add_controller = gtk::GestureClick {
                    connect_released[sender] => move |_, _, _, _| {
                        sender.input(MediaTileInput::ShowDetails);
                    },
                },
            },
            gtk::Button {
                set_visible: removal_supported,
                set_halign: gtk::Align::Center,
                set_width_request: tile_width,
                set_label: &removal_label,
                update_property: &[gtk::accessible::Property::Label(&removal_accessible_label)],
                set_tooltip_text: Some(tr!("seerr-removal-entry-hint")),
                add_css_class: "flat",
                connect_clicked[sender] => move |_| {sender.input(MediaTileInput::Remove);},
            },
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let (media, tile_display, api_client) = init;

        let removal_supported = media.id.is_some()
            && matches!(
                media.type_,
                Some(
                    BaseItemKind::Movie
                        | BaseItemKind::Series
                        | BaseItemKind::Season
                        | BaseItemKind::Episode
                )
            );
        let rich_movie = tile_display.rich_movie(&media);
        let (tile_width, tile_height) = tile_display.dimensions(&media);
        let removal_label = tr!("seerr-remove-everywhere").to_string();
        let removal_accessible_label = format!(
            "{}: {}",
            tr!("seerr-remove-everywhere"),
            media.name.as_deref().unwrap_or("")
        );
        let mut model = Self::new(media.clone(), api_client.clone());
        if rich_movie {
            model.movie_info = Some(MovieCardInfo::new(&media));
        }
        let imdb_url = movie_card::imdb_url(&media);
        let item_label = model.get_item_label();
        let movie_caption = model
            .movie_info
            .as_ref()
            .map(|i| movie_card::caption(i, tile_width))
            .unwrap_or_else(|| gtk::Box::new(gtk::Orientation::Vertical, 0));
        let details_tooltip = model.movie_info.as_ref().map(MovieCardInfo::summary);

        let widgets = view_output!();

        if matches!(
            media.type_,
            Some(BaseItemKind::Movie | BaseItemKind::Series)
        ) && tile_display.height() > 0
        {
            // Separate from image loading: slow show totals never delay thumbnails.
            sender.oneshot_command({
                let api_client = api_client.clone();
                let media = media.clone();
                async move {
                    let size = match api_client.get_storage_size(&media).await {
                        Ok(size) => size,
                        Err(_) => {
                            // Missing permissions/metadata must not break browsing or
                            // leak server URLs or authentication details to the log.
                            tracing::warn!(item_id = ?media.id, "Could not load media file size");
                            None
                        }
                    };
                    MediaTileCommandOutput::StorageSizeLoaded(size)
                }
            });
        }

        sender.oneshot_command({
            let media = model.media.clone();
            async move {
                let thumbnail = get_thumbnail(api_client, &media, &tile_display).await;
                MediaTileCommandOutput::ThumbnailLoaded(thumbnail)
            }
        });

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            MediaTileInput::Remove => APP_BROKER.send(AppInput::RemoveMedia(
                self.media.clone(),
                self.api_client.server_id().to_string().replace('-', ""),
            )),
            MediaTileInput::Play => {
                match self.media.type_ {
                    Some(BaseItemKind::CollectionFolder) => {
                        APP_BROKER.send(AppInput::ShowCollection(self.media.clone()));
                    }
                    _ => {
                        match get_next_playable_media(self.api_client.clone(), self.media.clone())
                            .await
                        {
                            Some(media) => APP_BROKER.send(AppInput::PlayVideo(media)),
                            _ => {
                                let mut message = "No playable media found".to_string();
                                if let Some(name) = self.media.name.as_ref() {
                                    message += &format!(" for {name}");
                                }
                                APP_BROKER.send(AppInput::Toast(message, None))
                            }
                        };
                    }
                };
            }
            MediaTileInput::ShowDetails => {
                match self.media.type_ {
                    Some(BaseItemKind::CollectionFolder) => {
                        APP_BROKER.send(AppInput::ShowCollection(self.media.clone()));
                    }
                    _ => {
                        APP_BROKER.send(AppInput::ShowDetails(self.media.clone()));
                    }
                };
            }
        }
    }

    async fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            MediaTileCommandOutput::ThumbnailLoaded(thumbnail) => {
                self.thumbnail = thumbnail;
            }
            MediaTileCommandOutput::StorageSizeLoaded(size) => {
                self.storage_size = size.map(format_file_size);
            }
        }
    }
}

impl MediaTile {
    pub fn new(media: BaseItemDto, api_client: Arc<ApiClient>) -> Self {
        Self {
            media,
            api_client,
            thumbnail: None,
            storage_size: None,
            movie_info: None,
        }
    }

    fn progress_fraction(&self) -> Option<f64> {
        if let Some(info) = &self.movie_info {
            return info.progress;
        }
        self.media
            .user_data
            .as_ref()?
            .played_percentage
            .filter(|p| p.is_finite())
            .map(|p| (p / 100.0).clamp(0.0, 1.0))
    }
    fn poster_accessible_label(&self) -> String {
        let title = self
            .media
            .name
            .clone()
            .unwrap_or_else(|| tr!("library-media-tile-unnamed-item").to_string());
        tr!("movie-card-play", {"title" => title.as_str()}).to_string()
    }
    fn poster_tooltip(&self) -> Option<String> {
        let size = self.size_tooltip();
        if let Some(info) = &self.movie_info {
            let mut summary = info.summary();
            if let Some(size) = size {
                summary.push_str(&format!("\n{size}"));
            }
            Some(summary)
        } else {
            size
        }
    }
    fn size_tooltip(&self) -> Option<String> {
        let size = self.storage_size.as_ref()?;
        Some(if self.media.type_ == Some(BaseItemKind::Series) {
            tr!("library-media-tile-show-size", { "size" => size.as_str() }).to_string()
        } else {
            tr!("library-media-tile-movie-size", { "size" => size.as_str() }).to_string()
        })
    }

    fn get_item_label(&self) -> String {
        match (
            self.media
                .series_name
                .as_ref()
                .map(|s| markup_escape_text(s)),
            self.media
                .episode_name_with_number()
                .as_ref()
                .map(|s| markup_escape_text(s)),
            self.media.display_years(),
        ) {
            (Some(series_name), Some(name), _) => format!(
                r#"{series_name}
<span size="small">{name}</span>"#
            ),
            (_, Some(name), Some(display_years)) => format!(
                r#"{name}
<span size="small">{display_years}</span>"#
            ),
            (_, Some(name), _) => name.to_string(),
            _ => tr!("library-media-tile-unnamed-item").to_string(),
        }
    }
}

async fn get_thumbnail(
    api_client: Arc<ApiClient>,
    media: &BaseItemDto,
    tile_display: &MediaTileDisplay,
) -> Option<gdk::Texture> {
    let (tile_width, tile_height) = tile_display.dimensions(media);
    let img_url = match tile_display {
        MediaTileDisplay::Wide => {
            let config = CONFIG.read();
            let aspect_ratio_good = media.primary_image_aspect_ratio.unwrap_or_default() > 1.5;

            if config.general.use_episode_image && aspect_ratio_good {
                api_client.get_episode_primary_image_url(media, tile_display.height())
            } else if aspect_ratio_good {
                api_client.get_parent_or_item_thumbnail_url(media, tile_display.height())
            } else {
                api_client.get_episode_thumbnail_or_backdrop_url(media, tile_display.height())
            }
        }
        MediaTileDisplay::Cover | MediaTileDisplay::CoverLarge => {
            api_client.get_parent_or_item_primary_image_url(media)
        }
        MediaTileDisplay::CollectionWide | MediaTileDisplay::Buttons => {
            api_client.get_collection_thumbnail_url(media)
        }
    };

    let img_url = match img_url {
        Ok(img_url) => img_url,
        _ => return None,
    };

    let img_bytes: VecDeque<u8> = reqwest::get(img_url)
        .await
        .expect("Error getting media tile image: {img_url}")
        .bytes()
        .await
        .expect("Error getting media tile image bytes: {img_url}")
        .into_iter()
        .collect();

    let pixbuf = match gdk_pixbuf::Pixbuf::from_read(img_bytes) {
        Ok(pixbuf) => pixbuf,
        _ => {
            error!("Error creating media tile pixbuf: {:#?}", media.id);
            return None;
        }
    };

    // TODO: merge resizing with how it's done for episode list

    let resized = gdk_pixbuf::Pixbuf::new(
        gdk_pixbuf::Colorspace::Rgb,
        true,
        8,
        tile_width,
        tile_height,
    )?;

    let scale = tile_height as f64 / pixbuf.height() as f64;

    pixbuf.scale(
        &resized,
        0,
        0,
        tile_width,
        tile_height,
        0.0,
        0.0,
        scale,
        scale,
        gdk_pixbuf::InterpType::Bilinear,
    );

    Some(gdk::Texture::for_pixbuf(&resized))
}
