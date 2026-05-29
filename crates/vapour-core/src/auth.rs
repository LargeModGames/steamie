use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthState {
    pub account_name: String,
    pub refresh_token: String,
}

impl AuthState {
    pub fn load() -> Result<Option<Self>> {
        Self::load_from(auth_state_path())
    }

    pub fn load_from(path: PathBuf) -> Result<Option<Self>> {
        match fs::read_to_string(&path) {
            Ok(raw) => toml::from_str(&raw)
                .with_context(|| format!("invalid auth state at {}", path.display()))
                .map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error)
                .with_context(|| format!("could not read auth state at {}", path.display())),
        }
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(auth_state_path())
    }

    pub fn save_to(&self, path: PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("could not create auth state dir {}", parent.display()))?;
        }

        let raw = toml::to_string(self).context("could not serialize auth state")?;
        let mut options = OpenOptions::new();
        options.create(true).write(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let mut file = options
            .open(&path)
            .with_context(|| format!("could not open auth state {}", path.display()))?;
        file.write_all(raw.as_bytes())
            .with_context(|| format!("could not write auth state {}", path.display()))?;
        file.sync_all()
            .with_context(|| format!("could not sync auth state {}", path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).with_context(|| {
                format!("could not set auth state permissions on {}", path.display())
            })?;
        }

        Ok(())
    }

    pub fn delete() -> Result<()> {
        Self::delete_at(auth_state_path())
    }

    pub fn delete_at(path: PathBuf) -> Result<()> {
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error)
                .with_context(|| format!("could not delete auth state {}", path.display())),
        }
    }
}

pub fn auth_state_path() -> PathBuf {
    dirs::state_dir()
        .or_else(dirs::config_dir)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("vapour")
        .join("auth.toml")
}

#[cfg(test)]
mod tests {
    use super::AuthState;
    use anyhow::Result;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn auth_state_round_trips_and_deletes() -> Result<()> {
        let dir = unique_test_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("auth.toml");
        let state = AuthState {
            account_name: "alice".to_owned(),
            refresh_token: "refresh-token".to_owned(),
        };

        state.save_to(path.clone())?;
        let loaded = AuthState::load_from(path.clone())?;
        assert_eq!(loaded, Some(state));

        AuthState::delete_at(path.clone())?;
        assert_eq!(AuthState::load_from(path)?, None);

        std::fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn auth_state_is_saved_with_private_permissions() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let dir = unique_test_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("auth.toml");
        let state = AuthState {
            account_name: "alice".to_owned(),
            refresh_token: "refresh-token".to_owned(),
        };

        state.save_to(path.clone())?;
        let mode = std::fs::metadata(&path)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        std::fs::remove_file(path)?;
        std::fs::remove_dir_all(dir)?;
        Ok(())
    }

    fn unique_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "vapour-core-auth-tests-{}-{}",
            std::process::id(),
            nanos
        ))
    }
}
