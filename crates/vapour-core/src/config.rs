use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    #[serde(default)]
    pub launch: LaunchConfig,
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

/// Game-launch behaviour (v0.4.0 "Launch Day"). Launches are Steam-mediated by default; the
/// experimental direct path (v0.4.1) starts DRM-free games without waking Steam.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LaunchConfig {
    /// Explicit Steam executable path; empty/omitted means auto-detect.
    #[serde(default)]
    pub steam_path: String,
    /// Log the launch command instead of spawning it (for testing).
    #[serde(default)]
    pub dry_run: bool,
    /// If Vapour started Steam, shut it down once the game exits (best-effort).
    #[serde(default)]
    pub kill_steam_on_exit: bool,
    /// Start Steam quietly (minimized to tray, no window) for Steam-mediated launches. On by
    /// default so the game appears without the Steam UI; set `false` to show Steam's window.
    #[serde(default = "default_true")]
    pub silent: bool,
    /// Experimental: launch games on the DRM-free list directly, with no Steam client running.
    /// Off by default — every launch stays Steam-mediated unless this is enabled.
    #[serde(default)]
    pub direct_launch: bool,
    /// Experimental: with `direct_launch`, also try the direct path for installed games that are
    /// *not* on the DRM-free list. Such games may fail to start (DRM needs Steam). Off by default.
    #[serde(default)]
    pub force_direct: bool,
    /// Arguments appended to every launch (whitespace-separated).
    #[serde(default)]
    pub extra_args: String,
    /// Per-game launch arguments, keyed by appid as a string (e.g. `"730" = "-novid"`).
    #[serde(default)]
    pub game_args: HashMap<String, String>,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            steam_path: String::new(),
            dry_run: false,
            kill_steam_on_exit: false,
            silent: true,
            direct_launch: false,
            force_direct: false,
            extra_args: String::new(),
            game_args: HashMap::new(),
        }
    }
}

impl LaunchConfig {
    /// Build [`LaunchOptions`](crate::launcher::LaunchOptions) for `appid`, merging the global
    /// `extra_args` with this game's `game_args` entry.
    pub fn options_for(&self, appid: u32) -> crate::launcher::LaunchOptions {
        let mut args: Vec<String> = self
            .extra_args
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        if let Some(per_game) = self.game_args.get(&appid.to_string()) {
            args.extend(per_game.split_whitespace().map(str::to_owned));
        }
        let steam_path = Some(self.steam_path.trim())
            .filter(|p| !p.is_empty())
            .map(PathBuf::from);
        crate::launcher::LaunchOptions {
            steam_path,
            dry_run: self.dry_run,
            kill_steam_on_exit: self.kill_steam_on_exit,
            silent: self.silent,
            direct_launch: self.direct_launch,
            force_direct: self.force_direct,
            args,
        }
    }
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
    use super::{AuthMethod, ChatConfig, Config, LaunchConfig};
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

    #[test]
    fn config_defaults_launch_to_auto_detect_no_dry_run() -> Result<()> {
        let config: Config = toml::from_str("")?;
        assert_eq!(config.launch, LaunchConfig::default());
        assert!(config.launch.steam_path.is_empty());
        assert!(!config.launch.dry_run);
        assert!(!config.launch.kill_steam_on_exit);
        // Silent (Steam hidden) is on by default; the direct path is opt-in.
        assert!(config.launch.silent);
        assert!(!config.launch.direct_launch);
        assert!(!config.launch.force_direct);
        assert!(config.launch.game_args.is_empty());
        Ok(())
    }

    #[test]
    fn config_parses_launch_section() -> Result<()> {
        let config: Config = toml::from_str(
            r#"
[launch]
steam_path = "C:/Steam/steam.exe"
dry_run = true
kill_steam_on_exit = true
silent = false
direct_launch = true
force_direct = true
extra_args = "-silent"

[launch.game_args]
"730" = "-novid -high"
"#,
        )?;
        assert_eq!(config.launch.steam_path, "C:/Steam/steam.exe");
        assert!(config.launch.dry_run);
        assert!(config.launch.kill_steam_on_exit);
        assert!(!config.launch.silent);
        assert!(config.launch.direct_launch);
        assert!(config.launch.force_direct);
        assert_eq!(config.launch.extra_args, "-silent");
        assert_eq!(
            config.launch.game_args.get("730").map(String::as_str),
            Some("-novid -high")
        );
        Ok(())
    }

    #[test]
    fn launch_options_merge_global_and_per_game_args() {
        let mut launch = LaunchConfig {
            extra_args: "-silent".to_owned(),
            dry_run: true,
            ..Default::default()
        };
        launch
            .game_args
            .insert("730".to_owned(), "-novid -high".to_owned());

        let opts = launch.options_for(730);
        assert_eq!(opts.args, vec!["-silent", "-novid", "-high"]);
        assert!(opts.dry_run);
        assert!(opts.steam_path.is_none());

        // A game with no per-game entry gets only the global args.
        assert_eq!(launch.options_for(570).args, vec!["-silent"]);
    }

    #[test]
    fn launch_options_use_steam_path_override_when_set() {
        let launch = LaunchConfig {
            steam_path: "  /opt/steam/steam  ".to_owned(),
            ..Default::default()
        };
        assert_eq!(
            launch.options_for(1).steam_path,
            Some(std::path::PathBuf::from("/opt/steam/steam"))
        );
    }
}
