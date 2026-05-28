use vapour_api::SteamApiClient;

use crate::config::Config;

pub struct Session {
    pub client: SteamApiClient,
    pub config: Config,
}

impl Session {
    pub fn new(config: Config) -> Self {
        let client = SteamApiClient::new(
            config.api_key.clone(),
            config.steam_id.clone(),
        );
        Self { client, config }
    }
}
