# Vapour

> A terminal-native Steam client written in Rust. Lightweight, fast, and keyboard-driven.
> Steam only wakes up when DRM demands it.

---

## Vision

Vapour is a fully standalone Steam TUI that replaces the official Steam client for everything except DRM game launches. It connects directly to Steam's servers using a Rust-native implementation of the Steam client protocol (ported from SteamKit2/node-steam-user), giving users chat, friends, store browsing, library management, and more from the terminal.

When a user launches a DRM-protected game, Vapour silently spins up the official Steam client in the background, launches the game, and kills Steam again once the game closes.

For the 90% of the time you're browsing, chatting, or managing your library, Steam is not running. Vapour uses ~20MB of RAM instead of 500MB+.

---

## Why This Will Get Stars

- **No competition.** The only existing Steam TUI (`steam-tui` by dmadisetti) is abandoned and was just a wrapper around `steamcmd`. No proper Steam TUI client exists.
- **Proven formula.** Same playbook as spotatui: take a bloated desktop app, rebuild the experience in a beautiful Rust TUI, target the Linux/terminal power user crowd.
- **Same audience.** The r/unixporn, r/linux, r/rust, r/Steam communities overlap heavily. spotatui already has credibility in this space.
- **The `vapour-protocol` crate.** The only existing Rust Steam protocol library (`steam-vent`) is buried on Codeberg with 4 stars. A well-documented, GitHub-native alternative will become the Rust community's go-to Steam library. This crate alone will attract stars and contributors independently from the TUI.
- **Steam Deck appeal.** SteamOS users who want a lighter interface for browsing/chatting without the full Steam UI eating resources.
- **AI-accelerated development.** Built using Claude Design for UI prototyping and Claude Code + Codex for implementation. Ships faster than any solo dev could traditionally manage.

---

## Architecture

```
vapour-protocol/              # Standalone repo and crate
├── src/
│   ├── connection.rs         # TCP/UDP connection to Steam CM servers
│   ├── crypto.rs             # Encryption handshake (AES-256-CBC + RSA)
│   ├── auth.rs               # Authentication (credentials, Steam Guard, 2FA)
│   ├── messages.rs           # Protobuf message encoding/decoding
│   ├── friends.rs            # Friends list, presence, personas
│   ├── chat.rs               # Real-time chat messaging
│   ├── store.rs              # Store browsing, app details, pricing
│   ├── library.rs            # Owned games, licenses, playtime
│   ├── market.rs             # Community market data
│   └── cdn.rs                # Content download (for non-DRM games)
└── Cargo.toml

vapour/                       # Main application repo
├── crates/
│   ├── vapour-api/           # Steam Web API wrapper (fallback + supplementary data)
│   │   ├── src/
│   │   │   ├── player.rs       # Player summaries, achievements, stats
│   │   │   ├── news.rs         # Game news
│   │   │   ├── store.rs        # Store API (app details, reviews, screenshots)
│   │   │   └── workshop.rs     # Workshop/community content
│   │   └── Cargo.toml
│   │
│   ├── vapour-core/          # Business logic bridging protocol + API
│   │   ├── src/
│   │   │   ├── session.rs      # Session management, reconnection
│   │   │   ├── launcher.rs     # Game launch logic (direct or via Steam client)
│   │   │   ├── config.rs       # User configuration, keybinds, themes
│   │   │   └── cache.rs        # Local caching of friends, library, images
│   │   └── Cargo.toml
│   │
│   └── vapour-tui/           # Terminal UI (ratatui)
│       ├── src/
│       │   ├── app.rs           # Application state machine
│       │   ├── views/
│       │   │   ├── friends.rs   # Friends list + online status + current game
│       │   │   ├── chat.rs      # Chat window (conversation view)
│       │   │   ├── library.rs   # Game library browser
│       │   │   ├── store.rs     # Store browser with search
│       │   │   ├── game.rs      # Game detail view (achievements, stats, news)
│       │   │   ├── market.rs    # Community market browser
│       │   │   └── wishlist.rs  # Wishlist management
│       │   ├── widgets/         # Reusable TUI components
│       │   ├── keybinds.rs      # Vim-style keybind system
│       │   └── theme.rs         # Theming engine (colors, borders, layout)
│       └── Cargo.toml
│
├── proto/                    # Steam protobuf definitions (from SteamKit2/SteamDatabase)
├── design/                   # Claude Design mockups and UI references
├── Cargo.toml                # Workspace manifest
└── README.md
```

