# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Run (config optional; defaults are used if missing)
cargo run --bin vapour

# Run with custom config
cargo run --bin vapour -- --config /path/to/config.toml

# Check all crates
cargo check --workspace

# Run tests
cargo test --workspace

# Run tests for a single crate
cargo test -p vapour-core

# Lint
cargo clippy --workspace
```

## Configuration

`~/.config/vapour/config.toml` (optional):
```toml
# api_key = "YOUR_STEAM_WEB_API_KEY"   # https://steamcommunity.com/dev/apikey
# steam_id = "76561198XXXXXXXXX"       # 17-digit SteamID64 (auto-detected after login)

[auth]
method = "qr"         # "qr" (default) or "credentials"
account_name = ""     # optional hint for credential login

[ui]
tick_rate_ms = 250   # optional, default 250
theme = "dark"       # optional, default "dark"

[chat]
notifications_enabled = true    # optional, default true — terminal bell on incoming message
desktop_notifications = false   # optional, default false — also raise a notify-rust desktop notification
history_retention_days = 30     # optional, default 30 — local chat history kept; 0 = keep everything

[launch]
steam_path = ""                 # optional override; auto-detected (registry/default path) if empty
dry_run = false                 # optional, default false — log the launch command instead of spawning
kill_steam_on_exit = false      # optional, default false — if Vapour started Steam, shut it down on game exit
silent = true                   # optional, default true — start Steam minimized to tray (no window); false shows it
direct_launch = false           # optional, default false — experimental: launch DRM-free-listed games with NO Steam
force_direct = false            # optional, default false — experimental: with direct_launch, try the direct path for
                                #   games not on the DRM-free list too (may fail for DRM titles; falls back to Steam)
extra_args = ""                 # optional — args appended to every launch (e.g. "-silent")
# [launch.game_args]            # optional per-game args, keyed by appid string
# "730" = "-novid -high"
```

The DRM-free game list lives in the repo root at `DRM-FREE-GAMES.md` (a contributable markdown table
of AppIDs); a personal list can be added at `~/.config/vapour/drm-free.md`.

## Architecture

This is a Cargo workspace with three crates:

- **`vapour-api`** — Steam Web API HTTP client (`SteamApiClient`). All network calls to the public Steam Web API and Store API live here. No UI or business logic.
- **`vapour-core`** — Config loading (`Config`), session management, and local caching. Bridges `vapour-api` and `vapour-protocol` with the TUI. Config is read from `~/.config/vapour/config.toml` via `dirs` when present.
- **`vapour-tui`** — The ratatui terminal UI binary (`vapour`). Owns all rendering, input handling, and application state.

`vapour-protocol` is a **separate repository** that implements the raw Steam CM protocol (WebSocket connection, auth, friends, service method calls). `vapour-core` has a path dependency pointing to `../../../vapour-protocol`.

### TUI threading model

```
main thread: event loop (keyboard input + tick)
       │
       │  std::sync::mpsc::Sender<IoEvent>
       ▼
tokio tasks: network I/O (network.rs::handle_io)
       │
       │  Arc<Mutex<App>>
       ▼
