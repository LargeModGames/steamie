# DRM-free games (launch without Steam)

A **community-maintained** list of Steam games confirmed to launch directly from their executable
with **no Steam client running** — games that ship without Steam DRM and don't hard-require the
Steamworks runtime to start.

Steamie's experimental direct-launch path uses this list. When `[launch] direct_launch = true` is set
in `config.toml`, a game on this list is started straight from its executable (resolved from Steam's
own launch metadata) and **Steam is never woken**. Everything else falls back to a quiet,
Steam-mediated launch.

> **Note:** "DRM-free" (you can copy the files / it's also sold on GOG) is **not** the same as
> "launches without the Steam client." Many DRM-free games' Steam builds still call Steam's
> `RestartAppIfNecessary`, which **relaunches Steam and bounces the game through it** — so only the
> hands-on test below tells you whether a game truly qualifies. The list starts empty on purpose:
> every entry must be verified first.

## How to contribute

1. Quit Steam **completely**, then launch the game's executable directly from
   `steamapps/common/<game>/`. The game qualifies **only if it starts and keeps running with Steam
   still closed**. It does **not** qualify if it shows *"Steam must be running"*, exits immediately,
   **or opens/launches the Steam client itself**.
2. Add a row below with the game's **Steam AppID** and name, keeping the table sorted by AppID.
   The AppID is the number in the store URL: `store.steampowered.com/app/<AppID>/`.
3. Open a pull request.

You can also keep a personal list at `~/.config/steamie/drm-free.md` (same table format); its entries
are merged with this one at runtime.

| AppID | Game |
|------:|------|

