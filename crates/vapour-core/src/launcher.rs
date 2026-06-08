//! Steam-mediated game launching (v0.4.0 "Launch Day").
//!
//! Every game launches through the official Steam client via `steam -applaunch <appid> [args]`.
//! Steam starts itself if it is not already running, so the core launch path needs no
//! "is Steam up?" probe. The probes here exist only to support the optional, best-effort
//! [`LaunchOptions::kill_steam_on_exit`] behaviour: shut Steam down once the launched game
//! exits, but only when *we* were the ones who started it.
//!
//! No extra crates are pulled in for this: on Windows we read Steam's registry via `reg.exe`;
//! on Linux we read `~/.steam/steam.pid` and `~/.steam/registry.vdf`; macOS is best-effort.
//! The string parsers are pure functions and unit-tested.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use vapour_protocol::LaunchEntry;

use crate::steam_apps::InstalledApp;

/// How to launch a game. Built by the caller from `[launch]` config plus the selected appid.
#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    /// Explicit Steam executable path; `None`/empty means auto-detect.
    pub steam_path: Option<PathBuf>,
    /// Log the command instead of spawning it.
    pub dry_run: bool,
    /// If we started Steam, shut it down once the game exits (best-effort).
    pub kill_steam_on_exit: bool,
    /// Start Steam quietly (minimized to tray) for Steam-mediated launches.
    pub silent: bool,
    /// Try the direct (no-Steam) launch path for eligible games.
    pub direct_launch: bool,
    /// With `direct_launch`, try the direct path even for games not on the DRM-free list.
    pub force_direct: bool,
    /// Extra arguments appended after `-applaunch <appid>` (Steam path) or after the game's own
    /// launch arguments (direct path).
    pub args: Vec<String>,
}

/// Result of a launch attempt — carries the exact command for dry-run display / logging.
#[derive(Debug, Clone)]
pub struct LaunchOutcome {
    /// The exact command line that was (or, for a dry run, would be) executed.
    pub command_line: String,
    /// Whether this call started Steam (false if Steam was already running or unknown).
    pub started_steam: bool,
    /// Whether this was a dry run (nothing spawned).
    pub dry_run: bool,
    /// Whether the game was launched directly, with no Steam client.
    pub direct: bool,
}

/// Launch `appid`, preferring the direct (no-Steam) path when it is enabled and the game is
/// eligible, and falling back to a Steam-mediated launch otherwise.
///
/// `entries` are the app's PICS `config/launch` options (from the library load); they are only
/// consulted on the direct path. Returns quickly — it never blocks waiting on the game.
pub fn launch_game(appid: u32, entries: &[LaunchEntry], opts: &LaunchOptions) -> Result<LaunchOutcome> {
    if opts.direct_launch
        && let Some(outcome) = try_direct_launch(appid, entries, opts)?
    {
        return Ok(outcome);
    }
    launch(appid, opts)
}

/// Launch `appid` through Steam.
///
/// Returns quickly: it spawns the launch command and, when `kill_steam_on_exit` applies, a
/// detached watcher thread — it never blocks waiting on the game itself.
pub fn launch(appid: u32, opts: &LaunchOptions) -> Result<LaunchOutcome> {
    let exe = resolve_steam_exe(opts.steam_path.as_deref())?;
    let (program, args) = build_command(&exe, appid, opts.silent, &opts.args);
    let command_line = format_command(&program, &args);

    if opts.dry_run {
        return Ok(LaunchOutcome {
            command_line,
            started_steam: false,
            dry_run: true,
            direct: false,
        });
    }

    let was_running = is_steam_running();

    detach_io(Command::new(&program).args(&args))
        .spawn()
        .with_context(|| format!("failed to launch Steam at {}", program.display()))?;

    let started_steam = !was_running;
    if started_steam && opts.kill_steam_on_exit {
        spawn_exit_watcher(exe, appid);
    }

    Ok(LaunchOutcome {
        command_line,
        started_steam,
        dry_run: false,
        direct: false,
    })
}

/// Detach a launch command from Vapour's terminal: the spawned process gets null stdio so it can't
/// paint over the TUI (some games — Factorio — log to stdout), and on Windows it is detached from
/// our console entirely. Without this, a launched game's console output corrupts the ratatui screen.
fn detach_io(cmd: &mut Command) -> &mut Command {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS — the child does not inherit/attach Vapour's console.
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(DETACHED_PROCESS);
    }
    cmd
}

