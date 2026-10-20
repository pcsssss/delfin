use std::sync::Arc;

use adw::prelude::*;
use anyhow::Result;
use jellyfin_api::types::BaseItemDto;
use relm4::{
    actions::{AccelsPlus, RelmAction, RelmActionGroup},
    prelude::*,
};

use crate::{
    borgar::borgar_menu::{BorgarMenu, BorgarMenuAuth},
    config::{Account, Server},
    jellyfin_api::{api_client::ApiClient, models::collection_sort::CollectionSort},
    library::{
        media_page::{MediaPage, MediaPageInput},
        media_tile::MediaTileDisplay,
    },
    tr,
    utils::{empty_component::EmptyComponent, main_window::get_main_window},
};

use super::{media_fetcher::Fetcher, media_page::MediaPageInit};

pub struct Collection {
    api_client: Arc<ApiClient>,
    collection: BaseItemDto,
    sort: CollectionSort,
    borgar_menu: Controller<BorgarMenu>,
    media_page: Controller<MediaPage<CollectionItemsFetcher, EmptyComponent>>,
}

#[derive(Debug)]
pub enum CollectionInput {
    Refresh,
    SortChanged(u32),
}

#[relm4::component(pub)]
impl Component for Collection {
    type Init = (Arc<ApiClient>, BaseItemDto, Server, Account);
    type Input = CollectionInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::NavigationPage {
            #[name = "toolbar_view"]
            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    pack_end = model.borgar_menu.widget(),
                    pack_end = &gtk::DropDown::from_strings(&[
                        tr!("library-collection-sort-name-ascending"),
                        tr!("library-collection-sort-name-descending"),
                        tr!("library-collection-sort-date-descending"),
                        tr!("library-collection-sort-date-ascending"),
                    ]) {
                        set_tooltip: tr!("library-collection-sort-label"),
                        set_valign: gtk::Align::Center,
                        connect_selected_notify[sender] => move |dropdown| {
                            sender.input(CollectionInput::SortChanged(dropdown.selected()));
                        },
                    },
                    pack_end = &gtk::Button::from_icon_name("refresh") {
                        set_tooltip: tr!("library-collection-refresh-button"),
                        connect_clicked[sender] => move |_| {
                            sender.input(CollectionInput::Refresh);
                        },
                    },
                },

                #[wrap(Some)]
                set_content = model.media_page.widget(),
            },

            set_title: &model.collection
                .name
                .as_ref()
                .unwrap_or(tr!("library-unnamed-collection"))
                .clone(),
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (api_client, collection, server, account) = init;

        let model = Collection {
            api_client: api_client.clone(),
            collection: collection.clone(),
            sort: CollectionSort::default(),
            borgar_menu: BorgarMenu::builder()
                .launch(Some(BorgarMenuAuth {
                    api_client: api_client.clone(),
                    server,
                    account,
                }))
                .detach(),
            media_page: new_media_page(&api_client, collection, CollectionSort::default()),
        };

        model.media_page.emit(MediaPageInput::NextPage);

        let widgets = view_output!();

        model.register_actions(&sender);

        ComponentParts { model, widgets }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        self.unregister_actions();
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            CollectionInput::Refresh => {}
            CollectionInput::SortChanged(selected) => {
                let Some(sort) = CollectionSort::from_selected(selected) else {
                    return;
                };
                if sort == self.sort {
                    return;
                }
                self.sort = sort;
            }
        }
        // Replace the page (and its fetcher) so a new order always starts at page one.
        // Old in-flight requests belong to the old component, never the new grid.
        let media_page = new_media_page(&self.api_client, self.collection.clone(), self.sort);
        widgets.toolbar_view.set_content(Some(media_page.widget()));
        media_page.emit(MediaPageInput::NextPage);
        self.media_page = media_page;
        self.update_view(widgets, sender);
    }
}

impl Collection {
    fn register_actions(&self, sender: &ComponentSender<Self>) {
        relm4::main_application().set_accelerators_for_action::<RefreshAction>(&["<Ctrl>r"]);

        let refresh_action: RelmAction<RefreshAction> = RelmAction::new_stateless({
            let sender = sender.clone();
            move |_| {
                sender.input(CollectionInput::Refresh);
            }
        });
        let mut group = RelmActionGroup::<CollectionActionGroup>::new();
        group.add_action(refresh_action);
        if let Some(main_window) = get_main_window() {
            group.register_for_widget(main_window);
        }
    }

    fn unregister_actions(&self) {
        relm4::main_application().set_accelerators_for_action::<RefreshAction>(&[]);
    }
}

fn new_media_page(
    api_client: &Arc<ApiClient>,
    collection: BaseItemDto,
    sort: CollectionSort,
) -> Controller<MediaPage<CollectionItemsFetcher, EmptyComponent>> {
    let fetcher = CollectionItemsFetcher {
        api_client: api_client.clone(),
        collection,
        sort,
    };
    MediaPage::builder()
        .launch(MediaPageInit {
            api_client: api_client.clone(),
            fetcher,
            empty_component: None,
            media_tile_display: Some(MediaTileDisplay::CoverLarge),
        })
        .detach()
}

struct CollectionItemsFetcher {
    api_client: Arc<ApiClient>,
    collection: BaseItemDto,
    sort: CollectionSort,
}

impl Fetcher for CollectionItemsFetcher {
    async fn fetch(&self, start_index: usize, limit: usize) -> Result<(Vec<BaseItemDto>, usize)> {
        self.api_client
            .get_collection_items(&self.collection, self.sort, start_index, limit)
            .await
    }

    fn title(&self) -> String {
        self.collection
            .name
            .as_ref()
            .unwrap_or(tr!("library-unnamed-collection"))
            .clone()
    }
}

relm4::new_action_group!(CollectionActionGroup, "collection");
relm4::new_stateless_action!(RefreshAction, CollectionActionGroup, "refresh");
