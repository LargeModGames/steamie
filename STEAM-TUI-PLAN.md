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
- [~] Game library loads keylessly via CM `ClientLicenseList` + PICS + `Player.ClientGetLastPlayedTimes#1`; Web API owned-games remains a disconnected fallback only. **Live-validated 2026-06-05: the pipeline works end-to-end (1336 apps resolved from 673 packages, names populate), but two defects remain — see "Known issues" below.**
- [~] Achievements via CM protocol service call — `Player.GetUserStats#1` + binary KV schema parser implemented; **pending live validation** (needs one end-to-end run against a real game with achievements)

**Known issues from live validation (2026-06-05):**
- **Playtime shows 0 for every game.** Log: `ClientGetLastPlayedTimes returned playtime data games=0` on every run. The call returns (no error, no timeout) but with an empty list. Root cause: `library::get_last_played_times` sends the method via `service_method::call` (`ServiceMethodCallFromClientNonAuthed`, 9804) with no session `steamid` in the header; the request body carries no steamid field, so Steam has no user to report on. Fix: send via `call_authed` (session `steamid` + `client_sessionid`), the same envelope achievements uses. Needs live re-validation that the authed method responds rather than timing out like `GetOwnedGames#1` did.
- **DLCs appear as separate library entries** (e.g. every Assetto Corsa DLC listed individually with 0 min). Root cause: PICS package resolution collects every appid in each package (`appids=1336` from 588 packages — base games + DLC + tools) and `AppCatalogInfo` never inspects `common.type`. Fix: capture `type` in `parse_app_info` and keep only games (drop `dlc`/`music`/`video`/etc.).
- **`wait_for_package_ids` is an unbounded wait** (`client.rs`): `notify_waiters()` stores no permit (check-then-await missed-wakeup race) and there is no overall timeout; a disconnect doesn't fire `license_notify`. Low severity and recoverable. Hardening follow-up (deferred by request): wrap the library load in a timeout (mirror the old 30s `GetOwnedGames`) plus the race-free `Notify` idiom (`enable()` before the state check).

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
5. For playtime: `CMsgClientLicenseList` has `minutes_used` per license (not per app). A dedicated
   playtime call — `Player.ClientGetLastPlayedTimes#1` via `ServiceMethodCallFromClientNonAuthed`
   (EMsg 9804) — returns per-app `rtime_last_played` and `playtime_forever`. This call DOES work
   (uses the NonAuthed EMsg like auth service calls, not the broken 9802 authed variant).
6. Merge the PICS name/icon data with the playtime data to produce the final `Vec<ProtocolGame>`.

PICS `ClientPICSProductInfoResponse` uses EMsg 8904 and can stream multiple response packets under
one job ID. `vapour-protocol` handles that with a dedicated `pending_streams` path so the existing
single-response job correlation remains unchanged.

Note on achievements validation: The KV schema parser handles the SteamAchievementManager-confirmed
format (type stored as string "4", display names as language-keyed nested blocks). Once live-validated,
capture raw schema bytes from a real game and add as a parser regression test fixture.

Note: The plan listed "RSA + AES encryption handshake" — this is the SteamKit2-era TCP handshake. The WebSocket CM endpoint uses TLS at the transport layer instead; the RSA/AES session layer is not needed and has been intentionally omitted.

Tech: Port core authentication and friends logic from SteamKit2 (C#) and node-steam-user (JS) to Rust.

### v0.3.0 -- "We Need to Talk"

Real-time chat. This is the feature that makes Vapour a daily driver.

- [ ] 1-on-1 chat messaging
- [ ] Group chat support
- [ ] Chat history (locally cached)
- [ ] Message notifications (terminal bell / desktop notification)
- [ ] Typing indicators
- [ ] Chat embedded alongside friends list (split pane layout)

### v0.4.0 -- "Launch Day"

Game launching. The DRM-aware hybrid approach.

- [ ] Detect whether a game uses DRM (maintain/crowdsource a list, cross-reference with PCGamingWiki)
- [ ] Non-DRM games: launch directly from TUI (no Steam client needed)
- [ ] DRM games: silently start Steam in background, launch game, kill Steam on game exit
- [ ] Steam process lifecycle management (detect if already running, reuse)
- [ ] Launch options support (custom args, Proton/Wine prefix on Linux)
- [ ] Recently played quick-launch bar

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
