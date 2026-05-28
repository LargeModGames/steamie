use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result, anyhow};
use serde_json::Value;

use crate::models::{Achievement, AppDetails, Game, NewsItem, PlayerSummary, WishlistItem};

const STEAM_API_BASE: &str = "https://api.steampowered.com";
const STORE_API_BASE: &str = "https://store.steampowered.com";

/// The `steam_id` field is wrapped in `Arc<RwLock>` so that a clone of this
/// client (held by the network task) automatically sees the value written by
/// the protocol task after login, without requiring `Arc<Mutex<SteamApiClient>>`.
#[derive(Clone)]
pub struct SteamApiClient {
    http: reqwest::Client,
    api_key: Option<String>,
    steam_id: Arc<RwLock<Option<String>>>,
}

impl SteamApiClient {
    pub fn new(api_key: Option<String>, steam_id: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("Vapour/0.1.0")
            .build()
            .expect("failed to build HTTP client");
        Self { http, api_key, steam_id: Arc::new(RwLock::new(steam_id)) }
    }

    /// Called by the protocol task after login to fill in the user's SteamID.
    /// Because `steam_id` is an `Arc<RwLock<…>>`, all clones of this client
    /// (including the one held by the network dispatch thread) see the update.
    pub fn set_steam_id(&self, id: String) {
        *self.steam_id.write().unwrap() = Some(id);
    }

    fn require_api_key(&self) -> Result<&str> {
        self.api_key
            .as_deref()
            .ok_or_else(|| anyhow!("this feature requires an api_key in your config (~/.config/vapour/config.toml)"))
    }

