use anyhow::Result;
use vapour_api::SteamApiClient;
use vapour_protocol::SteamClient;

use crate::{
    auth::AuthState,
    config::{AuthMethod, Config},
};

pub struct Session {
    pub api_client: SteamApiClient,
    pub protocol_client: SteamClient,
    pub config: Config,
    pub stored_auth: Option<AuthState>,
}

impl Session {
    pub fn new(config: Config) -> Result<Self> {
        let api_client = SteamApiClient::new(config.api_key.clone(), config.steam_id.clone());
        let stored_auth = AuthState::load()?;
        let mut protocol_client = SteamClient::new();

        if let Some(account_name) = config
            .auth
            .account_name
            .clone()
            .or_else(|| stored_auth.as_ref().map(|auth| auth.account_name.clone()))
        {
            protocol_client.set_account_name_hint(account_name);
        }

        Ok(Self {
            api_client,
            protocol_client,
            config,
            stored_auth,
        })
    }

    pub fn preferred_auth_method(&self) -> AuthMethod {
        self.config.auth.method
    }

    pub fn stored_auth(&self) -> Option<&AuthState> {
        self.stored_auth.as_ref()
    }

    pub fn save_auth(&mut self, auth: AuthState) -> Result<()> {
        auth.save()?;
        self.protocol_client
            .set_account_name_hint(auth.account_name.clone());
        self.stored_auth = Some(auth);
        Ok(())
    }

    pub fn clear_auth(&mut self) -> Result<()> {
        AuthState::delete()?;
        self.stored_auth = None;
        Ok(())
    }
}