/// Build the `(program, args)` for a Steam-mediated launch. Pure → unit-tested. `-silent` keeps
/// Steam in the tray (no window) when Vapour has to start it.
fn build_command(exe: &Path, appid: u32, silent: bool, extra: &[String]) -> (PathBuf, Vec<String>) {
    let mut args = Vec::new();
    if silent {
        args.push("-silent".to_owned());
    }
    args.push("-applaunch".to_owned());
    args.push(appid.to_string());
    args.extend(extra.iter().cloned());
    (exe.to_path_buf(), args)
}

/// The launch route chosen by [`plan_launch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchPlan {
    /// Run the game's own executable directly — Steam stays closed.
    Direct {
        exe: PathBuf,
        workingdir: PathBuf,
        args: Vec<String>,
    },
    /// Defer to Steam (`steam -applaunch`).
    Steam,
}

/// Decide how to launch `appid`, given what's installed, the app's launch entries, and whether it
/// is known DRM-free. Pure → unit-tested. Returns [`LaunchPlan::Steam`] unless every direct-launch
/// precondition holds: the direct path is enabled, the game is fully installed, it is DRM-free (or
/// `force_direct` is set), and a launch entry matches this OS.
pub fn plan_launch(
    installed: Option<&InstalledApp>,
    entries: &[LaunchEntry],
    drm_free: bool,
    opts: &LaunchOptions,
) -> LaunchPlan {
    if !opts.direct_launch {
        return LaunchPlan::Steam;
    }
    let Some(app) = installed.filter(|app| app.is_fully_installed()) else {
        return LaunchPlan::Steam;
    };
    if !(drm_free || opts.force_direct) {
        return LaunchPlan::Steam;
    }
    let Some(entry) = select_launch_entry(entries, current_os()) else {
        return LaunchPlan::Steam;
    };

    let install = app.install_path();
    let exe = install.join(normalize_rel(&entry.executable));
    let workingdir = match entry.workingdir.as_deref() {
        Some(dir) if !dir.is_empty() => install.join(normalize_rel(dir)),
        _ => install,
    };
    let mut args: Vec<String> = entry
        .arguments
        .as_deref()
        .map(split_args)
        .unwrap_or_default();
    args.extend(opts.args.iter().cloned());

    LaunchPlan::Direct {
        exe,
        workingdir,
        args,
    }
}

/// Resolve a direct launch for `appid`, or `Ok(None)` to fall back to Steam (game not eligible,
/// not installed, no launch metadata, Steam dir unresolvable, or the spawn itself failed).
fn try_direct_launch(
    appid: u32,
    entries: &[LaunchEntry],
    opts: &LaunchOptions,
) -> Result<Option<LaunchOutcome>> {
    let Ok(steam_exe) = resolve_steam_exe(opts.steam_path.as_deref()) else {
        return Ok(None);
    };
    let Some(steam_root) = steam_exe.parent() else {
        return Ok(None);
    };
    let installed = crate::steam_apps::find_installed(steam_root, appid);
    let drm_free = crate::drm_free::is_known_drm_free(appid);

    let LaunchPlan::Direct {
        exe,
        workingdir,
        args,
    } = plan_launch(installed.as_ref(), entries, drm_free, opts)
    else {
        return Ok(None);
    };

    let command_line = format_command(&exe, &args);
    if opts.dry_run {
        return Ok(Some(LaunchOutcome {
            command_line,
            started_steam: false,
            dry_run: true,
            direct: true,
        }));
    }

    match detach_io(Command::new(&exe).current_dir(&workingdir).args(&args)).spawn() {
        Ok(_) => Ok(Some(LaunchOutcome {
            command_line,
            started_steam: false,
            dry_run: false,
            direct: true,
        })),
        // Spawn failed (e.g. a stale launch entry pointing at a missing exe): fall back to Steam.
        Err(_) => Ok(None),
    }
}

/// Pick the best `config/launch` entry for `os` ("windows"/"linux"/"macos"), or `None` if none
/// applies. Skips beta-gated and explicitly-disabled (`type "none"`) entries; prefers an entry
/// that names this OS over an OS-agnostic one.
fn select_launch_entry<'a>(entries: &'a [LaunchEntry], os: &str) -> Option<&'a LaunchEntry> {
    let mut os_specific: Option<&LaunchEntry> = None;
    let mut os_agnostic: Option<&LaunchEntry> = None;
    for entry in entries {
        if entry.executable.is_empty() || entry.betakey.is_some() {
            continue;
        }
        if entry
            .launch_type
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("none"))
        {
            continue;
        }
        match entry.oslist.as_deref() {
            Some(list) if oslist_contains(list, os) => os_specific.get_or_insert(entry),
            Some(_) => continue, // names other OSes but not ours
            None => os_agnostic.get_or_insert(entry),
        };
    }
    os_specific.or(os_agnostic)
}