---

## Features by Release

### v0.1.0 -- "First Light"

The minimum viable product. Prove it works, ship it, get feedback.

- [x] Steam Web API authentication (API key based)
- [x] Game library view (list all owned games, playtime, last played)
- [x] Game detail view (achievements, stats, screenshots)
- [x] Friends list (online/offline/in-game status)
- [x] Wishlist viewer
- [x] News feed (aggregated from owned games)
- [x] Vim-style navigation (hjkl, /, gg, G)
- [x] Basic theming (at minimum: dark theme that looks good in screenshots)
- [x] Configurable via `~/.config/vapour/config.toml`

Tech: Uses only the Steam Web API. No protocol work yet. This ships fast.

### v0.2.0 -- "Direct Connect"

The protocol crate lands. Vapour connects directly to Steam's servers.

- [x] `vapour-protocol` crate: WebSocket connection to Steam CM servers
- [x] Credential-based login with Steam Guard (email code, device code, device confirmation) and QR login
- [x] Refresh token persistence (`~/.local/state/vapour/auth.toml`, 0o600) with silent re-auth on startup
- [x] Auth modal in TUI (QR code rendered in-terminal, guard code input, connecting/failure states)
- [x] `[auth]` section in config.toml (`method = "qr"` or `"credentials"`, `account_name` hint)
- [x] Friends list via protocol (real-time online/offline/in-game events, persona merge)
- [x] Persona state (set online, away, invisible, playing)
- [x] Migrate friends from Web API to protocol
- [x] `api_key` and `steam_id` made optional in config — `steam_id` auto-derived from login token
- [x] Game library loads keylessly via CM `ClientLicenseList` + PICS (names/icons populate); Web API owned-games remains a disconnected fallback only. **Live-validated 2026-06-07.**
- [x] Library shows only games + software/tools — DLC/soundtracks/videos filtered out via PICS `common.type`; a Steam-style type filter (`t` cycles All / Games / Software-Tools) composes with search. **Fixed 2026-06-07: dropped the 1336-row DLC explosion to 458 real entries.**
- [x] Library load hardened: bounded service-method timeouts (no infinite hang) + race-free `wait_for_package_ids` (`Notify` `enable()` before state check + overall 30s timeout).
- [x] **Per-game playtime — done in v0.2.5.** Now native via authed `Player.ClientGetLastPlayedTimes#1`; the real fix was the service-method EMsg constants (see v0.2.5).
- [x] **Achievements — done in v0.2.5.** Native via the dedicated `ClientGetUserStats` EMsg (818/819) + binary-KV schema parser (see v0.2.5).

**Resolved / re-scoped 2026-06-07 (supersedes the 2026-06-05 known issues):**
- **DLC rows — FIXED.** `AppCatalogInfo`/`ProtocolGame`/`Game` now carry `app_type` (lowercased `common.type`); `pics::is_library_entry` keeps game/application/tool + named-untyped and drops dlc/music/video/empty rows. Filter applied to the final collected Vec. Live: 1336 → 458 entries. Plus the new TUI type filter.
- **`wait_for_package_ids` — HARDENED.** Race-free `Notify` (`enable()` before the state check) + overall 30s timeout. Service-method calls also wrapped in timeouts so a silently-ignored method can't hang the load.
- **Playtime + achievements — RELOCATED to v0.2.5.** The 2026-06-05 hypothesis ("use `call_authed`") was **tested live 2026-06-07 and disproven.** The `Player.*` *unified* service methods do not deliver user-scoped data over the client CM connection: the authed envelope (9802) gets only a `ServiceMethodSendToClient` (9803) token push and never a `ServiceMethodResponse` (147) → it times out; the NonAuthed envelope (9804) responds but with no user context (playtime `games=0`; achievements empty schema). Dedicated client EMsgs (PICS, friends, persona) all work — only the unified path is dead. Fix is native dedicated-EMsg work — see **v0.2.5** below. Both calls are currently left on NonAuthed (fast empty, no stall) with KNOWN-LIMITATION comments in code.