    fn require_steam_id(&self) -> Result<String> {
        self.steam_id
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow!("steam_id not yet available — still connecting"))
    }

    pub async fn get_owned_games(&self) -> Result<Vec<Game>> {
        let api_key = self.require_api_key()?;
        let steam_id = self.require_steam_id()?;
        let url = format!("{STEAM_API_BASE}/IPlayerService/GetOwnedGames/v1/");
        let resp: Value = self
            .http
            .get(&url)
            .query(&[
                ("key", api_key),
                ("steamid", steam_id.as_str()),
                ("include_appinfo", "1"),
                ("include_played_free_games", "1"),
                ("format", "json"),
            ])
            .send()
            .await
            .context("GET GetOwnedGames")?
            .json()
            .await
            .context("parse GetOwnedGames")?;

        let games: Vec<Game> = serde_json::from_value(
            resp["response"]["games"].clone(),
        )
        .unwrap_or_default();

        let mut games = games;
        games.sort_by(|a, b| b.playtime_forever.cmp(&a.playtime_forever));
        Ok(games)
    }

    pub async fn get_friend_list(&self) -> Result<Vec<String>> {
        let api_key = self.require_api_key()?;
        let steam_id = self.require_steam_id()?;
        let url = format!("{STEAM_API_BASE}/ISteamUser/GetFriendList/v1/");
        let resp: Value = self
            .http
            .get(&url)
            .query(&[
                ("key", api_key),
                ("steamid", steam_id.as_str()),
                ("relationship", "friend"),
            ])
            .send()
            .await
            .context("GET GetFriendList")?
            .json()
            .await
            .context("parse GetFriendList")?;

        let ids = resp["friendslist"]["friends"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| f["steamid"].as_str().map(|s| s.to_owned()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(ids)
    }

    pub async fn get_player_summaries(&self, steam_ids: &[String]) -> Result<Vec<PlayerSummary>> {
        if steam_ids.is_empty() {
            return Ok(vec![]);
        }

        let api_key = self.require_api_key()?;
        let url = format!("{STEAM_API_BASE}/ISteamUser/GetPlayerSummaries/v2/");
        let mut all = Vec::new();

        for chunk in steam_ids.chunks(100) {
            let ids_str = chunk.join(",");
            let resp: Value = self
                .http
                .get(&url)
                .query(&[
                    ("key", api_key),
                    ("steamids", ids_str.as_str()),
                ])
                .send()
                .await
                .context("GET GetPlayerSummaries")?
                .json()
                .await
                .context("parse GetPlayerSummaries")?;

            let players: Vec<PlayerSummary> =
                serde_json::from_value(resp["response"]["players"].clone())
                    .unwrap_or_default();
            all.extend(players);
        }

        Ok(all)
    }

    pub async fn get_achievements(&self, appid: u32) -> Result<Vec<Achievement>> {
        let api_key = self.require_api_key()?;
        let steam_id = self.require_steam_id()?;
        let url = format!("{STEAM_API_BASE}/ISteamUserStats/GetPlayerAchievements/v1/");
        let resp: Value = self
            .http
            .get(&url)
            .query(&[
                ("key", api_key),
                ("steamid", steam_id.as_str()),
                ("appid", appid.to_string().as_str()),
                ("l", "english"),
            ])
            .send()
            .await
            .context("GET GetPlayerAchievements")?
            .json()
            .await
            .context("parse GetPlayerAchievements")?;

        if resp["playerstats"]["success"].as_bool() == Some(false) {
            return Ok(vec![]);
        }

        let achievements: Vec<Achievement> =
            serde_json::from_value(resp["playerstats"]["achievements"].clone())
                .unwrap_or_default();

        Ok(achievements)
    }

    pub async fn get_app_details(&self, appid: u32) -> Result<Option<AppDetails>> {
        let url = format!("{STORE_API_BASE}/api/appdetails");
        let resp: Value = self
            .http
            .get(&url)
            .query(&[
                ("appids", appid.to_string().as_str()),
                ("cc", "us"),
                ("l", "english"),
            ])
            .send()
            .await
            .context("GET appdetails")?
            .json()
            .await
            .context("parse appdetails")?;

        let entry = &resp[appid.to_string()];
        if entry["success"].as_bool() != Some(true) {
            return Ok(None);
        }

        let details: AppDetails = serde_json::from_value(entry["data"].clone())
            .context("deserialize AppDetails")?;
        Ok(Some(details))
    }

    pub async fn get_news(&self, appid: u32, count: u32) -> Result<Vec<NewsItem>> {
        let url = format!("{STEAM_API_BASE}/ISteamNews/GetNewsForApp/v2/");
        let resp: Value = self
            .http
            .get(&url)
            .query(&[
                ("appid", appid.to_string().as_str()),
                ("count", count.to_string().as_str()),
                ("maxlength", "500"),
                ("format", "json"),
            ])
            .send()
            .await
            .context("GET GetNewsForApp")?
            .json()
            .await
            .context("parse GetNewsForApp")?;

        let items: Vec<NewsItem> =
            serde_json::from_value(resp["appnews"]["newsitems"].clone()).unwrap_or_default();
        Ok(items)
    }

    /// Fetch display names for a batch of app IDs using the Store API.
    /// No API key required. Returns only entries that succeeded.
    pub async fn get_app_names(&self, app_ids: &[u32]) -> Result<HashMap<u32, String>> {
        if app_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let ids_str = app_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
        let url = format!("{STORE_API_BASE}/api/appdetails");
        let resp: Value = self
            .http
            .get(&url)
            .query(&[("appids", ids_str.as_str()), ("filters", "basic")])
            .send()
            .await
            .context("GET appdetails batch")?
            .json()
            .await
            .context("parse appdetails batch")?;

        let mut names = HashMap::new();
        if let Some(obj) = resp.as_object() {
            for (key, entry) in obj {
                if let Ok(app_id) = key.parse::<u32>() {
                    if entry["success"].as_bool() == Some(true) {
                        if let Some(name) = entry["data"]["name"].as_str() {
                            names.insert(app_id, name.to_owned());
                        }
                    }
                }
            }
        }
        Ok(names)
    }

    pub async fn get_wishlist(&self) -> Result<Vec<WishlistItem>> {
        let steam_id = self.require_steam_id()?;
        let url = format!(
            "{STORE_API_BASE}/wishlist/profiles/{}/wishlistdata/",
            steam_id
        );
        let text = self
            .http
            .get(&url)
            .query(&[("p", "0")])
            .send()
            .await
            .context("GET wishlist")?
            .text()
            .await
            .context("read wishlist response")?;

        // Private wishlist or unauthenticated → Steam returns HTML or {"success":2}
        let resp: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => return Ok(vec![]), // HTML redirect / login page
        };

        // Not a map of games → empty or private
        let obj = match resp.as_object() {
            Some(o) => o,
            None => return Ok(vec![]),
        };

        // {"success": N} with no game keys → empty wishlist
        let mut items: Vec<WishlistItem> = obj
            .iter()
            .filter_map(|(appid_str, v)| {
                let appid: u32 = appid_str.parse().ok()?;
                // Skip non-game keys like "success"
                v.as_object()?;
                Some(WishlistItem {
                    appid,
                    name: v["name"].as_str().unwrap_or("").to_owned(),
                    priority: v["priority"].as_u64().unwrap_or(0) as u32,
                    added: v["added"].as_u64().unwrap_or(0),
                    capsule: v["capsule"].as_str().map(|s| s.to_owned()),
                })
            })
            .collect();

        items.sort_by_key(|i| i.priority);
        Ok(items)
    }
}
