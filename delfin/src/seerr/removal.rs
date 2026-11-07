//! Account-bound removal API. Preview tokens are ephemeral capabilities, not logs/config.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub(crate) enum RemovalTarget {
    Jellyfin {
        #[serde(rename = "jellyfinItemId")]
        jellyfin_item_id: String,
        #[serde(rename = "jellyfinServerId")]
        jellyfin_server_id: String,
    },
    Torrent {
        #[serde(rename = "torrentHash")]
        torrent_hash: String,
    },
    Media {
        #[serde(rename = "mediaType")]
        media_type: MediaType,
        #[serde(rename = "tmdbId")]
        tmdb_id: u64,
    },
}
impl RemovalTarget {
    fn valid(&self) -> bool {
        match self {
            Self::Jellyfin {
                jellyfin_item_id: id,
                jellyfin_server_id,
            } => {
                let plain = id.replace('-', "");
                plain.len() == 32 && plain.bytes().all(|b| b.is_ascii_hexdigit()) && {
                    let plain = jellyfin_server_id.replace('-', "");
                    plain.len() == 32 && plain.bytes().all(|b| b.is_ascii_hexdigit())
                }
            }
            Self::Torrent { torrent_hash: hash } => {
                (hash.len() == 40 || hash.len() == 64)
                    && hash.bytes().all(|b| b.is_ascii_hexdigit())
            }
            Self::Media { tmdb_id, .. } => *tmdb_id > 0 && *tmdb_id <= 9_007_199_254_740_991,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemovalEpisodeKey {
    pub season_number: u32,
    pub episode_number: u32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "scope", rename_all = "lowercase")]
pub(crate) enum RemovalScope {
    All,
    Seasons { seasons: Vec<u32> },
    Episodes { episodes: Vec<RemovalEpisodeKey> },
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct RemovalSelection {
    pub target: RemovalTarget,
    #[serde(flatten)]
    pub scope: RemovalScope,
}
impl RemovalSelection {
    fn valid(&self) -> bool {
        if !self.target.valid() {
            return false;
        }
        if matches!(
            self.target,
            RemovalTarget::Media {
                media_type: MediaType::Movie,
                ..
            }
        ) && !matches!(self.scope, RemovalScope::All)
        {
            return false;
        }
        match &self.scope {
            RemovalScope::All => true,
            RemovalScope::Seasons { seasons } => {
                !seasons.is_empty()
                    && seasons.len() <= 100
                    && seasons
                        .iter()
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                        == seasons.len()
            }
            RemovalScope::Episodes { episodes } => {
                !episodes.is_empty()
                    && episodes.len() <= 1000
                    && episodes.iter().all(|e| e.episode_number > 0)
                    && episodes
                        .iter()
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                        == episodes.len()
            }
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedRemovalTarget {
    pub media_type: MediaType,
    pub tmdb_id: u64,
    pub title: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemovalEpisode {
    pub season_number: u32,
    pub episode_number: u32,
    pub title: String,
    pub has_file: bool,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemovalOptions {
    pub target: ResolvedRemovalTarget,
    #[serde(default)]
    pub episodes: Vec<RemovalEpisode>,
    pub initially_selected_episode: Option<RemovalEpisodeKey>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub(crate) struct RemovalToken(String);
impl fmt::Debug for RemovalToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted removal capability]")
    }
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemovalPreview {
    pub token: Option<RemovalToken>,
    pub title: String,
    #[serde(default)]
    pub effects: Vec<String>,
    pub can_commit: bool,
    #[serde(default)]
    pub blockers: Vec<String>,
}
impl RemovalPreview {
    pub fn confirmable(&self) -> bool {
        self.can_commit
            && self.blockers.is_empty()
            && !self.effects.is_empty()
            && !self.title.trim().is_empty()
            && self
                .token
                .as_ref()
                .is_some_and(|t| !t.0.is_empty() && t.0.len() <= 512)
    }
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RemovalStatus {
    Complete,
    Partial,
    Failed,
    Running,
    Pending,
    #[serde(other)]
    Unknown,
}
#[derive(Debug, Deserialize)]
pub(crate) struct RemovalStep {
    pub name: String,
    pub status: RemovalStatus,
    pub message: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemovalReceipt {
    pub operation_id: String,
    #[serde(default)]
    pub kind: Option<String>,
    pub title: Option<String>,
    pub status: RemovalStatus,
    #[serde(default)]
    pub physical_complete: bool,
    #[serde(default)]
    pub steps: Vec<RemovalStep>,
}
#[derive(Debug, Deserialize)]
pub(crate) struct RemovalHistory {
    pub operations: Vec<RemovalReceipt>,
}
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
impl SeerrClient {
    pub(crate) async fn cancel_download(
        &mut self,
        hash: &str,
        delete_files: bool,
    ) -> Result<(), Error> {
        if !(RemovalTarget::Torrent {
            torrent_hash: hash.to_string(),
        })
        .valid()
        {
            return Err(Error::InvalidSelection);
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Cancelled {
            cancelled: bool,
            files_deleted: bool,
        }
        let request = self
            .request(
                Method::POST,
                &format!("downloads/torrents/{hash}/cancel"),
                true,
            )?
            .json(&serde_json::json!({"confirmed": true, "deleteFiles": delete_files}));
        let result: Cancelled = self.json(request).await?;
        if !result.cancelled || result.files_deleted != delete_files {
            return Err(Error::InvalidResponse);
        }
        Ok(())
    }
    pub(crate) async fn removal_options(
        &mut self,
        target: &RemovalTarget,
    ) -> Result<RemovalOptions, Error> {
        if !target.valid() {
            return Err(Error::InvalidSelection);
        }
        let request = self
            .request(Method::POST, "removals/options", true)?
            .json(target);
        self.json(request).await
    }
    pub(crate) async fn removal_preview(
        &mut self,
        selection: &RemovalSelection,
    ) -> Result<RemovalPreview, Error> {
        if !selection.valid() {
            return Err(Error::InvalidSelection);
        }
        let request = self
            .request(Method::POST, "removals/preview", true)?
            .json(selection)
            .timeout(Duration::from_secs(120));
        self.json(request).await
    }
    pub(crate) async fn removal_commit(
        &mut self,
        token: &RemovalToken,
    ) -> Result<RemovalReceipt, Error> {
        if token.0.is_empty() || token.0.len() > 512 {
            return Err(Error::InvalidSelection);
        }
        let request = self
            .request(Method::POST, "removals/commit", true)?
            .json(&serde_json::json!({"token":token,"confirmed":true}))
            .timeout(Duration::from_secs(120));
        self.json(request).await
    }
    pub(crate) async fn removal_receipt(&mut self, id: &str) -> Result<RemovalReceipt, Error> {
        if !safe_id(id) {
            return Err(Error::InvalidSelection);
        }
        let request = self.request(Method::GET, &format!("removals/{id}"), true)?;
        self.json(request).await
    }
    pub(crate) async fn removal_history(&mut self) -> Result<RemovalHistory, Error> {
        let request = self.request(Method::GET, "removals", true)?;
        self.json(request).await
    }
    pub(crate) async fn removal_retry(&mut self, id: &str) -> Result<RemovalReceipt, Error> {
        if !safe_id(id) {
            return Err(Error::InvalidSelection);
        }
        let request = self
            .request(Method::POST, &format!("removals/{id}/retry"), true)?
            .json(&serde_json::json!({"confirmed":true}))
            .timeout(Duration::from_secs(120));
        self.json(request).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selections_reject_ambiguous_empty_or_duplicate_scopes() {
        let target = RemovalTarget::Media {
            media_type: MediaType::Tv,
            tmdb_id: 1,
        };
        for scope in [
            RemovalScope::Seasons { seasons: vec![] },
            RemovalScope::Seasons {
                seasons: vec![1, 1],
            },
            RemovalScope::Episodes {
                episodes: vec![RemovalEpisodeKey {
                    season_number: 1,
                    episode_number: 0,
                }],
            },
        ] {
            assert!(!RemovalSelection {
                target: target.clone(),
                scope
            }
            .valid());
        }
        assert!(RemovalSelection {
            target,
            scope: RemovalScope::Seasons {
                seasons: vec![0, 1]
            }
        }
        .valid());
        assert!(!RemovalTarget::Jellyfin {
            jellyfin_item_id: "../../etc".into(),
            jellyfin_server_id: "b".repeat(32),
        }
        .valid());
        assert!(!RemovalTarget::Torrent {
            torrent_hash: "all".into()
        }
        .valid());
        assert!(!RemovalTarget::Jellyfin {
            jellyfin_item_id: "a".repeat(32),
            jellyfin_server_id: String::new()
        }
        .valid());
        assert!(!safe_id("../commit"));
        assert!(!safe_id("x?token=secret"));
    }
    #[test]
    fn preview_cannot_enable_confirmation_without_explicit_safe_effects() {
        let mut p:RemovalPreview=serde_json::from_value(serde_json::json!({"title":"Movie","scopeLabel":"All versions","expiresAt":"2099-01-01T00:00:00Z","canCommit":true,"effects":["Delete selected files"],"warnings":[],"blockers":[]})).unwrap();
        assert!(!p.confirmable());
        p.token = Some(RemovalToken("secret-plan".into()));
        assert!(p.confirmable());
        assert!(!format!("{p:?}").contains("secret-plan"));
        p.blockers.push("Shared file".into());
        assert!(!p.confirmable());
    }
}
