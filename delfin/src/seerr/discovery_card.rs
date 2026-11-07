//! Discovery uses TMDB list metadata, not Jellyfin file/watch metadata.
use super::{dto::*, ui_state::media_status};
use crate::tr;
use chrono::Datelike;

pub(super) struct AvailabilityLine {
    pub text: String,
    pub class: &'static str,
    pub icon: &'static str,
}
pub(super) struct DiscoveryCardInfo {
    pub title: String,
    pub subtitle: String,
    pub genres: Vec<String>,
    pub trailer_search: Option<String>,
    pub rating: Option<String>,
    pub rating_tooltip: Option<String>,
    pub summary: String,
    pub kind: Option<MediaType>,
    pub availability: Vec<AvailabilityLine>,
}
fn availability(quality: &str, status: MediaStatus) -> AvailabilityLine {
    let (class, icon) = match status {
        MediaStatus::AVAILABLE => ("discovery-available", "check-round-outline"),
        MediaStatus::PROCESSING | MediaStatus::PENDING => ("discovery-processing", "refresh"),
        MediaStatus::PARTIALLY_AVAILABLE => ("discovery-processing", "folder-filled"),
        MediaStatus::BLOCKLISTED | MediaStatus::DELETED => ("discovery-neutral", "warning"),
        _ => ("discovery-neutral", "movies-and-tv-filled"),
    };
    AvailabilityLine {
        text: format!("{quality} · {}", media_status(status)),
        class,
        icon,
    }
}
// Stable TMDB movie/TV genre identifiers; unknown future IDs are omitted.
// Localized names avoid a lookup request for every card or page.
fn genre_name(id: u64) -> Option<String> {
    let key = match id {
        28 => "seerr-genre-action",
        12 => "seerr-genre-adventure",
        16 => "seerr-genre-animation",
        35 => "seerr-genre-comedy",
        80 => "seerr-genre-crime",
        99 => "seerr-genre-documentary",
        18 => "seerr-genre-drama",
        10751 => "seerr-genre-family",
        14 => "seerr-genre-fantasy",
        36 => "seerr-genre-history",
        27 => "seerr-genre-horror",
        10402 => "seerr-genre-music",
        9648 => "seerr-genre-mystery",
        10749 => "seerr-genre-romance",
        878 => "seerr-genre-science-fiction",
        10770 => "seerr-genre-tv-movie",
        53 => "seerr-genre-thriller",
        10752 => "seerr-genre-war",
        37 => "seerr-genre-western",
        10759 => "seerr-genre-action-adventure",
        10762 => "seerr-genre-kids",
        10763 => "seerr-genre-news",
        10764 => "seerr-genre-reality",
        10765 => "seerr-genre-scifi-fantasy",
        10766 => "seerr-genre-soap",
        10767 => "seerr-genre-talk",
        10768 => "seerr-genre-war-politics",
        _ => return None,
    };
    Some(tr!(key).to_string())
}

