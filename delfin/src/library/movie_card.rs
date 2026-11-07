//! Formatting and native caption widgets for the information-rich movie grid.
use crate::tr;
use jellyfin_api::types::{BaseItemDto, MediaStreamType};
use relm4::gtk::{self, prelude::*};

pub(super) struct MovieCardInfo {
    pub title: String,
    pub subtitle: String,
    pub community: Option<String>,
    pub critic: Option<String>,
    pub resolution: Option<String>,
    pub resolution_tooltip: Option<String>,
    pub watch_label: String,
    pub watch_class: &'static str,
    pub watch_icon: &'static str,
    pub progress: Option<f64>,
}

/// Construct only canonical HTTPS title links, never an arbitrary provider URL.
pub(super) fn imdb_url(item: &BaseItemDto) -> Option<String> {
    item.provider_ids
        .as_ref()?
        .iter()
        .find_map(|(provider, value)| {
            if !provider.eq_ignore_ascii_case("imdb") {
                return None;
            }
            let id = value.as_ref()?.trim().to_ascii_lowercase();
            let digits = id.strip_prefix("tt")?;
            if !(7..=12).contains(&digits.len()) || !digits.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            Some(format!("https://www.imdb.com/title/{id}/"))
        })
}

fn rating(value: Option<f32>, maximum: f32, decimals: usize) -> Option<String> {
    value
        .filter(|v| v.is_finite() && *v >= 0.0 && *v <= maximum)
        .map(|v| format!("{v:.decimals$}"))
}
fn dimensions(item: &BaseItemDto) -> Option<(i32, i32)> {
    let valid = |w: Option<i32>, h: Option<i32>| w.zip(h).filter(|(w, h)| *w > 0 && *h > 0);
    valid(item.width, item.height).or_else(|| {
        item.media_streams
            .as_ref()?
            .iter()
            .filter(|s| s.type_ == Some(MediaStreamType::Video))
            .find_map(|s| valid(s.width, s.height))
    })
}
fn resolution(width: i32, height: i32) -> &'static str {
    // Width recognizes letterboxed cinema encodes (e.g. 1920×800) correctly.
    // Normalize orientation so a portrait encode is not mistaken for 4K.
    let (long, short) = (width.max(height), width.min(height));
    if long >= 7600 || short >= 4200 {
        "8K"
    } else if long >= 3800 || short >= 2100 {
        "4K"
    } else if long >= 2500 || short >= 1400 {
        "1440p"
    } else if long >= 1900 || short >= 1060 {
        "1080p"
    } else if long >= 1250 || short >= 700 {
        "720p"
    } else if short >= 560 {
        "576p"
    } else if short >= 470 {
        "480p"
    } else {
        "SD"
    }
}
impl MovieCardInfo {
    pub fn new(item: &BaseItemDto) -> Self {
        let title = item
            .name
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| tr!("library-media-tile-unnamed-item").into());
        let runtime = item.run_time_ticks.filter(|ticks| *ticks > 0).map(|ticks| {
            let minutes = (ticks / 600_000_000).max(1);
            tr!("media-details-run-time", { "hours" => minutes / 60, "minutes" => minutes % 60 })
                .to_string()
        });
        let subtitle = [
            item.production_year
                .filter(|y| *y > 0)
                .map(|y| y.to_string()),
            runtime,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        let subtitle = if subtitle.is_empty() {
            tr!("movie-card-runtime-missing").into()
        } else {
            subtitle
        };
        let dims = dimensions(item);
        let user = item.user_data.as_ref();
        let played = user.and_then(|u| u.played);
        let position = user
            .and_then(|u| u.playback_position_ticks)
            .filter(|p| *p > 0);
        let progress = user
            .and_then(|u| u.played_percentage)
            .filter(|p| p.is_finite() && *p > 0.0 && *p < 100.0)
            .map(|p| p / 100.0)
            .or_else(|| {
                position
                    .zip(item.run_time_ticks.filter(|t| *t > 0))
                    .map(|(p, t)| p as f64 / t as f64)
                    .filter(|f| *f > 0.0 && *f < 1.0)
            });
        let in_progress = progress.is_some() || (position.is_some() && played != Some(true));
        let (watch_label, watch_class, watch_icon) = if in_progress {
            let label = if played == Some(true) {
                tr!("movie-card-watching-again")
            } else {
                tr!("movie-card-in-progress")
            };
            (
                progress.map_or_else(
                    || label.to_string(),
                    |p| format!("{label} · {:.0}%", p * 100.0),
                ),
                "movie-watching",
                "play-filled",
            )
        } else if played == Some(true) {
            (
                tr!("movie-card-watched").into(),
                "movie-watched",
                "check-round-outline",
            )
        } else if played == Some(false) {
            (
                tr!("movie-card-unwatched").into(),
                "movie-unwatched",
                "eye-open-negative-filled",
            )
        } else {
            (
                tr!("movie-card-status-missing").into(),
                "movie-unwatched",
                "eye-open-negative-filled",
            )
        };
        Self {
            title,
            subtitle,
            community: rating(item.community_rating, 10.0, 1),
            critic: rating(item.critic_rating, 100.0, 0),
            resolution: dims.map(|(w, h)| resolution(w, h).into()),
            resolution_tooltip: dims.map(|(w, h)| {
                tr!("movie-card-video-size", {"width" => w, "height" => h}).to_string()
            }),
            watch_label,
            watch_class,
            watch_icon,
            progress,
        }
    }
    pub fn summary(&self) -> String {
        let mut lines = vec![self.title.clone(), self.subtitle.clone()];
        if let Some(rating) = &self.community {
            lines.push(
                tr!("movie-card-community-tooltip", {"rating" => rating.as_str()}).to_string(),
            );
        }
        if let Some(rating) = &self.critic {
            lines.push(tr!("movie-card-critic-tooltip", {"rating" => rating.as_str()}).to_string());
        }
        if let Some(resolution) = &self.resolution_tooltip {
            lines.push(resolution.clone());
        }
        lines.push(self.watch_label.clone());
        lines.join("\n")
    }
}
fn text(value: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(value));
    label.set_xalign(0.0);
    label.add_css_class(class);
    label
}
pub(super) fn caption(info: &MovieCardInfo, width: i32) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 5);
    content.set_width_request(width);
    content.add_css_class("movie-card-caption");
    let title = text(&info.title, "movie-card-title");
    title.set_wrap(true);
    title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    title.set_lines(2);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_max_width_chars(1);
    title.set_height_request(crate::utils::constants::POSTER_TITLE_HEIGHT);
    title.set_valign(gtk::Align::Start);
    title.set_yalign(0.0);
    content.append(&title);
    let subtitle = text(&info.subtitle, "movie-card-subtitle");
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
    content.append(&subtitle);
    let ratings = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    ratings.add_css_class("movie-card-ratings");
    if let Some(value) = &info.community {
        let community = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        community.set_tooltip_text(Some(
            &tr!("movie-card-community-tooltip", {"rating" => value.as_str()}),
        ));
        let star = gtk::Image::from_icon_name("star-large");
        star.set_pixel_size(14);
        star.add_css_class("movie-rating-star");
        community.append(&star);
        community.append(&text(value, "movie-card-score"));
        community.append(&text("/10", "movie-card-secondary"));
        ratings.append(&community);
    }
    if let Some(value) = &info.critic {
        let critic = text(
            &tr!("movie-card-critic", {"rating" => value.as_str()}),
            "movie-card-secondary",
        );
        critic.set_hexpand(true);
        critic.set_xalign(if info.community.is_some() { 1.0 } else { 0.0 });
        critic.set_tooltip_text(Some(
            &tr!("movie-card-critic-tooltip", {"rating" => value.as_str()}),
        ));
        ratings.append(&critic);
    }
    if info.community.is_none() && info.critic.is_none() {
        ratings.append(&text(
            tr!("movie-card-ratings-missing"),
            "movie-card-secondary",
        ));
    }
    content.append(&ratings);
    let watch = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    watch.add_css_class(info.watch_class);
    watch.add_css_class("movie-card-watch");
    let icon = gtk::Image::from_icon_name(info.watch_icon);
    icon.set_pixel_size(14);
    watch.append(&icon);
    let status = text(&info.watch_label, "movie-card-watch-label");
    status.set_ellipsize(gtk::pango::EllipsizeMode::End);
    watch.append(&status);
    content.append(&watch);
    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn item(value: serde_json::Value) -> BaseItemDto {
        serde_json::from_value(value).unwrap()
    }
    #[test]
    fn imdb_links_use_validated_provider_ids_without_guessing_or_trusting_urls() {
        for key in ["Imdb", "IMDb", "imdb"] {
            let info = item(json!({"ProviderIds":{key:"tt0133093"}}));
            assert_eq!(
                imdb_url(&info).as_deref(),
                Some("https://www.imdb.com/title/tt0133093/")
            );
        }
        for id in [
            "",
            "tt1",
            "tt1234567/evil",
            "tt1234567?x=y",
            "https://evil.test/",
            "tt123456x",
            "1234567",
            "tt1234567890123",
        ] {
            assert!(imdb_url(&item(json!({"ProviderIds":{"Imdb":id}}))).is_none());
        }
        assert!(imdb_url(&item(json!({"ProviderIds":{"Tmdb":"1234567","Imdb":null}}))).is_none());
        assert!(imdb_url(&item(json!({}))).is_none());
    }

    #[test]
    fn movie_metadata_uses_local_runtime_ratings_and_cropped_video_dimensions() {
        let info = MovieCardInfo::new(&item(
            json!({"Name":"A <movie> & its title", "ProductionYear":2016,
            "RunTimeTicks":80880000000_i64, "CommunityRating":7.279, "CriticRating":52,
            "MediaStreams":[{"Type":"Audio","Width":9999,"Height":9999},{"Type":"Video","Width":1920,"Height":800}],
            "UserData":{"Played":false,"PlaybackPositionTicks":0}}),
        ));
        assert_eq!(info.title, "A <movie> & its title");
        assert_eq!(info.subtitle, "2016 · 2h 14m");
        assert_eq!(info.community.as_deref(), Some("7.3"));
        assert_eq!(info.critic.as_deref(), Some("52"));
        assert_eq!(info.resolution.as_deref(), Some("1080p"));
        assert_eq!(info.watch_label, "Unwatched");
        assert!(info.summary().contains("Community"));
        assert!(!info.summary().contains("IMDb"));
    }
    #[test]
    fn unknown_metadata_is_not_fabricated_or_treated_as_unwatched() {
        let info = MovieCardInfo::new(&item(
            json!({"RunTimeTicks":-1,"CommunityRating":-1,"CriticRating":101,
            "Width":0,"Height":0}),
        ));
        assert_eq!(info.subtitle, "Runtime unavailable");
        assert!(info.community.is_none() && info.critic.is_none() && info.resolution.is_none());
        assert_eq!(info.watch_label, "Watch status unavailable");
        let zero = MovieCardInfo::new(&item(json!({"CommunityRating":0,"CriticRating":0})));
        assert_eq!(zero.community.as_deref(), Some("0.0"));
        assert_eq!(zero.critic.as_deref(), Some("0"));
        assert!(rating(Some(f32::NAN), 10.0, 1).is_none());
    }
    #[test]
    fn resolution_handles_cinema_crops_standard_definition_and_portrait_without_network() {
        for (w, h, label) in [
            (3840, 1600, "4K"),
            (1920, 800, "1080p"),
            (1280, 536, "720p"),
            (720, 576, "576p"),
            (720, 480, "480p"),
            (1080, 1920, "1080p"),
            (7680, 4320, "8K"),
        ] {
            assert_eq!(resolution(w, h), label);
        }
    }
    #[test]
    fn watched_resume_and_rewatch_states_use_current_user_data() {
        let runtime = 6000000000_i64;
        let info = MovieCardInfo::new(&item(
            json!({"RunTimeTicks":runtime,"UserData":{"Played":false,"PlaybackPositionTicks":runtime/4}}),
        ));
        assert_eq!(info.watch_label, "In progress · 25%");
        assert_eq!(info.progress, Some(0.25));
        let watched = MovieCardInfo::new(&item(
            json!({"UserData":{"Played":true,"PlayedPercentage":100}}),
        ));
        assert_eq!(watched.watch_label, "Watched");
        assert!(watched.progress.is_none());
        let rewatch = MovieCardInfo::new(&item(
            json!({"UserData":{"Played":true,"PlayedPercentage":40}}),
        ));
        assert_eq!(rewatch.watch_label, "Watching again · 40%");
    }
}
