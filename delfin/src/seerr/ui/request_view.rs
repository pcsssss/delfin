use super::*;
pub(super) fn render(hub: &Shared, page: RequestPage) {
    let body = active_body(hub);
    clear(&body);
    {
        let mut model = hub.borrow_mut();
        model.posters.clear();
        model.request_total = page.page_info.results;
        model.request_previous.set_sensitive(model.request_skip > 0);
        model.request_next.set_sensitive(
            (model.request_skip as u64 + page.results.len() as u64) < model.request_total,
        );
    }
    status(
        hub,
        &format!(
            "{} · {}: {}",
            tr!("seerr-connected"),
            tr!("seerr-requests"),
            page.page_info.results
        ),
    );
    if page.results.is_empty() {
        body.append(&label(tr!("seerr-no-requests")));
    }
    for request in page.results {
        let item = row(16);
        item.set_valign(gtk::Align::Start);
        let art = gtk::Overlay::new();
        art.set_size_request(80, 120);
        art.set_valign(gtk::Align::Start);
        art.set_overflow(gtk::Overflow::Hidden);
        art.add_css_class("discovery-art");
        let fallback = gtk::Image::from_icon_name("video-x-generic-symbolic");
        fallback.set_pixel_size(32);
        art.set_child(Some(&fallback));
        let picture = gtk::Picture::new();
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_can_shrink(true);
        picture.set_size_request(80, 120);
        picture.set_alternative_text(request.title.as_deref());
        art.add_overlay(&picture);
        item.append(&art);
        if request
            .poster_path
            .as_deref()
            .and_then(poster_url)
            .is_some()
        {
            hub.borrow_mut().posters.insert(request.id, picture);
        }
        let content = column(8);
        content.set_hexpand(true);
        let heading = label(
            request
                .title
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(tr!("seerr-request-title-unavailable")),
        );
        heading.add_css_class("title-3");
        content.append(&heading);
        let mut basics = vec![if request.is4k {
            "4K".to_string()
        } else {
            tr!("seerr-detail-standard").to_string()
        }];
        if let Some(media) = &request.media {
            if let Some(kind) = media.media_type {
                basics.insert(
                    0,
                    match kind {
                        MediaType::Movie => tr!("seerr-discover-movie").to_string(),
                        MediaType::Tv => tr!("seerr-discover-tv").to_string(),
                    },
                );
            }
        }
        if let Some(year) = request
            .release_date
            .as_deref()
            .and_then(|s| s.get(..4))
            .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
        {
            basics.push(year.to_string());
        }
        if let Some(profile) = request.profile_name.as_deref().filter(|s| !s.is_empty()) {
            basics.push(profile.to_string());
        }
        content.append(&label(&basics.join(" · ")));
        content.append(&label(&request_status(request.status)));
        if let Some(media) = &request.media {
            content.append(&label(&media_availability(media.status, media.status4k)));
        }
        if let Some(overview) = request.overview.as_deref().filter(|s| !s.is_empty()) {
            let text = label(overview);
            text.set_lines(2);
            text.set_ellipsize(gtk::pango::EllipsizeMode::End);
            content.append(&text);
        }
        for season in &request.seasons {
            content.append(&label(&format!(
                "{} {} · {}",
                tr!("seerr-season"),
                season.season_number,
                request_status(season.status)
            )));
        }
        if let Some(user) = &request.requested_by {
            if let Some(name) = user
                .username
                .as_deref()
                .or(user.display_name.as_deref())
                .or(user.jellyfin_username.as_deref())
                .or(user.email.as_deref())
                .filter(|s| !s.is_empty())
            {
                content.append(&label(&format!(
                    "{}: {name}",
                    tr!("seerr-request-requester")
                )));
            }
        }
        if let Some(date) = &request.created_at {
            let text = label(&format!(
                "{}: {}",
                tr!("seerr-request-created"),
                date.split('T').next().unwrap_or(date)
            ));
            text.set_tooltip_text(Some(date));
            content.append(&text);
        }
        content.append(&label(&format!("{} #{}", tr!("seerr-request"), request.id)));
        if let Some(media) = request.media {
            if let Some(kind) = media.media_type {
                let open = button(tr!("seerr-details"));
                open.set_halign(gtk::Align::Start);
                on_click(&open, hub, move |hub| {
                    details(hub, kind, media.tmdb_id, Some(request.id), request.is4k)
                });
                content.append(&open);
            }
        }
        item.append(&content);
        body.append(&item);
        body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    }
}
