use anyhow::Result;

use super::{versions::config_v1::ConfigV1, Config};

pub(crate) trait Migrate<T> {
    fn migrate(self) -> T;
}

pub(crate) enum ConfigVersions {
    V1(ConfigV1),
    V2(Config),
}

impl From<ConfigVersions> for Config {
    fn from(val: ConfigVersions) -> Self {
        match val {
            ConfigVersions::V1(config) => config.migrate(),
            ConfigVersions::V2(config) => config,
        }
    }
}

impl ConfigVersions {
    pub(crate) fn new(config: &str) -> Result<Self> {
        let config_table = config.parse::<toml::Table>()?;
        let version = config_table.get("version").and_then(|val| val.as_integer());
        Ok(match version {
            Some(1) => Self::V1(toml::from_str(config)?),
            Some(2) => Self::V2(toml::from_str(config)?),
            // V1 is missing version field, default to V1
            _ => Self::V1(toml::from_str(config)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use uuid::uuid;

    use crate::config::{
        general::{GeneralConfig, THEME_DARK},
        video_player_config::{VideoPlayerConfig, VideoPlayerOnLeftClick},
        Account, Server, Window,
    };

    use super::*;

    #[test]
    fn collection_sort_survives_config_reload_without_changing_other_settings() -> Result<()> {
        use crate::jellyfin_api::models::collection_sort::CollectionSort;

        let mut config = Config::default();
        config.general.theme = THEME_DARK;
        config.servers.push(Server {
            id: uuid!("bed62d79-11b3-4d1e-b185-cb33a41d889b"),
            name: "Test server".into(),
            url: "https://jellyfin.example.test".into(),
            accounts: vec![Account {
                id: uuid!("88c09387-ec04-412f-a236-5004d6d06869"),
                username: "test".into(),
                access_token: "test-token-not-a-credential".into(),
                device_id: uuid!("ecd59636-b66e-4bff-a721-1d29fd7437fe"),
            }],
        });
        for selected in 0..4 {
            config.general.collection_sort = CollectionSort::from_selected(selected).unwrap();
            let reloaded: Config = ConfigVersions::new(&config.to_toml()?)?.into();
            assert_eq!(reloaded, config);
            assert_eq!(reloaded.general.collection_sort.selected(), selected);
        }
        Ok(())
    }

    #[test]
    fn existing_v2_config_defaults_to_alphabetical_sort() -> Result<()> {
        use crate::jellyfin_api::models::collection_sort::CollectionSort;

        let config: Config =
            ConfigVersions::new("version = 2\nservers = []\n[general]\ntheme = \"Dark\"\n")?.into();
        assert_eq!(
            config.general.collection_sort,
            CollectionSort::NameAscending
        );
        assert_eq!(config.general.theme, THEME_DARK);
        assert!(ConfigVersions::new(
            "version = 2\nservers = []\n[general]\ncollection_sort = \"invalid\"\n"
        )
        .is_err());
        Ok(())
    }

    #[test]
    fn test_migrate_v1() -> Result<()> {
        let input = r##"
device_id = "ecd59636-b66e-4bff-a721-1d29fd7437fe"

[window]
width = 967
height = 670
maximized = true

[[servers]]
id = "bed62d7911b34d1eb185cb33a41d889b"
url = "https://jellyfin.localhost"
name = "localhost"

[[servers.accounts]]
id = "88c09387ec04412fa2365004d6d06869"
username = "foo"
access_token = "ad5348ff6a304ce59404d1c99d5d1afc"

[[servers]]
id = "28a2bc08c2424460a493030e5048d1f6"
url = "http://localhost:1234"
name = "thingy"

[[servers.accounts]]
id = "90d328046d474a9dbe9a34ff04daa582"
username = "bar"
access_token = "f07087a794044a2db1c66614dad03327"

[general]
theme = "Dark"

[video_player]
position_update_frequency = 10
volume = 0.0
muted = true
skip_backwards_amount = 10
skip_forwards_amount = 10
on_left_click = "ToggleControls"
subtitle_scale = 1.0
subtitle_colour = "#FFFFFFFF"
subtitle_background_colour = "#00000000"
subtitle_position = 100
intro_skipper = true
intro_skipper_auto_skip = true
jellyscrub = true
backend = "Mpv"
hls_playback = false

[video_player.subtitle_font]
family = "Sans"
size = 55
bold = false
italic = false
        "##;

        let config_version = ConfigVersions::new(input)?;

        assert!(
            matches!(config_version, ConfigVersions::V1(_)),
            "original config was not version 1"
        );

        let config: Config = config_version.into();

        let expected = Config {
            version: 2,
            seerr: None,
            window: Window {
                width: 967,
                height: 670,
                maximized: true,
            },
            servers: vec![
                Server {
                    id: uuid!("bed62d7911b34d1eb185cb33a41d889b"),
                    url: "https://jellyfin.localhost".into(),
                    name: "localhost".into(),
                    accounts: vec![Account {
                        id: uuid!("88c09387ec04412fa2365004d6d06869"),
                        username: "foo".into(),
                        access_token: "ad5348ff6a304ce59404d1c99d5d1afc".into(),
                        device_id: uuid!("ecd59636-b66e-4bff-a721-1d29fd7437fe"),
                    }],
                },
                Server {
                    id: uuid!("28a2bc08c2424460a493030e5048d1f6"),
                    url: "http://localhost:1234".into(),
                    name: "thingy".into(),
                    accounts: vec![Account {
                        id: uuid!("90d328046d474a9dbe9a34ff04daa582"),
                        username: "bar".into(),
                        access_token: "f07087a794044a2db1c66614dad03327".into(),
                        device_id: uuid!("ecd59636-b66e-4bff-a721-1d29fd7437fe"),
                    }],
                },
            ],
            general: GeneralConfig {
                language: None,
                theme: THEME_DARK,
                most_recent_login: None,
                ..Default::default()
            },
            video_player: VideoPlayerConfig {
                volume: 0.0,
                muted: true,
                skip_forwards_amount: 10,
                on_left_click: VideoPlayerOnLeftClick::ToggleControls,
                ..Default::default()
            },
        };

        assert_eq!(config, expected);

        Ok(())
    }
}
