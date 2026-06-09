# AUR packaging

Seed `PKGBUILD`s for the two AUR packages:

| Package | Builds from | Use |
|---------|-------------|-----|
| [`steamie`](./steamie/PKGBUILD) | source (cargo) | `yay -S steamie` |
| [`steamie-bin`](./steamie-bin/PKGBUILD) | the prebuilt `steamie-linux-x86_64.tar.gz` release asset | `yay -S steamie-bin` (faster) |

These are the **initial seeds**. After the first import, the release pipeline
(`.github/workflows/cd.yml`, jobs `publish-aur` / `publish-aur-bin`) keeps the
AUR copies up to date on every tagged release: it bumps `pkgver`/`pkgrel`,
recomputes the `sha256sums`, regenerates `.SRCINFO`, and pushes. The
copies here are the source of truth for that first import only — they are not
read by CI (CI mutates the AUR repos directly).

> The source `PKGBUILD` deliberately keeps `build() {` on its own line: the CD
> injects `export CARGO_PROFILE_RELEASE_LTO=false` right after it to avoid OOM
> on AUR builders. The bin `PKGBUILD` keeps each `sha256sums*` assignment on a
> single line so the CD's line-delete/insert `sed`s stay correct.

## First-time import (run once per package, on an Arch host)

You need an [AUR account](https://aur.archlinux.org) with your SSH key
registered. From this directory:

```bash
for pkg in steamie steamie-bin; do
  git clone "ssh://aur@aur.archlinux.org/$pkg.git" "/tmp/aur-$pkg"
  cp "$pkg/PKGBUILD" "/tmp/aur-$pkg/"
  ( cd "/tmp/aur-$pkg" \
      && makepkg --printsrcinfo > .SRCINFO \
      && git add PKGBUILD .SRCINFO \
      && git commit -m "Initial import: $pkg 0.4.1" \
      && git push )
done
```

Importing now **reserves the `steamie` / `steamie-bin` names on the AUR**
(worth doing promptly, since name collisions are exactly what the rebrand
avoided). The packages won't be installable until the GitHub repo is public
and a `v0.4.1` release exists — the `SKIP` checksums become real on the first
CD run, which fires from the release tag.
