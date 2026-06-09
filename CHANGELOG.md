# Changelog

## [Unreleased]

## [v0.4.1] 2026-06-09

### Added

- **Experimental direct (no-Steam) launch**: Games on the community [`DRM-FREE-GAMES.md`](DRM-FREE-GAMES.md) list can be launched straight from their executable so Steam never wakes (`[launch] direct_launch = true`). Executable resolution comes from PICS `config.installdir` + `config/launch` entries, with the install directory located on disk by parsing `libraryfolders.vdf` + `appmanifest_<appid>.acf`.
- **Silent Steam launch**: With `[launch] silent` (on by default), the Steam-mediated path starts Steam minimized to the tray with no window — the game appears, you never see Steam.
- **Launch option controls**: Added `force_direct`, per-game `game_args` (keyed by appid), and global `extra_args` for arguments appended to every launch.

### Changed

- **Graceful fallback**: Anything not eligible for direct launch (not installed, no launch metadata, or a spawn error) falls back to the silent Steam-mediated launch, with the status line distinguishing `▶ Launched X (no Steam)` from `▶ Launched X`.

## [v0.4.0] 2026-06-07

### Added

- **Game launching ("Launch Day")**: Press `l` on a library row (or `Enter`/`l` in the detail view) to launch the highlighted game; `L` opens a recently-played quick-launch overlay.
- **Steam-mediated launch**: Launches via `steam -applaunch <appid>`, with Steam executable auto-detection on Windows (registry / default path), Linux (`PATH` / `~/.steam`), and macOS (`Steam.app`).
- **Launch safeguards**: Added optional `kill_steam_on_exit` (only shuts down a Steam that steamie started) and `dry_run` (logs the exact command instead of spawning).

## [v0.3.0] 2026-06-07

### Added

- **Real-time 1-on-1 chat ("We Need to Talk")**: A Chat tab with conversation list, scrollable history, and a composer. Messaging runs natively over the modern unified `FriendMessages.*` service.
- **Local chat history**: Conversations are cached per-conversation under `~/.local/state/steamie/chat/` and lazily loaded before any save.
- **Notifications**: Terminal-bell (default) plus optional `notify-rust` desktop notifications, both configurable under `[chat]`.

### Fixed

- **Live message receive**: The logon now sets `chat_mode = 2`, and incoming pushes are decoded from `ServiceMethod` (EMsg 146) rather than 152, so friend messages arrive in real time instead of only appearing after a restart.

## [v0.2.5] 2026-06-07

### Added

- **Native playtime & achievements ("Personal Best")**: Per-game playtime (`Player.ClientGetLastPlayedTimes`) and achievements (`ClientGetUserStats`) are now pulled over the protocol with no scraping and no Web API key.

### Fixed

- **Service-method EMsg constants**: Corrected `ServiceMethodCallFromClient` to 151 and `ServiceMethodSendToClient` to 152, unblocking the unified service calls that previously only ever received a timestamp ping.

## [v0.2.0] 2026-06-07

### Added

- **Protocol authentication**: Steam CM protocol login over QR code (default) or username/password, including Steam Guard and refresh-token handling.
- **Friends & library over CM**: Friends list and owned library load directly over the connection-manager protocol, with a race-free paginated load.
- **Library type filter**: Library is filtered Steam-style by PICS `common.type`; `t` cycles All / Games / Software-Tools.
- **News**: Sourced from your library, recently-played, and wishlist apps.
