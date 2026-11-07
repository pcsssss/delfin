pub mod general;
mod migrate;
mod versions;
pub mod video_player_config;

use std::{fs, path::PathBuf};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use self::{
    general::GeneralConfig, migrate::ConfigVersions, video_player_config::VideoPlayerConfig,
};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Config {
    version: usize,
    #[serde(default)]
    pub window: Window,
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub video_player: VideoPlayerConfig,
    pub servers: Vec<Server>,
    /// Nonsecret Seerr connection only; authentication is never persisted here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seerr: Option<crate::seerr::ConnectionConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 2,
            window: Window::default(),
            general: GeneralConfig::default(),
            video_player: VideoPlayerConfig::default(),
            servers: Vec::default(),
            seerr: None,
        }
    }
}

impl Config {
    pub fn new() -> Result<Self> {
        let config_file = match get_config_file_exists() {
            Some(f) => f,
            None => return Ok(Self::default()),
        };
        let config = fs::read_to_string(config_file)?;
        let config_version = ConfigVersions::new(&config)?;
        Ok(config_version.into())
    }

    pub fn save(&self) -> Result<()> {
        let config_file = match get_config_file(true) {
            Some(f) => f,
            None => return Err(anyhow!("Error getting config file")),
        };
        let toml = toml::to_string_pretty(&self)?;
        fs::write(config_file, toml)?;
        Ok(())
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(&self)?)
    }
}

fn get_config_file(create_dir: bool) -> Option<PathBuf> {
    let config_dir = match dirs::config_dir() {
        Some(d) => d,
        None => return None,
    };

    let dir = config_dir.join("delfin");

    if create_dir {
        fs::create_dir_all(&dir).expect("Error creating config directory");
    }

    let config_file = dir.join("config.toml");

    Some(config_file)
}

fn get_config_file_exists() -> Option<PathBuf> {
    let config_file = match get_config_file(false) {
        Some(f) => f,
        None => return None,
    };

    if !config_file
        .try_exists()
        .expect("Error checking if config file exists.")
    {
        return None;
    }

    Some(config_file)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Server {
    pub id: Uuid,
    pub url: String,
    pub name: String,
    pub accounts: Vec<Account>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Account {
    pub id: Uuid,
    pub username: String,
    // TODO: move to keyring
    pub access_token: String,
    pub device_id: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Window {
    pub width: usize,
    pub height: usize,
    pub maximized: bool,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            width: 960,
            height: 540,
            maximized: false,
        }
    }
}
