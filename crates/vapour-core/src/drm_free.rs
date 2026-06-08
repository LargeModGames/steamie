//! The community-maintained list of games that launch without the Steam client.
//!
//! The canonical list ships in the repo-root [`DRM-FREE-GAMES.md`](../../../DRM-FREE-GAMES.md) and is
//! embedded into the binary at build time, so it works offline with no extra files to deploy. An
//! optional per-user list at `~/.config/vapour/drm-free.md` (same markdown-table format) is merged
//! on top. Used by the direct (no-Steam) launch path to decide which games to start without Steam.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The repo-root list, baked in at compile time.
const EMBEDDED_LIST: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../DRM-FREE-GAMES.md"));

static APPIDS: OnceLock<HashSet<u32>> = OnceLock::new();

/// Whether `appid` is known to launch without the Steam client.
pub fn is_known_drm_free(appid: u32) -> bool {
    appids().contains(&appid)
}

fn appids() -> &'static HashSet<u32> {
    APPIDS.get_or_init(|| {
        let mut set = parse_appids(EMBEDDED_LIST);
        if let Some(path) = user_list_path()
            && let Ok(text) = std::fs::read_to_string(path)
        {
            set.extend(parse_appids(&text));
        }
        set
    })
}

fn user_list_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("vapour").join("drm-free.md"))
}

/// Parse AppIDs from a markdown table: the first numeric cell of each `| … |` row. The header row
/// (`| AppID | Game |`) and separator (`|------:|------|`) have no numeric first cell and are
/// naturally skipped.
fn parse_appids(md: &str) -> HashSet<u32> {
    md.lines()
        .filter_map(|line| {
            let cell = line.trim().strip_prefix('|')?.split('|').next()?.trim();
            cell.parse::<u32>().ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_list_is_parseable_and_clean() {
        // Content-independent (the list is user-curated): the embedded file must parse, and the
        // header/separator rows must never be mistaken for appids.
        let set = parse_appids(EMBEDDED_LIST);
        assert!(!set.contains(&0));
        assert!(set.iter().all(|&id| id > 0));
    }

    #[test]
    fn known_drm_free_lookup_rejects_non_listed() {
        // DRM titles are never on the list, regardless of how the list is curated.
        assert!(!is_known_drm_free(730)); // CS2
        assert!(!is_known_drm_free(0));
    }

    #[test]
    fn parse_skips_header_and_separator_rows() {
        let md = "| AppID | Game |\n|------:|------|\n| 42 | Demo |\n| abc | Bad |";
        let set = parse_appids(md);
        assert_eq!(set, HashSet::from([42]));
    }
}
