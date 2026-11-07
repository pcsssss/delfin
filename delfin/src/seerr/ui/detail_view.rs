//! Detail metadata from the existing authenticated Seerr response.
use super::*;
pub(super) const DETAIL_POSTER: u64 = u64::MAX;
fn heading(body: &gtk::Box, title: &str) {
    let text = label(title);
    text.add_css_class("title-3");
    text.set_margin_top(12);
    body.append(&text);
}
fn fact(body: &gtk::Box, title: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|s| !s.trim().is_empty()) {
        let line = label(&format!("{title}: {value}"));
        line.set_max_width_chars(75);
        body.append(&line);
    }
}
fn names(values: &[NamedDetail]) -> String {
    values
        .iter()
        .map(|v| v.name.trim())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}
pub(super) fn imdb_link(id: &str) -> Option<String> {
    let digits = id.strip_prefix("tt")?;
    ((7..=12).contains(&digits.len()) && digits.bytes().all(|c| c.is_ascii_digit()))
        .then(|| format!("https://www.imdb.com/title/{id}/"))
}
pub(super) fn trailer_link(video: &DetailVideo) -> Option<String> {
    (video.site == "YouTube"
        && video.kind == "Trailer"
        && video.key.len() == 11
        && video
            .key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
    .then(|| format!("https://www.youtube.com/watch?v={}", video.key))
}
#[allow(clippy::too_many_arguments)]
pub(super) fn header(
    hub: &Shared,
    title: &str,
    date: Option<&str>,
    runtime: Option<u32>,
    genres: &[Genre],
    rating: Option<f64>,
    poster: Option<&str>,
    metadata: &DetailMetadata,
    info: Option<&MediaInfo>,
    kind: MediaType,
    id: u64,
    imdb: Option<&str>,
) -> gtk::Box {
    let body = detail_header(hub);
    let hero = row(20);
    hero.set_valign(gtk::Align::Start);
    let art = gtk::Overlay::new();
    art.set_size_request(144, 216);
    art.set_valign(gtk::Align::Start);
    art.set_overflow(gtk::Overflow::Hidden);
    art.add_css_class("discovery-art");
    let fallback = gtk::Image::from_icon_name("video-x-generic-symbolic");
    fallback.set_pixel_size(48);
    art.set_child(Some(&fallback));
    let picture = gtk::Picture::new();
    picture.set_size_request(144, 216);
    picture.set_can_shrink(true);
    picture.set_content_fit(gtk::ContentFit::Cover);
    picture.set_alternative_text(Some(title));
    art.add_overlay(&picture);
    // Only registered when the service has a valid fixed-origin artwork path.
    if poster.and_then(poster_url).is_some() {
        hub.borrow_mut().posters.insert(DETAIL_POSTER, picture);
    }
    hero.append(&art);
    let summary = column(10);
    summary.set_hexpand(true);
    let name = label(title);
    name.add_css_class("title-1");
    summary.append(&name);
    if let Some(tagline) = metadata.tagline.as_deref().filter(|s| !s.trim().is_empty()) {
        let text = label(tagline);
        summary.append(&text);
    }
    let mut basics = vec![match kind {
        MediaType::Movie => tr!("seerr-discover-movie").to_string(),
        MediaType::Tv => tr!("seerr-discover-tv").to_string(),
    }];
    if let Some(date) = date.filter(|s| !s.is_empty()) {
        basics.push(date.to_string());
    }
    if let Some(minutes) = runtime.filter(|m| *m > 0) {
        basics.push(format!("{} {}", minutes, tr!("seerr-detail-minutes")));
    }
    summary.append(&label(&basics.join(" · ")));
    let genre = genres
        .iter()
        .map(|g| g.name.as_str())
        .collect::<Vec<_>>()
        .join(" · ");
    if !genre.is_empty() {
        summary.append(&label(&genre));
    }
    if let Some(score) = rating.filter(|r| r.is_finite() && *r > 0.0 && *r <= 10.0) {
        let count = metadata
            .vote_count
            .filter(|c| *c > 0)
            .map(|n| format!(" · {n} {}", tr!("seerr-detail-votes")))
            .unwrap_or_default();
        summary.append(&label(&format!("TMDB {score:.1}/10{count}")));
    }
    summary.append(&label(
        &info
            .map(|i| media_availability(i.status, i.status4k))
            .unwrap_or_else(|| tr!("seerr-discover-not-requested").to_string()),
    ));
    if let Some(info) = info {
        for request in &info.requests {
            summary.append(&label(&format!(
                "{} #{} · {} · {}",
                tr!("seerr-request"),
                request.id,
                if request.is4k {
                    "4K".to_string()
                } else {
                    tr!("seerr-detail-standard").to_string()
                },
                request_status(request.status)
            )));
        }
    }
    hero.append(&summary);
    body.append(&hero);
    let links = gtk::FlowBox::new();
    links.set_halign(gtk::Align::Start);
    links.set_selection_mode(gtk::SelectionMode::None);
    links.set_max_children_per_line(3);
    links.set_min_children_per_line(1);
    links.set_column_spacing(8);
    links.set_row_spacing(4);
    let path = match kind {
        MediaType::Movie => "movie",
        MediaType::Tv => "tv",
    };
    links.insert(
        &gtk::LinkButton::with_label(&format!("https://www.themoviedb.org/{path}/{id}"), "TMDB"),
        -1,
    );
    if let Some(url) = imdb
        .or_else(|| {
            metadata
                .external_ids
                .as_ref()
                .and_then(|e| e.imdb_id.as_deref())
        })
        .and_then(imdb_link)
    {
        links.insert(&gtk::LinkButton::with_label(&url, "IMDb"), -1);
    }
    if let Some(url) = metadata.related_videos.iter().find_map(trailer_link) {
        links.insert(
            &gtk::LinkButton::with_label(&url, tr!("seerr-detail-trailer")),
            -1,
        );
    }
    body.append(&links);
    status(hub, tr!("seerr-connected"));
    body
}
pub(super) fn story(body: &gtk::Box, overview: Option<&str>, metadata: &DetailMetadata) {
    heading(body, tr!("seerr-detail-synopsis"));
    let text = label(
        overview
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(tr!("seerr-detail-no-synopsis")),
    );
    text.set_max_width_chars(75);
    text.set_halign(gtk::Align::Start);
    let prose = adw::Clamp::new();
    prose.set_maximum_size(760);
    prose.set_tightening_threshold(760);
    prose.set_halign(gtk::Align::Start);
    prose.set_child(Some(&text));
    body.append(&prose);
    heading(body, tr!("seerr-details"));
    fact(body, tr!("seerr-detail-status"), metadata.status.as_deref());
    fact(
        body,
        tr!("seerr-detail-language"),
        metadata.original_language.as_deref(),
    );
    fact(
        body,
        tr!("seerr-detail-languages"),
        Some(&names(&metadata.spoken_languages)),
    );
    fact(
        body,
        tr!("seerr-detail-studios"),
        Some(&names(&metadata.production_companies)),
    );
    fact(
        body,
        tr!("seerr-detail-countries"),
        Some(&names(&metadata.production_countries)),
    );
}
pub(super) fn credits(body: &gtk::Box, metadata: &DetailMetadata) {
    if let Some(credits) = &metadata.credits {
        let mut crew = Vec::new();
        for member in &credits.crew {
            if matches!(
                member.job.as_deref(),
                Some("Director" | "Screenplay" | "Writer" | "Novel" | "Executive Producer")
            ) {
                let text = format!("{} — {}", member.name, member.job.as_deref().unwrap_or(""));
                if !crew.contains(&text) {
                    crew.push(text);
                }
            }
        }
        if !crew.is_empty() {
            heading(body, tr!("seerr-detail-crew"));
            for member in crew.iter().take(16) {
                body.append(&label(member));
            }
        }
        if !credits.cast.is_empty() {
            heading(body, tr!("seerr-detail-cast"));
            for cast in credits.cast.iter().take(20) {
                let text = match cast.character.as_deref().filter(|s| !s.is_empty()) {
                    Some(role) => format!("{} — {role}", cast.name),
                    None => cast.name.clone(),
                };
                body.append(&label(&text));
            }
        }
    }
}
fn money(value: u64) -> String {
    let digits = value.to_string();
    let mut text = String::from("USD ");
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            text.push(',');
        }
        text.push(c);
    }
    text
}
pub(super) fn movie_facts(body: &gtk::Box, d: &MovieDetails) {
    fact(
        body,
        tr!("seerr-detail-original-title"),
        d.original_title.as_deref().filter(|s| *s != d.title),
    );
    if let Some(budget) = d.budget.filter(|n| *n > 0) {
        fact(body, tr!("seerr-detail-budget"), Some(&money(budget)));
    }
    if let Some(revenue) = d.revenue.filter(|n| *n > 0) {
        fact(body, tr!("seerr-detail-revenue"), Some(&money(revenue)));
    }
}
pub(super) fn tv_facts(body: &gtk::Box, d: &TvDetails) {
    fact(
        body,
        tr!("seerr-detail-original-title"),
        d.original_name.as_deref().filter(|s| *s != d.name),
    );
    fact(
        body,
        tr!("seerr-detail-last-air"),
        d.last_air_date.as_deref(),
    );
    fact(
        body,
        tr!("seerr-detail-networks"),
        Some(&names(&d.networks)),
    );
    fact(
        body,
        tr!("seerr-detail-creators"),
        Some(&names(&d.created_by)),
    );
    if let Some(count) = d.number_of_seasons {
        fact(body, tr!("seerr-seasons"), Some(&count.to_string()));
    }
    if let Some(count) = d.number_of_episodes {
        fact(body, tr!("seerr-episodes"), Some(&count.to_string()));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_only_use_validated_ids_on_fixed_origins() {
        assert_eq!(
            imdb_link("tt1234567").unwrap(),
            "https://www.imdb.com/title/tt1234567/"
        );
        for id in [
            "tt123",
            "tt1234567/evil",
            "https://evil.test",
            "tt１２３４５６７",
        ] {
            assert!(imdb_link(id).is_none());
        }
        let mut video = DetailVideo {
            site: "YouTube".into(),
            kind: "Trailer".into(),
            key: "aB12_-34567".into(),
        };
        assert!(trailer_link(&video)
            .unwrap()
            .starts_with("https://www.youtube.com/watch?v="));
        video.key = "../bad?x=1".into();
        assert!(trailer_link(&video).is_none());
        video.key = "aB12_-34567".into();
        video.site = "evil".into();
        assert!(trailer_link(&video).is_none());
    }
    #[test]
    fn optional_detail_fields_do_not_fabricate_missing_metadata() {
        let movie: MovieDetails =
            serde_json::from_value(serde_json::json!({"id":1,"title":"Minimal"})).unwrap();
        assert!(movie.metadata.vote_count.is_none());
        assert!(movie.metadata.credits.is_none());
        assert!(movie.runtime.is_none());
        let tv:TvDetails=serde_json::from_value(serde_json::json!({"id":2,"name":"Minimal TV","credits":{"cast":[{"name":"Actor","character":"Role"}]},"episodeRunTime":[48],"numberOfEpisodes":8,"externalIds":{"imdbId":"tt1234567"}})).unwrap();
        assert_eq!(tv.episode_run_time, vec![48]);
        assert_eq!(
            tv.metadata.credits.unwrap().cast[0].character.as_deref(),
            Some("Role")
        );
        assert_eq!(tv.number_of_episodes, Some(8));
    }
}
