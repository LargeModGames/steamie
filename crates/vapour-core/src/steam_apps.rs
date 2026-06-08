//! Local Steam install detection: which owned apps are installed on disk and where.
//!
//! Steam records installs in two text-VDF files under each library's `steamapps/` directory:
//! `libraryfolders.vdf` maps every library `path`, and `appmanifest_<appid>.acf` records each
//! installed app's `installdir` + `StateFlags`. The game's files live at
//! `<library>/steamapps/common/<installdir>/`. We read these directly — no Steam process, no
//! network. Used by the direct (no-Steam) launch path to resolve a game's executable.

use std::path::{Path, PathBuf};

use crate::vdf::{self, VdfValue};

/// `StateFlags` bit set when an app is fully installed (not updating / staging).
const STATE_FULLY_INSTALLED: u32 = 4;

/// An app found installed in a Steam library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub appid: u32,
    pub name: String,
    /// Folder name under `steamapps/common/`.
    pub installdir: String,
    /// Library root that contains this app (e.g. `E:\Games\Steam`).
    pub library_path: PathBuf,
    /// Raw `StateFlags` from the appmanifest.
    pub state_flags: u32,
}

impl InstalledApp {
    /// Absolute path to the game's install directory.
    pub fn install_path(&self) -> PathBuf {
        self.library_path
            .join("steamapps")
            .join("common")
            .join(&self.installdir)
    }

    /// Whether the app is fully installed (ready to launch).
    pub fn is_fully_installed(&self) -> bool {
        self.state_flags & STATE_FULLY_INSTALLED != 0
    }
}

/// Find an installed app by appid across all of `steam_root`'s libraries, or `None` if not
/// installed. `steam_root` is the Steam install directory (the parent of `steam.exe`).
pub fn find_installed(steam_root: &Path, appid: u32) -> Option<InstalledApp> {
    for library in library_paths(steam_root) {
        let acf = library
            .join("steamapps")
            .join(format!("appmanifest_{appid}.acf"));
        if let Ok(text) = std::fs::read_to_string(&acf)
            && let Some(app) = parse_appmanifest(&text, &library)
        {
            return Some(app);
        }
    }
    None
}

/// All library roots known to this Steam install: the root itself plus every `path` in
/// `steamapps/libraryfolders.vdf`.
pub fn library_paths(steam_root: &Path) -> Vec<PathBuf> {
    let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
    let from_file = std::fs::read_to_string(&vdf_path)
        .ok()
        .map(|text| parse_library_paths(&text))
        .unwrap_or_default();

    let mut paths = vec![steam_root.to_path_buf()];
    paths.extend(from_file);
    paths.sort();
    paths.dedup();
    paths
}

/// Pure: extract library `path` values from `libraryfolders.vdf` text.
fn parse_library_paths(text: &str) -> Vec<PathBuf> {
    let Some(root) = vdf::parse(text) else {
        return Vec::new();
    };
    let Some(folders) = root.map("libraryfolders") else {
        return Vec::new();
    };
    folders
        .entries()
        .iter()
        .filter_map(|(_, value)| match value {
            VdfValue::Map(m) => m.str("path").map(PathBuf::from),
            VdfValue::Str(_) => None,
        })
        .collect()
}

/// Pure: parse an `appmanifest_<appid>.acf` body into an [`InstalledApp`].
fn parse_appmanifest(text: &str, library_path: &Path) -> Option<InstalledApp> {
    let root = vdf::parse(text)?;
    let state = root.map("AppState")?;
    let appid = state.str("appid")?.trim().parse().ok()?;
    let installdir = state.str("installdir")?.to_owned();
    if installdir.is_empty() {
        return None;
    }
    let name = state.str("name").unwrap_or_default().to_owned();
    let state_flags = state
        .str("StateFlags")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    Some(InstalledApp {
        appid,
        name,
        installdir,
        library_path: library_path.to_path_buf(),
        state_flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CIV_VII_ACF: &str = r#"
"AppState"
{
	"appid"		"1295660"
	"name"		"Sid Meier's Civilization VII"
	"StateFlags"		"4"
	"installdir"		"Sid Meier's Civilization VII"
}
"#;

    const LIBRARYFOLDERS: &str = r#"
"libraryfolders"
{
	"0" { "path" "C:\\Program Files (x86)\\Steam" }
	"1" { "path" "E:\\Games\\Steam" }
}
"#;

    #[test]
    fn parses_appmanifest_into_installed_app() {
        let lib = PathBuf::from(r"E:\Games\Steam");
        let app = parse_appmanifest(CIV_VII_ACF, &lib).expect("acf parses");
        assert_eq!(app.appid, 1295660);
        assert_eq!(app.name, "Sid Meier's Civilization VII");
        assert_eq!(app.installdir, "Sid Meier's Civilization VII");
        assert_eq!(app.state_flags, 4);
        assert!(app.is_fully_installed());
        assert_eq!(
            app.install_path(),
            PathBuf::from(r"E:\Games\Steam\steamapps\common\Sid Meier's Civilization VII")
        );
    }

    #[test]
    fn not_fully_installed_when_state_flag_unset() {
        let lib = PathBuf::from("/lib");
        let acf = r#""AppState" { "appid" "1" "installdir" "X" "StateFlags" "1026" }"#;
        let app = parse_appmanifest(acf, &lib).unwrap();
        assert!(!app.is_fully_installed());
    }

    #[test]
    fn parses_library_paths_from_vdf() {
        let paths = parse_library_paths(LIBRARYFOLDERS);
        assert_eq!(
            paths,
            vec![
                PathBuf::from(r"C:\Program Files (x86)\Steam"),
                PathBuf::from(r"E:\Games\Steam"),
            ]
        );
    }

    /// Live, machine-specific: confirms install detection finds a real game. Ignored by default
    /// (depends on this machine's Steam libraries); run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn live_find_installed_civ_vii() {
        let steam_root = PathBuf::from(r"C:\Program Files (x86)\Steam");
        let app = find_installed(&steam_root, 1295660).expect("Civ VII should be installed");
        assert_eq!(app.library_path, PathBuf::from(r"E:\Games\Steam"));
        assert!(app.is_fully_installed());
        assert!(app.install_path().exists());
    }
}