impl DiscoveryCardInfo {
    pub fn new(item: &DiscoveryResult) -> Self {
        let source_title = item
            .title
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or(item.name.as_deref().filter(|s| !s.trim().is_empty()));
        let title = source_title.unwrap_or(tr!("seerr-untitled")).to_string();
        let (kind, kind_label) = match item.media_type {
            ResultType::Movie => (Some(MediaType::Movie), tr!("seerr-discover-movie")),
            ResultType::Tv => (Some(MediaType::Tv), tr!("seerr-discover-tv")),
            ResultType::Person => (None, tr!("seerr-discover-person")),
            ResultType::Other => (None, tr!("seerr-discover-other")),
        };
        let date = match item.media_type {
            ResultType::Movie => item.release_date.as_deref(),
            ResultType::Tv => item.first_air_date.as_deref(),
            _ => None,
        };
        let year = date
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .map(|d| d.year())
            .filter(|y| *y > 0)
            .map(|y| y.to_string());
        let subtitle = year.as_ref().map_or_else(
            || kind_label.to_string(),
            |year| format!("{year} · {kind_label}"),
        );
        let mut genres = Vec::new();
        if kind.is_some() {
            for id in item.genre_ids.as_deref().unwrap_or_default() {
                if let Some(name) = genre_name(*id) {
                    if !genres.contains(&name) {
                        genres.push(name);
                    }
                }
            }
        }
        // A search, not a claimed/guessed direct video. Only fixed-origin HTTPS
        // URLs are constructed; titles are encoded as query data, never paths.
        let trailer_search = kind.zip(source_title).map(|(kind, title)| {
            let mut query = title.trim().to_string();
            if let Some(year) = &year {
                query.push(' ');
                query.push_str(year);
            }
            query.push_str(match kind {
                MediaType::Movie => " official trailer",
                MediaType::Tv => " official TV series trailer",
            });
            let mut url = reqwest::Url::parse("https://www.youtube.com/results").unwrap();
            url.query_pairs_mut().append_pair("search_query", &query);
            url.to_string()
        });
        let rating = item
            .vote_average
            .filter(|r| {
                kind.is_some()
                    && r.is_finite()
                    && *r > 0.0
                    && *r <= 10.0
                    && item.vote_count != Some(0)
            })
            .map(|r| format!("{r:.1}"));
        let rating_tooltip = rating.as_ref().map(|rating| {
            let mut tooltip = tr!("seerr-discover-score",{"rating"=>rating.as_str()}).to_string();
            if let Some(votes) = item.vote_count {
                tooltip.push_str(&format!(
                    "\n{}",
                    tr!("seerr-discover-votes",{"votes"=>votes})
                ));
            }
            tooltip
        });
        let availability = if kind.is_none() {
            vec![AvailabilityLine {
                text: tr!("seerr-not-requestable").into(),
                class: "discovery-neutral",
                icon: "movies-and-tv-filled",
            }]
        } else if let Some(info) = &item.media_info {
            vec![
                availability(tr!("seerr-standard"), info.status),
                availability(
                    tr!("seerr-resolution-4k"),
                    info.status4k.unwrap_or(MediaStatus::UNKNOWN),
                ),
            ]
        } else {
            vec![AvailabilityLine {
                text: tr!("seerr-discover-not-requested").into(),
                class: "discovery-neutral",
                icon: "tag-multiple-filled",
            }]
        };
        let mut lines = vec![title.clone(), subtitle.clone()];
        if !genres.is_empty() {
            lines.push(tr!("seerr-discover-genres",{"genres"=>genres.join(" · ")}).to_string());
        }
        if let Some(rating) = &rating_tooltip {
            lines.push(rating.clone());
        }
        if let Some(date) = date.filter(|d| !d.trim().is_empty()) {
            lines.push(tr!("seerr-discover-date",{"date"=>date}).to_string());
        }
        if let Some(language) = item
            .original_language
            .as_deref()
            .filter(|s| (2..=3).contains(&s.len()) && s.bytes().all(|c| c.is_ascii_alphabetic()))
        {
            lines.push(
                tr!("seerr-discover-language",{"language"=>language.to_ascii_uppercase()})
                    .to_string(),
            );
        }
        lines.extend(availability.iter().map(|line| line.text.clone()));
        if let Some(overview) = item.overview.as_deref().filter(|s| !s.trim().is_empty()) {
            lines.push(overview.to_string());
        }
        Self {
            title,
            subtitle,
            genres,
            trailer_search,
            rating,
            rating_tooltip,
            summary: lines.join("\n"),
            kind,
            availability,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    fn info(value: Value) -> DiscoveryCardInfo {
        DiscoveryCardInfo::new(&serde_json::from_value(value).unwrap())
    }
    #[test]
    fn genres_are_named_deduplicated_and_unknown_ids_are_not_guessed() {
        let card = info(
            json!({"id":1,"mediaType":"movie","title":"A film","genreIds":[28,18,28,999999,10752]}),
        );
        assert_eq!(card.genres, vec!["Action", "Drama", "War"]);
        assert!(card.summary.contains("Genres: Action · Drama · War"));
        assert!(genre_name(999999).is_none());
        for genres in [json!(null), json!([]), json!([999999])] {
            assert!(info(json!({"id":1,"mediaType":"movie","genreIds":genres}))
                .genres
                .is_empty());
        }
    }
    #[test]
    fn trailer_search_encodes_titles_without_changing_host_or_adding_parameters() {
        let title = "Dune & Friends / 测试 #1?x=y";
        let card =
            info(json!({"id":1,"mediaType":"movie","title":title,"releaseDate":"2024-03-01"}));
        let url = reqwest::Url::parse(card.trailer_search.as_deref().unwrap()).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("www.youtube.com"));
        assert_eq!(url.path(), "/results");
        assert!(url.fragment().is_none());
        let pairs: Vec<_> = url.query_pairs().collect();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "search_query");
        assert_eq!(pairs[0].1, format!("{title} 2024 official trailer"));
        let tv = info(
            json!({"id":2,"mediaType":"tv","name":"Dark","firstAirDate":"2017-12-01","genreIds":[10765,9648]}),
        );
        assert_eq!(tv.genres, vec!["Sci-Fi & Fantasy", "Mystery"]);
        let url = reqwest::Url::parse(tv.trailer_search.as_deref().unwrap()).unwrap();
        assert_eq!(
            url.query_pairs().next().unwrap().1,
            "Dark 2017 official TV series trailer"
        );
    }
    #[test]
    fn untitled_nonmedia_and_bad_dates_do_not_generate_misleading_trailer_searches() {
        for value in [
            json!({"id":1,"mediaType":"movie"}),
            json!({"id":1,"mediaType":"tv","name":"  "}),
            json!({"id":1,"mediaType":"person","name":"Person","genreIds":[18]}),
            json!({"id":1,"mediaType":"collection","title":"Collection"}),
        ] {
            let card = info(value);
            assert!(card.trailer_search.is_none());
            assert!(card.genres.is_empty());
        }
        let card =
            info(json!({"id":1,"mediaType":"movie","title":"Arrival","releaseDate":"bad date"}));
        let url = reqwest::Url::parse(card.trailer_search.as_deref().unwrap()).unwrap();
        assert_eq!(
            url.query_pairs().next().unwrap().1,
            "Arrival official trailer"
        );
    }
    #[test]
    fn movie_cards_use_existing_list_scores_dates_and_separate_quality_statuses() {
        let card = info(
            json!({"id":1,"mediaType":"movie","title":"A <film> & its title","releaseDate":"2026-05-02","voteAverage":7.279,"voteCount":1200,"originalLanguage":"en","overview":"An overview.","mediaInfo":{"id":1,"tmdbId":1,"status":5,"status4k":3}}),
        );
        assert_eq!(card.title, "A <film> & its title");
        assert_eq!(card.subtitle, "2026 · Movie");
        assert_eq!(card.rating.as_deref(), Some("7.3"));
        assert_eq!(card.availability[0].text, "Standard · Available");
        assert_eq!(card.availability[1].text, "4K · Processing");
        assert_eq!(card.availability[0].class, "discovery-available");
        assert_eq!(card.availability[1].class, "discovery-processing");
        assert!(card.summary.contains("TMDB"));
        assert!(card.summary.contains("1200"));
        assert!(card.summary.contains("EN"));
        assert!(card.summary.contains("An overview."));
        assert!(!card.summary.contains("IMDb"));
        assert!(!card.summary.contains("Watched"));
    }
    #[test]
    fn tv_dates_and_untracked_or_unknown_media_are_not_confused() {
        let tv =
            info(json!({"id":2,"mediaType":"tv","name":"A series","firstAirDate":"2020-09-01"}));
        assert_eq!(tv.subtitle, "2020 · TV series");
        assert_eq!(tv.kind, Some(MediaType::Tv));
        assert!(tv.rating.is_none());
        assert_eq!(tv.availability[0].text, "Not requested");
        let unknown =
            info(json!({"id":2,"mediaType":"tv","mediaInfo":{"id":2,"tmdbId":2,"status":99}}));
        assert_eq!(unknown.availability[0].text, "Standard · Unknown (99)");
        assert_eq!(unknown.availability[1].text, "4K · Unknown");
    }
    #[test]
    fn unrated_future_missing_or_invalid_metadata_is_not_fabricated() {
        for value in [
            json!({}),
            json!({"voteAverage":0,"voteCount":0}),
            json!({"voteAverage":8,"voteCount":0}),
            json!({"voteAverage":-1}),
            json!({"voteAverage":11}),
        ] {
            let mut item = json!({"id":1,"mediaType":"movie","releaseDate":"2026-02-30"});
            item.as_object_mut()
                .unwrap()
                .extend(value.as_object().unwrap().clone());
            let card = info(item);
            assert!(card.rating.is_none());
            assert_eq!(card.subtitle, "Movie");
        }
        let unknown =
            info(json!({"id":3,"mediaType":"person","name":"A person","voteAverage":8.0}));
        assert!(unknown.kind.is_none());
        assert!(unknown.rating.is_none());
        assert_eq!(unknown.subtitle, "Person");
        assert_eq!(unknown.availability[0].text, "Not a movie or TV series");
    }
}
