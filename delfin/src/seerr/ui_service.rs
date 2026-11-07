//! One in-memory Seerr client per hub, owned exclusively by this serial task.
use super::*;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, Arc,
};
use tokio::sync::mpsc as async_mpsc;

pub(super) enum RemovalCommand {
    Options(RemovalTarget),
    Preview(RemovalSelection),
    Commit(RemovalToken),
    Receipt(String),
    History,
    Retry(String),
    CancelDownload(String, bool),
}
pub(super) enum RemovalData {
    Options(RemovalOptions),
    Preview(RemovalPreview),
    Receipt(RemovalReceipt),
    History(RemovalHistory),
    DownloadCancelled(bool),
}
pub(super) enum Operation {
    Login(ConnectionConfig, String),
    Restore(ConnectionConfig),
    Logout,
    Discover(DiscoverFeed, MediaType, String, u32),
    Details(MediaType, u64, Option<u64>, bool),
    Request(RequestSelection),
    Requests(u32, bool),
    Diagnostics(u64, bool, Option<u64>),
    Search(u64, SourceSelection),
    Grab(u64, SourceSelection, ReleaseToken, bool, bool),
    Torrents,
    Artwork(Vec<(u64, String)>),
    Removal(u64, RemovalCommand),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CredentialStatus {
    Saved,
    MemoryOnly,
    SaveFailed,
    ClearFailed,
}
pub(super) enum Data {
    Login(User, CredentialStatus),
    RestoreUnavailable(bool),
    Logout(Result<(), Error>, bool),
    Discover(Page<DiscoveryResult>),
    Movie(MovieDetails, Option<u64>, bool),
    Tv(TvDetails, Option<u64>, bool),
    Requested(MediaRequest),
    Requests(RequestPage),
    Diagnostics(u64, Option<u64>, TvDownloadDiagnostics),
    Releases(u64, SourceSelection, TvDownloadSearch),
    Grabbed(GrabAccepted),
    Torrents(TorrentDownloads),
    Poster(u64, Vec<u8>),
    Artwork(Vec<(u64, String)>),
    Removal(u64, Result<RemovalData, Error>),
}
pub(super) struct Job {
    pub account: u64,
    pub view: u64,
    pub operation: Operation,
}
pub(super) struct Reply {
    pub auth: bool,
    pub account: u64,
    pub view: u64,
    pub result: Result<Data, Error>,
}
pub(super) struct Service {
    pub sender: async_mpsc::UnboundedSender<Job>,
    pub account: Arc<AtomicU64>,
    pub view: Arc<AtomicU64>,
    task: tokio::task::JoinHandle<()>,
}
impl Service {
    pub fn new() -> (Self, mpsc::Receiver<Reply>) {
        Self::with_keyring(Arc::new(super::keyring::SystemKeyring))
    }
    pub(super) fn with_keyring(
        keyring: Arc<dyn super::keyring::CredentialStore>,
    ) -> (Self, mpsc::Receiver<Reply>) {
        let (sender, mut jobs) = async_mpsc::unbounded_channel::<Job>();
        let (replies, receiver) = mpsc::channel();
        let account = Arc::new(AtomicU64::new(0));
        let view = Arc::new(AtomicU64::new(0));
        let current_account = account.clone();
        let current_view = view.clone();
        let task = relm4::spawn(async move {
            let mut client: Option<SeerrClient> = None;
            let mut connection: Option<ConnectionConfig> = None;
            while let Some(job) = jobs.recv().await {
                if job.account != current_account.load(Ordering::Relaxed) {
                    continue;
                }
                let auth = matches!(
                    job.operation,
                    Operation::Login(..) | Operation::Restore(..) | Operation::Logout
                );
                let removal = matches!(&job.operation, Operation::Removal(..));
                if !auth && !removal && job.view != current_view.load(Ordering::Relaxed) {
                    continue;
                }
                let result = match job.operation {
                    Operation::Login(config, password) => {
                        client = None;
                        connection = Some(config.clone());
                        match authenticate(&config, &password).await {
                            Ok((mut next, user)) => {
                                if job.account != current_account.load(Ordering::Relaxed) {
                                    next.forget_session();
                                    continue;
                                }
                                let persistence = if config.remember_password {
                                    if super::keyring::save(
                                        keyring.clone(),
                                        config.clone(),
                                        password,
                                        current_account.clone(),
                                        job.account,
                                    )
                                    .await
                                    .is_ok()
                                    {
                                        CredentialStatus::Saved
                                    } else {
                                        CredentialStatus::SaveFailed
                                    }
                                } else if super::keyring::clear(keyring.clone(), config.clone())
                                    .await
                                    .is_ok()
                                {
                                    CredentialStatus::MemoryOnly
                                } else {
                                    CredentialStatus::ClearFailed
                                };
                                if job.account != current_account.load(Ordering::Relaxed) {
                                    // A cancelled login must not leave a newly saved password.
                                    if persistence == CredentialStatus::Saved {
                                        let _ =
                                            super::keyring::clear(keyring.clone(), config).await;
                                    }
                                    next.forget_session();
                                    continue;
                                }
                                client = Some(next);
                                Ok(Data::Login(user, persistence))
                            }
                            Err(error) => Err(error),
                        }
                    }
                    Operation::Restore(config) => {
                        client = None;
                        connection = Some(config.clone());
                        let secret = if config.remember_password {
                            super::keyring::lookup(keyring.clone(), config.clone()).await
                        } else {
                            Ok(None)
                        };
                        if job.account != current_account.load(Ordering::Relaxed) {
                            continue;
                        }
                        match secret {
                            Ok(Some(password)) => match authenticate(&config, &password).await {
                                Ok((mut next, user)) => {
                                    if job.account != current_account.load(Ordering::Relaxed) {
                                        next.forget_session();
                                        continue;
                                    }
                                    client = Some(next);
                                    Ok(Data::Login(user, CredentialStatus::Saved))
                                }
                                Err(error) => {
                                    if matches!(error, Error::Unauthorized | Error::Forbidden) {
                                        let _ =
                                            super::keyring::clear(keyring.clone(), config).await;
                                    }
                                    Err(error)
                                }
                            },
                            Ok(None) => Ok(Data::RestoreUnavailable(false)),
                            Err(()) => Ok(Data::RestoreUnavailable(true)),
                        }
                    }
                    Operation::Logout => {
                        // Forget saved credentials even if the remote logout fails.
                        let clear_failed = if let Some(config) = connection.take() {
                            super::keyring::clear(keyring.clone(), config)
                                .await
                                .is_err()
                        } else {
                            false
                        };
                        let result = if let Some(client) = client.as_mut() {
                            client.logout().await
                        } else {
                            Ok(())
                        };
                        client = None;
                        Ok(Data::Logout(result, clear_failed))
                    }
                    Operation::Removal(id, command) => {
                        let result = match client.as_mut() {
                            Some(client) => execute_removal(client, command).await,
                            None => Err(Error::NotAuthenticated),
                        };
                        Ok(Data::Removal(id, result))
                    }
                    operation => match client.as_mut() {
                        None => Err(Error::NotAuthenticated),
                        Some(client) => execute(client, operation).await,
                    },
                };
                let paths: Vec<_> = match &result {
                    Ok(Data::Movie(details, ..)) => details
                        .poster_path
                        .as_deref()
                        .and_then(super::ui_state::poster_url)
                        .map(|url| vec![(u64::MAX, url)])
                        .unwrap_or_default(),
                    Ok(Data::Tv(details, ..)) => details
                        .poster_path
                        .as_deref()
                        .and_then(super::ui_state::poster_url)
                        .map(|url| vec![(u64::MAX, url)])
                        .unwrap_or_default(),
                    Ok(Data::Artwork(paths)) => paths
                        .iter()
                        .filter_map(|(id, path)| Some((*id, super::ui_state::poster_url(path)?)))
                        .collect(),
                    Ok(Data::Requests(page)) => page
                        .results
                        .iter()
                        .filter_map(|item| {
                            Some((
                                item.id,
                                super::ui_state::poster_url(item.poster_path.as_deref()?)?,
                            ))
                        })
                        .collect(),
                    Ok(Data::Discover(page)) => page
                        .results
                        .iter()
                        .enumerate()
                        .filter_map(|(index, item)| {
                            Some((
                                index as u64,
                                super::ui_state::poster_url(
                                    item.poster_path
                                        .as_deref()
                                        .or(item.profile_path.as_deref())?,
                                )?,
                            ))
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                if !paths.is_empty() {
                    let replies = replies.clone();
                    let epoch = current_account.clone();
                    let view = current_view.clone();
                    // Public fixed-origin artwork, entirely separate from authenticated requests.
                    relm4::spawn(async move {
                        let Ok(images) = reqwest::Client::builder()
                            .redirect(reqwest::redirect::Policy::none())
                            .timeout(std::time::Duration::from_secs(10))
                            .build()
                        else {
                            return;
                        };
                        for (id, url) in paths {
                            if epoch.load(Ordering::Relaxed) != job.account
                                || view.load(Ordering::Relaxed) != job.view
                            {
                                break;
                            }
                            if let Ok(mut response) = images.get(url).send().await {
                                if !response.status().is_success()
                                    || response
                                        .content_length()
                                        .is_some_and(|size| size > 2_000_000)
                                {
                                    continue;
                                }
                                let mut bytes = Vec::new();
                                let complete = loop {
                                    match response.chunk().await {
                                        Ok(Some(chunk))
                                            if bytes.len() + chunk.len() <= 2_000_000 =>
                                        {
                                            bytes.extend_from_slice(&chunk)
                                        }
                                        Ok(None) => break true,
                                        _ => break false,
                                    }
                                };
                                if complete {
                                    let _ = replies.send(Reply {
                                        account: job.account,
                                        view: job.view,
                                        auth: false,
                                        result: Ok(Data::Poster(id, bytes)),
                                    });
                                }
                            }
                        }
                    });
                }
                if replies
                    .send(Reply {
                        account: job.account,
                        view: job.view,
                        auth,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        (
            Self {
                sender,
                account,
                view,
                task,
            },
            receiver,
        )
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        self.account.fetch_add(1, Ordering::Relaxed);
        self.task.abort();
    }
}
async fn authenticate(
    config: &ConnectionConfig,
    password: &str,
) -> Result<(SeerrClient, User), Error> {
    let mut next = SeerrClient::new(config)?;
    let username = config.username.as_deref().unwrap_or("");
    let user = match config.login_method {
        LoginMethod::Local => next.login_local(username, password).await?,
        _ => next.login_jellyfin(username, password).await?,
    };
    Ok((next, user))
}
async fn execute_removal(
    client: &mut SeerrClient,
    command: RemovalCommand,
) -> Result<RemovalData, Error> {
    match command {
        RemovalCommand::Options(target) => client
            .removal_options(&target)
            .await
            .map(RemovalData::Options),
        RemovalCommand::Preview(selection) => client
            .removal_preview(&selection)
            .await
            .map(RemovalData::Preview),
        RemovalCommand::Commit(token) => client
            .removal_commit(&token)
            .await
            .map(RemovalData::Receipt),
        RemovalCommand::Receipt(id) => client.removal_receipt(&id).await.map(RemovalData::Receipt),
        RemovalCommand::History => client.removal_history().await.map(RemovalData::History),
        RemovalCommand::Retry(id) => client.removal_retry(&id).await.map(RemovalData::Receipt),
        RemovalCommand::CancelDownload(hash, delete_files) => client
            .cancel_download(&hash, delete_files)
            .await
            .map(|()| RemovalData::DownloadCancelled(delete_files)),
    }
}
async fn execute(client: &mut SeerrClient, operation: Operation) -> Result<Data, Error> {
    match operation {
        Operation::Discover(feed, kind, query, page) => client
            .discover_filtered(feed, kind, &query, page)
            .await
            .map(Data::Discover),
        Operation::Details(MediaType::Movie, id, request_id, is4k) => client
            .movie_details(id)
            .await
            .map(|d| Data::Movie(d, request_id, is4k)),
        Operation::Details(MediaType::Tv, id, request_id, is4k) => client
            .tv_details(id)
            .await
            .map(|d| Data::Tv(d, request_id, is4k)),
        Operation::Request(selection) => {
            client.create_request(&selection).await.map(Data::Requested)
        }
        Operation::Requests(skip, finished) => client
            .requests(&RequestListOptions {
                skip,
                filter: if finished {
                    RequestFilter::Finished
                } else {
                    RequestFilter::Unfinished
                },
                ..Default::default()
            })
            .await
            .map(Data::Requests),
        Operation::Diagnostics(id, is4k, request_id) => client
            .tv_downloads(id, is4k, request_id)
            .await
            .map(|d| Data::Diagnostics(id, request_id, d)),
        Operation::Search(id, selection) => {
            let releases = match &selection {
                SourceSelection::Tv(value) => client.search_tv_releases(id, value).await,
                SourceSelection::Movie(value) => client.search_movie_releases(id, value).await,
            }?;
            Ok(Data::Releases(id, selection, releases))
        }
        Operation::Grab(id, selection, token, rejected, duplicate) => {
            let result = match &selection {
                SourceSelection::Tv(value) => {
                    client
                        .grab_tv_release(id, value, &token, rejected, duplicate, true)
                        .await
                }
                SourceSelection::Movie(value) => {
                    client
                        .grab_movie_release(id, value, &token, rejected, duplicate, true)
                        .await
                }
            }?;
            Ok(Data::Grabbed(result))
        }
        Operation::Torrents => client.torrent_downloads().await.map(Data::Torrents),
        Operation::Artwork(paths) => Ok(Data::Artwork(paths)),
        Operation::Login(..)
        | Operation::Restore(..)
        | Operation::Logout
        | Operation::Removal(..) => {
            unreachable!("authentication handled by service")
        }
    }
}