fn oslist_contains(list: &str, os: &str) -> bool {
    list.split(',').any(|item| item.trim().eq_ignore_ascii_case(os))
}

fn current_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// Normalize a Steam-relative path (which may use `\` separators) for `Path::join`.
fn normalize_rel(rel: &str) -> String {
    rel.replace('\\', "/")
}

/// Split launch arguments on whitespace (no quoted-group handling — a known limitation shared with
/// the per-game `game_args` config).
fn split_args(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_owned).collect()
}

/// Render a command for display/logging (dry-run). Not shell-escaped — informational only.
fn format_command(program: &Path, args: &[String]) -> String {
    let mut parts = vec![program.display().to_string()];
    parts.extend(args.iter().cloned());
    parts.join(" ")
}

/// Resolve the Steam executable, honouring an explicit override first.
fn resolve_steam_exe(override_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = override_path
        && !p.as_os_str().is_empty()
    {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
        return Err(anyhow!(
            "configured [launch] steam_path does not exist: {}",
            p.display()
        ));
    }
    platform_steam_exe().ok_or_else(|| {
        anyhow!("could not locate the Steam executable; set [launch] steam_path in config.toml")
    })
}

// --- Steam lifecycle probes (best-effort, used only for kill-on-exit) ---

/// Shut Steam down. Steam interprets `-shutdown` and exits its running instance.
fn shutdown_steam(exe: &Path) {
    let _ = detach_io(Command::new(exe).arg("-shutdown")).spawn();
}

/// Watch a launched game and, once it exits, shut the Steam *we* started back down.
fn spawn_exit_watcher(exe: PathBuf, appid: u32) {
    std::thread::spawn(move || {
        // Wait (bounded) for the game to actually register as running. If it never does
        // — wrong appid, install missing, unsupported platform probe — leave Steam alone.
        if !wait_until(Duration::from_secs(60), || is_app_running(appid)) {
            return;
        }
        // Game is running; poll until it stops.
        while is_app_running(appid) {
            std::thread::sleep(Duration::from_secs(10));
        }
        shutdown_steam(&exe);
    });
}

/// Poll `cond` every couple of seconds until it is true or `timeout` elapses.
fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let step = Duration::from_secs(2);
    let mut waited = Duration::ZERO;
    while waited < timeout {
        if cond() {
            return true;
        }
        std::thread::sleep(step);
        waited += step;
    }
    cond()
}

#[cfg(windows)]
fn platform_steam_exe() -> Option<PathBuf> {
    // Authoritative: HKCU\Software\Valve\Steam\SteamExe.
    if let Some(p) = reg_query_sz(r"HKCU\Software\Valve\Steam", "SteamExe") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    // Fallbacks: the standard install location.
    let mut candidates = Vec::new();
    if let Some(pf) = std::env::var_os("ProgramFiles(x86)") {
        candidates.push(PathBuf::from(pf).join("Steam").join("steam.exe"));
    }
    candidates.push(PathBuf::from(r"C:\Program Files (x86)\Steam\steam.exe"));
    candidates.into_iter().find(|p| p.exists())
}

#[cfg(windows)]
fn is_steam_running() -> bool {
    reg_query_dword(r"HKCU\Software\Valve\Steam\ActiveProcess", "pid").is_some_and(|pid| pid != 0)
}

#[cfg(windows)]
fn is_app_running(appid: u32) -> bool {
    reg_query_dword(&format!(r"HKCU\Software\Valve\Steam\Apps\{appid}"), "Running")
        .is_some_and(|v| v != 0)
}

#[cfg(windows)]
fn reg_query_sz(key: &str, value: &str) -> Option<String> {
    let out = Command::new("reg")
        .args(["query", key, "/v", value])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_reg_sz(&String::from_utf8_lossy(&out.stdout), value)
}

#[cfg(windows)]
fn reg_query_dword(key: &str, value: &str) -> Option<u64> {
    let out = Command::new("reg")
        .args(["query", key, "/v", value])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_reg_dword(&String::from_utf8_lossy(&out.stdout), value)
}

