use std::sync::{Arc, Mutex};

use crate::io_event::IoEvent;
use tokio::{
    sync::mpsc,
    time::{Duration, sleep},
};
use vapour_api::{Achievement, Game};
use vapour_core::{AuthMethod as ConfigAuthMethod, AuthState, Session};
use vapour_protocol::{
    AuthEvent, AuthMethod, Error as ProtocolError, FriendsEvent, GuardKind, LoggedOn, Persona,
    RunCommand,
};

use crate::app::App;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolGuardKind {
    EmailCode,
    DeviceCode,
    DeviceConfirmation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolStatus {
    Disconnected,
    Connecting,
    AwaitingQrScan { qr_url: String },
    AwaitingGuardCode { kind: ProtocolGuardKind },
    LoggedOn { account_name: String },
    Failed(String),
}

impl ProtocolStatus {
    pub fn modal_visible(&self) -> bool {
        matches!(
            self,
            Self::Connecting | Self::AwaitingQrScan { .. } | Self::AwaitingGuardCode { .. }
        )
    }

    pub fn accepts_text_input(&self) -> bool {
        matches!(
            self,
            Self::AwaitingGuardCode {
                kind: ProtocolGuardKind::EmailCode | ProtocolGuardKind::DeviceCode,
            }
        )
    }
}

#[derive(Debug)]
pub enum ProtocolCommand {
    SubmitGuardCode(String),
    Cancel,
}

#[derive(Debug)]
pub struct ProtocolBootstrap {
    pub primary: AuthMethod,
    pub fallback: Option<AuthMethod>,
}

pub fn spawn_protocol_task(
    app: Arc<Mutex<App>>,
    mut session: Session,
    bootstrap: ProtocolBootstrap,
    mut command_rx: mpsc::UnboundedReceiver<ProtocolCommand>,
) {
    tokio::spawn(async move {
        let result = run_protocol_task(&app, &mut session, bootstrap, &mut command_rx).await;
        if let Err(error) = result {
            set_status(&app, ProtocolStatus::Failed(error.to_string()));
        }
        // Drop the command sender and clear loading flags so the UI doesn't get
        // stuck if run() exits early (connection error, CM disconnect, etc.).
        {
            let mut app = app.lock().unwrap();
            app.friend_cmd_tx = None;
            app.loading.library = false;
        }
    });
}

async fn run_protocol_task(
    app: &Arc<Mutex<App>>,
    session: &mut Session,
    bootstrap: ProtocolBootstrap,
    command_rx: &mut mpsc::UnboundedReceiver<ProtocolCommand>,
) -> anyhow::Result<()> {
    set_status(app, ProtocolStatus::Connecting);

    let logged_on = match drive_auth(app, session, bootstrap.primary.clone(), command_rx).await {
        Ok(logged_on) => logged_on,
        Err(error) => {
            if matches!(bootstrap.primary, AuthMethod::RefreshToken(_)) {
                session.clear_auth()?;
                if let Some(fallback) = bootstrap.fallback {
                    set_status(app, ProtocolStatus::Connecting);
                    drive_auth(app, session, fallback, command_rx).await?
                } else {
                    return Err(error.into());
                }
            } else {
                return Err(error.into());
            }
        }
    };

    session.save_auth(AuthState {
        account_name: logged_on.account_name.clone(),
        refresh_token: logged_on.refresh_token.clone(),
    })?;

    // Propagate the SteamID to the Web API client so key-backed features
    // that still use the Web API (wishlist, news, store) work without steam_id in config.
    session
        .api_client
        .set_steam_id(logged_on.steamid.to_string());

    set_status(
        app,
        ProtocolStatus::LoggedOn {
            account_name: logged_on.account_name.clone(),
        },
    );

    let (run_cmd_tx, run_cmd_rx) = mpsc::unbounded_channel::<RunCommand>();
    let (friends_evt_tx, mut friends_evt_rx) = mpsc::unbounded_channel::<FriendsEvent>();

    {
        let mut app = app.lock().unwrap();
        app.friend_cmd_tx = Some(run_cmd_tx);
        let _ = app.io_tx.send(IoEvent::LoadLibrary);
        let _ = app.io_tx.send(IoEvent::LoadWishlist);
    }

    // Drain events from the protocol run loop into App on a background task.
    let app_friends = Arc::clone(app);
    tokio::spawn(async move {
        while let Some(event) = friends_evt_rx.recv().await {
            let mut app = app_friends.lock().unwrap();
            match event {
                FriendsEvent::PersonaStates(personas) => {
                    merge_personas(&mut app.protocol_friends, personas);
                    queue_game_name_lookups(&mut app);
                }
                FriendsEvent::FriendsList(friends) => {
                    let ids: std::collections::HashSet<u64> =
                        friends.iter().map(|f| f.steamid).collect();
                    app.protocol_friends.retain(|p| ids.contains(&p.steamid));
                }
                FriendsEvent::RecentlyPlayedGames(protocol_games) => {
                    app.recently_played_appids =
                        protocol_games.into_iter().map(|game| game.appid).collect();
                }
                FriendsEvent::OwnedGames(protocol_games) => {
                    let mut games: Vec<Game> = protocol_games
                        .into_iter()
                        .map(|g| Game {
                            appid: g.appid,
                            name: if g.name.is_empty() {
                                None
                            } else {
                                Some(g.name)
                            },
                            playtime_forever: g.playtime_forever.max(0) as u32,
                            img_icon_url: g.img_icon_url,
                            rtime_last_played: Some(g.rtime_last_played as u64),
                        })
                        .collect();
                    sort_library_games(&mut games);
                    app.recently_played_appids = games
                        .iter()
                        .filter(|game| game.rtime_last_played.unwrap_or_default() > 0)
                        .map(|game| game.appid)
                        .collect();
                    let len = games.len();
                    app.games = games;
                    app.filtered_games = (0..len).collect();
                    app.loading.library = false;
                    queue_library_game_name_lookups(&mut app);
                    app.update_search();
                }
                FriendsEvent::PlayerAchievements {
                    achievements: protocol_achs,
                    ..
                } => {
                    let mut achs: Vec<Achievement> = protocol_achs
                        .into_iter()
                        .map(|a| Achievement {
                            apiname: a.apiname,
                            achieved: if a.achieved { 1 } else { 0 },
                            unlocktime: a.unlocktime,
                            name: a.name,
                            description: a.description,
                        })
                        .collect();
                    achs.sort_by(|a, b| {
                        b.achieved
                            .cmp(&a.achieved)
                            .then(a.display_name().cmp(b.display_name()))
                    });
                    app.achievements = achs;
                }
            }
        }
    });

    session
        .protocol_client
        .run(run_cmd_rx, friends_evt_tx)
        .await?;
    Ok(())
}

fn merge_personas(existing: &mut Vec<Persona>, updates: Vec<Persona>) {
    for update in updates {
        if let Some(entry) = existing.iter_mut().find(|p| p.steamid == update.steamid) {
            entry.state = update.state;
            if !update.name.is_empty() {
                entry.name = update.name;
            }
            // Only overwrite game fields when the update explicitly included them.
            // An absent game_played_app_id means "unchanged", not "not in game".
            if update.game_fields_present {
                entry.game_app_id = update.game_app_id;
                entry.game_name = update.game_name;
            }
            if update.avatar_hash.is_some() {
                entry.avatar_hash = update.avatar_hash;
            }
        } else {
            existing.push(update);
        }
    }
}

fn sort_library_games(games: &mut [Game]) {
    games.sort_by(|a, b| {
        b.playtime_forever
            .cmp(&a.playtime_forever)
            .then_with(|| a.name.is_none().cmp(&b.name.is_none()))
            .then_with(|| {
                a.name
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .cmp(&b.name.as_deref().unwrap_or_default().to_lowercase())
            })
            .then_with(|| a.appid.cmp(&b.appid))
    });
}

async fn drive_auth(
    app: &Arc<Mutex<App>>,
    session: &mut Session,
    method: AuthMethod,
    command_rx: &mut mpsc::UnboundedReceiver<ProtocolCommand>,
) -> Result<LoggedOn, ProtocolError> {
    loop {
        let mut events = session.protocol_client.begin_auth(method.clone()).await?;

        while let Some(event) = events.recv().await {
            match event {
                AuthEvent::QrChallenge(qr_url) => {
                    set_status(app, ProtocolStatus::AwaitingQrScan { qr_url });
                }
                AuthEvent::GuardRequired(kind) => {
                    let guard_kind = map_guard_kind(kind.clone());
                    set_status(
                        app,
                        ProtocolStatus::AwaitingGuardCode {
                            kind: guard_kind.clone(),
                        },
                    );

                    if matches!(
                        guard_kind,
                        ProtocolGuardKind::EmailCode | ProtocolGuardKind::DeviceCode
                    ) {
                        match command_rx.recv().await {
                            Some(ProtocolCommand::SubmitGuardCode(code)) => {
                                session.protocol_client.submit_guard_code(code)?;
                                set_status(app, ProtocolStatus::Connecting);
                            }
                            Some(ProtocolCommand::Cancel) => {
                                return Err(ProtocolError::Authentication(
                                    "authentication cancelled".to_owned(),
                                ));
                            }
                            None => {
                                return Err(ProtocolError::Authentication(
                                    "authentication command channel closed".to_owned(),
                                ));
                            }
                        }
                    }
                }
                AuthEvent::Success(logged_on) => return Ok(logged_on),
                AuthEvent::Failure(error) => {
                    if should_retry_auth(&method, &error) {
                        set_status(app, ProtocolStatus::Connecting);
                        sleep(Duration::from_secs(1)).await;
                        break;
                    }
                    return Err(error);
                }
            }
        }

        if should_retry_auth(&method, &ProtocolError::Closed) {
            set_status(app, ProtocolStatus::Connecting);
            sleep(Duration::from_secs(1)).await;
            continue;
        }

        return Err(ProtocolError::Closed);
    }
}

fn set_status(app: &Arc<Mutex<App>>, status: ProtocolStatus) {
    let mut app = app.lock().unwrap();
    app.protocol_status = status;
    if !app.protocol_status.accepts_text_input() {
        app.protocol_input.clear();
    }
}

fn map_guard_kind(kind: GuardKind) -> ProtocolGuardKind {
    match kind {
        GuardKind::EmailCode => ProtocolGuardKind::EmailCode,
        GuardKind::DeviceCode => ProtocolGuardKind::DeviceCode,
        GuardKind::DeviceConfirmation => ProtocolGuardKind::DeviceConfirmation,
    }
}

pub fn build_bootstrap(
    session: &Session,
    credentials: Option<(String, String)>,
) -> ProtocolBootstrap {
    let fallback = match session.preferred_auth_method() {
        ConfigAuthMethod::Qr => Some(AuthMethod::Qr),
        ConfigAuthMethod::Credentials => {
            credentials.map(|(account, password)| AuthMethod::Credentials { account, password })
        }
    };

    if let Some(stored_auth) = session.stored_auth().cloned() {
        ProtocolBootstrap {
            primary: AuthMethod::RefreshToken(stored_auth.refresh_token),
            fallback,
        }
    } else {
        ProtocolBootstrap {
            primary: fallback.unwrap_or(AuthMethod::Qr),
            fallback: None,
        }
    }
}

fn should_retry_auth(method: &AuthMethod, error: &ProtocolError) -> bool {
    matches!(method, AuthMethod::Qr | AuthMethod::Credentials { .. }) && is_closed_error(error)
}

fn queue_game_name_lookups(app: &mut App) {
    use crate::io_event::IoEvent;
    use std::collections::HashSet;

    let unknown: Vec<u32> = app
        .protocol_friends
        .iter()
        .filter_map(|p| p.game_app_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .filter(|id| {
            !app.games.iter().any(|g| g.appid == *id) && !app.game_name_cache.contains_key(id)
        })
        .collect();

    if !unknown.is_empty() {
        // Mark IDs as pending so concurrent ticks don't queue duplicate requests.
        for id in &unknown {
            app.game_name_cache.entry(*id).or_default();
        }
        let _ = app.io_tx.send(IoEvent::LookupGameNames(unknown));
    }
}

fn queue_library_game_name_lookups(app: &mut App) {
    use crate::io_event::IoEvent;

    let unknown: Vec<u32> = app
        .games
        .iter()
        .filter(|game| game.name.as_deref().is_none_or(str::is_empty))
        .map(|game| game.appid)
        .filter(|appid| !app.game_name_cache.contains_key(appid))
        .collect();

    if !unknown.is_empty() {
        for appid in &unknown {
            app.game_name_cache.entry(*appid).or_default();
        }
        let _ = app.io_tx.send(IoEvent::LookupGameNames(unknown));
    }
}

fn is_closed_error(error: &ProtocolError) -> bool {
    match error {
        ProtocolError::Closed => true,
        ProtocolError::Transport(message) => message.contains("closed"),
        ProtocolError::WebSocket(error) => error.to_string().contains("closed"),
        _ => false,
    }
}
