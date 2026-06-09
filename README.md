# vapour

A terminal-native Steam client written in Rust. Browse your library, chat with friends, and launch
games — all from a fast keyboard-driven TUI, no browser and no Steam window in your way.

> **Unofficial.** vapour is an independent project and is **not affiliated with, endorsed by, or
> sponsored by Valve or Steam.** It talks to Steam's own connection-manager protocol the same way the
> official client does; use it at your own discretion.

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

## Build from source

vapour requires **Rust 1.85+** (it uses edition 2024).

```bash
git clone https://github.com/LargeModGames/vapour.git
cd vapour
cargo build --release
./target/release/vapour
```

`cargo run --bin vapour` works too for a debug build.

## Configuration

Configuration is optional — vapour runs with sensible defaults. To customize, create
`~/.config/vapour/config.toml`:

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
kill_steam_on_exit = false      # if Vapour started Steam, shut it down when the game exits (best-effort)
silent = true                   # start Steam minimized to tray (no window); false shows it
direct_launch = false           # experimental: launch DRM-free-listed games with NO Steam running
force_direct = false            # experimental: with direct_launch, try the direct path for any installed game

[launch.game_args]              # optional per-game args, keyed by appid string
# "730" = "-novid -high"
```

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

`kill_steam_on_exit` only ever shuts down a Steam that Vapour itself started, and is best-effort
(reliable on Windows, weaker on Linux, a no-op on macOS). Set `dry_run = true` to preview the exact
command a launch would run without starting anything.

### Contributing to the DRM-free list

[`DRM-FREE-GAMES.md`](DRM-FREE-GAMES.md) is a community-maintained table of Steam games that run with
**Steam fully closed**. To add a game: quit Steam, confirm the game still launches from its
`steamapps/common/<game>/` executable, then add a row with its AppID and name and open a PR. You can
also keep a private list at `~/.config/vapour/drm-free.md` (same format).

## Built on

The Steam connection-manager protocol layer is its own crate:
[**vapour-protocol**](https://github.com/LargeModGames/vapour-protocol)
([crates.io](https://crates.io/crates/vapour-protocol)).

## License

[MIT](LICENSE).
