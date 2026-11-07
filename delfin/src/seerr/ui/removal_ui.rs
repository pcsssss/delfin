//! One confirmation; server-side preflight and removal run behind the scenes.
use super::super::ui_service::{RemovalCommand, RemovalData};
use super::*;
use gtk::gdk;
use std::collections::BTreeSet;

pub(super) struct Wizard {
    pub window: gtk::Window,
    body: gtk::Box,
    actions: gtk::Box,
    loading: gtk::Box,
    status: gtk::Label,
    spinner: gtk::Spinner,
    target: Option<RemovalTarget>,
    request_id: u64,
    account: u64,
    busy: bool,
    committing: bool,
    confirmed: bool,
    history: bool,
    download: bool,
    delete_download_files: bool,
}
fn current(hub: &Shared) -> Option<Rc<RefCell<Wizard>>> {
    hub.borrow().removal.clone()
}
fn toast(text: String, timeout: u32) {
    crate::app::APP_BROKER.send(crate::app::AppInput::Toast(text, Some(timeout)));
}
fn close(hub: &Shared) {
    let wizard = hub.borrow_mut().removal.take();
    if let Some(wizard) = wizard {
        wizard.borrow().window.close();
    }
}
pub(super) fn build(hub: &Shared, target: Option<RemovalTarget>) -> Rc<RefCell<Wizard>> {
    let history = target.is_none();
    let title_key = if history {
        "seerr-removal-history"
    } else {
        "seerr-removal-delete"
    };
    let window = gtk::Window::builder()
        .title(tr!(title_key))
        .default_width(if history { 640 } else { 480 })
        .resizable(history)
        .modal(true)
        .build();
    if let Some(parent) = get_main_window() {
        window.set_transient_for(Some(&parent));
    }
    let header = adw::HeaderBar::new();
    window.set_titlebar(Some(&header));
    let layout = column(16);
    layout.set_margin_all(20);
    let state_row = row(8);
    let spinner = gtk::Spinner::new();
    let status = label(tr!("seerr-loading"));
    status.set_selectable(false);
    status.set_hexpand(true);
    state_row.append(&spinner);
    state_row.append(&status);
    layout.append(&state_row);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroll.set_propagate_natural_height(true);
    scroll.set_max_content_height(360);
    let body = column(12);
    scroll.set_child(Some(&body));
    layout.append(&scroll);
    let actions = row(12);
    actions.set_halign(gtk::Align::End);
    layout.append(&actions);
    window.set_child(Some(&layout));
    let wizard = Rc::new(RefCell::new(Wizard {
        window: window.clone(),
        body,
        actions,
        loading: state_row,
        status,
        spinner,
        target,
        request_id: 0,
        account: hub.borrow().state.account,
        busy: false,
        committing: false,
        confirmed: false,
        history,
        download: false,
        delete_download_files: false,
    }));
    hub.borrow_mut().removal = Some(wizard.clone());
    let weak = Rc::downgrade(hub);
    window.connect_close_request(move |window| {
        if let Some(hub) = weak.upgrade() {
            if let Some(wizard) = current(&hub) {
                if wizard.borrow().window == *window {
                    // A dismissed confirmation must not discard an accepted job.
                    if wizard.borrow().confirmed || wizard.borrow().committing {
                        window.set_visible(false);
                        return glib::Propagation::Stop;
                    }
                    hub.borrow_mut().removal = None;
                }
            }
        }
        glib::Propagation::Proceed
    });
    let keys = gtk::EventControllerKey::new();
    let weak = window.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            if let Some(window) = weak.upgrade() {
                window.close();
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(keys);
    window.present();
    wizard
}
pub(super) fn open(hub: &Shared, target: Option<RemovalTarget>) {
    if let Some(wizard) = current(hub) {
        let replace_history = target.is_some() && wizard.borrow().history && !wizard.borrow().busy;
        if replace_history {
            close(hub);
        } else {
            if wizard.borrow().confirmed {
                toast(tr!("seerr-removal-working").to_string(), 4);
            } else {
                wizard.borrow().window.present();
            }
            return;
        }
    }
    if !hub
        .borrow()
        .user
        .as_ref()
        .is_some_and(User::can_manage_requests)
    {
        toast(tr!("seerr-removal-manager-required").to_string(), 8);
        show_account(hub);
        return;
    }
    build(hub, target.clone());
    send_command(
        hub,
        match target {
            Some(target) => RemovalCommand::Options(target),
            None => RemovalCommand::History,
        },
    );
}
pub(super) fn open_download(hub: &Shared, hash: String, name: &str, delete_files: bool) {
    if let Some(wizard) = current(hub) {
        if wizard.borrow().history && !wizard.borrow().busy {
            close(hub);
        } else {
            if wizard.borrow().confirmed {
                toast(tr!("seerr-removal-working").to_string(), 4);
            } else {
                wizard.borrow().window.present();
            }
            return;
        }
    }
    if !hub
        .borrow()
        .user
        .as_ref()
        .is_some_and(User::can_manage_requests)
    {
        toast(tr!("seerr-removal-manager-required").to_string(), 8);
        show_account(hub);
        return;
    }
    let wizard = build(
        hub,
        Some(RemovalTarget::Torrent {
            torrent_hash: hash.clone(),
        }),
    );
    {
        let mut wizard = wizard.borrow_mut();
        wizard.download = true;
        wizard.delete_download_files = delete_files;
    }
    let action = if delete_files {
        "seerr-download-delete"
    } else {
        "seerr-download-cancel"
    };
    wizard.borrow().window.set_title(Some(tr!(action)));
    wizard.borrow().loading.set_visible(false);
    let heading = if delete_files {
        tr!("seerr-download-delete-title", {"title" => name})
    } else {
        tr!("seerr-download-cancel-title", {"title" => name})
    };
    wizard.borrow().body.append(&label(&heading));
    let body = if delete_files {
        "seerr-download-delete-body"
    } else {
        "seerr-download-cancel-body"
    };
    wizard.borrow().body.append(&label(tr!(body)));
    cancel_button(hub);
    let cancel = button(tr!(action));
    cancel.add_css_class("destructive-action");
    on_click(&cancel, hub, move |hub| {
        let Some(wizard) = current(hub) else {
            return;
        };
        {
            let mut wizard = wizard.borrow_mut();
            if wizard.busy || wizard.confirmed || wizard.account != hub.borrow().state.account {
                return;
            }
            wizard.confirmed = true;
            wizard.window.set_visible(false);
        }
        toast(tr!("seerr-download-cancelling").to_string(), 3);
        send_command(
            hub,
            RemovalCommand::CancelDownload(hash.clone(), delete_files),
        );
    });
    wizard.borrow().actions.append(&cancel);
}
fn send_command(hub: &Shared, command: RemovalCommand) {
    let Some(wizard) = current(hub) else {
        return;
    };
    if wizard.borrow().busy || wizard.borrow().account != hub.borrow().state.account {
        return;
    }
    if !hub
        .borrow()
        .user
        .as_ref()
        .is_some_and(User::can_manage_requests)
    {
        show_error(hub, Error::Forbidden);
        return;
    }
    let mutating = matches!(
        &command,
        RemovalCommand::Commit(..) | RemovalCommand::Retry(..) | RemovalCommand::CancelDownload(..)
    );
    // No mutation, including a token received out of order, bypasses confirmation.
    if mutating && !wizard.borrow().confirmed {
        return;
    }
    let (id, account, view) = {
        let mut model = hub.borrow_mut();
        model.removal_sequence = model.removal_sequence.wrapping_add(1);
        (
            model.removal_sequence,
            model.state.account,
            model.state.view,
        )
    };
    let sent = hub.borrow().service.sender.send(Job {
        account,
        view,
        operation: Operation::Removal(id, command),
    });
    if sent.is_err() {
        show_error(hub, Error::ClientSetup);
        return;
    }
    let mut wizard = wizard.borrow_mut();
    wizard.request_id = id;
    wizard.busy = true;
    wizard.committing = mutating;
    wizard.body.set_sensitive(false);
    wizard.actions.set_sensitive(false);
    wizard.loading.set_visible(true);
    wizard.status.set_visible(true);
    wizard.spinner.set_spinning(true);
    wizard.window.set_deletable(!wizard.confirmed);
    let key = if wizard.confirmed && wizard.download {
        "seerr-download-cancelling"
    } else if wizard.confirmed {
        "seerr-removal-working"
    } else {
        "seerr-loading"
    };
    wizard.status.set_label(tr!(key));
}
fn cancel_button(hub: &Shared) {
    let Some(wizard) = current(hub) else {
        return;
    };
    let cancel = button(tr!("seerr-cancel"));
    on_click(&cancel, hub, close);
    wizard.borrow().actions.append(&cancel);
    wizard.borrow().window.set_default_widget(Some(&cancel));
    cancel.grab_focus();
}
fn show_error(hub: &Shared, error: Error) {
    if let Some(delete_files) = current(hub).and_then(|wizard| {
        let wizard = wizard.borrow();
        wizard.download.then_some(wizard.delete_download_files)
    }) {
        close(hub);
        let key = if delete_files && matches!(error, Error::Http(409)) {
            "seerr-download-delete-blocked"
        } else if delete_files {
            "seerr-download-delete-uncertain"
        } else {
            "seerr-download-cancel-uncertain"
        };
        toast(tr!(key).to_string(), 10);
        if hub.borrow().active == Some(2) {
            send(hub, Operation::Torrents);
        }
        return;
    }
    let text = match error {
        Error::Timeout | Error::Transport => tr!("seerr-removal-uncertain").to_string(),
        Error::Http(409 | 410) => tr!("seerr-removal-stale").to_string(),
        Error::Forbidden => tr!("seerr-removal-manager-required").to_string(),
        _ => error_text(error),
    };
    fail(hub, &text);
}
fn fail(hub: &Shared, reason: &str) {
    close(hub);
    toast(
        tr!("seerr-removal-failed-toast", {"reason" => reason}).to_string(),
        10,
    );
}
fn confirm(hub: &Shared, selection: RemovalSelection) {
    let Some(wizard) = current(hub) else {
        return;
    };
    {
        let mut wizard = wizard.borrow_mut();
        if wizard.busy || wizard.confirmed || wizard.account != hub.borrow().state.account {
            return;
        }
        wizard.confirmed = true;
        wizard.window.set_visible(false);
    }
    toast(tr!("seerr-removal-working").to_string(), 3);
    // The exact selection is sealed by the single confirmation. Preflight may
    // reject it, but never widens it or asks the user to review a second page.
    send_command(hub, RemovalCommand::Preview(selection));
}
struct Choices {
    all: gtk::CheckButton,
    seasons: gtk::CheckButton,
    season_checks: Vec<(u32, gtk::CheckButton)>,
    episode_checks: Vec<(RemovalEpisodeKey, gtk::CheckButton)>,
}
impl Choices {
    fn scope(&self) -> Option<RemovalScope> {
        if self.all.is_active() {
            return Some(RemovalScope::All);
        }
        if self.seasons.is_active() {
            let seasons: Vec<_> = self
                .season_checks
                .iter()
                .filter(|(_, c)| c.is_active())
                .map(|(s, _)| *s)
                .collect();
            return (!seasons.is_empty()).then_some(RemovalScope::Seasons { seasons });
        }
        let episodes: Vec<_> = self
            .episode_checks
            .iter()
            .filter(|(_, c)| c.is_active())
            .map(|(e, _)| e.clone())
            .collect();
        (!episodes.is_empty()).then_some(RemovalScope::Episodes { episodes })
    }
}
pub(super) fn render_options(hub: &Shared, options: RemovalOptions) {
    let Some(wizard) = current(hub) else {
        return;
    };
    // Late options cannot reopen a confirmation after Delete was accepted.
    if wizard.borrow().confirmed {
        return;
    }
    let body = wizard.borrow().body.clone();
    clear(&body);
    clear(&wizard.borrow().actions);
    wizard.borrow().loading.set_visible(false);
    let heading =
        label(&tr!("seerr-removal-confirm-title", {"title" => options.target.title.as_str()}));
    heading.add_css_class("title-2");
    body.append(&heading);
    body.append(&label(tr!("seerr-removal-confirm-body")));
    let target = wizard
        .borrow()
        .target
        .clone()
        .unwrap_or(RemovalTarget::Media {
            media_type: options.target.media_type,
            tmdb_id: options.target.tmdb_id,
        });
    cancel_button(hub);
    let delete = button(tr!("seerr-removal-delete"));
    delete.add_css_class("destructive-action");
    if options.target.media_type == MediaType::Movie {
        on_click(&delete, hub, move |hub| {
            confirm(
                hub,
                RemovalSelection {
                    target: target.clone(),
                    scope: RemovalScope::All,
                },
            )
        });
    } else {
        let all = check_button(tr!("seerr-removal-series"));
        let seasons = check_button(tr!("seerr-removal-seasons"));
        let episodes = check_button(tr!("seerr-removal-episodes"));
        seasons.set_group(Some(&all));
        episodes.set_group(Some(&all));
        episodes.set_active(true);
        for radio in [&all, &seasons, &episodes] {
            body.append(radio);
        }
        let stack = gtk::Stack::new();
        let season_list = column(8);
        let episode_list = column(8);
        stack.add_named(&label(tr!("seerr-removal-entire-series-note")), Some("all"));
        stack.add_named(&season_list, Some("seasons"));
        stack.add_named(&episode_list, Some("episodes"));
        stack.set_visible_child_name("episodes");
        let mut season_checks = Vec::new();
        for season in options
            .episodes
            .iter()
            .map(|e| e.season_number)
            .collect::<BTreeSet<_>>()
        {
            let check = check_button(&format!("{} {season}", tr!("seerr-season")));
            season_list.append(&check);
            season_checks.push((season, check));
        }
        let mut episode_checks = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for episode in options.episodes.into_iter().take(5000) {
            let key = RemovalEpisodeKey {
                season_number: episode.season_number,
                episode_number: episode.episode_number,
            };
            if !seen.insert(key.clone()) {
                continue;
            }
            let missing = if episode.has_file {
                String::new()
            } else {
                format!(" · {}", tr!("seerr-removal-no-file"))
            };
            let check = check_button(&format!(
                "S{} E{} — {}{}",
                key.season_number, key.episode_number, episode.title, missing
            ));
            if options.initially_selected_episode.as_ref() == Some(&key) {
                check.set_active(true);
            }
            episode_list.append(&check);
            episode_checks.push((key, check));
        }
        if episode_checks.is_empty() {
            episode_list.append(&label(tr!("seerr-removal-no-episodes")));
        }
        body.append(&stack);
        let choices = Rc::new(Choices {
            all: all.clone(),
            seasons: seasons.clone(),
            season_checks,
            episode_checks,
        });
        for (name, radio) in [
            ("all", all.clone()),
            ("seasons", seasons.clone()),
            ("episodes", episodes.clone()),
        ] {
            let weak = stack.downgrade();
            radio.connect_toggled(move |r| {
                if r.is_active() {
                    if let Some(stack) = weak.upgrade() {
                        stack.set_visible_child_name(name);
                    }
                }
            });
        }
        let checks: Vec<_> = choices
            .season_checks
            .iter()
            .map(|(_, c)| c.clone())
            .chain(choices.episode_checks.iter().map(|(_, c)| c.clone()))
            .chain([all, seasons, episodes])
            .collect();
        for check in checks {
            let weak = Rc::downgrade(&choices);
            let delete = delete.downgrade();
            check.connect_toggled(move |_| {
                if let (Some(choices), Some(delete)) = (weak.upgrade(), delete.upgrade()) {
                    delete.set_sensitive(choices.scope().is_some());
                }
            });
        }
        delete.set_sensitive(choices.scope().is_some());
        on_click(&delete, hub, move |hub| {
            if let Some(scope) = choices.scope() {
                confirm(
                    hub,
                    RemovalSelection {
                        target: target.clone(),
                        scope,
                    },
                );
            }
        });
    }
    wizard.borrow().actions.append(&delete);
}
pub(super) fn render_preview(hub: &Shared, preview: RemovalPreview) {
    let Some(wizard) = current(hub) else {
        return;
    };
    if !wizard.borrow().confirmed {
        return;
    }
    if !preview.confirmable() {
        let reason = preview
            .blockers
            .first()
            .cloned()
            .unwrap_or_else(|| tr!("seerr-removal-blocked").to_string());
        fail(hub, &reason);
        return;
    }
    if let Some(token) = preview.token {
        send_command(hub, RemovalCommand::Commit(token));
    }
}
fn status_label(status: &RemovalStatus) -> String {
    let key = match status {
        RemovalStatus::Complete => "seerr-completed",
        RemovalStatus::Partial => "seerr-removal-partial",
        RemovalStatus::Failed => "seerr-failed",
        RemovalStatus::Running => "seerr-removal-running",
        RemovalStatus::Pending => "seerr-pending",
        RemovalStatus::Unknown => "seerr-unknown",
    };
    tr!(key).to_string()
}
// Recovery stays available separately, not as a step in everyday deletion.
fn receipt_controls(hub: &Shared, body: &gtk::Box, receipt: &RemovalReceipt) {
    let check = button(tr!("seerr-removal-check"));
    let id = receipt.operation_id.clone();
    on_click(&check, hub, move |hub| {
        send_command(hub, RemovalCommand::Receipt(id.clone()))
    });
    body.append(&check);
    if matches!(
        receipt.status,
        RemovalStatus::Partial | RemovalStatus::Failed
    ) {
        let retry = button(tr!("seerr-removal-retry"));
        retry.add_css_class("destructive-action");
        let id = receipt.operation_id.clone();
        on_click(&retry, hub, move |hub| {
            let dialog = adw::MessageDialog::new(
                current(hub)
                    .as_ref()
                    .map(|w| w.borrow().window.clone())
                    .as_ref(),
                Some(tr!("seerr-removal-retry")),
                Some(tr!("seerr-removal-retry-note")),
            );
            dialog.add_response("cancel", tr!("seerr-cancel"));
            dialog.add_response("retry", tr!("seerr-removal-retry"));
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");
            dialog.set_response_appearance("retry", adw::ResponseAppearance::Destructive);
            let weak = Rc::downgrade(hub);
            let id = id.clone();
            let account = hub.borrow().state.account;
            dialog.connect_response(None, move |_, response| {
                if response == "retry" {
                    if let Some(hub) = weak.upgrade() {
                        if hub.borrow().state.account == account {
                            if let Some(wizard) = current(&hub) {
                                wizard.borrow_mut().confirmed = true;
                            }
                            send_command(&hub, RemovalCommand::Retry(id.clone()));
                        }
                    }
                }
            });
            dialog.present();
        });
        body.append(&retry);
    }
}
pub(super) fn render_receipt(hub: &Shared, receipt: RemovalReceipt, notify: bool) {
    let Some(wizard) = current(hub) else {
        return;
    };
    let download = receipt.kind.as_deref() == Some("download");
    let complete = receipt.status == RemovalStatus::Complete
        && !receipt.steps.is_empty()
        && receipt
            .steps
            .iter()
            .all(|s| s.status == RemovalStatus::Complete);
    if !wizard.borrow().history {
        if notify {
            // A partial removal may have changed availability too.
            hub.borrow_mut().saved_grid = None;
            crate::app::APP_BROKER.send(crate::app::AppInput::RemovalFinished);
        }
        if complete {
            close(hub);
            let key = if download {
                "seerr-download-deleted"
            } else {
                "seerr-removal-complete"
            };
            toast(tr!(key).to_string(), 4);
        } else if receipt.physical_complete {
            close(hub);
            toast(tr!("seerr-removal-cleanup-pending").to_string(), 10);
        } else {
            let reason = receipt
                .steps
                .iter()
                .find(|s| s.status != RemovalStatus::Complete)
                .and_then(|s| s.message.clone())
                .unwrap_or_else(|| tr!("seerr-removal-incomplete").to_string());
            fail(hub, &reason);
        }
        return;
    }
    let body = wizard.borrow().body.clone();
    clear(&body);
    wizard.borrow_mut().confirmed = false;
    let key = if complete && download {
        "seerr-download-deleted"
    } else if complete {
        "seerr-removal-complete"
    } else if receipt.physical_complete {
        "seerr-removal-cleanup-pending"
    } else {
        "seerr-removal-incomplete"
    };
    wizard.borrow().status.set_label(tr!(key));
    if let Some(title) = &receipt.title {
        body.append(&label(title));
    }
    for step in &receipt.steps {
        body.append(&label(&format!(
            "{} — {}",
            step.name,
            status_label(&step.status)
        )));
        if let Some(message) = &step.message {
            body.append(&label(message));
        }
    }
    receipt_controls(hub, &body, &receipt);
    let history = button(tr!("seerr-removal-history"));
    on_click(&history, hub, |hub| {
        send_command(hub, RemovalCommand::History)
    });
    body.append(&history);
}
pub(super) fn receive(hub: &Shared, id: u64, result: Result<RemovalData, Error>) {
    let Some(wizard) = current(hub) else {
        return;
    };
    if wizard.borrow().request_id != id || wizard.borrow().account != hub.borrow().state.account {
        return;
    }
    {
        let mut wizard = wizard.borrow_mut();
        wizard.busy = false;
        wizard.committing = false;
        wizard.body.set_sensitive(true);
        wizard.actions.set_sensitive(true);
        wizard.spinner.set_spinning(false);
        wizard.window.set_deletable(true);
    }
    match result {
        Ok(RemovalData::Options(options)) => render_options(hub, options),
        Ok(RemovalData::Preview(preview)) => render_preview(hub, preview),
        Ok(RemovalData::Receipt(receipt)) => render_receipt(hub, receipt, true),
        Ok(RemovalData::DownloadCancelled(delete_files)) => {
            close(hub);
            let key = if delete_files {
                "seerr-download-deleted"
            } else {
                "seerr-download-cancelled"
            };
            toast(tr!(key).to_string(), 5);
            if hub.borrow().active == Some(2) {
                send(hub, Operation::Torrents);
            }
        }
        Ok(RemovalData::History(history)) => {
            let body = wizard.borrow().body.clone();
            clear(&body);
            clear(&wizard.borrow().actions);
            cancel_button(hub);
            wizard
                .borrow()
                .status
                .set_label(tr!("seerr-removal-history"));
            if history.operations.is_empty() {
                body.append(&label(tr!("seerr-removal-no-history")));
            }
            for receipt in history.operations.into_iter().take(20) {
                let heading = label(&format!(
                    "{} — {}",
                    receipt
                        .title
                        .as_deref()
                        .unwrap_or(tr!("seerr-remove-everywhere")),
                    status_label(&receipt.status)
                ));
                heading.add_css_class("heading");
                body.append(&heading);
                receipt_controls(hub, &body, &receipt);
                body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            }
        }
        Err(error) => show_error(hub, error),
    }
}

#[cfg(test)]
pub(super) fn exercise_single_confirmation(hub: &Shared) {
    use serde_json::json;
    fn value<T: serde::de::DeserializeOwned>(v: serde_json::Value) -> T {
        serde_json::from_value(v).unwrap()
    }
    fn widgets(w: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut all = vec![w.clone()];
        let mut child = w.first_child();
        while let Some(w) = child {
            all.extend(widgets(&w));
            child = w.next_sibling();
        }
        all
    }
    fn start(hub: &Shared) -> (Rc<RefCell<Wizard>>, gtk::Button) {
        let w = build(
            hub,
            Some(RemovalTarget::Media {
                media_type: MediaType::Movie,
                tmdb_id: 1,
            }),
        );
        render_options(
            hub,
            value(
                json!({"target":{"mediaType":"movie","tmdbId":1,"title":"Fixture movie"},"episodes":[],"warnings":[]}),
            ),
        );
        let delete = widgets(w.borrow().window.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().as_deref() == Some(tr!("seerr-removal-delete").as_str()))
            .unwrap();
        (w, delete)
    }
    fn preview() -> RemovalPreview {
        value(
            json!({"token":"fixture-token-not-a-real-capability","title":"Fixture movie","scopeLabel":"All versions","expiresAt":"2099-01-01T00:00:00Z","canCommit":true,"effects":["Delete selected files"],"blockers":[]}),
        )
    }
    fn job(jobs: &mut tokio::sync::mpsc::UnboundedReceiver<Job>) -> (u64, RemovalCommand) {
        let next = jobs.try_recv().expect("one queued command");
        match next.operation {
            Operation::Removal(id, c) => (id, c),
            _ => panic!("Unexpected non-removal job"),
        }
    }
    // Replace the transport, not the HTTP server: no fixture can reach a real service.
    let (sender, mut jobs) = tokio::sync::mpsc::unbounded_channel();
    let original = std::mem::replace(&mut hub.borrow_mut().service.sender, sender);
    let (w, _) = start(hub);
    render_preview(hub, preview());
    assert!(
        jobs.try_recv().is_err(),
        "An unsolicited preview cannot commit"
    );
    w.borrow().window.close();
    assert!(hub.borrow().removal.is_none());
    assert!(jobs.try_recv().is_err(), "Cancel sends no deletion command");
    for outcome in ["blocked", "complete", "partial", "timeout"] {
        let (w, delete) = start(hub);
        delete.emit_clicked();
        delete.emit_clicked();
        assert!(
            !w.borrow().window.is_visible(),
            "The one confirmation dismisses immediately"
        );
        let (id, command) = job(&mut jobs);
        match command {
            RemovalCommand::Preview(s) => assert_eq!(
                serde_json::to_value(s).unwrap(),
                json!({"target":{"mediaType":"movie","tmdbId":1},"scope":"all"})
            ),
            _ => panic!("Expected a sealed preflight selection"),
        }
        assert!(
            jobs.try_recv().is_err(),
            "Double click sends only one preflight"
        );
        w.borrow().window.close();
        assert!(
            hub.borrow().removal.is_some(),
            "Closing cannot discard an accepted job"
        );
        if outcome == "blocked" {
            receive(
                hub,
                id,
                Ok(RemovalData::Preview(value(
                    json!({"title":"Fixture movie","scopeLabel":"All","expiresAt":"2099-01-01T00:00:00Z","canCommit":false,"effects":[],"blockers":["The selected files are shared with unselected content."]}),
                ))),
            );
            assert!(hub.borrow().removal.is_none());
            assert!(jobs.try_recv().is_err(), "Blocked preflight never commits");
            continue;
        }
        // A different account and an obsolete view must not steal the operation.
        super::receive(
            hub,
            Reply {
                account: hub.borrow().state.account.wrapping_add(1),
                view: 0,
                auth: false,
                result: Ok(Data::Removal(id, Ok(RemovalData::Preview(preview())))),
            },
        );
        assert!(jobs.try_recv().is_err());
        hub.borrow_mut().state.navigate();
        receive(hub, id, Ok(RemovalData::Preview(preview())));
        let (commit, command) = job(&mut jobs);
        assert!(
            matches!(command, RemovalCommand::Commit(_)),
            "A valid preflight commits without another confirmation"
        );
        receive(hub, id, Ok(RemovalData::Preview(preview())));
        assert!(
            jobs.try_recv().is_err(),
            "A duplicate response cannot commit twice"
        );
        if outcome == "timeout" {
            receive(hub, commit, Err(Error::Timeout));
        } else {
            let status = if outcome == "complete" {
                "complete"
            } else {
                "partial"
            };
            let step = if outcome == "complete" {
                "complete"
            } else {
                "failed"
            };
            receive(
                hub,
                commit,
                Ok(RemovalData::Receipt(value(
                    json!({"operationId":"fixture-operation","status":status,"steps":[{"name":"Selected files","status":step,"message":"Fixture failure"}]}),
                ))),
            );
        }
        assert!(
            hub.borrow().removal.is_none(),
            "Completion/errors use toasts, not another dialog"
        );
        assert!(jobs.try_recv().is_err(), "No automatic mutation retry");
    }
    let active = hub.borrow().active;
    hub.borrow_mut().active = None;
    let history = build(hub, None);
    open_download(hub, "a".repeat(40), "Fixture download", false);
    assert!(
        !history.borrow().window.is_visible(),
        "Idle history must not intercept another item"
    );
    let wizard = current(hub).expect("download confirmation");
    let labels = widgets(wizard.borrow().window.upcast_ref())
        .into_iter()
        .filter_map(|w| {
            w.downcast::<gtk::Label>()
                .ok()
                .map(|l| l.text().to_string())
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert!(labels.contains("Fixture download") && labels.contains("downloaded files are kept"));
    wizard.borrow().window.close();
    assert!(
        jobs.try_recv().is_err(),
        "Closing confirmation cancels no downloads"
    );
    open_download(hub, "a".repeat(40), "Fixture download", false);
    let wizard = current(hub).unwrap();
    let cancel = widgets(wizard.borrow().window.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Button>().ok())
        .find(|b| b.label().as_deref() == Some(tr!("seerr-download-cancel").as_str()))
        .unwrap();
    cancel.emit_clicked();
    cancel.emit_clicked();
    let (id, command) = job(&mut jobs);
    assert!(
        matches!(command, RemovalCommand::CancelDownload(hash, false) if hash == "a".repeat(40))
    );
    assert!(
        jobs.try_recv().is_err(),
        "Double confirmation must not send twice"
    );
    hub.borrow_mut().active = Some(2);
    receive(hub, id, Ok(RemovalData::DownloadCancelled(false)));
    assert!(current(hub).is_none());
    assert!(matches!(
        jobs.try_recv().unwrap().operation,
        Operation::Torrents
    ));
    assert!(jobs.try_recv().is_err());
    hub.borrow_mut().state.busy = false;
    hub.borrow_mut().active = None;
    open_download(hub, "b".repeat(40), "Other download", true);
    let wizard = current(hub).unwrap();
    let labels = widgets(wizard.borrow().window.upcast_ref())
        .into_iter()
        .filter_map(|w| {
            w.downcast::<gtk::Label>()
                .ok()
                .map(|l| l.text().to_string())
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        labels.contains("Permanently deletes")
            && labels.contains("Library media and requests are kept")
    );
    let delete = widgets(wizard.borrow().window.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Button>().ok())
        .find(|b| b.label().as_deref() == Some(tr!("seerr-download-delete").as_str()))
        .unwrap();
    delete.emit_clicked();
    delete.emit_clicked();
    let (id, command) = job(&mut jobs);
    assert!(
        matches!(command, RemovalCommand::CancelDownload(hash, true) if hash == "b".repeat(40))
    );
    assert!(jobs.try_recv().is_err());
    receive(hub, id, Err(Error::Http(409)));
    assert!(current(hub).is_none());
    assert!(
        jobs.try_recv().is_err(),
        "Blocked file deletion must not retry"
    );
    let history = build(hub, None);
    render_receipt(
        hub,
        value(
            json!({"operationId":"fixture-download","kind":"download","status":"complete","steps":[{"name":"Delete download files","status":"complete"}]}),
        ),
        false,
    );
    assert_eq!(
        history.borrow().status.label().as_str(),
        tr!("seerr-download-deleted").as_str()
    );
    history.borrow().window.close();
    hub.borrow_mut().active = active;
    hub.borrow_mut().service.sender = original;
}
