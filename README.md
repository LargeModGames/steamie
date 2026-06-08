# vapour

A terminal-native Steam client written in Rust.

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

Optional `[launch]` settings in `~/.config/vapour/config.toml`:

```toml
[launch]
steam_path = ""             # override; auto-detected (registry / default path) when empty
dry_run = false             # log the exact launch command instead of spawning it
kill_steam_on_exit = false  # if Vapour started Steam, shut it down when the game exits (best-effort)
silent = true               # start Steam minimized to tray (no window); false shows it
direct_launch = false       # experimental: launch DRM-free-listed games with NO Steam running
force_direct = false        # experimental: with direct_launch, try the direct path for any installed
                            #   game (not just listed ones) — may fail for DRM titles
extra_args = ""             # args appended to every launch, e.g. "-silent"

[launch.game_args]          # optional per-game args, keyed by appid string
# "730" = "-novid -high"
```

`kill_steam_on_exit` only ever shuts down a Steam that Vapour itself started, and is best-effort
(reliable on Windows, weaker on Linux, a no-op on macOS). Set `dry_run = true` to preview the exact
command a launch would run without starting anything.

### Contributing to the DRM-free list

[`DRM-FREE-GAMES.md`](DRM-FREE-GAMES.md) is a community-maintained table of Steam games that run with
**Steam fully closed**. To add a game: quit Steam, confirm the game still launches from its
`steamapps/common/<game>/` executable, then add a row with its AppID and name and open a PR. You can
also keep a private list at `~/.config/vapour/drm-free.md` (same format).
