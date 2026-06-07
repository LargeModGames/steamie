# vapour

A terminal-native Steam client written in Rust.

## Launching games (v0.4.0)

From the **Library** tab: press `l` to launch the highlighted game, `Enter` to open its detail
view (then `Enter`/`l` to launch), or `L` for a recently-played quick-launch overlay. Every game
launches through the official Steam client (`steam -applaunch <appid>`), which starts itself if it
isn't already running.

Optional `[launch]` settings in `~/.config/vapour/config.toml`:

```toml
[launch]
steam_path = ""             # override; auto-detected (registry / default path) when empty
dry_run = false             # log the exact launch command instead of spawning it
kill_steam_on_exit = false  # if Vapour started Steam, shut it down when the game exits (best-effort)
extra_args = ""             # args appended to every launch, e.g. "-silent"

[launch.game_args]          # optional per-game args, keyed by appid string
# "730" = "-novid -high"
```

`kill_steam_on_exit` only ever shuts down a Steam that Vapour itself started, and is best-effort
(reliable on Windows, weaker on Linux, a no-op on macOS). Set `dry_run = true` to preview the exact
command a launch would run without starting anything.
