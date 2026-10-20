use anyhow::{Context, Ok, Result};
use jellyfin_api::types::{BaseItemDto, BaseItemDtoQueryResult};

use crate::jellyfin_api::{
    api_client::ApiClient,
    models::{
        collection_sort::CollectionSort, collection_type::CollectionType, user_view::UserView,
    },
};

impl ApiClient {
    pub async fn get_user_views(
        &self,
        start_index: Option<usize>,
        limit: Option<usize>,
    ) -> Result<(Vec<UserView>, usize)> {
        let mut url = self
            .root
            .join(&format!("Users/{}/Views", self.account.id))
            .unwrap();

        {
            let mut query_pairs = url.query_pairs_mut();
            if let Some(start_index) = start_index {
                query_pairs.append_pair("StartIndex", &start_index.to_string());
            }
            if let Some(limit) = limit {
                query_pairs.append_pair("Limit", &limit.to_string());
            }
        }

        let res: BaseItemDtoQueryResult = self.client.get(url).send().await?.json().await?;

        let items = res.items.context("No items returned")?;
        let total_record_count = res
            .total_record_count
            .context("Total record count not returned")?;

        let items: Result<Vec<UserView>, _> =
            items.into_iter().map(|item| item.try_into()).collect();

        Ok((items?, total_record_count as usize))
    }

    pub async fn get_collection_items(
        &self,
        collection: &BaseItemDto,
        sort: CollectionSort,
        start_index: usize,
        limit: usize,
    ) -> Result<(Vec<BaseItemDto>, usize)> {
        let collection_type = CollectionType::from(collection.collection_type);

        let mut url = self
            .root
            .join(&format!("Users/{}/Items", self.account.id))
            .unwrap();

        url.query_pairs_mut()
            .append_pair("ParentId", &collection.id.unwrap().to_string())
            .append_pair("SortBy", sort.sort_by())
            .append_pair("SortOrder", sort.sort_order())
            .append_pair("Recursive", "true")
            .append_pair("StartIndex", &start_index.to_string())
            .append_pair("Limit", &limit.to_string());

        url.query_pairs_mut().append_pair(
            "IncludeItemTypes",
            &collection_type
                .item_type()
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<String>>()
                .join(","),
        );

        let res: BaseItemDtoQueryResult = self.client.get(url).send().await?.json().await?;

        let items = res.items.context("No items returned")?;
        let total_record_count = res
            .total_record_count
            .context("Total record count not returned")?;

        Ok((items, total_record_count as usize))
    }
}
