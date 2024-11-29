use std::time::Duration;

use adw::prelude::*;
use anyhow::Result;
use gtk::gdk;
use relm4::{prelude::*, JoinHandle};
use uuid::Uuid;

use crate::{
    config::Server,
    jellyfin_api::api::{
        quick_connect::QuickConnectClient,
        user::{authenticate_by_pin, AuthenticateByNameRes},
    },
    tr,
};

#[derive(Debug)]
pub(crate) struct QuickConnectSignIn {
    poll_handle: JoinHandle<()>,
    timeout_handle: Option<JoinHandle<()>>,
    quick_connect_code: Option<String>,
    auth_state: AuthState,
}

#[derive(Debug, Default)]
enum AuthState {
    #[default]
    Waiting,
    Loading {
        // To avoid flickering UI changes, there's a timer that ensures that the loading state will
        // be shown for a bit, even if loading finishes very quickly
        ready: bool,
    },
}

#[derive(Debug)]
pub(crate) enum QuickConnectSignInInput {
    SetQuickConnectCode(String),
    AccountAdded(AuthenticateByNameRes),
    CopyCodeToClipboard,
    SetLoading,
    SetWaiting,
}

#[derive(Debug)]
pub(crate) enum QuickConnectSignInOutput {
    AccountAdded(AuthenticateByNameRes),
    Toast(String, u32),
}

#[relm4::component(pub(crate))]
impl Component for QuickConnectSignIn {
    type Init = (Server, Uuid);
    type Input = QuickConnectSignInInput;
    type Output = QuickConnectSignInOutput;
    type CommandOutput = ();

    view! {
        adw::NavigationPage {
            set_title: tr!("account-list-quick-connect"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar::default(),

                gtk::Box::new(gtk::Orientation::Vertical, 20) {
                    set_valign: gtk::Align::Center,

                    gtk::Label::new(Some(tr!("account-list-quick-connect-instructions"))),

                    #[transition = "Crossfade"]
                    if let Some(code) = model.quick_connect_code.as_ref() {
                        gtk::Box {
                            set_halign: gtk::Align::Center,
                            set_spacing: 8,

                            gtk::Label {
                                add_css_class: "title-1",
                                #[watch]
                                set_label: &code,
                            },

                            gtk::Button::from_icon_name("copy") {
                                add_css_class: "circular",
                                set_tooltip: tr!("account-list-quick-connect-copy-tooltip"),
                                connect_clicked[sender] => move |_| {
                                    sender.input(QuickConnectSignInInput::CopyCodeToClipboard);
                                },
                            },
                        }
                    } else {
                        gtk::Spinner {
                            set_spinning: true,
                        }
                    },

                    #[transition = "Crossfade"]
                    if let AuthState::Loading { ready: _ } = model.auth_state {
                        gtk::Spinner {
                            set_spinning: true,
                        }
                    } else {
                        gtk::Label::new(Some(tr!("account-list-quick-connect-loading"))) {
                            add_css_class: "dim-label",
                        }
                    },
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (server, device_id) = init;

        let model = QuickConnectSignIn {
            poll_handle: begin_quick_connect(&sender, &server, device_id),
            timeout_handle: None,
            quick_connect_code: None,
            auth_state: AuthState::default(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            QuickConnectSignInInput::SetQuickConnectCode(code) => {
                self.quick_connect_code = Some(code);
            }
            QuickConnectSignInInput::AccountAdded(account) => {
                sender
                    .output(QuickConnectSignInOutput::AccountAdded(account))
                    .unwrap();
            }
            QuickConnectSignInInput::CopyCodeToClipboard => {
                if let (Some(code), Some(display)) =
                    (&self.quick_connect_code, gdk::Display::default())
                {
                    display.clipboard().set_text(code);
                    sender
                        .output(QuickConnectSignInOutput::Toast(
                            tr!("account-list-quick-connect-copy-toast").into(),
                            2,
                        ))
                        .unwrap();
                }
            }
            QuickConnectSignInInput::SetLoading => {
                self.auth_state = AuthState::Loading { ready: false };
                self.timeout_handle = Some(relm4::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    sender.input(QuickConnectSignInInput::SetWaiting);
                }));
            }
            QuickConnectSignInInput::SetWaiting => {
                self.auth_state = match self.auth_state {
                    AuthState::Loading { ready } if !ready => AuthState::Loading { ready: true },
                    _ => AuthState::Waiting,
                };
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        self.poll_handle.abort();
        if let Some(timeout_handle) = &self.timeout_handle {
            timeout_handle.abort();
        }
    }
}

fn begin_quick_connect(
    sender: &ComponentSender<QuickConnectSignIn>,
    server: &Server,
    device_id: Uuid,
) -> JoinHandle<()> {
    let sender = sender.clone();
    let url = server.url.clone();
    relm4::spawn(async move {
        if let Err(err) = try_quick_connect(&sender, &url, &device_id).await {
            tracing::error!("Quick connect failed: {err:#?}");
            sender
                .output(QuickConnectSignInOutput::Toast(err.to_string(), 3))
                .unwrap();
        }
    })
}

async fn try_quick_connect(
    sender: &ComponentSender<QuickConnectSignIn>,
    url: &str,
    device_id: &Uuid,
) -> Result<()> {
    let client = QuickConnectClient::new(url, device_id)?;

    let auth_info = client.init().await?;
    sender.input(QuickConnectSignInInput::SetQuickConnectCode(
        auth_info.code.unwrap(),
    ));

    let secret = loop {
        sender.input(QuickConnectSignInInput::SetWaiting);
        tokio::time::sleep(Duration::from_secs(3)).await;

        sender.input(QuickConnectSignInInput::SetLoading);
        let quick_connect_res = client.connect(auth_info.secret.as_ref().unwrap()).await?;
        if quick_connect_res.authenticated.unwrap() {
            break quick_connect_res.secret.unwrap();
        }
    };

    let account = authenticate_by_pin(url, device_id, &secret).await?;
    sender.input(QuickConnectSignInInput::AccountAdded(account));

    Ok(())
}
