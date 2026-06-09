# Contributing to vapour

Thanks for your interest in vapour! 🎮 We welcome all kinds of contributions — code, docs,
bug reports, and additions to the DRM-free games list.

## Ways to Contribute

### 🐛 Report Bugs
Found something broken? [Open an issue](https://github.com/LargeModGames/vapour/issues/new/choose) with:
- What you expected vs what happened
- Steps to reproduce
- Your OS, terminal, and vapour version (`vapour --version`)

### 💡 Suggest Features
Have an idea? Start a [Discussion](https://github.com/LargeModGames/vapour/discussions) or open an
issue. We love hearing what would make vapour better for you.

### 🎯 Add Games to the DRM-Free List
[`DRM-FREE-GAMES.md`](DRM-FREE-GAMES.md) is a community-maintained table of Steam games that run with
**Steam fully closed** (used by the experimental direct-launch path). To add a game:
1. Quit Steam entirely.
2. Confirm the game still launches from its `steamapps/common/<game>/` executable.
3. Add a row with its AppID and name, and open a PR.

### 📖 Improve Documentation
Fix typos, clarify setup steps, or expand the README — all welcome.

---

## Code Contributions

### Project layout
vapour is a Cargo workspace with three crates:

- **`vapour-api`** — Steam Web API / Store API HTTP client.
- **`vapour-core`** — config, session, caching, launcher, local chat history.
- **`vapour-tui`** — the ratatui terminal UI (produces the `vapour` binary).

The raw Steam connection-manager protocol lives in a **separate** crate,
[`vapour-protocol`](https://github.com/LargeModGames/vapour-protocol)
([crates.io](https://crates.io/crates/vapour-protocol)). Protocol-level changes belong in that repo.

### Getting set up
1. Install a recent stable Rust toolchain (`rustup` recommended). vapour requires **Rust 1.85+**
   (edition 2024).
2. No special system libraries are required — networking uses `rustls` and notifications use `zbus`,
   so there's no OpenSSL, X11, or audio dependency to install.
3. Clone your fork and create a topic branch from `main`.

```bash
git clone https://github.com/LargeModGames/vapour.git
cd vapour
cargo run --bin vapour
```

### Before opening a PR
Run the same checks CI runs:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### PR tips
- Open an issue first for new features or larger refactors.
- Add or adjust tests when changing behavior.
- Update `README.md` and `CHANGELOG.md` for user-facing changes.
- Include a screenshot or recording for UI changes.
- Keep PRs focused; keep commits logical (squashing welcome but not required).

---

## Ground Rules
Be kind and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## Questions?
Start a [Discussion](https://github.com/LargeModGames/vapour/discussions) or ask in an issue — happy to help!