Note on CM library loading:
`Player.GetOwnedGames#1` called via `EMsg::ServiceMethodCallFromClient` (9802) receives no response
from Steam — the server silently ignores it. Confirmed via 30-second timeouts across multiple runs.
The implemented CM path for keyless library loading is `ClientLicenseList` (EMsg 780), which Steam
pushes automatically after every login, followed by PICS package/app resolution and the Player
last-played service method.

**What Steam actually sends after login (EMsg 780):**
Steam pushes `ClientLicenseList` (~43KB for a large library) to the client immediately after
`ClientLogOnResponse`. This contains the user's package licenses with `package_id`, `time_created`,
`minutes_used`, and `minute_limit` fields — but NOT individual app IDs or game names. Resolving
packages to games requires PICS queries (see below).

**Why `Player.GetOwnedGames#1` was abandoned:**
- Steam sends `ServiceMethodSendToClient` (EMsg 9803) with `job_target=N` immediately after our
  request — this is a session token push (17 bytes: two uint64 fields — token + Unix timestamp),
  NOT the game list. It uses the same job ID as our pending request because Steam's server-side job
  numbering happened to match.
- No `ServiceMethodResponse` (147) ever arrives for this call. The request times out after 30s.
- Routing fix applied: `ServiceMethodSendToClient` (9803) must never be routed to pending jobs
  (`pending_jobs` in `connection.rs`) even when `jobid_target` matches — it is always a server push,
  not a request response. This fix is in place in `vapour-protocol`.

