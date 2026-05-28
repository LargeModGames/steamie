use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub steam_id: Option<String>,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct AuthConfig {
    #[serde(default)]
    pub method: AuthMethod,
    #[serde(default)]
    pub account_name: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    #[default]
    Qr,
    Credentials,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    #[serde(default = "default_tick_rate")]
    pub tick_rate_ms: u64,
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: default_tick_rate(),
            theme: default_theme(),
        }
    }
}

fn default_tick_rate() -> u64 {
    250
}

fn default_theme() -> String {
    "dark".to_owned()
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();
        let raw = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "vapour: no config found at {}\n\nCreate it (all fields optional):\n\n  # api_key = \"...\"  # https://steamcommunity.com/dev/apikey (enables library & achievements)\n  # steam_id = \"...\" # auto-detected after login if omitted\n",
                path.display()
            )
        })?;
        toml::from_str(&raw).with_context(|| format!("invalid config at {}", path.display()))
    }

    pub fn load_from(path: PathBuf) -> Result<Self> {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("could not read config at {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("invalid config at {}", path.display()))
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("vapour")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::{AuthMethod, Config};
    use anyhow::Result;

    #[test]
    fn config_defaults_auth_to_qr() -> Result<()> {
        let config: Config = toml::from_str("")?;
        assert_eq!(config.auth.method, AuthMethod::Qr);
        assert_eq!(config.auth.account_name, None);
        assert_eq!(config.api_key, None);
        assert_eq!(config.steam_id, None);
        Ok(())
    }

    #[test]
    fn config_parses_optional_api_key_and_steam_id() -> Result<()> {
        let config: Config = toml::from_str(
            r#"
api_key = "key"
steam_id = "76561198000000000"
"#,
        )?;
        assert_eq!(config.api_key.as_deref(), Some("key"));
        assert_eq!(config.steam_id.as_deref(), Some("76561198000000000"));
        Ok(())
    }

    #[test]
    fn config_parses_credentials_auth() -> Result<()> {
        let config: Config = toml::from_str(
            r#"
[auth]
method = "credentials"
account_name = "alice"
"#,
        )?;

        assert_eq!(config.auth.method, AuthMethod::Credentials);
        assert_eq!(config.auth.account_name.as_deref(), Some("alice"));
        Ok(())
    }
}