shared App state (app.rs)
```

`IoEvent` (defined in `io_event.rs`) is the only way the UI thread triggers network work. Results are written back into `App` through the shared `Arc<Mutex<App>>`. Friend loading is paginated: `LoadFriendIds` chains into `LoadFriendPage(0)`, which chains into `LoadFriendPage(n+1)` until exhausted.

### Navigation model

`App.navigation_stack: Vec<Route>` is a push/pop stack. Each `Route` carries a `RouteId` (which view is visible) and an `ActiveBlock` (which pane has keyboard focus). `Route::load_event()` returns the `IoEvent` to fire when a route first becomes active. Views are rendered in `vapour-tui/src/views/`.

### Current state (v0.4.1)

Game launching is live. Press **`l`** on a library row (or **`Enter`/`l`** in the game-detail view) to launch the highlighted game; **`L`** opens a recently-played **quick-launch overlay** (`views/quick_launch.rs` + `handlers/quick_launch.rs`, an `ActiveBlock::QuickLaunch` modal mirroring the help/error overlays). Launching is a **local system action**, not a Steam-protocol RPC: `IoEvent::LaunchGame(appid)` is handled in `network.rs` via `tokio::task::spawn_blocking`, which calls `vapour-core::launcher::launch_game`. `launch_game` first tries the **experimental direct (no-Steam) path** (v0.4.1) and otherwise falls back to a **silent Steam-mediated launch** (v0.4.0 "Launch Day").

**Steam-mediated (default, always reliable):** the launcher resolves the Steam executable (Windows registry via `reg.exe` then `%ProgramFiles(x86)%\Steam\steam.exe`; Linux `PATH`/`~/.steam/steam.sh`; macOS `Steam.app`) and runs `steam [-silent] -applaunch <appid> [args]`. With `[launch] silent` (on by default) Steam starts **minimized to the tray with no window** — the game appears, you never see Steam — and presence/"playing X" is handled by the client we delegate to. Launch options come from the `[launch]` config section (`LaunchConfig::options_for` merges global `extra_args` with per-game `game_args`); a transient `App.launch_status` shows the result (and, in `dry_run` mode, the exact command — also logged via `tracing` under `target: "launch"`). The optional, off-by-default `kill_steam_on_exit` spawns a detached best-effort watcher: only if Vapour started Steam, it polls Steam's per-app running flag (Windows registry / Linux `registry.vdf`) and runs `steam -shutdown` once the game exits.

**Direct, no-Steam (experimental, opt-in via `[launch] direct_launch`):** for games on the contributable repo-root `DRM-FREE-GAMES.md` list, Vapour runs the game's executable directly so **Steam never wakes**. Executable resolution is authoritative: `vapour-protocol`'s PICS pass now surfaces each app's `config.installdir` + `config/launch` entries (`LaunchEntry`) — the **only `vapour-protocol` change** this release — captured into `App.app_launch_info`. The install directory is found on disk by `vapour-core::steam_apps` (parsing `steamapps/libraryfolders.vdf` + `appmanifest_<appid>.acf` via the new hand-rolled `vapour-core::vdf` text-KeyValues parser); `vapour-core::drm_free` embeds the list at build time (`include_str!`) and merges an optional `~/.config/vapour/drm-free.md`. The pure `launcher::plan_launch` decides Direct-vs-Steam: direct iff `direct_launch` **and** the game is fully installed (`StateFlags & 4`) **and** it's DRM-free-listed (or `force_direct`) **and** a `config/launch` entry matches this OS. Anything unresolved (not installed, no launch metadata, spawn error) **falls back to silent Steam** — the status line reads "▶ Launched X (no Steam)" vs "▶ Launched X". No new crates (offline-friendly: `reg.exe` + filesystem reads; all parsers are pure and unit-tested). **Still deferred:** Proton/Wine prefix wrapping for the direct path (Steam applies Proton itself for the mediated path). **Known limitations:** direct launch needs the library load to have populated PICS launch info (else it falls back to Steam); `force_direct` on a DRM game spawns an exe that bounces (Steam not running) and won't auto-recover; kill-on-exit detection is best-effort (reliable on Windows via the registry; weaker on Linux, a no-op on macOS); per-game `game_args` and launch arguments are whitespace-split (no quoted-arg grouping); the quick-launch overlay opens from the library only and recently-played-only games lack launch metadata (so they go via Steam).

Real-time 1-on-1 chat is live (v0.3.0 "We Need to Talk"). A **Chat tab** (key `5`) shows a split pane — conversation list on the left, scrollable message history + composer on the right; press `Enter` on a friend to open a chat. Messaging runs natively over the modern unified **`FriendMessages.*`** service in `vapour-protocol` (`src/chat.rs`): `send_message`/`send_typing`/`get_recent_messages` via `call_authed` (EMsg 151→147), and an unsolicited incoming push decoded from **`ServiceMethod` (EMsg 146)** identified by `target_job_name == "FriendMessagesClient.IncomingMessage#1"`. Real-time receive requires **two** things that were initially missing (fixed + live-validated 2026-06-07): (1) the logon must set **`chat_mode = 2`** ("new chat") in `CMsgClientLogon`, or Steam never pushes friend messages to the session; and (2) the push arrives as **146 `ServiceMethod`**, *not* 152 `ServiceMethodSendToClient` — server-initiated unified notifications all come on 146 and are demultiplexed by `target_job_name` (verified against node-steam-user + SteamKit). `decode_incoming` accepts 146 (and 152 defensively) and gates on the job name; sending and `GetRecentMessages` history work even without `chat_mode`, which is why a restart used to "find" messages that never arrived live. Sends surface on Steam's confirmation, stamped with the authoritative `server_timestamp`+`ordinal`, so every message (sent/received/backfilled) dedupes on one stable `(timestamp, ordinal)` key. Protocol events reach the UI as new `FriendsEvent` variants (`IncomingMessage`/`MessageSent`/`TypingNotification`/`RecentMessages`); the UI sends `RunCommand::{SendMessage,SendTyping,GetRecentMessages}` over the existing `friend_cmd_tx`. History is cached locally per-conversation as JSON under `~/.local/state/vapour/chat/` (`vapour-core::chat_history`), lazily loaded before any save so an incoming message can't truncate prior history, and persisted on a serialized off-lock task. Notifications are the terminal bell (default) plus optional `notify-rust` desktop notifications, both `[chat]`-configurable. Group chat (`ChatRoom.*`) is deferred to v0.3.1. **Known limitations:** the conversation/friends list selects by position, so a reorder under the cursor can open the adjacent entry; the chat list is not preloaded from disk on startup (a conversation appears once a message arrives or it is opened from Friends); per-message wall-clock timestamps are not yet displayed.

Protocol auth is live (QR + credentials), friends and library load via CM, and news is sourced from keyless library/recently-played/wishlist appids. The library is filtered by PICS `common.type` (games + software/tools only; DLC/soundtracks/videos dropped) with a Steam-style type filter (`t` cycles All / Games / Software-Tools), and the load is hardened with bounded service-method timeouts plus a race-free `wait_for_package_ids`. Web API owned-games remains only a disconnected fallback.

**Per-game playtime and achievements are now native (v0.2.5 "Personal Best", live-validated 2026-06-07).**
- *Achievements* load via the dedicated `ClientGetUserStats` EMsg (818 → `ClientGetUserStatsResponse` 819). The binary-KV stats schema is parsed by `kv.rs`; achievement stats are identified by the presence of a `bits` block (the schema's `type` is a word like `"INT"`, not the numeric `"4"`), and unlock state comes from the response's `achievement_blocks` (global bit = `achievement_id*32 + pos`, unlocked iff `unlock_time[pos] != 0`). If the first request returns `eresult != OK`, the app is briefly marked games-played and the request retried.
- *Playtime* loads via the authed unified `Player.ClientGetLastPlayedTimes#1` (`call_authed`).
- The unblocking fix for both was correcting the service-method EMsg constants: `ServiceMethodCallFromClient` is **151** and `ServiceMethodSendToClient` is **152**. `9802`/`9803` are actually `ClientServerTimestamp` Request/Response, so the old authed unified call was really a timestamp ping that only ever got a `{0, server_time_ms}` reply and never a `ServiceMethodResponse` (147) — which is why playtime/achievements over the unified path appeared dead in v0.2.0.
