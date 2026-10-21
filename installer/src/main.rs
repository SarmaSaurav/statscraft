// StatsCraft single-file installer.
// Embeds the prebuilt app and installs it silently to
// %LOCALAPPDATA%\Programs\StatsCraft (+ Start Menu shortcuts).
// No prompts, no archive extraction, no admin rights required.

use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

// Payloads embedded at compile time
static APP_EXE: &[u8] = include_bytes!("../../target/release/statscraft.exe");
static SAMPLE_CSV: &[u8] = include_bytes!("../payload/samples/demo.csv");
static README_MD: &[u8] = include_bytes!("../payload/README.md");
static UNINSTALL_PS: &str = r#"$ErrorActionPreference = "SilentlyContinue"
$dest = Join-Path $env:LOCALAPPDATA "Programs\StatsCraft"
$start = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
Remove-Item (Join-Path $start "StatsCraft.lnk") -Force
Remove-Item (Join-Path $start "Uninstall StatsCraft.lnk") -Force
Remove-Item -Recurse -Force $dest
"#;

static SHORTCUT_PS: &str = r#"$dest = Join-Path $env:LOCALAPPDATA "Programs\StatsCraft"
$start = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$wsh = New-Object -ComObject WScript.Shell
$sc = $wsh.CreateShortcut((Join-Path $start "StatsCraft.lnk"))
$sc.TargetPath = Join-Path $dest "statscraft.exe"
$sc.WorkingDirectory = $dest
$sc.Description = "StatsCraft - lightweight SPSS-like statistical workbench"
$sc.IconLocation = Join-Path $dest "statscraft.exe,0"
$sc.Save()
$usc = $wsh.CreateShortcut((Join-Path $start "Uninstall StatsCraft.lnk"))
$usc.TargetPath = "powershell.exe"
$usc.Arguments = "-ExecutionPolicy Bypass -File `"$dest\uninstall.ps1`""
$usc.Save()
"#;

fn install_dir() -> PathBuf {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(local).join("Programs").join("StatsCraft")
}

/// Remove an installation made by an earlier name (the project was called
/// StateCraft before it was renamed to StatsCraft). Without this the old
/// build, its Start Menu shortcuts and its install directory would linger.
fn remove_legacy_install() {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    if local.is_empty() {
        return;
    }
    let legacy_dir = PathBuf::from(&local).join("Programs").join("StateCraft");
    let legacy_exe = legacy_dir.join("statecraft.exe");
    if legacy_exe.exists() {
        let _ = Command::new("taskkill")
            .args(["/IM", "statecraft.exe", "/F"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
    }
    // Start Menu shortcuts from the old name.
    let start = std::env::var("APPDATA").unwrap_or_default();
    if !start.is_empty() {
        let start = PathBuf::from(start).join("Microsoft").join("Windows")
            .join("Start Menu").join("Programs");
        for name in ["StateCraft.lnk", "Uninstall StateCraft.lnk"] {
            let _ = fs::remove_file(start.join(name));
        }
    }
    if legacy_dir.exists() {
        let _ = fs::remove_dir_all(&legacy_dir);
    }
}

fn main() {
    let dest = install_dir();
    let app_path = dest.join("statscraft.exe");

    // Clear out an install made under the previous name.
    remove_legacy_install();

    // If the app is currently running, close it so the exe can be replaced.
    if app_path.exists() {
        let _ = Command::new("taskkill")
            .args(["/IM", "statscraft.exe", "/F"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
    }

    if let Err(e) = write_payload(&dest, "statscraft.exe", APP_EXE) {
        eprintln!("FAILED to install statscraft.exe: {}", e);
        eprintln!("Please close StatsCraft and run the installer again.");
        pause_and_exit(1);
    }
    let _ = write_payload(&dest.join("samples"), "demo.csv", SAMPLE_CSV);
    let _ = write_payload(&dest, "README.md", README_MD);
    let _ = write_payload(&dest, "uninstall.ps1", UNINSTALL_PS.as_bytes());

    // Create Start Menu shortcuts (per-user, no admin needed)
    let sc_ps = dest.join("_make_shortcuts.ps1");
    if fs::write(&sc_ps, SHORTCUT_PS).is_ok() {
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&sc_ps)
            .creation_flags(0x08000000)
            .status();
        let _ = fs::remove_file(&sc_ps);
    }

    println!("StatsCraft installed successfully");
    println!("Location: {}", dest.display());
    println!("Start Menu shortcut: StatsCraft");
    pause_and_exit(0);
}

fn write_payload(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    if let Err(e) = fs::create_dir_all(dir) {
        return Err(format!("create dir {}: {}", dir.display(), e));
    }
    let target = dir.join(name);
    fs::write(&target, bytes).map_err(|e| format!("write {}: {}", target.display(), e))
}

fn pause_and_exit(_code: i32) {
    // Keep the console open briefly so double-click users can read the result
    std::thread::sleep(std::time::Duration::from_millis(2500));
    std::process::exit(_code);
}