**Implemented keyless CM library pipeline:**
1. Handle EMsg 780 in the `incoming` arm of `client.rs::run()`. Decode as `CMsgClientLicenseList`.
   This gives a list of `package_id` values (the user's owned packages).
2. Fire `ClientPICSAccessTokenRequest` (EMsg 8905) for package IDs to get PICS access tokens.
3. Fire `ClientPICSProductInfoRequest` (EMsg 8903) with the package IDs + tokens to get app IDs
   per package.
4. Fire a second `ClientPICSProductInfoRequest` for the resolved app IDs to get names, icons, and
   metadata.
5. For playtime: `CMsgClientLicenseList` has `minutes_used` per license (not per app). The attempted
   per-app call — `Player.ClientGetLastPlayedTimes#1` — does **NOT** work (corrected 2026-06-07):
   over NonAuthed (9804) it responds but with `games=0` (its request body has no steamid field, so no
   user context), and over authed (9802) it gets no response at all. Per-app playtime is deferred to
   v0.2.5 (native dedicated-EMsg work); the pipeline currently merges empty playtime, so library rows
   show names/icons with 0 playtime.
6. Merge the PICS name/icon data with the (currently empty) playtime data into the final `Vec<ProtocolGame>`.

PICS `ClientPICSProductInfoResponse` uses EMsg 8904 and can stream multiple response packets under
one job ID. `vapour-protocol` handles that with a dedicated `pending_streams` path so the existing
single-response job correlation remains unchanged.

Note on achievements validation: The KV schema parser handles the SteamAchievementManager-confirmed
format (type stored as string "4", display names as language-keyed nested blocks). Once live-validated,
capture raw schema bytes from a real game and add as a parser regression test fixture.

Note: The plan listed "RSA + AES encryption handshake" — this is the SteamKit2-era TCP handshake. The WebSocket CM endpoint uses TLS at the transport layer instead; the RSA/AES session layer is not needed and has been intentionally omitted.

Tech: Port core authentication and friends logic from SteamKit2 (C#) and node-steam-user (JS) to Rust.

### v0.2.5 -- "Personal Best"

Native per-user stats. Finishes the two v0.2.0 items that can't go through the Web API — playtime and
achievements — implemented entirely in `vapour-protocol` (no `api_key`, no scraping).

**STATUS: COMPLETE (live-validated 2026-06-07).** Both items work natively over CM. See the checklist
below for the exact fixes; the key insight is in the "Why" note's correction.

**Why this was its own release (and the real root cause).** v0.2.0 concluded the `Player.*` *unified*
service methods (`Player.GetUserStats#1`, `Player.ClientGetLastPlayedTimes#1`) didn't return user-scoped
data over CM — the "authed (9802)" call yielded only a `9803` push and never a `147`. **That diagnosis
was right about the symptom but wrong about the cause: `9802`/`9803` are not `ServiceMethodCallFromClient`/
`ServiceMethodSendToClient` at all — they are `ClientServerTimestampRequest`/`Response`.** The correct
service-method EMsgs are **151** and **152**. So the v0.2.0 "authed" call was literally a timestamp ping,
and the `9803` `{0, server_time_ms}` reply was the server clock. With the constants corrected, the authed
unified call (playtime) returns a normal `147`, and achievements use the dedicated `ClientGetUserStats`
EMsg (818/819) — both proven live.

- [x] **Achievements via dedicated `ClientGetUserStats` (EMsg 818 → `ClientGetUserStatsResponse` 819).**
      Done & live-validated 2026-06-07 (Marvel Rivals: 49/49 unlocked with names + unlock times; AoE II
      DE: 357 definitions). Added `CMsgClientGetUserStats`/`…Response` proto defs + EMsg constants, sent
      via the job-correlated `request` path. **Parser fix:** achievement stats are identified by the
      presence of a `bits` block, not `type == 4` — real schemas store `type` as a word (`"INT"`, …).
      Unlock state from `achievement_blocks` (global bit = `achievement_id*32 + pos`, unlocked iff
      `unlock_time != 0`); on `eresult != OK`, briefly mark games-played and retry. Captured a real
      schema (appid 410110) as the `parses_real_captured_schema` regression fixture.
- [x] **Per-game playtime — native.** Done & live-validated 2026-06-07 (181/458 games with real
      playtime; e.g. Marvel Rivals 768h, Rocket League 1803h) via authed `Player.ClientGetLastPlayedTimes#1`.
      **Root cause was not the framing but wrong EMsg constants:** `ServiceMethodCallFromClient` is **151**
      and `ServiceMethodSendToClient` is **152** — `9802`/`9803` are actually `ClientServerTimestamp`
      Request/Response. The old "authed" call on 9802 was a timestamp ping, so Steam only returned a
      `{0, server_time_ms}` `9803` and never a `ServiceMethodResponse` (147). Correcting the constants
      made the authed unified call return a real `147`.
- [x] Removed the NonAuthed `Player.*` stop-gaps and their KNOWN-LIMITATION comments (playtime now
      `call_authed`; achievements now the dedicated EMsg).
- [x] Flipped the v0.2.0 playtime + achievements items to done; updated `AGENTS.md` "Current state".

Tech: dedicated client EMsgs (not unified service methods), modelled on SteamKit2's `ClientGetUserStats`
and node-steam-user's stats handling. The `kv.rs` binary-KV schema parser and `ProtocolAchievement` model
already exist — this is wiring the right message, not new parsing.

### v0.3.0 -- "We Need to Talk"

Real-time chat. This is the feature that makes Vapour a daily driver.

- [x] 1-on-1 chat messaging — native via the unified `FriendMessages.*` service in `vapour-protocol`
      (`send_message`/`get_recent_messages`/incoming `FriendMessagesClient.IncomingMessage#1` push).
      Sends surface on Steam's confirmation, stamped with the authoritative `server_timestamp`+`ordinal`.
- [ ] Group chat support — **deferred to v0.3.1** (`ChatRoom.*` is a whole separate protocol surface).
- [x] Chat history (locally cached) — per-conversation JSON under `~/.local/state/vapour/chat/`
      (`vapour-core::chat_history`), `(timestamp, ordinal)` dedupe + retention pruning, lazily loaded
      before any save so an incoming message can't truncate prior history; persisted off the UI lock.
- [x] Message notifications (terminal bell / desktop notification) — bell by default + optional
      `notify-rust` desktop notification, `[chat]`-configurable, suppressed for the focused conversation.
- [x] Typing indicators — send (throttled while composing) + receive (`chat_entry_type` typing push).
- [x] Chat embedded alongside friends list (split pane layout) — Chat tab (key `5`): conversation list
      + message history + composer; `Enter` on a friend opens a chat.

**STATUS: 1-on-1 chat COMPLETE (code-reviewed, unit-tested). Live send/receive/typing e2e pending an
interactive run with a second Steam account.** Implemented sequentially across two repos (protocol PR in
`vapour-protocol`, UI PR in `vapour`) rather than the usual parallel split — chat is a hard dependency
chain (protocol → core → app → view → handlers) and the relative `../../../vapour-protocol` path dep
breaks inside git worktrees. **Known limitations:** list selection is positional (a reorder under the
cursor can open the adjacent entry); the chat list is not preloaded from disk on startup; per-message
wall-clock timestamps are not displayed (no `chrono` dependency yet).

### v0.4.0 -- "Launch Day"

Game launching. **Shipped as Steam-mediated launch:** every game launches through the official Steam
client via `steam -applaunch <appid> [args]` (Steam starts itself when down; presence/"playing X" is
handled by the client we delegate to). Implemented as one sequential PR — the `../../../vapour-protocol`
path dep breaks in git worktrees and the feature is a vertical chain over shared TUI files — with
**zero `vapour-protocol` changes**, since launching is a local process action on the `IoEvent` path.

**STATUS: COMPLETE.** v0.4.0 shipped Steam-mediated launch (dry-run + unit-tested; live launch
validated by the user). **v0.4.1 then added the direct (no-Steam) path + silent launch** — see the
checklist below. No new crates in either: the Steam exe/running-state is read via `reg.exe` on Windows
and the filesystem on Linux/macOS, install metadata via on-disk VDF, and all parsers are pure and
unit-tested. (v0.4.1 added one `vapour-protocol` change — PICS now surfaces `installdir` + `config/launch`.)

- [x] **Launch a game from the TUI** — `l` on a library row, `Enter`/`l` in game detail. Routes
      `IoEvent::LaunchGame(appid)` → `vapour-core::launcher::launch` off-thread via `spawn_blocking`.
- [x] **Steam process lifecycle (detect if already running, reuse)** — resolves the Steam exe
      (Windows registry via `reg.exe` then `%ProgramFiles(x86)%\Steam`; Linux `PATH`/`~/.steam`; macOS
      `Steam.app`); reuses a running Steam and starts it if down.
- [x] **Kill Steam on game exit — best-effort, opt-in** (`[launch] kill_steam_on_exit`, default off):
      only if *we* started Steam. A detached watcher polls Steam's per-app running flag (Windows
      registry / Linux `registry.vdf`) then runs `steam -shutdown`.
- [x] **Launch options (custom args)** — `[launch] extra_args` + per-game `[launch.game_args]`, merged
      by `LaunchConfig::options_for`. A `dry_run` mode logs the exact command without spawning.
- [x] **Recently-played quick-launch bar** — `L` opens an overlay over the library
      (`views/quick_launch.rs`); `Enter` launches the selected recently-played appid.
- [x] **Detect whether a game uses DRM — done in v0.4.1 via a curated list.** Instead of scraping
      PCGamingWiki, a contributable repo-root `DRM-FREE-GAMES.md` (embedded via `include_str!`, plus an
      optional `~/.config/vapour/drm-free.md`) gates the no-Steam path: a game on the list is launched
      directly, everything else stays Steam-mediated. `vapour-core::drm_free::is_known_drm_free`.
- [x] **Non-DRM games: launch directly from the TUI (no Steam client) — done in v0.4.1 (experimental,
      opt-in `[launch] direct_launch`).** Steam never wakes. Exe resolved authoritatively from PICS
      `config/launch` (new `vapour-protocol` `LaunchEntry`) + on-disk install detection
      (`vapour-core::steam_apps` parsing `libraryfolders.vdf`/`appmanifest_*.acf` with the new
      `vapour-core::vdf` text-KV parser). Pure `launcher::plan_launch` gates on installed + DRM-free
      (or `force_direct`) + an OS-matching launch entry; anything unresolved falls back to silent Steam.
      Also shipped: **silent launch** (`[launch] silent`, default on) — `steam -silent` keeps the
      mediated path's Steam in the tray with no window. Live-validated (install detection + direct
      dry-run) on this machine; end-user live no-Steam launch of a DRM-free title pending.