/// Extract a `REG_SZ` value from `reg query` output. Pure → unit-tested.
#[cfg(any(windows, test))]
fn parse_reg_sz(stdout: &str, value: &str) -> Option<String> {
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(value)
            && let Some(after) = rest.trim_start().strip_prefix("REG_SZ")
        {
            return Some(after.trim().to_owned());
        }
    }
    None
}

/// Extract a `REG_DWORD` value (hex) from `reg query` output. Pure → unit-tested.
#[cfg(any(windows, test))]
fn parse_reg_dword(stdout: &str, value: &str) -> Option<u64> {
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(value)
            && let Some(after) = rest.trim_start().strip_prefix("REG_DWORD")
        {
            let hex = after.trim();
            let hex = hex
                .strip_prefix("0x")
                .or_else(|| hex.strip_prefix("0X"))
                .unwrap_or(hex);
            return u64::from_str_radix(hex, 16).ok();
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn platform_steam_exe() -> Option<PathBuf> {
    if let Some(p) = which("steam") {
        return Some(p);
    }
    if let Some(home) = dirs::home_dir() {
        for rel in [
            ".steam/steam.sh",
            ".local/share/Steam/steam.sh",
            ".steam/root/ubuntu12_32/steam",
        ] {
            let p = home.join(rel);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn is_steam_running() -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let Ok(contents) = std::fs::read_to_string(home.join(".steam/steam.pid")) else {
        return false;
    };
    let Ok(pid) = contents.trim().parse::<u32>() else {
        return false;
    };
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(target_os = "linux")]
fn is_app_running(appid: u32) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let Ok(contents) = std::fs::read_to_string(home.join(".steam/registry.vdf")) else {
        return false;
    };
    parse_registry_vdf_running(&contents, appid)
}

#[cfg(target_os = "macos")]
fn platform_steam_exe() -> Option<PathBuf> {
    let app = PathBuf::from("/Applications/Steam.app/Contents/MacOS/steam_osx");
    if app.exists() {
        return Some(app);
    }
    which("steam")
}

// macOS has no cheap, dependency-free running probe; treat Steam as "unknown" (never auto-kill).
#[cfg(target_os = "macos")]
fn is_steam_running() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn is_app_running(_appid: u32) -> bool {
    false
}

#[cfg(unix)]
fn which(cmd: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(cmd))
        .find(|p| p.is_file())
}

/// Best-effort parse of Steam's Linux `registry.vdf`: is `appid`'s block flagged `running`?
///
/// Finds the `"<appid>" { … }` block and looks for `"running" "1"` inside it. The VDF nesting
/// is brace-balanced so we only inspect that app's own block.
#[cfg(any(target_os = "linux", test))]
fn parse_registry_vdf_running(vdf: &str, appid: u32) -> bool {
    let needle = format!("\"{appid}\"");
    let Some(start) = vdf.find(&needle) else {
        return false;
    };
    let after = &vdf[start + needle.len()..];
    let Some(open) = after.find('{') else {
        return false;
    };
    let mut depth = 0i32;
    let mut end = after.len();
    for (i, b) in after[open..].bytes().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = open + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    let block = after[open..end].to_lowercase();
    if let Some(rp) = block.find("\"running\"") {
        let tail = block[rp + "\"running\"".len()..]
            .trim_start()
            .trim_start_matches('"');
        return tail.starts_with('1');
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_steam_exe(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("vapour_test_steam_{tag}"));
        std::fs::write(&p, b"").unwrap();
        p
    }

    #[test]
    fn build_command_prepends_applaunch_and_appid() {
        let exe = PathBuf::from("/opt/steam/steam");
        let (program, args) = build_command(&exe, 730, false, &[]);
        assert_eq!(program, exe);
        assert_eq!(args, vec!["-applaunch".to_owned(), "730".to_owned()]);
    }

    #[test]
    fn build_command_prepends_silent_flag_before_applaunch() {
        let exe = PathBuf::from("steam.exe");
        let (_program, args) = build_command(&exe, 730, true, &[]);
        assert_eq!(args, vec!["-silent", "-applaunch", "730"]);
    }

    #[test]
    fn build_command_appends_extra_args_in_order() {
        let exe = PathBuf::from("steam.exe");
        let extra = vec!["-novid".to_owned(), "-high".to_owned()];
        let (_program, args) = build_command(&exe, 570, false, &extra);
        assert_eq!(args, vec!["-applaunch", "570", "-novid", "-high"]);
    }

    #[test]
    fn format_command_joins_program_and_args() {
        let line = format_command(Path::new("steam.exe"), &["-applaunch".into(), "440".into()]);
        assert_eq!(line, "steam.exe -applaunch 440");
    }

    #[test]
    fn resolve_steam_exe_uses_existing_override() {
        let exe = temp_steam_exe("override");
        let resolved = resolve_steam_exe(Some(&exe)).unwrap();
        assert_eq!(resolved, exe);
        let _ = std::fs::remove_file(&exe);
    }

    #[test]
    fn resolve_steam_exe_errors_on_missing_override() {
        let missing = PathBuf::from("/definitely/not/here/steam.exe");
        assert!(resolve_steam_exe(Some(&missing)).is_err());
    }

    #[test]
    fn dry_run_returns_command_without_spawning() {
        let exe = temp_steam_exe("dryrun");
        let opts = LaunchOptions {
            steam_path: Some(exe.clone()),
            dry_run: true,
            silent: true,
            args: vec!["-windowed".to_owned()],
            ..Default::default()
        };
        let outcome = launch(620, &opts).unwrap();
        assert!(outcome.dry_run);
        assert!(!outcome.started_steam);
        assert!(!outcome.direct);
        assert!(outcome.command_line.contains("-silent"));
        assert!(outcome.command_line.contains("-applaunch 620"));
        assert!(outcome.command_line.contains("-windowed"));
        let _ = std::fs::remove_file(&exe);
    }

    fn installed_app(state_flags: u32) -> InstalledApp {
        InstalledApp {
            appid: 105600,
            name: "Terraria".to_owned(),
            installdir: "Terraria".to_owned(),
            library_path: PathBuf::from("/lib"),
            state_flags,
        }
    }

    fn os_agnostic_entry() -> LaunchEntry {
        LaunchEntry {
            executable: "game.exe".to_owned(),
            arguments: Some("-fullscreen".to_owned()),
            ..Default::default()
        }
    }

    fn direct_opts() -> LaunchOptions {
        LaunchOptions {
            direct_launch: true,
            ..Default::default()
        }
    }

    #[test]
    fn plan_launch_defers_to_steam_when_direct_disabled() {
        let app = installed_app(4);
        let entries = [os_agnostic_entry()];
        let opts = LaunchOptions::default(); // direct_launch = false
        assert_eq!(plan_launch(Some(&app), &entries, true, &opts), LaunchPlan::Steam);
    }

    #[test]
    fn plan_launch_defers_when_not_installed_or_not_complete() {
        let entries = [os_agnostic_entry()];
        assert_eq!(plan_launch(None, &entries, true, &direct_opts()), LaunchPlan::Steam);
        let updating = installed_app(1026); // fully-installed bit (4) not set
        assert_eq!(
            plan_launch(Some(&updating), &entries, true, &direct_opts()),
            LaunchPlan::Steam
        );
    }

    #[test]
    fn plan_launch_defers_when_not_drm_free_and_not_forced() {
        let app = installed_app(4);
        let entries = [os_agnostic_entry()];
        assert_eq!(
            plan_launch(Some(&app), &entries, false, &direct_opts()),
            LaunchPlan::Steam
        );
    }

    #[test]
    fn plan_launch_direct_for_drm_free_installed_game() {
        let app = installed_app(4);
        let entries = [os_agnostic_entry()];
        let plan = plan_launch(Some(&app), &entries, true, &direct_opts());
        let expected_exe = PathBuf::from("/lib")
            .join("steamapps")
            .join("common")
            .join("Terraria")
            .join("game.exe");
        match plan {
            LaunchPlan::Direct { exe, workingdir, args } => {
                assert_eq!(exe, expected_exe);
                assert_eq!(workingdir, app.install_path());
                assert_eq!(args, vec!["-fullscreen".to_owned()]);
            }
            LaunchPlan::Steam => panic!("expected a direct plan"),
        }
    }

    #[test]
    fn plan_launch_force_direct_overrides_drm_free_gate() {
        let app = installed_app(4);
        let entries = [os_agnostic_entry()];
        let opts = LaunchOptions {
            direct_launch: true,
            force_direct: true,
            ..Default::default()
        };
        assert!(matches!(
            plan_launch(Some(&app), &entries, false, &opts),
            LaunchPlan::Direct { .. }
        ));
    }

    #[test]
    fn plan_launch_appends_user_args_after_game_args() {
        let app = installed_app(4);
        let entries = [os_agnostic_entry()];
        let opts = LaunchOptions {
            direct_launch: true,
            args: vec!["-extra".to_owned()],
            ..Default::default()
        };
        match plan_launch(Some(&app), &entries, true, &opts) {
            LaunchPlan::Direct { args, .. } => assert_eq!(args, vec!["-fullscreen", "-extra"]),
            LaunchPlan::Steam => panic!("expected a direct plan"),
        }
    }

    #[test]
    fn select_launch_entry_prefers_matching_os() {
        let entries = vec![
            LaunchEntry {
                executable: "game_linux".to_owned(),
                oslist: Some("linux".to_owned()),
                ..Default::default()
            },
            LaunchEntry {
                executable: "game.exe".to_owned(),
                oslist: Some("windows".to_owned()),
                ..Default::default()
            },
        ];
        assert_eq!(select_launch_entry(&entries, "windows").unwrap().executable, "game.exe");
        assert_eq!(select_launch_entry(&entries, "linux").unwrap().executable, "game_linux");
        // No entry names macOS → none selected.
        assert!(select_launch_entry(&entries, "macos").is_none());
    }

    #[test]
    fn select_launch_entry_falls_back_to_os_agnostic() {
        let entries = vec![LaunchEntry {
            executable: "game.exe".to_owned(),
            ..Default::default()
        }];
        assert_eq!(select_launch_entry(&entries, "macos").unwrap().executable, "game.exe");
    }

    /// Live, machine-specific: dry-runs the full direct-launch chain (resolve Steam → find the
    /// installed game → plan → command) against a real installed game, with a synthesized launch
    /// entry. Ignored by default; run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn live_direct_launch_dry_run_resolves_installed_exe() {
        let entries = [LaunchEntry {
            executable: "game.exe".to_owned(),
            ..Default::default()
        }];
        let opts = LaunchOptions {
            dry_run: true,
            direct_launch: true,
            force_direct: true, // Civ VII isn't on the DRM-free list
            ..Default::default()
        };
        let outcome = launch_game(1295660, &entries, &opts).expect("dry-run launch");
        assert!(outcome.dry_run);
        assert!(outcome.direct, "expected the direct (no-Steam) path");
        assert!(
            outcome.command_line.contains("Civilization VII"),
            "command should reference the real install dir: {}",
            outcome.command_line
        );
        assert!(outcome.command_line.ends_with("game.exe"));
    }

    #[test]
    fn select_launch_entry_skips_disabled_and_beta_entries() {
        let entries = vec![
            LaunchEntry {
                executable: "installer.exe".to_owned(),
                launch_type: Some("none".to_owned()),
                ..Default::default()
            },
            LaunchEntry {
                executable: "beta.exe".to_owned(),
                betakey: Some("beta".to_owned()),
                ..Default::default()
            },
            LaunchEntry {
                executable: "game.exe".to_owned(),
                ..Default::default()
            },
        ];
        assert_eq!(select_launch_entry(&entries, "windows").unwrap().executable, "game.exe");
    }

    #[test]
    fn parse_reg_sz_extracts_path() {
        let out = "\r\nHKEY_CURRENT_USER\\Software\\Valve\\Steam\r\n    SteamExe    REG_SZ    c:\\program files (x86)\\steam\\steam.exe\r\n\r\n";
        assert_eq!(
            parse_reg_sz(out, "SteamExe").as_deref(),
            Some("c:\\program files (x86)\\steam\\steam.exe")
        );
        assert_eq!(parse_reg_sz(out, "Missing"), None);
    }

    #[test]
    fn parse_reg_dword_extracts_hex_value() {
        let running = "\r\n    Running    REG_DWORD    0x1\r\n";
        assert_eq!(parse_reg_dword(running, "Running"), Some(1));
        let pid = "    pid    REG_DWORD    0x2a3c\r\n";
        assert_eq!(parse_reg_dword(pid, "pid"), Some(0x2a3c));
        let zero = "    Running    REG_DWORD    0x0\r\n";
        assert_eq!(parse_reg_dword(zero, "Running"), Some(0));
    }

    #[test]
    fn registry_vdf_running_reads_app_block() {
        let vdf = r#"
"Registry"
{
    "HKCU"
    {
        "Software" { "Valve" { "Steam" { "apps"
        {
            "730" { "running" "1" "installed" "1" }
            "570" { "running" "0" "installed" "1" }
        } } } }
    }
}
"#;
        assert!(parse_registry_vdf_running(vdf, 730));
        assert!(!parse_registry_vdf_running(vdf, 570));
        assert!(!parse_registry_vdf_running(vdf, 999));
    }
}
