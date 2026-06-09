# steamie

> A terminal-native Steam client written in Rust, powered by [Ratatui](https://github.com/ratatui-org/ratatui).
>
> Browse your library, chat with friends, and launch games — all from a fast, keyboard-driven TUI,
> with no browser and no Steam window in your way.

[![Crates.io](https://img.shields.io/crates/v/steamie.svg)](https://crates.io/crates/steamie)
[![CI](https://github.com/LargeModGames/steamie/actions/workflows/ci.yml/badge.svg)](https://github.com/LargeModGames/steamie/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
![Platforms](https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-informational)
[![X](https://img.shields.io/badge/@LargeModGames-000000?logo=x&logoColor=white)](https://twitter.com/LargeModGames)

> **Unofficial.** steamie is an independent project and is **not affiliated with, endorsed by, or
> sponsored by Valve or Steam.** It talks to Steam's own connection-manager protocol the same way the
> official client does; use it at your own discretion.

<!-- 📷 A demo recording lives here once available:
![Demo](.github/demo.gif)
Want to record one? See CONTRIBUTING.md. -->

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [Launching games](#launching-games)
  - [Contributing to the DRM-free list](#contributing-to-the-drm-free-list)
- [Limitations](#limitations)
- [Architecture](#architecture)
- [Development](#development)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [Security](#security)
- [Maintainer](#maintainer)
- [License](#license)

## Features

- **Login over the Steam CM protocol** — QR-code sign-in (default) or username/password, no Web API
  key required.
- **Friends** — your friends list, loaded directly from Steam.
- **Library** — owned games and tools, filtered Steam-style by app type (`t` cycles
  All / Games / Software-Tools).
- **Real-time 1-on-1 chat** — a Chat tab with conversation list, scrollable history, and a composer;
  messages send and arrive live over the modern `FriendMessages` service, with local history kept
  per-conversation.
- **Per-game playtime & achievements** — native, pulled over the protocol (no scraping, no API key).
- **News** — sourced from your library, recently-played, and wishlist apps.
- **Launch games** — start any installed game from the library. By default Steam is launched
  **silently** (minimized to the tray, no window); an experimental mode can launch DRM-free titles
  with **no Steam at all**. See [Launching games](#launching-games) below.

## Installation

> steamie requires **Rust 1.85+** (it uses edition 2024) when building from source.

```bash
# Cargo (installs the `steamie` binary)
cargo install steamie

# Arch Linux (AUR) — prebuilt binary (faster)
yay -S steamie-bin

# Arch Linux (AUR) — build from source
yay -S steamie

# Homebrew (macOS)
brew install LargeModGames/steamie/steamie

# Windows (winget)
winget install LargeModGames.steamie
```

Or download a prebuilt binary from [GitHub Releases](https://github.com/LargeModGames/steamie/releases/latest).

### Build from source

```bash
git clone https://github.com/LargeModGames/steamie.git
cd steamie
cargo build --release
./target/release/steamie
```

`cargo run --bin steamie` works too for a debug build. No special system libraries are required —
networking uses `rustls` and desktop notifications use `zbus`, so there's no OpenSSL, X11, or audio
dependency to install.

## Configuration

Configuration is optional — steamie runs with sensible defaults. To customize, create
`~/.config/steamie/config.toml`:

```toml
# api_key = "YOUR_STEAM_WEB_API_KEY"   # optional fallback; https://steamcommunity.com/dev/apikey
# steam_id = "76561198XXXXXXXXX"       # optional; auto-detected after login

[auth]
method = "qr"          # "qr" (default) or "credentials"
account_name = ""      # optional hint for credential login

[ui]
tick_rate_ms = 250     # default 250
theme = "dark"         # default "dark"

[chat]
notifications_enabled = true    # terminal bell on incoming message
desktop_notifications = false   # also raise a notify-rust desktop notification
history_retention_days = 30     # local chat history kept; 0 = keep everything

[launch]
steam_path = ""                 # override; auto-detected (registry / default path) when empty
dry_run = false                 # log the exact launch command instead of spawning it
kill_steam_on_exit = false      # if steamie started Steam, shut it down when the game exits (best-effort)
silent = true                   # start Steam minimized to tray (no window); false shows it
direct_launch = false           # experimental: launch DRM-free-listed games with NO Steam running
force_direct = false            # experimental: with direct_launch, try the direct path for any installed game

[launch.game_args]              # optional per-game args, keyed by appid string
# "730" = "-novid -high"
```

## Usage

Run `steamie` to start the UI. Press `?` at any time for the in-app keybinding help.

| Key | Action |
| --- | ------ |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g g` | Jump to top |
| `G` | Jump to bottom |
| `Enter` | Open detail · launch in detail view |
| `l` | Launch highlighted game |
| `L` | Recently-played quick-launch overlay |
| `Esc` / `Backspace` | Go back |
| `/` | Search library |
| `t` | Cycle library type filter (All / Games / Software-Tools) |
| `r` | Reload current view |
| `1` … `5` | Switch tab: Library · Friends · Wishlist · News · Chat |
| `?` | Toggle help |
| `q` / `Ctrl+C` | Quit |

## Launching games

From the **Library** tab: press `l` to launch the highlighted game, `Enter` to open its detail
view (then `Enter`/`l` to launch), or `L` for a recently-played quick-launch overlay.

By default the game launches through the official Steam client, but **Steam stays out of your way**:
it starts minimized to the system tray with no window (`steam -silent -applaunch <appid>`), the game
appears, and DRM/presence work normally. Set `silent = false` if you'd rather see Steam's window.

**Experimental — launch with no Steam at all.** Games that are confirmed to run without the Steam
client (DRM-free titles) can be launched directly from their executable, so Steam never wakes. Enable
it with `direct_launch = true`; a game is eligible when it's on the community
[`DRM-FREE-GAMES.md`](DRM-FREE-GAMES.md) list and installed. Anything else falls back to the silent
Steam launch above. The status line shows `▶ Launched X (no Steam)` when it goes direct.

`kill_steam_on_exit` only ever shuts down a Steam that steamie itself started, and is best-effort
(reliable on Windows, weaker on Linux, a no-op on macOS). Set `dry_run = true` to preview the exact
command a launch would run without starting anything.

### Contributing to the DRM-free list

[`DRM-FREE-GAMES.md`](DRM-FREE-GAMES.md) is a community-maintained table of Steam games that run with
**Steam fully closed**. To add a game: quit Steam, confirm the game still launches from its
`steamapps/common/<game>/` executable, then add a row with its AppID and name and open a PR. You can
also keep a private list at `~/.config/steamie/drm-free.md` (same format).

## Limitations

- **Direct launch** needs the library load to have populated PICS launch info; otherwise it falls
  back to Steam. Proton/Wine prefix wrapping for the direct path is not yet implemented (Steam still
  applies Proton itself for the mediated path).
- `force_direct` on a DRM-protected game spawns an executable that will bounce (Steam isn't running)
  and won't auto-recover.
- `kill_steam_on_exit` detection is best-effort: reliable on Windows (registry), weaker on Linux, a
  no-op on macOS.
- Launch arguments are whitespace-split (no quoted-argument grouping).
- Group chat is not yet supported (1-on-1 only); the chat list is populated as conversations arrive
  or are opened, and per-message wall-clock timestamps aren't displayed yet.

## Architecture

steamie is a Cargo workspace with three crates:

- **`steamie-api`** — Steam Web API / Store API HTTP client.
- **`steamie-core`** — config, session, caching, the game launcher, and local chat history.
- **`steamie`** — the ratatui terminal UI (produces the `steamie` binary).

The raw Steam connection-manager protocol (WebSocket connection, auth, friends, service calls) lives
in a **separate** crate, [**vapour-protocol**](https://github.com/LargeModGames/vapour-protocol)
([crates.io](https://crates.io/crates/vapour-protocol)).

## Development

```bash
cargo run --bin steamie            # run (debug)
cargo build --release             # release build
cargo check --workspace           # type-check everything
cargo test --workspace            # run tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all                   # format
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contributor guide.

## Roadmap

- Group chat (`ChatRoom.*`)
- Proton/Wine prefix wrapping for the direct (no-Steam) launch path
- Per-message wall-clock timestamps in chat
- Preloading the conversation list from disk on startup
- Quoted-argument grouping for launch options

Have an idea? Open a [Discussion](https://github.com/LargeModGames/steamie/discussions) or an issue.

## Contributing

Contributions are welcome — code, docs, bug reports, and additions to the DRM-free games list. See
[CONTRIBUTING.md](CONTRIBUTING.md) and our [Code of Conduct](CODE_OF_CONDUCT.md).

## Security

Found a vulnerability? Please report it privately — see [SECURITY.md](SECURITY.md).

## Maintainer

Built and maintained by [LargeModGames](https://github.com/LargeModGames).

## License

[MIT](LICENSE).