- [ ] Proton/Wine prefix wrapping — **still deferred**: Steam applies Proton itself for Steam-mediated
      launches; external prefix wrapping only matters for the direct-launch path (native-OS exe only for now).

### v0.5.0 -- "The Bazaar"

Store and market integration.

- [ ] Store browsing with search, filters (genre, price, tags)
- [ ] Game store pages (description, reviews, media, system requirements)
- [ ] Wishlist management (add/remove from TUI)
- [ ] Sale/discount highlights
- [ ] Community Market browser (item prices, trends)
- [ ] Inventory viewer

Note: Purchasing will NOT be supported. This keeps Vapour cleanly within Valve's comfort zone. Users click a "Open in browser" action to complete purchases.

### v0.6.0+ -- "Polish"

- [ ] Desktop notifications (via notify-rust)
- [ ] Steam Deck optimizations (gamepad navigation?)
- [ ] Trade offer viewer
- [ ] Workshop browsing
- [ ] Screenshot viewer (sixel/kitty image protocol for terminals that support it)
- [ ] Plugin system for community extensions
- [ ] Profile viewer (own + friends)

---

## Key Technical Decisions

### Protocol Strategy: Competing with steam-vent

There is one existing Rust implementation of the Steam client protocol: `steam-vent` by Robin Appelman, hosted on Codeberg with 4 stars. It covers authentication, RPC calls, and game coordinator communication. However, it has minimal visibility (Codeberg), sparse documentation, and an API designed for server queries rather than client-side use.

