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
```

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

### Current state (v0.2.5)

Protocol auth is live (QR + credentials), friends and library load via CM, and news is sourced from keyless library/recently-played/wishlist appids. The library is filtered by PICS `common.type` (games + software/tools only; DLC/soundtracks/videos dropped) with a Steam-style type filter (`t` cycles All / Games / Software-Tools), and the load is hardened with bounded service-method timeouts plus a race-free `wait_for_package_ids`. Web API owned-games remains only a disconnected fallback.

**Per-game playtime and achievements are now native (v0.2.5 "Personal Best", live-validated 2026-06-07).**
- *Achievements* load via the dedicated `ClientGetUserStats` EMsg (818 → `ClientGetUserStatsResponse` 819). The binary-KV stats schema is parsed by `kv.rs`; achievement stats are identified by the presence of a `bits` block (the schema's `type` is a word like `"INT"`, not the numeric `"4"`), and unlock state comes from the response's `achievement_blocks` (global bit = `achievement_id*32 + pos`, unlocked iff `unlock_time[pos] != 0`). If the first request returns `eresult != OK`, the app is briefly marked games-played and the request retried.
- *Playtime* loads via the authed unified `Player.ClientGetLastPlayedTimes#1` (`call_authed`).
- The unblocking fix for both was correcting the service-method EMsg constants: `ServiceMethodCallFromClient` is **151** and `ServiceMethodSendToClient` is **152**. `9802`/`9803` are actually `ClientServerTimestamp` Request/Response, so the old authed unified call was really a timestamp ping that only ever got a `{0, server_time_ms}` reply and never a `ServiceMethodResponse` (147) — which is why playtime/achievements over the unified path appeared dead in v0.2.0.
