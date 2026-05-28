use anyhow::{Context, Result};
use serde_json::Value;

use crate::models::{Achievement, AppDetails, Game, NewsItem, PlayerSummary, WishlistItem};

const STEAM_API_BASE: &str = "https://api.steampowered.com";
const STORE_API_BASE: &str = "https://store.steampowered.com";

pub struct SteamApiClient {
    http: reqwest::Client,
    api_key: String,
    pub steam_id: String,
}

impl SteamApiClient {
    pub fn new(api_key: String, steam_id: String) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("Vapour/0.1.0")
            .build()
            .expect("failed to build HTTP client");
        Self { http, api_key, steam_id }
    }

    pub async fn get_owned_games(&self) -> Result<Vec<Game>> {
        let url = format!("{STEAM_API_BASE}/IPlayerService/GetOwnedGames/v1/");
        let resp: Value = self
            .http
            .get(&url)
            .query(&[
                ("key", self.api_key.as_str()),
                ("steamid", self.steam_id.as_str()),
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
        let url = format!("{STEAM_API_BASE}/ISteamUser/GetFriendList/v1/");
        let resp: Value = self
            .http
            .get(&url)
            .query(&[
                ("key", self.api_key.as_str()),
                ("steamid", self.steam_id.as_str()),
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

        let url = format!("{STEAM_API_BASE}/ISteamUser/GetPlayerSummaries/v2/");
        let mut all = Vec::new();

        for chunk in steam_ids.chunks(100) {
            let ids_str = chunk.join(",");
            let resp: Value = self
                .http
                .get(&url)
                .query(&[
                    ("key", self.api_key.as_str()),
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
        let url = format!("{STEAM_API_BASE}/ISteamUserStats/GetPlayerAchievements/v1/");
        let resp: Value = self
            .http
            .get(&url)
            .query(&[
                ("key", self.api_key.as_str()),
                ("steamid", self.steam_id.as_str()),
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

    pub async fn get_wishlist(&self) -> Result<Vec<WishlistItem>> {
        let url = format!(
            "{STORE_API_BASE}/wishlist/profiles/{}/wishlistdata/",
            self.steam_id
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
