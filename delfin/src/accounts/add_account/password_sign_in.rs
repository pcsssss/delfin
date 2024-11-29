use adw::prelude::*;
use relm4::prelude::*;
use uuid::Uuid;

use crate::{
    config::Server,
    jellyfin_api::api::user::{authenticate_by_name, AuthenticateByNameRes},
    tr,
};

#[derive(Debug, Default)]
enum ValidationState {
    #[default]
    Invalid,
    Loading,
}

#[derive(Default)]
pub(crate) struct PasswordSignIn {
    server: Server,
    device_id: Uuid,
    username: String,
    password: String,
    valid: ValidationState,
}

#[derive(Debug)]
pub(crate) enum PasswordSignInInput {
    UsernameChanged(String),
    PasswordChanged(String),
    SignIn,
    QuickConnect,
}

#[derive(Debug)]
pub(crate) enum PasswordSignInOutput {
    AccountAdded(AuthenticateByNameRes),
    QuickConnect,
    Toast(String, u32),
}

#[derive(Debug)]
pub enum PasswordSignInCommandOutput {
    SignInSuccess(AuthenticateByNameRes),
    SignInFail(anyhow::Error),
}

#[relm4::component(pub(crate))]
impl Component for PasswordSignIn {
    type Init = (Server, Uuid);
    type Input = PasswordSignInInput;
    type Output = PasswordSignInOutput;
    type CommandOutput = PasswordSignInCommandOutput;

    view! {
        adw::NavigationPage {
            set_title: tr!("account-list-add-account-title"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar::default(),

                adw::Clamp {
                    set_margin_all: 20,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 20,

                        adw::PreferencesGroup {
                            adw::EntryRow {
                                set_title: tr!("account-list-add-account-username"),
                                set_activates_default: true,
                                connect_changed[sender] => move |entry| {
                                    sender.input(PasswordSignInInput::UsernameChanged(entry.text().to_string()))
                                },
                            },
                            adw::PasswordEntryRow {
                                set_title: tr!("account-list-add-account-password"),
                                set_activates_default: true,
                                connect_changed[sender] => move |entry| {
                                    sender.input(PasswordSignInInput::PasswordChanged(entry.text().to_string()))
                                },
                            },
                        },

                        #[transition = "Crossfade"]
                        append = if matches!(model.valid, ValidationState::Invalid) {
                            gtk::Box::new(gtk::Orientation::Vertical, 20) {
                                set_halign: gtk::Align::Center,

                                gtk::Button {
                                    set_halign: gtk::Align::Fill,
                                    set_label: tr!("account-list-add-account-submit-button"),
                                    add_css_class: "pill",
                                    add_css_class: "suggested-action",
                                    set_receives_default: true,
                                    connect_realize => move |btn| {
                                        if let Some(root) = btn.toplevel_window() {
                                            // TODO: this lets you press enter to submit the form,
                                            // but it only works once for some reason
                                            root.set_default_widget(Some(btn));
                                        }
                                    },
                                    #[watch]
                                    set_sensitive: !model.username.is_empty(),
                                    connect_clicked[sender] => move |_| {
                                        sender.input(PasswordSignInInput::SignIn);
                                    }
                                },
                                #[name = "quick_connect_btn"]
                                gtk::Button {
                                    set_halign: gtk::Align::Fill,
                                    set_label: tr!("account-list-quick-connect"),
                                    add_css_class: "pill",
                                    connect_clicked[sender] => move |_| {
                                        sender.input(PasswordSignInInput::QuickConnect);
                                    }
                                },
                            }
                        } else {
                            gtk::Spinner { set_spinning: true }
                        },
                    },
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (server, device_id) = init;

        let model = Self {
            server,
            device_id,
            username: String::default(),
            password: String::default(),
            valid: ValidationState::default(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            PasswordSignInInput::UsernameChanged(username) => {
                self.username = username;
            }
            PasswordSignInInput::PasswordChanged(password) => {
                self.password = password;
            }
            PasswordSignInInput::SignIn => {
                self.valid = ValidationState::Loading;
                let url = self.server.url.clone();
                let device_id = self.device_id;
                let username = self.username.clone();
                let password = self.password.clone();
                sender.oneshot_command(async move {
                    let auth_info =
                        authenticate_by_name(&url, &device_id, &username, &password).await;
                    match auth_info {
                        Ok(auth_info) => PasswordSignInCommandOutput::SignInSuccess(auth_info),
                        Err(err) => PasswordSignInCommandOutput::SignInFail(err),
                    }
                });
            }
            PasswordSignInInput::QuickConnect => {
                sender.output(PasswordSignInOutput::QuickConnect).unwrap();
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            PasswordSignInCommandOutput::SignInSuccess(res) => {
                sender
                    .output(PasswordSignInOutput::AccountAdded(res))
                    .unwrap();
            }
            PasswordSignInCommandOutput::SignInFail(err) => {
                tracing::error!("Sign in failed: {err:#?}");
                sender
                    .output(PasswordSignInOutput::Toast(err.to_string(), 3))
                    .unwrap();
                self.valid = ValidationState::Invalid;
            }
        }
    }
}
