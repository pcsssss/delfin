mod password_sign_in;
mod quick_connect_sign_in;

use std::fmt::Display;

use adw::prelude::*;
use gtk::glib;
use password_sign_in::{PasswordSignIn, PasswordSignInOutput};
use quick_connect_sign_in::{QuickConnectSignIn, QuickConnectSignInOutput};
use relm4::{adw, gtk, prelude::*, Component, ComponentParts};
use uuid::Uuid;

use crate::{config::Server, jellyfin_api::api::user::AuthenticateByNameRes, tr};

pub struct AddAccountDialog {
    server: Server,
    device_id: Uuid,
    toaster: adw::ToastOverlay,
    password_sign_in: Controller<PasswordSignIn>,
    quick_connect_sign_in: Option<Controller<QuickConnectSignIn>>,
}

#[derive(Debug)]
pub enum AddAccountInput {
    Close,
    PagePopped(Option<String>),
    Toast(String, u32),
    QuickConnect,
    AccountAdded(AuthenticateByNameRes),
}

#[derive(Debug)]
pub enum AddAccountOutput {
    AccountAdded(AuthenticateByNameRes),
}

pub enum AddAccountPage {
    Password,
    QuickConnect,
}

impl Display for AddAccountPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AddAccountPage::Password => "password",
                AddAccountPage::QuickConnect => "quick_connect",
            }
        )
    }
}

#[relm4::component(pub)]
impl Component for AddAccountDialog {
    type Init = Server;
    type Input = AddAccountInput;
    type Output = AddAccountOutput;
    type CommandOutput = ();

    view! {
        adw::Window {
            set_application: Some(&relm4::main_application()),
            set_title: Some(tr!("account-list-add-account-title")),
            set_modal: true,
            set_visible: true,

            connect_close_request[sender] => move |_| {
                sender.input(AddAccountInput::Close);
                glib::Propagation::Proceed
            },

            #[local_ref]
            #[wrap(Some)]
            set_content = toaster -> adw::ToastOverlay {
                #[name = "navigation"]
                adw::NavigationView {
                    add = model.password_sign_in.widget() {
                        set_tag: Some(&AddAccountPage::Password.to_string()),
                    },

                    connect_popped[sender] => move |_, page| {
                        sender.input(AddAccountInput::PagePopped(page.tag().as_deref().map(ToString::to_string)));
                    },
                },

            },
        }
    }

    fn init(
        server: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let device_id = Uuid::new_v4();
        let model = AddAccountDialog {
            server: server.clone(),
            device_id,
            toaster: adw::ToastOverlay::default(),
            password_sign_in: PasswordSignIn::builder()
                .launch((server.clone(), device_id))
                .forward(sender.input_sender(), AddAccountInput::from),
            quick_connect_sign_in: None,
        };

        let toaster = &model.toaster;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AddAccountInput::Close => {
                self.quick_connect_sign_in = None;
            }
            AddAccountInput::PagePopped(tag) => match tag {
                Some(tag) if tag == AddAccountPage::QuickConnect.to_string() => {
                    self.quick_connect_sign_in = None;
                }
                _ => {}
            },
            AddAccountInput::Toast(message, duration) => {
                let toast = adw::Toast::new(&message);
                toast.set_timeout(duration);
                self.toaster.add_toast(toast);
            }
            AddAccountInput::QuickConnect => {
                let quick_connect_sign_in = QuickConnectSignIn::builder()
                    .launch((self.server.clone(), self.device_id))
                    .forward(sender.input_sender(), AddAccountInput::from);
                quick_connect_sign_in
                    .widget()
                    .set_tag(Some(&AddAccountPage::QuickConnect.to_string()));
                widgets.navigation.push(quick_connect_sign_in.widget());
                self.quick_connect_sign_in = Some(quick_connect_sign_in);
            }
            AddAccountInput::AccountAdded(account) => {
                self.quick_connect_sign_in = None;
                sender
                    .output(AddAccountOutput::AccountAdded(account))
                    .unwrap();
                root.close();
            }
        }

        self.update_view(widgets, sender);
    }
}

impl From<PasswordSignInOutput> for AddAccountInput {
    fn from(value: PasswordSignInOutput) -> Self {
        match value {
            PasswordSignInOutput::AccountAdded(account) => AddAccountInput::AccountAdded(account),
            PasswordSignInOutput::QuickConnect => AddAccountInput::QuickConnect,
            PasswordSignInOutput::Toast(message, duration) => {
                AddAccountInput::Toast(message, duration)
            }
        }
    }
}
impl From<QuickConnectSignInOutput> for AddAccountInput {
    fn from(value: QuickConnectSignInOutput) -> Self {
        match value {
            QuickConnectSignInOutput::AccountAdded(account) => {
                AddAccountInput::AccountAdded(account)
            }
            QuickConnectSignInOutput::Toast(message, duration) => {
                AddAccountInput::Toast(message, duration)
            }
        }
    }
}
