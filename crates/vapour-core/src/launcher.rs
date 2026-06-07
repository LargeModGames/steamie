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
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};

/// How to launch a game. Built by the caller from `[launch]` config plus the selected appid.
#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    /// Explicit Steam executable path; `None`/empty means auto-detect.
    pub steam_path: Option<PathBuf>,
    /// Log the command instead of spawning it.
    pub dry_run: bool,
    /// If we started Steam, shut it down once the game exits (best-effort).
    pub kill_steam_on_exit: bool,
    /// Extra arguments appended after `-applaunch <appid>`.
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
}

/// Launch `appid` through Steam.
///
/// Returns quickly: it spawns the launch command and, when `kill_steam_on_exit` applies, a
/// detached watcher thread — it never blocks waiting on the game itself.
pub fn launch(appid: u32, opts: &LaunchOptions) -> Result<LaunchOutcome> {
    let exe = resolve_steam_exe(opts.steam_path.as_deref())?;
    let (program, args) = build_command(&exe, appid, &opts.args);
    let command_line = format_command(&program, &args);

    if opts.dry_run {
        return Ok(LaunchOutcome {
            command_line,
            started_steam: false,
            dry_run: true,
        });
    }

    let was_running = is_steam_running();

    Command::new(&program)
        .args(&args)
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
    })
}

/// Build the `(program, args)` for a Steam-mediated launch. Pure → unit-tested.
fn build_command(exe: &Path, appid: u32, extra: &[String]) -> (PathBuf, Vec<String>) {
    let mut args = vec!["-applaunch".to_owned(), appid.to_string()];
    args.extend(extra.iter().cloned());
    (exe.to_path_buf(), args)
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
    let _ = Command::new(exe).arg("-shutdown").spawn();
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
        let (program, args) = build_command(&exe, 730, &[]);
        assert_eq!(program, exe);
        assert_eq!(args, vec!["-applaunch".to_owned(), "730".to_owned()]);
    }

    #[test]
    fn build_command_appends_extra_args_in_order() {
        let exe = PathBuf::from("steam.exe");
        let extra = vec!["-novid".to_owned(), "-high".to_owned()];
        let (_program, args) = build_command(&exe, 570, &extra);
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
            kill_steam_on_exit: false,
            args: vec!["-windowed".to_owned()],
        };
        let outcome = launch(620, &opts).unwrap();
        assert!(outcome.dry_run);
        assert!(!outcome.started_steam);
        assert!(outcome.command_line.contains("-applaunch 620"));
        assert!(outcome.command_line.contains("-windowed"));
        let _ = std::fs::remove_file(&exe);
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
