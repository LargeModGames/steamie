# How to cut a release

Releases are automated via GitHub Actions in [`.github/workflows/cd.yml`](.github/workflows/cd.yml).
The workflow runs when you push a tag matching `v*.*.*`.

## Stable release

1. Bump `version` in the root `Cargo.toml` (`[workspace.package]`) and the internal dependency
   versions in `[workspace.dependencies]` (`vapour-api` / `vapour-core`) to match. Run the app once
   to refresh `Cargo.lock`.
2. Move the `Unreleased` items in `CHANGELOG.md` under a new version heading.
3. Commit and push.
4. Create an annotated tag (the tag message is shown on the GitHub release page):
   ```bash
   git tag -a v0.4.1 -m "Release v0.4.1"
   git push origin v0.4.1
   ```
5. Watch the build on the [Actions page](https://github.com/LargeModGames/vapour/actions).
6. Stable tags (no `-` suffix) trigger the publish jobs (crates.io, AUR, Homebrew, winget).

## Pre-release / canary

Use a SemVer pre-release tag like `v0.4.2-rc1`:

```bash
git tag -a v0.4.2-rc1 -m "RC1: canary build"
git push origin v0.4.2-rc1
```

The GitHub release is marked `prerelease: true` automatically, and all ecosystem publish jobs are
skipped (they only run for tags without a `-`).

## crates.io publish order

vapour is a three-crate workspace. The CD workflow publishes them in dependency order with retries
to absorb index propagation delay:

```
vapour-api  →  vapour-core  →  vapour-tui
```

The installable binary crate is `vapour-tui` (it produces the `vapour` binary), so end users run
`cargo install vapour-tui`.

## Required repository secrets

| Secret | Used by | Notes |
| ------ | ------- | ----- |
| `CARGO_REGISTRY_TOKEN` | crates.io | API token from <https://crates.io/settings/tokens> |
| `AUR_SSH_PRIVATE_KEY`, `AUR_USERNAME`, `AUR_EMAIL` | AUR | SSH key registered with your AUR account |
| `HOMEBREW_TAP_SSH_KEY` | Homebrew | Deploy key with write access to the tap repo |
| `WINGET_TOKEN` | winget | PAT with access to your `winget-pkgs` fork |

## External repositories to create first

The publish jobs push to repositories that must already exist:

- **AUR:** `vapour` (build-from-source) and `vapour-bin` (prebuilt) packages.
- **Homebrew:** a tap repo `LargeModGames/homebrew-vapour` with a `Formula/` directory.
- **winget:** a fork of `microsoft/winget-pkgs` owned by `LargeModGames`.

If you only want GitHub Releases for now, leave those secrets unset — the publish jobs will fail
fast and the release itself (binaries + checksums) still succeeds. Remove or comment out the publish
jobs you don't use.
