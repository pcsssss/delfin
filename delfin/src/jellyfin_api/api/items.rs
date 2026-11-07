use super::views::decode_collection_page;
use anyhow::{Context, Result};
use derive_builder::Builder;
use jellyfin_api::types::BaseItemDto;

use crate::jellyfin_api::api_client::ApiClient;

#[derive(Builder)]
pub struct GetItemsOptions {
    #[builder(default)]
    search_term: Option<String>,
    #[builder(default)]
    include_item_types: Option<String>,
    #[builder(default)]
    sort_by: Option<String>,
    #[builder(default)]
    sort_order: Option<String>,
    #[builder(default = "true")]
    recursive: bool,
    #[builder(default)]
    start_index: Option<usize>,
    #[builder(default)]
    limit: Option<usize>,
}

impl ApiClient {
    pub async fn get_items(&self, options: &GetItemsOptions) -> Result<(Vec<BaseItemDto>, usize)> {
        let mut url = self
            .root
            .join(&format!("Users/{}/Items", self.account.id))?;

        {
            let mut query_pairs = url.query_pairs_mut();
            if let Some(include_item_types) = &options.include_item_types {
                query_pairs.append_pair("IncludeItemTypes", include_item_types);
            }
            if let Some(limit) = &options.limit {
                query_pairs.append_pair("Limit", &limit.to_string());
            }
            query_pairs.append_pair("Recursive", &options.recursive.to_string());
            if let Some(search_term) = &options.search_term {
                query_pairs.append_pair("SearchTerm", search_term);
            }
            if let Some(sort_by) = &options.sort_by {
                query_pairs.append_pair("SortBy", sort_by);
            }
            if let Some(sort_order) = &options.sort_order {
                query_pairs.append_pair("SortOrder", sort_order);
            }
            if let Some(start_index) = &options.start_index {
                query_pairs.append_pair("StartIndex", &start_index.to_string());
            }
        }

        url.query_pairs_mut()
            .append_pair("Fields", "MediaStreams,ProviderIds");
        let response = self.client.get(url).send().await?.error_for_status()?;
        let res = decode_collection_page(response.json().await?)?;

        let items = res.items.context("No items returned")?;
        let total_record_count = res
            .total_record_count
            .context("Total record count not returned")?;

        Ok((items, total_record_count as usize))
    }
}
