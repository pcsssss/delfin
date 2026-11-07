//! Pure UI formatting and stale-result guards; no network or GTK dependency.
use super::{EpisodeDownloadState, Error, MediaStatus, RequestStatus};
use crate::{tr, utils::file_size::format_file_size};

#[derive(Default)]
pub(super) struct ViewState {
    pub account: u64,
    pub view: u64,
    pub busy: bool,
}
impl ViewState {
    pub fn navigate(&mut self) {
        self.view += 1;
        self.busy = false;
    }
    pub fn account_changed(&mut self) {
        self.account += 1;
        self.navigate();
    }
    pub fn accepts(&self, account: u64, view: u64) -> bool {
        self.account == account && self.view == view
    }
}

pub(super) fn request_status(status: RequestStatus) -> String {
    match status {
        RequestStatus::PENDING => tr!("seerr-pending").into(),
        RequestStatus::APPROVED => tr!("seerr-approved").into(),
        RequestStatus::DECLINED => tr!("seerr-declined").into(),
        RequestStatus::FAILED => tr!("seerr-failed").into(),
        RequestStatus::COMPLETED => tr!("seerr-completed").into(),
        _ => format!("{} ({})", tr!("seerr-unknown"), status.0),
    }
}
pub(super) fn media_status(status: MediaStatus) -> String {
    match status {
        MediaStatus::UNKNOWN => tr!("seerr-unknown").into(),
        MediaStatus::PENDING => tr!("seerr-pending").into(),
        MediaStatus::PROCESSING => tr!("seerr-processing").into(),
        MediaStatus::PARTIALLY_AVAILABLE => tr!("seerr-partial").into(),
        MediaStatus::AVAILABLE => tr!("seerr-available").into(),
        MediaStatus::BLOCKLISTED => tr!("seerr-blocklisted").into(),
        MediaStatus::DELETED => tr!("seerr-deleted").into(),
        _ => format!("{} ({})", tr!("seerr-unknown"), status.0),
    }
}
pub(super) fn media_availability(status: MediaStatus, status4k: Option<MediaStatus>) -> String {
    format!(
        "{}: {} · {}: {}",
        tr!("seerr-standard"),
        media_status(status),
        tr!("seerr-resolution-4k"),
        media_status(status4k.unwrap_or(MediaStatus::UNKNOWN)),
    )
}
pub(super) fn error_text(error: Error) -> String {
    let key = match error {
        Error::InvalidUrl => "seerr-error-url",
        Error::InvalidSelection => "seerr-error-selection",
        Error::ClientSetup => "seerr-error-setup",
        Error::NotAuthenticated | Error::Unauthorized => "seerr-error-auth",
        Error::Forbidden => "seerr-error-forbidden",
        Error::RedirectDenied => "seerr-error-redirect",
        Error::Timeout | Error::Transport => "seerr-error-network",
        Error::Http(404) | Error::Http(503) => "seerr-error-unavailable",
        Error::Http(409) => "seerr-error-conflict",
        Error::Http(_) => "seerr-error-http",
        Error::InvalidResponse => "seerr-error-response",
    };
    let text = tr!(key).to_string();
    if let Error::Http(code) = error {
        format!("{text} (HTTP {code})")
    } else {
        text
    }
}
pub(super) fn yes_no(value: bool) -> String {
    if value {
        tr!("seerr-yes").into()
    } else {
        tr!("seerr-no").into()
    }
}
pub(super) fn episode_state(state: EpisodeDownloadState) -> String {
    tr!(match state {
        EpisodeDownloadState::Imported => "seerr-episode-imported",
        EpisodeDownloadState::Queued => "seerr-episode-queued",
        EpisodeDownloadState::Unmonitored => "seerr-episode-unmonitored",
        EpisodeDownloadState::Unaired => "seerr-episode-unaired",
        EpisodeDownloadState::Missing => "seerr-episode-missing",
        EpisodeDownloadState::Other => "seerr-unknown",
    })
    .into()
}
pub(super) fn speed(bytes: u64) -> String {
    format!("{}/s", format_file_size(bytes))
}
pub(super) fn eta(seconds: Option<u64>) -> String {
    seconds.map_or_else(
        || tr!("seerr-unknown").into(),
        |s| format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60),
    )
}
/// Poster paths are relative TMDB image paths, never arbitrary server-supplied URLs.
pub(super) fn poster_url(path: &str) -> Option<String> {
    let name = path.strip_prefix('/')?;
    if name.is_empty()
        || matches!(name, "." | "..")
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'_' || c == b'-')
    {
        return None;
    }
    Some(format!("https://image.tmdb.org/t/p/w185/{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ui_state_discards_navigation_account_logout_and_search_results() {
        let mut state = ViewState::default();
        assert!(state.accepts(0, 0));
        state.busy = true;
        state.navigate();
        assert!(!state.busy);
        assert!(!state.accepts(0, 0));
        assert!(state.accepts(0, 1));
        state.account_changed();
        assert!(!state.accepts(0, 1));
        assert!(state.accepts(1, 2));
    }
    #[test]
    fn ui_formatting_preserves_units_and_unknown_statuses() {
        assert_eq!(eta(Some(3661)), "1:01:01");
        assert_eq!(eta(None), "Unknown");
        assert!(speed(1024).ends_with("/s"));
        assert!(request_status(RequestStatus(99)).contains("99"));
        assert_eq!(media_status(MediaStatus::BLOCKLISTED), "Blocklisted");
        assert!(error_text(Error::Http(503)).contains("503"));
        assert!(error_text(Error::Timeout).contains("may have succeeded"));
    }
    #[test]
    fn availability_distinguishes_standard_and_4k_copies() {
        assert_eq!(
            media_availability(MediaStatus::AVAILABLE, Some(MediaStatus::PROCESSING)),
            "Standard: Available · 4K: Processing"
        );
        assert_eq!(
            media_availability(MediaStatus::AVAILABLE, None),
            "Standard: Available · 4K: Unknown"
        );
    }
    #[test]
    fn ui_poster_urls_cannot_redirect_to_upstream_urls() {
        assert_eq!(
            poster_url("/cover.jpg").as_deref(),
            Some("https://image.tmdb.org/t/p/w185/cover.jpg")
        );
        for path in [
            "https://evil/cover.jpg",
            "//evil/cover.jpg",
            "/../secret",
            "/a?token=x",
        ] {
            assert!(poster_url(path).is_none());
        }
    }
}
