use relm4::adw;
use serde::{Deserialize, Serialize};
use unic_langid::LanguageIdentifier;
use uuid::Uuid;

use crate::jellyfin_api::models::collection_sort::CollectionSort;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
enum InnerTheme {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct Theme(Option<InnerTheme>);

pub static THEME_LIGHT: Theme = Theme(Some(InnerTheme::Light));
pub static THEME_DARK: Theme = Theme(Some(InnerTheme::Dark));

impl From<Theme> for u32 {
    fn from(value: Theme) -> Self {
        match value.0 {
            None => 0,
            Some(InnerTheme::Light) => 1,
            Some(InnerTheme::Dark) => 2,
        }
    }
}

impl From<u32> for Theme {
    fn from(value: u32) -> Self {
        match value {
            0 => Theme(None),
            1 => THEME_LIGHT,
            2 => THEME_DARK,
            _ => unreachable!("theme index {value} does not exist"),
        }
    }
}

impl From<Theme> for adw::ColorScheme {
    fn from(val: Theme) -> Self {
        match val.0 {
            None => adw::ColorScheme::Default,
            Some(InnerTheme::Light) => adw::ColorScheme::ForceLight,
            Some(InnerTheme::Dark) => adw::ColorScheme::ForceDark,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MostRecentLogin {
    pub server_id: Uuid,
    pub account_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GeneralConfig {
    pub language: Option<LanguageIdentifier>,
    pub theme: Theme,
    pub most_recent_login: Option<MostRecentLogin>,
    pub restore_most_recent_login: bool,
    pub use_episode_image: bool,
    pub collection_sort: CollectionSort,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            language: Option::default(),
            theme: Theme::default(),
            most_recent_login: Option::default(),
            restore_most_recent_login: true,
            use_episode_image: false,
            collection_sort: CollectionSort::default(),
        }
    }
}

impl GeneralConfig {
    pub fn theme(&self) -> Theme {
        self.theme
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        adw::StyleManager::default().set_color_scheme(self.theme.into());
    }
}
