use std::sync::{Arc, Mutex};

use tokio::task::JoinSet;
use vapour_api::SteamApiClient;
use vapour_protocol::RunCommand;

use crate::app::App;
use crate::io_event::IoEvent;

pub async fn handle_io(app: Arc<Mutex<App>>, client: Arc<SteamApiClient>, event: IoEvent) {
    match event {
        IoEvent::LoadLibrary => {
            app.lock().unwrap().loading.library = true;

            if let Some(tx) = app.lock().unwrap().friend_cmd_tx.clone() {
                let _ = tx.send(RunCommand::GetLibrary);
                return;
            }

            match client.get_owned_games().await {
                Ok(games) => {
                    let mut a = app.lock().unwrap();
                    let len = games.len();
                    a.games = games;
                    a.filtered_games = (0..len).collect();
                    a.loading.library = false;
                }
                Err(e) => {
                    let mut a = app.lock().unwrap();
                    a.loading.library = false;
                    // Missing api_key or steam_id is expected in protocol-only mode;
                    // don't surface it as a blocking error.
                    if !e.to_string().contains("api_key") && !e.to_string().contains("steam_id") {
                        a.set_error(e.to_string());
                    }
                }
            }
        }

        IoEvent::LoadFriendIds => {
            app.lock().unwrap().loading.friends = true;
            match client.get_friend_list().await {
                Err(e) => {
                    let mut a = app.lock().unwrap();
                    a.loading.friends = false;
                    if !e.to_string().contains("api_key") {
                        a.set_error(e.to_string());
                    }
                }
                Ok(ids) => {
                    let has_ids = !ids.is_empty();
                    {
                        let mut a = app.lock().unwrap();
                        a.friend_ids = ids;
                        a.friends.clear();
                    }
                    if has_ids {
                        // Chain: kick off page 0 immediately
                        let tx = app.lock().unwrap().io_tx.clone();
                        let _ = tx.send(IoEvent::LoadFriendPage(0));
                    } else {
                        app.lock().unwrap().loading.friends = false;
                    }
                }
            }
        }

        IoEvent::LoadFriendPage(page) => {
            let chunk: Vec<String> = {
                let a = app.lock().unwrap();
                a.friend_ids
                    .iter()
                    .skip(page * 100)
                    .take(100)
                    .cloned()
                    .collect()
            };

            if chunk.is_empty() {
                app.lock().unwrap().loading.friends = false;
                return;
            }

            match client.get_player_summaries(&chunk).await {
                Err(e) => set_error(&app, e.to_string()),
                Ok(summaries) => {
                    let next_page = {
                        let mut a = app.lock().unwrap();
                        a.friends.extend(summaries);
                        a.friends.sort_by_key(|p| {
                            if p.is_in_game() {
                                0u8
                            } else if p.personastate > 0 {
                                1
                            } else {
                                2
                            }
                        });
                        a.loading.friends = false;
                        let loaded = a.friends.len();
                        let total = a.friend_ids.len();
                        if loaded < total { Some(page + 1) } else { None }
                    };

                    if let Some(next) = next_page {
                        // Chain: send next page back through the channel,
                        // same pattern as LoadFriendIds → LoadFriendPage(0).
                        let tx = app.lock().unwrap().io_tx.clone();
                        let _ = tx.send(IoEvent::LoadFriendPage(next));
                    }
                }
            }
        }

        IoEvent::LoadWishlist => {
            app.lock().unwrap().loading.wishlist = true;
            match client.get_wishlist().await {
                Ok(items) => {
                    let mut a = app.lock().unwrap();
                    a.wishlist = items;
                    a.loading.wishlist = false;
                }
                Err(e) => set_error(&app, e.to_string()),
            }
        }

        IoEvent::LoadNews => {
            app.lock().unwrap().loading.news = true;

            let appids: Vec<u32> = {
                let a = app.lock().unwrap();
                let mut appids: Vec<u32> = a.games.iter().take(20).map(|g| g.appid).collect();
                if appids.is_empty() {
                    appids = a.recently_played_appids.iter().take(20).copied().collect();
                }
                if appids.is_empty() {
                    appids = a.wishlist.iter().take(20).map(|item| item.appid).collect();
                }
                appids
            };

            if appids.is_empty() {
                let mut a = app.lock().unwrap();
                a.news_feed.clear();
                a.loading.news = false;
                return;
            }

            // Fetch each game's news in its own spawned task (no lifetime issues).
            let mut set = JoinSet::new();
            for appid in appids {
                let c = Arc::clone(&client);
                set.spawn(async move { c.get_news(appid, 5).await });
            }

            let mut all_news = vec![];
            while let Some(result) = set.join_next().await {
                if let Ok(Ok(items)) = result {
                    all_news.extend(items);
                }
            }

            all_news.sort_by(|a, b| b.date.cmp(&a.date));
            all_news.truncate(60);

            let mut a = app.lock().unwrap();
            a.news_feed = all_news;
            a.loading.news = false;
        }

        IoEvent::LoadGameDetail(appid) => {
            app.lock().unwrap().loading.game_detail = true;
            match client.get_app_details(appid).await {
                Ok(details) => {
                    let mut a = app.lock().unwrap();
                    a.selected_game = details;
                    a.loading.game_detail = false;
                }
                Err(e) => set_error(&app, e.to_string()),
            }
        }

        IoEvent::LoadAchievements(appid) => {
            // Prefer the protocol path when connected (no api_key needed).
            let cmd_tx = app.lock().unwrap().friend_cmd_tx.clone();
            if let Some(tx) = cmd_tx {
                let _ = tx.send(RunCommand::GetPlayerAchievements(appid));
                // Result arrives via FriendsEvent::PlayerAchievements in protocol.rs.
                return;
            }

            // Fall back to Web API (requires api_key + steam_id).
            match client.get_achievements(appid).await {
                Ok(mut achs) => {
                    achs.sort_by(|a, b| {
                        b.achieved
                            .cmp(&a.achieved)
                            .then(a.display_name().cmp(b.display_name()))
                    });
                    let mut a = app.lock().unwrap();
                    a.achievements = achs;
                }
                Err(_) => {
                    app.lock().unwrap().achievements.clear();
                }
            }
        }

        IoEvent::RefreshAll => {
            let tx = app.lock().unwrap().io_tx.clone();
            let _ = tx.send(IoEvent::LoadLibrary);
            let _ = tx.send(IoEvent::LoadFriendIds);
            let _ = tx.send(IoEvent::LoadWishlist);
        }

        IoEvent::LookupGameNames(app_ids) => {
            if let Ok(names) = client.get_app_names(&app_ids).await {
                let mut a = app.lock().unwrap();
                a.game_name_cache.extend(names);
                a.update_search();
            }
        }
    }
}

fn set_error(app: &Arc<Mutex<App>>, msg: String) {
    app.lock().unwrap().set_error(msg);
}
