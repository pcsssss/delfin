use anyhow::{Context, Ok, Result};
use jellyfin_api::types::{BaseItemDto, BaseItemDtoQueryResult};

use crate::jellyfin_api::{
    api_client::ApiClient,
    models::{
        collection_sort::CollectionSort, collection_type::CollectionType, user_view::UserView,
    },
};

/// The generated 10.9 client rejects new stream fields returned by Jellyfin
/// 10.11. The grid only needs dimensions: lift those and discard the expanded
/// stream payload before decoding the existing item DTO. No extra request.
pub(crate) fn decode_collection_page(
    mut value: serde_json::Value,
) -> Result<BaseItemDtoQueryResult> {
    if let Some(items) = value
        .get_mut("Items")
        .and_then(serde_json::Value::as_array_mut)
    {
        for item in items {
            let Some(object) = item.as_object_mut() else {
                continue;
            };
            let Some(streams) = object.remove("MediaStreams") else {
                continue;
            };
            let dimensions = streams.as_array().and_then(|streams| {
                streams.iter().find_map(|stream| {
                    if stream.get("Type").and_then(serde_json::Value::as_str) != Some("Video") {
                        return None;
                    }
                    let width = i32::try_from(stream.get("Width")?.as_i64()?).ok()?;
                    let height = i32::try_from(stream.get("Height")?.as_i64()?).ok()?;
                    (width > 0 && height > 0).then_some((width, height))
                })
            });
            if let Some((width, height)) = dimensions {
                object.insert("Width".into(), width.into());
                object.insert("Height".into(), height.into());
            }
        }
    }
    Ok(serde_json::from_value(value)?)
}

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

        // Get video dimensions in the same paginated request as the posters.
        // Ratings/runtime/user state are already included by Jellyfin by default.
        if matches!(
            collection_type,
            CollectionType::Movies | CollectionType::Other
        ) {
            url.query_pairs_mut()
                .append_pair("Fields", "MediaStreams,ProviderIds");
        } else {
            url.query_pairs_mut().append_pair("Fields", "ProviderIds");
        }
        let response = self.client.get(url).send().await?.error_for_status()?;
        let res = decode_collection_page(response.json().await?)?;

        let items = res.items.context("No items returned")?;
        let total_record_count = res
            .total_record_count
            .context("Total record count not returned")?;

        Ok((items, total_record_count as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Account, Config, Server};
    use std::sync::{Arc, RwLock};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn movie_dimensions_are_fetched_in_the_existing_sorted_paginated_request() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut targets = Vec::new();
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                loop {
                    let mut buf = [0; 4096];
                    let count = socket.read(&mut buf).await.unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&buf[..count]);
                    if request.windows(4).any(|p| p == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(request).unwrap();
                targets.push(
                    request
                        .lines()
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .to_string(),
                );
                let body = r#"{"Items":[{"Id":"00000000000000000000000000000000","Type":"Movie","CommunityRating":7.3,"RunTimeTicks":60000000000,"ProviderIds":{"Imdb":"tt0133093"},"MediaStreams":[{"Type":"Video","Width":1920,"Height":800,"ReferenceFrameRate":24.0,"FutureServerField":true}],"UserData":{"Played":false}}],"TotalRecordCount":25}"#;
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
            targets
        });
        let account = Account {
            id: uuid::Uuid::nil(),
            username: "test".into(),
            access_token: "dummy".into(),
            device_id: uuid::Uuid::nil(),
        };
        let api = ApiClient::new(
            Arc::new(RwLock::new(Config::default())),
            &Server {
                url: format!("http://{address}/jellyfin/"),
                ..Default::default()
            },
            &account,
        );
        for kind in ["movies", "tvshows"] {
            let collection: BaseItemDto = serde_json::from_value(
                serde_json::json!({"Id":uuid::Uuid::nil(),"CollectionType":kind}),
            )
            .unwrap();
            let (items, total) = api
                .get_collection_items(&collection, CollectionSort::DateAddedDescending, 24, 24)
                .await
                .unwrap();
            assert_eq!(total, 25);
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].width, Some(1920));
            assert_eq!(items[0].height, Some(800));
            assert!(items[0].media_streams.is_none());
            assert_eq!(
                items[0].provider_ids.as_ref().unwrap()["Imdb"].as_deref(),
                Some("tt0133093")
            );
        }
        let targets = server.await.unwrap();
        for (index, target) in targets.iter().enumerate() {
            let url = reqwest::Url::parse(&format!("http://localhost{target}")).unwrap();
            let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            assert_eq!(params.get("StartIndex").unwrap(), "24");
            assert_eq!(params.get("Limit").unwrap(), "24");
            assert_eq!(
                params.get("SortBy").unwrap(),
                "DateCreated,SortName,ProductionYear"
            );
            assert_eq!(params.get("SortOrder").unwrap(), "Descending");
            assert_eq!(
                params.get("Fields").map(String::as_str),
                if index == 0 {
                    Some("MediaStreams,ProviderIds")
                } else {
                    Some("ProviderIds")
                }
            );
        }
    }
}
