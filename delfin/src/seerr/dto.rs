//! Seerr wire DTOs. Unknown response fields (including upstream credentials/URLs)
//! are intentionally discarded rather than retained in an untyped JSON map.
use serde::{Deserialize, Serialize};
use std::fmt;

/// An in-memory, user/selection-bound release capability. Never persist or log it.
#[derive(Clone, Deserialize)]
#[serde(transparent)]
pub struct ReleaseToken(pub(super) String);
impl fmt::Debug for ReleaseToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

/// A Quick Connect capability distinct from release tokens; never persist or log it.
#[derive(Deserialize)]
#[serde(transparent)]
pub struct QuickConnectSecret(pub(super) String);
impl fmt::Debug for QuickConnectSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: u64,
    pub email: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub jellyfin_username: Option<String>,
    #[serde(default)]
    pub permissions: u64,
}
impl User {
    /// Advisory UI gating only; Seerr remains the permission authority.
    pub fn can_manage_requests(&self) -> bool {
        self.permissions & (2 | 16) != 0
    }
}

/// Numeric newtypes preserve unknown future server statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct RequestStatus(pub u32);
impl RequestStatus {
    pub const PENDING: Self = Self(1);
    pub const APPROVED: Self = Self(2);
    pub const DECLINED: Self = Self(3);
    pub const FAILED: Self = Self(4);
    pub const COMPLETED: Self = Self(5);
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct MediaStatus(pub u32);
impl MediaStatus {
    pub const UNKNOWN: Self = Self(1);
    pub const PENDING: Self = Self(2);
    pub const PROCESSING: Self = Self(3);
    pub const PARTIALLY_AVAILABLE: Self = Self(4);
    pub const AVAILABLE: Self = Self(5);
    pub const BLOCKLISTED: Self = Self(6);
    pub const DELETED: Self = Self(7);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Movie,
    Tv,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub id: u64,
    pub tmdb_id: u64,
    pub tvdb_id: Option<u64>,
    pub media_type: Option<MediaType>,
    pub status: MediaStatus,
    pub status4k: Option<MediaStatus>,
    #[serde(default)]
    pub requests: Vec<MediaRequest>,
    #[serde(default)]
    pub seasons: Vec<MediaSeason>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSeason {
    pub season_number: u32,
    pub status: MediaStatus,
    pub status4k: Option<MediaStatus>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaRequest {
    pub title: Option<String>,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
    pub release_date: Option<String>,
    pub profile_name: Option<String>,
    pub id: u64,
    pub status: RequestStatus,
    pub media: Option<Box<MediaInfo>>,
    pub requested_by: Option<User>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub is4k: bool,
    #[serde(default)]
    pub seasons: Vec<RequestSeason>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestSeason {
    pub season_number: u32,
    pub status: RequestStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub page: u32,
    pub total_pages: u32,
    pub total_results: u64,
    pub results: Vec<T>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestPage {
    pub page_info: PageInfo,
    pub results: Vec<MediaRequest>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub pages: u32,
    pub page_size: u32,
    pub results: u64,
    pub page: u32,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResultType {
    Movie,
    Tv,
    Person,
    #[serde(other)]
    Other,
}
impl ResultType {
    pub(super) fn matches(self, kind: MediaType) -> bool {
        matches!(
            (self, kind),
            (Self::Movie, MediaType::Movie) | (Self::Tv, MediaType::Tv)
        )
    }
}
/// Shared movie/TV/person search card; people have no requestable MediaType.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    pub id: u64,
    pub media_type: ResultType,
    pub title: Option<String>,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub profile_path: Option<String>,
    pub release_date: Option<String>,
    pub first_air_date: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    pub genre_ids: Option<Vec<u64>>,
    pub original_language: Option<String>,
    pub media_info: Option<MediaInfo>,
}
#[derive(Debug, Deserialize)]
pub struct Genre {
    pub id: u64,
    pub name: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovieDetails {
    #[serde(flatten)]
    pub metadata: DetailMetadata,
    pub original_title: Option<String>,
    pub budget: Option<u64>,
    pub revenue: Option<u64>,
    pub imdb_id: Option<String>,
    pub id: u64,
    pub title: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<u32>,
    pub vote_average: Option<f64>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    pub media_info: Option<MediaInfo>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TvDetails {
    #[serde(flatten)]
    pub metadata: DetailMetadata,
    pub original_name: Option<String>,
    pub last_air_date: Option<String>,
    pub number_of_seasons: Option<u32>,
    pub number_of_episodes: Option<u32>,
    #[serde(default)]
    pub episode_run_time: Vec<u32>,
    #[serde(default)]
    pub networks: Vec<NamedDetail>,
    #[serde(default)]
    pub created_by: Vec<NamedDetail>,
    pub id: u64,
    pub name: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub first_air_date: Option<String>,
    pub vote_average: Option<f64>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    #[serde(default)]
    pub seasons: Vec<Season>,
    pub media_info: Option<MediaInfo>,
}
/// Optional metadata supplied by Seerr's existing detail response; no extra
/// per-card calls, arbitrary external URLs or service credentials are retained.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DetailMetadata {
    pub tagline: Option<String>,
    pub status: Option<String>,
    pub vote_count: Option<u64>,
    pub original_language: Option<String>,
    pub production_companies: Vec<NamedDetail>,
    pub production_countries: Vec<NamedDetail>,
    pub spoken_languages: Vec<NamedDetail>,
    pub credits: Option<Credits>,
    pub external_ids: Option<ExternalIds>,
    pub related_videos: Vec<DetailVideo>,
}
#[derive(Debug, Deserialize)]
pub struct NamedDetail {
    pub name: String,
}
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Credits {
    pub cast: Vec<CastMember>,
    pub crew: Vec<CrewMember>,
}
#[derive(Debug, Deserialize)]
pub struct CastMember {
    pub name: String,
    pub character: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct CrewMember {
    pub name: String,
    pub job: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalIds {
    pub imdb_id: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct DetailVideo {
    pub site: String,
    pub key: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub id: u64,
    pub season_number: u32,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub episode_count: Option<u32>,
    pub poster_path: Option<String>,
    #[serde(default)]
    pub episodes: Vec<Episode>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub id: u64,
    pub season_number: u32,
    pub episode_number: u32,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub still_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverFeed {
    Trending,
    PopularMovies,
    PopularTv,
    UpcomingMovies,
    UpcomingTv,
}
impl DiscoverFeed {
    pub(super) fn for_selection(selection: u32, kind: MediaType) -> Self {
        match (selection, kind) {
            (1, MediaType::Movie) => Self::PopularMovies,
            (1, MediaType::Tv) => Self::PopularTv,
            (2, MediaType::Movie) => Self::UpcomingMovies,
            (2, MediaType::Tv) => Self::UpcomingTv,
            _ => Self::Trending,
        }
    }
    pub(super) fn path(self) -> &'static str {
        match self {
            Self::Trending => "discover/trending",
            Self::PopularMovies => "discover/movies",
            Self::PopularTv => "discover/tv",
            Self::UpcomingMovies => "discover/movies/upcoming",
            Self::UpcomingTv => "discover/tv/upcoming",
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestFilter {
    Finished,
    Unfinished,
    All,
    Approved,
    Available,
    Pending,
    Processing,
    Unavailable,
    Failed,
    Deleted,
    Completed,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestListOptions {
    pub take: u32,
    pub skip: u32,
    pub filter: RequestFilter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<u64>,
}
impl Default for RequestListOptions {
    fn default() -> Self {
        Self {
            take: 20,
            skip: 0,
            filter: RequestFilter::All,
            requested_by: None,
        }
    }
}

/// Explicit selection only: no implicit "all seasons" or privileged routing overrides.
#[derive(Debug, Clone)]
pub enum RequestSelection {
    Movie {
        tmdb_id: u64,
        is4k: bool,
    },
    Tv {
        tmdb_id: u64,
        seasons: Vec<u32>,
        is4k: bool,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSelection {
    pub is4k: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<u64>,
    pub season_number: u32,
    /// Sonarr episode ID from diagnostics, NOT a TMDB episode ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<u64>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovieDownloadSelection {
    pub is4k: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<u64>,
}
#[derive(Debug, Clone)]
pub enum SourceSelection {
    Tv(DownloadSelection),
    Movie(MovieDownloadSelection),
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TvDownloadDiagnostics {
    pub server_name: String,
    pub is4k: bool,
    pub series_monitored: bool,
    pub queue_truncated: bool,
    pub history_truncated: bool,
    pub episodes: Vec<TvDownloadEpisode>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TvDownloadEpisode {
    pub id: u64,
    pub season_number: u32,
    pub episode_number: u32,
    pub title: String,
    pub has_file: bool,
    pub monitored: bool,
    pub air_date_utc: Option<String>,
    pub state: EpisodeDownloadState,
    pub queue: Vec<EpisodeQueueItem>,
    pub history: Vec<EpisodeHistoryItem>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EpisodeDownloadState {
    Imported,
    Queued,
    Unmonitored,
    Unaired,
    Missing,
    #[serde(other)]
    Other,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeQueueItem {
    pub title: String,
    pub status: String,
    pub tracked_status: String,
    pub tracked_state: String,
    /// Percentage 0..100 (unlike torrent progress).
    pub progress: Option<f64>,
    pub messages: Vec<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeHistoryItem {
    pub date: String,
    pub event_type: String,
    pub title: String,
    pub message: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TvDownloadRelease {
    /// Five-minute, user/selection-bound single-use capability. None means not grabbable.
    pub token: Option<ReleaseToken>,
    pub title: String,
    pub indexer: String,
    pub quality: String,
    pub size: u64,
    /// Indexers may use a negative sentinel for unknown counts, or null.
    pub seeders: Option<i64>,
    pub protocol: String,
    pub full_season: bool,
    pub episode_numbers: Vec<u32>,
    pub rejections: Vec<String>,
    pub rejected: bool,
    pub duplicate: bool,
    #[serde(default)]
    pub replacements: Vec<String>,
    pub replacement_blocked: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TvDownloadSearch {
    pub releases: Vec<TvDownloadRelease>,
    pub expires_at: String,
    pub truncated: bool,
}
#[derive(Debug, Deserialize)]
pub struct GrabAccepted {
    pub message: String,
    #[serde(default)]
    pub partial: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentDownloads {
    pub torrents: Vec<Torrent>,
    pub updated_at: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Torrent {
    pub hash: String,
    pub name: String,
    pub state: String,
    /// Fraction 0..1. Sizes are bytes; speeds are bytes/second.
    pub progress: f64,
    pub size: u64,
    pub downloaded: u64,
    pub download_speed: u64,
    pub upload_speed: u64,
    /// Seconds; None means unknown.
    pub eta: Option<u64>,
    pub seeds: u64,
    pub peers: u64,
    pub category: String,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct QuickConnect {
    pub code: String,
    pub secret: QuickConnectSecret,
}
impl fmt::Debug for QuickConnect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("QuickConnect { [redacted] }")
    }
}
#[derive(Debug, Deserialize)]
pub struct QuickConnectStatus {
    pub authenticated: bool,
}
