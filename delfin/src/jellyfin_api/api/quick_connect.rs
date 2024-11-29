use anyhow::{Context, Result};
use jellyfin_api::types::QuickConnectResult;
use reqwest::{header::HeaderMap, Url};
use uuid::Uuid;

use crate::jellyfin_api::{
    unauthed_client::{get_unauthed_client, get_unauthed_client_builder},
    util::{auth_header::get_auth_header, url::httpify},
};

#[derive(Debug)]
pub struct QuickConnectClient {
    client: reqwest::Client,
    root: Url,
}

impl QuickConnectClient {
    pub fn new(server_url: &str, device_id: &Uuid) -> Result<Self> {
        let auth_header = get_auth_header(device_id, None);

        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::AUTHORIZATION, auth_header.parse()?);

        let client = get_unauthed_client_builder()
            .default_headers(headers)
            .build()
            .context("Failed to build quick connect client")?;

        let root = Url::parse(&httpify(server_url))?.join("QuickConnect/")?;

        Ok(Self { client, root })
    }

    pub async fn init(&self) -> Result<QuickConnectResult> {
        let url = self.root.join("Initiate")?;
        Ok(self.client.get(url).send().await?.json().await?)
    }

    pub async fn connect(&self, secret: &str) -> Result<QuickConnectResult> {
        let mut url = self.root.join("Connect")?;
        url.query_pairs_mut().append_pair("Secret", secret);
        Ok(self.client.get(url).send().await?.json().await?)
    }
}

pub async fn quick_connect_enabled(url: &str, device_id: &Uuid) -> Result<bool> {
    let client = get_unauthed_client();

    let url = httpify(url);
    let url = format!("{}QuickConnect/Enabled", url);

    let res = client
        .get(url)
        .header(
            reqwest::header::AUTHORIZATION,
            get_auth_header(device_id, None),
        )
        .send()
        .await?
        .text()
        .await?;

    Ok(res == "true")
}
