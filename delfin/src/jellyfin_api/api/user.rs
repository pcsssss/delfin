use std::collections::VecDeque;

use anyhow::Result;
use jellyfin_api::types::QuickConnectDto;
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

use crate::jellyfin_api::{
    api_client::ApiClient,
    unauthed_client::get_unauthed_client,
    util::{auth_header::get_auth_header, url::httpify},
};

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct AuthenticateByNameReqBody {
    username: String,
    #[serde(rename = "Pw")]
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticateByNameRes {
    pub user: AuthenticateByNameResUser,
    pub access_token: String,
    pub server_id: Uuid,
    pub session_info: AuthenticateByNameResSessionInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticateByNameResUser {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticateByNameResSessionInfo {
    pub device_id: Uuid,
}

pub async fn authenticate_by_name(
    url: &str,
    device_id: &Uuid,
    username: &str,
    password: &str,
) -> Result<AuthenticateByNameRes> {
    let client = get_unauthed_client();

    let url = httpify(url);
    let url = format!("{url}Users/AuthenticateByName");

    let res = client
        .post(url)
        .header(
            reqwest::header::AUTHORIZATION,
            get_auth_header(device_id, None),
        )
        .json(&AuthenticateByNameReqBody {
            username: username.into(),
            password: password.into(),
        })
        .send()
        .await?;

    match res.status() {
        StatusCode::OK => {
            let res = res.json().await?;
            Ok(res)
        }
        StatusCode::UNAUTHORIZED => anyhow::bail!("Wrong username or password."),
        _ => {
            error!("Sign in error: {:#?}", res);
            anyhow::bail!("Error signing in.");
        }
    }
}

pub async fn authenticate_by_pin(
    url: &str,
    device_id: &Uuid,
    secret: &str,
) -> Result<AuthenticateByNameRes> {
    let client = get_unauthed_client();

    let url = httpify(url);
    let url = format!("{url}Users/AuthenticateWithQuickConnect");

    Ok(client
        .post(url)
        .header(
            reqwest::header::AUTHORIZATION,
            get_auth_header(device_id, None),
        )
        .json(&QuickConnectDto {
            secret: secret.into(),
        })
        .send()
        .await?
        .json()
        .await?)
}

pub async fn get_user_avatar(url: &str, user_id: &Uuid) -> Result<VecDeque<u8>> {
    let client = get_unauthed_client();

    let url = httpify(url);
    let url = Url::parse(&url)?.join(&format!("Users/{user_id}/Images/Primary?width=40"))?;

    Ok(client
        .get(url)
        .send()
        .await?
        .bytes()
        .await?
        .into_iter()
        .collect())
}

impl ApiClient {
    pub async fn sign_out(&self) -> Result<()> {
        let url = self.root.join("/Sessions/Logout")?;
        self.client.post(url).send().await?;
        Ok(())
    }
}
