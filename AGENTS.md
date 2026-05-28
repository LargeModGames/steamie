# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Run (requires config at ~/.config/vapour/config.toml)
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

`~/.config/vapour/config.toml`:
```toml
api_key = "YOUR_STEAM_WEB_API_KEY"   # https://steamcommunity.com/dev/apikey
steam_id = "76561198XXXXXXXXX"        # 17-digit SteamID64

[ui]
tick_rate_ms = 250   # optional, default 250
theme = "dark"       # optional, default "dark"
```

## Architecture

This is a Cargo workspace with three crates:

- **`vapour-api`** — Steam Web API HTTP client (`SteamApiClient`). All network calls to the public Steam Web API and Store API live here. No UI or business logic.
- **`vapour-core`** — Config loading (`Config`), session management, and local caching. Bridges `vapour-api` (and the future `vapour-protocol`) with the TUI. Config is read from `~/.config/vapour/config.toml` via `dirs`.
- **`vapour-tui`** — The ratatui terminal UI binary (`vapour`). Owns all rendering, input handling, and application state.

A fourth crate, `vapour-protocol`, is planned as a **separate repository** that will implement the raw Steam CM server protocol (TCP connection, RSA+AES handshake, credential auth, friends, chat). It does not exist yet; `vapour-core` already has a path dependency stub pointing to `../../../vapour-protocol`.

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

### Current state (v0.1)

Only Steam Web API is used — no protocol work yet. The features implemented are: game library, game detail (achievements), friends list (paginated), wishlist, and news feed.
