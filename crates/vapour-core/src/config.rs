use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub steam_id: Option<String>,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub chat: ChatConfig,
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

/// Chat behaviour: notifications and local history retention.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ChatConfig {
    /// Ring the terminal bell on an incoming message.
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    /// Also raise a desktop notification (via `notify-rust`). Off by default.
    #[serde(default)]
    pub desktop_notifications: bool,
    /// Days of locally-cached history to keep. `0` means keep everything.
    #[serde(default = "default_history_retention_days")]
    pub history_retention_days: u32,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
            desktop_notifications: false,
            history_retention_days: default_history_retention_days(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_history_retention_days() -> u32 {
    30
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();

        match std::fs::read_to_string(&path) {
            Ok(raw) => toml::from_str(&raw)
                .with_context(|| format!("invalid config at {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => {
                Err(e).with_context(|| format!("could not read config at {}", path.display()))
            }
        }
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
    use super::{AuthMethod, ChatConfig, Config};
    use anyhow::Result;

    #[test]
    fn config_defaults_chat_to_bell_on_30_day_retention() -> Result<()> {
        let config: Config = toml::from_str("")?;
        assert_eq!(config.chat, ChatConfig::default());
        assert!(config.chat.notifications_enabled);
        assert!(!config.chat.desktop_notifications);
        assert_eq!(config.chat.history_retention_days, 30);
        Ok(())
    }

    #[test]
    fn config_parses_chat_section() -> Result<()> {
        let config: Config = toml::from_str(
            r#"
[chat]
notifications_enabled = false
desktop_notifications = true
history_retention_days = 7
"#,
        )?;
        assert!(!config.chat.notifications_enabled);
        assert!(config.chat.desktop_notifications);
        assert_eq!(config.chat.history_retention_days, 7);
        Ok(())
    }

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
