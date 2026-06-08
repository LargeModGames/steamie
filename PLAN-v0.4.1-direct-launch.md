# Plan — v0.4.1: Direct launch for DRM-free games (no Steam client) + downloading roadmap

> Status: **direct launch IMPLEMENTED** (build + clippy + unit tests green; install detection and
> direct dry-run live-validated on this machine; end-user no-Steam launch of a DRM-free title still
> pending). Downloading roadmap unchanged (still deferred / blocked offline).
>
> **Divergences from this plan as built** (the plan was a working doc; these were the refinements):
> - **DRM-free list format:** `DRM-FREE-GAMES.md` (a markdown table parsed by `drm_free.rs`), not
>   `drm-free.toml` — per the user's request for a `.md` file. Same repo-root + `include_str!` +
>   `~/.config/vapour/drm-free.md` user-merge design. Seeded with Terraria/Stardew Valley/Factorio.
> - **`depots` field NOT added** to the PICS pass (Unit/PR-1) — it only serves the deferred,
>   offline-blocked downloader, so adding it now would be speculative. PICS surfaces `installdir` +
>   `config/launch` only. (Re-add when the downloader is actually built.)
> - **Try-then-fallback simplified:** no ~8s fast-exit heuristic (it adds UI latency and is fragile).
>   The curated list is the gate; fallback to (silent) Steam happens only on *resolution/spawn
>   failure*, which is instant and deterministic. Config is `direct_launch` + `force_direct` (the
>   plan's `prefer_direct`); no `try_fallback` knob.
> - **Added `[launch] silent` (default on)** — `steam -silent` — so the Steam-mediated path also hides
>   Steam. This was the user's primary ask ("launch quietly in the background").
> - **Optional install/DRM indicator (Unit E) not added** — deferred as non-essential UX.

## Context

v0.4.0 shipped **Steam-mediated** launching (`steam -applaunch`). The project's goal is to *replace the official client*; the README says "Steam only wakes up when DRM demands it." This work delivers the other half of the launch vision: **launch DRM-free games directly, with no Steam client running**, falling back to Steam only when a game needs it. Downloading games via depots/CDN is also wanted, but it is a from-scratch DepotDownloader-class effort and is **currently blocked offline** (see roadmap), so it gets a roadmap, not an implementation, this round.

### Decisions
- **Phasing:** Direct-launch now (buildable + live-testable on this machine); downloading deferred until the toolchain can fetch `aes`/`lzma` crates.
- **Exe resolution:** Extend `vapour-protocol`'s PICS pass to surface `config/launch` (+ `depots` for later) — authoritative, reuses `kv.rs`, pre-stages download metadata.
- **DRM-free list:** lives **in the repo root, contributable**, plus **try-then-fallback** for everything else.

### Key findings (from codebase + on-disk exploration — all confirmed)
- **This machine HAS ~29 games installed** in a second library `E:\Games\Steam` (Civ VII `1295660`, Deadlock `1422450`, …, all `StateFlags=4`). → direct-launch and install-detection are **live-testable here**. (The primary library `C:\Program Files (x86)\Steam` only has SteamVR + redistributables.)
- No install-detection exists in either repo (grep: zero `libraryfolders`/`appmanifest`/`.acf` parsing). The local `appmanifest_*.acf` gives `installdir` + `StateFlags` (`4` = fully installed); `libraryfolders.vdf` maps each library `path` → its `apps`. The game lives at `<library>/steamapps/common/<installdir>/`.
- The launch **executable is NOT in the .acf** — it's in PICS appinfo `config/launch[i]` (`executable`/`arguments`/`workingdir`/`type` + `config/oslist`/`osarch`/`betakey`). `vapour-protocol/src/pics.rs::parse_binary_app_info` already parses the full binary-KV tree via `kv.rs` but **discards** everything except name/icon/type.
- **No text-VDF parser exists** (only the binary `kv.rs`); the `vdf`/`keyvalues` crates are not in either lock and **cargo is offline** → hand-roll a tiny text-VDF parser in `vapour-core`.
- **Downloading is blocked offline:** depot manifests/chunks are AES-256 ECB/CBC; neither `aes` nor `cbc` is in the lock, `ring` doesn't expose raw ECB/CBC, LZMA crates are absent. `flate2` (zlib) + `sha1`/`sha2` + `reqwest` ARE present. So a downloader can't be *built* here until crates can be fetched (or AES is hand-rolled).

**Structure:** two repos, two sequential PRs — **PR 1 in `vapour-protocol`** (PICS extension) must exist locally before **PR 2 in `vapour`** (everything else) compiles against it (path dep `../../../vapour-protocol`).

---

## PR 1 — `vapour-protocol`: surface launch + install metadata from PICS

**File:** `E:\Code\vapour-protocol\src\pics.rs` (+ `src/friends.rs` for the model, `src/lib.rs` exports).

- New public struct `LaunchEntry { executable: String, arguments: Option<String>, workingdir: Option<String>, launch_type: Option<String>, oslist: Option<String>, osarch: Option<String>, betakey: Option<String> }`.
- Extend `ProtocolGame` (`friends.rs:76`) with `installdir: Option<String>` and `launch: Vec<LaunchEntry>` (default empty — the Web-API fallback and recently-played paths leave them empty).
- In `parse_binary_app_info` (and `parse_text_app_info`): after extracting `common`, navigate the **same root** to `config` → read `installdir` (string) and `config → launch` (numbered children) into `Vec<LaunchEntry>`, using the existing `KVValue::get`/`as_str` helpers (and the private `parse_vdf`/`VdfNode` for the text path). Existing name/icon/type extraction unchanged.
- Include a `depots` field now (depot id + manifest gid + size) so the download phase doesn't need a second protocol PR — but don't act on it.
- Tests: add a captured binary-KV appinfo fixture asserting `installdir` + ≥1 `launch` entry parse. Keep existing PICS tests green.
- Separate GitHub repo → its own branch + PR. Build/test standalone (`cargo test` in `E:\Code\vapour-protocol`).

## PR 2 — `vapour`: direct launcher, install detection, DRM-free list, UI

### Unit A — `vapour-core::vdf` (hand-rolled text-VDF parser)
**New `crates/vapour-core/src/vdf.rs`** — minimal `.acf`/`.vdf` parser (no crate): `parse(&str) -> Option<VdfMap>` with `VdfMap::{get,str,map}`. Handles `"key" "value"`, `"key" { … }`, quoted/bare tokens, `//` comments, escapes. ~60 lines + unit tests using the **real captured `.acf`/`libraryfolders.vdf` contents** as fixtures (Civ VII, Steamworks).

### Unit B — `vapour-core::steam_apps` (local install detection)
**New `crates/vapour-core/src/steam_apps.rs`:**
- `struct InstalledApp { appid, name, installdir, library_path: PathBuf, state_flags: u32 }` with `install_path()` → `<library>/steamapps/common/<installdir>` and `is_fully_installed()` → `state_flags & 4 != 0`.
- `discover_installed(steam_root: &Path) -> HashMap<u32, InstalledApp>`: parse `<steam_root>/steamapps/libraryfolders.vdf` → library paths; for each, scan `steamapps/appmanifest_*.acf`, parse `appid`/`name`/`installdir`/`StateFlags`. Reuse `launcher::resolve_steam_exe`'s parent (or add `resolve_steam_root`).
- Unit tests parse the real fixtures; a live integration test runs `discover_installed` on this machine and asserts a known appid (Civ VII `1295660`) is found + `is_fully_installed()`.

### Unit C — `vapour-core::drm_free` (contributable repo-root list)
- **New repo-root file `drm-free.toml`** (contributable): `games = [ { appid = 38600, name = "…" }, … ]` with a header explaining how to contribute.
- **New `crates/vapour-core/src/drm_free.rs`:** embed via `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../drm-free.toml"))`, parse once (`OnceLock`), merge an optional user file `~/.config/vapour/drm-free.toml`. `is_known_drm_free(appid) -> bool`. Tests parse the embedded list.

### Unit D — extend `vapour-core::launcher` (direct path + try-then-fallback)
- `LaunchOptions` gains `prefer_direct: bool` + `try_fallback: bool`; add a `launch_game(appid, &[LaunchEntry], &LaunchOptions)` entry point for the UI (keep `launch` for the pure Steam path).
- Pure decision fn `plan_launch(installed: Option<&InstalledApp>, entries: &[LaunchEntry], drm_free: bool, cfg) -> Decision` → `Direct { exe, workingdir, args }` or `Steam`. Direct iff: installed (`StateFlags=4`) **and** a launch entry matches this OS (`oslist`/`osarch` via `cfg!(target_os)`, `type` default/empty preferred) **and** (`drm_free` **or** `prefer_direct`). Pure → unit-tested across platforms.
- Execution: Direct → `Command::new(exe).current_dir(workingdir).args(args).spawn()`. **Try-then-fallback:** if spawn errors, or `try_fallback` and the child `try_wait()`s to an exit within ~8 s (DRM bounce / failure), fall back to `steam -applaunch`. `LaunchOutcome` gains `mode: Direct|Steam` (surfaced as "▶ Launched directly" vs "▶ Launched via Steam").
- `[launch]` config additions (`config.rs`): `prefer_direct: bool` (default false — list-only unless set), `try_fallback: bool` (default true).

### Unit E — `vapour-tui`: thread PICS data + wire the decision
- `protocol.rs`: in `FriendsEvent::OwnedGames`, capture each game's `installdir` + `launch` into a new `App.app_launch_info: HashMap<u32, (Option<String>, Vec<LaunchEntry>)>` — no `Game`-model change (keeps the api layer clean).
- `network.rs::IoEvent::LaunchGame`: gather `launch_entries` from `app.app_launch_info[appid]` + `config.launch`, call `vapour_core::launch_game(appid, &entries, &opts)` in `spawn_blocking`; status shows direct-vs-Steam; errors via `set_error`.
- Optional UX: a small install/DRM indicator in the library/detail (`●` installed, `🔓` known DRM-free) — only if cheap; else defer.

### Unit F — tests, docs, help
- Unit tests for vdf, steam_apps (real fixtures + live integration), drm_free, `plan_launch` (platform matrix), OS-filter selection.
- Docs: `AGENTS.md` current-state + `[launch]` (`prefer_direct`/`try_fallback`); `STEAM-TUI-PLAN.md` v0.4.0 checklist (flip "Detect DRM" + "Non-DRM direct launch" toward done, note the list + try-fallback); `README.md` direct-launch + how to contribute to `drm-free.toml`.

---

## Downloading games — roadmap (NOT built this round; blocked offline)

Document in `STEAM-TUI-PLAN.md` as v0.5.0 groundwork. Build only after the crates.io-fetch issue is resolved (needs `aes`, `cbc`, an LZMA crate) **or** AES-256 ECB/CBC is hand-rolled. Pipeline: PICS `depots` (from PR 1) → `ClientGetDepotDecryptionKey` (new EMsg + proto) → content-server discovery + `ClientGetCDNAuthToken` → download+decrypt+parse manifest (protobuf) → per-file chunk download (`reqwest`) + AES decrypt + `flate2`/LZMA decompress + SHA verify → write files + synthesize `appmanifest`. Progress via new `RunCommand::DownloadGame`/`FriendsEvent::DownloadProgress` + an `App.download_progress` field rendered as a gauge. **Largest single risk: AES + LZMA availability offline.** First step when unblocked = a feasibility spike confirming a single depot decrypts end-to-end.

## Verification
1. `vapour-protocol`: `cargo test` green (new PICS extraction fixture).
2. `vapour`: `cargo build/clippy/test --workspace --offline` clean; new unit tests pass.
3. **Live on this machine (real games installed):**
   - Install detection: `discover_installed` finds Civ VII `1295660` at `E:\Games\Steam` with `StateFlags=4`.
   - Direct launch: dry-run prints the resolved exe path/workingdir/args for an installed game; for a DRM title (Civ VII/Deadlock) verify it **falls back to Steam** (DRM bounce). If any installed game is on the DRM-free list, verify it launches directly with Steam not running.
4. **User live-verify:** confirm a known DRM-free title launches with Steam fully closed and stays running (no bounce).

## Risks / caveats
- **Exe resolution depends on PICS data being loaded** (library load) — if launch info isn't available yet, fall back to Steam-mediated.
- **Try-then-fallback is heuristic** (fast-exit window) — a game with a quick external launcher could misclassify; mitigated by the curated list (no fallback needed) and `try_fallback` scoped to non-listed games.
- **Cross-platform/Proton:** direct launch targets the native-OS exe; Linux Proton/Wine wrapping for Windows-only games is out of scope here (Steam-mediated remains the path for those).
- **Two-repo coordination:** PR 2 won't compile until PR 1 is present locally; both must merge (protocol first).

## Environment note
Cargo cannot reach crates.io here (schannel TLS revocation) → build with `--offline`, **do not add new crates**. `git push` + `gh` to GitHub work fine. (Same constraint that shaped v0.4.0.)