`vapour-protocol` will be a clean-room implementation, using the same public references but designed specifically for building Steam client applications:

1. **SteamKit2** (C#, 3.6K stars) -- the most complete reference implementation
2. **node-steam-user** (JavaScript, 1.1K stars) -- cleaner, more modern codebase
3. **SteamDatabase/Protobufs** -- up-to-date protobuf definitions maintained by the community
4. **steam-vent** (Rust, Codeberg) -- reference for Rust-specific patterns and pitfalls

The approach:
- Use SteamDatabase/Protobufs for the `.proto` files, compile with `prost`
- Read SteamKit2 and node-steam-user side-by-side when implementing each protocol feature
- Study steam-vent for Rust-specific async patterns but write original code
- Design the API around client use cases (login, chat, friends, library) not server queries
- Start with authentication (the hardest part), then friends, then chat
- Publish `vapour-protocol` as an independent crate early to attract contributors
- Host on GitHub with thorough documentation, examples, and a contributor-friendly setup

### TUI Framework

**ratatui** (the standard). Same ecosystem as spotatui.

**Design process:** All views are prototyped in Claude Design first, then implemented in ratatui. This means the TUI ships with polished, intentional layouts rather than whatever looked okay during coding. Claude Design mockups are stored in `design/` for reference.

Reference implementations to study:
- `agent-of-empires` -- multi-pane TUI with session management (Rust/ratatui)
- `ccboard` -- workspace-style crate split: `core` / `tui` / `web` (Rust/ratatui)
- `spotatui` itself -- already know the codebase intimately

### Game Launch Model

```
User presses Enter on a game
         │
         ▼
   ┌─────────────┐
   │ DRM check   │
   │ (local DB)  │
   └──────┬──────┘
          │
    ┌─────┴─────┐
    │           │
  No DRM    Has DRM
    │           │
    ▼           ▼
  Launch    Is Steam
  directly  running?
    │         │
    │    ┌────┴────┐
    │   Yes       No
    │    │         │
    │    │     Start Steam
    │    │     (headless/minimized)
    │    │         │
    │    │     Wait for ready
    │    │         │
    │    └────┬────┘
    │         │
    │    Launch via
    │    steam://run/<appid>
    │         │
    │    Monitor game process
    │         │
    │    Game exits
    │         │
    │    Kill Steam
    │    (if we started it)
    │         │
    └────┬────┘
         │
    Return to TUI
```

### Authentication Flow

One layer, zero config required:
1. **Protocol auth** -- QR code or credentials with Steam Guard/2FA. Same flow SteamKit2 uses. The user's SteamID is derived automatically from the login token — no `steam_id` in config needed.

`api_key` in `config.toml` is optional. It is only used by fallback Web API paths that have not been
fully retired, not by protocol auth, friends, library, news, store details, wishlists, or game name
lookups.

---

## Distribution Targets

Following the spotatui playbook:

- **Cargo** -- `cargo install vapour`
- **Homebrew** -- tap for macOS/Linux
- **AUR** -- Arch Linux (the core audience)
- **NixOS** -- Nix package
- **Winget** -- Windows support
- **Flatpak** -- for Steam Deck

---

## Promotion Strategy

Week 1 post-launch (v0.1):
- r/rust ("I built a Steam TUI in Rust")
- r/linux, r/unixporn (screenshots with a rice)
- r/Steam ("Terminal-based Steam client, open source")
- r/SteamDeck ("Lightweight Steam client for Deck")
- Show HN
- Terminal Trove submission

Ongoing:
- GitHub README with animated GIF demo (critical for stars)
- vapour.dev landing page (built with Claude Design, same playbook as spotatui.com)
- Update posts on each major release
- Publish `vapour-protocol` as a standalone crate announcement on r/rust when ready

---

## Legal Notes

- Steam Web API usage is explicitly permitted by Valve for third-party applications.
- The Steam client protocol has been reverse-engineered by multiple open-source projects (SteamKit2 since ~2013) without legal action from Valve. Valve hired SteamKit's creator.
- Reverse engineering for interoperability is protected under EU Software Directive (applicable in the Netherlands).
- Vapour will NOT support: game piracy, DRM circumvention, market manipulation, or any activity that undermines Valve's revenue.
- Purchasing is deliberately excluded and redirects to browser.

---

## Name Rationale

**Vapour** -- British English for "vapor." Steam is water vapour. It's clean, memorable, and available as a crate name. The terminal aesthetic fits: vapour is what's left when you strip away the bloat.

---

## Development Workflow

### AI Tooling

This project is built with AI-assisted development at every stage:

**Claude Design** -- Used for all UI/UX work:
- Prototype TUI layouts before writing code (friends list, chat, library, store views)
- Explore visual variations rapidly (split panes, tab layouts, popup modals)
- Design the README hero screenshots and demo aesthetics
- Iterate on themes and color schemes
- Hand off finalized designs to Claude Code for implementation

**Claude Code** -- Primary coding agent:
- Protocol implementation (porting from SteamKit2/node-steam-user to Rust)
- Core business logic
- TUI view implementation from Claude Design mockups
- Test writing, documentation

**Codex CLI** -- Secondary coding agent:
- Review Claude Code's output
- Parallel work on independent crates (e.g. vapour-api while Claude Code works on vapour-protocol)
- CI/CD setup, GitHub Actions, release automation
- When Claude Code hits rate limits, Codex picks up the work

**Workflow pattern:**
1. Design a view in Claude Design (e.g. the friends list layout)
2. Hand off to Claude Code to implement the ratatui view
3. Use Codex to review and test
4. Repeat for next view

This spreads token usage across providers and keeps development velocity high even when individual providers rate-limit.

---

## Getting Started (Day 1)

1. Create the `vapour` GitHub repository under LargeModGames (or a new org)
2. Create the separate `vapour-protocol` GitHub repository
3. `cargo init --name vapour` with workspace setup for the app repo
4. Add `vapour-protocol` as an external dependency while both repos iterate locally
5. **Claude Design:** Mock up the library list view and friends list layout
6. Implement Web API auth + game library fetch in `vapour-api`
7. **Claude Code:** Build the library list view in `vapour-tui` from the Design mockup
8. Ship v0.1-alpha to GitHub with a good README and a screenshot
9. Start porting SteamKit2 auth flow to `vapour-protocol` in parallel
10. Register vapour.dev (or similar) for the landing page
