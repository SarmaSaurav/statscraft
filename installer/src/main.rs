// StateCraft single-file installer.
// Embeds the prebuilt app and installs it silently to
// %LOCALAPPDATA%\Programs\StateCraft (+ Start Menu shortcuts).
// No prompts, no archive extraction, no admin rights required.

use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

// Payloads embedded at compile time
static APP_EXE: &[u8] = include_bytes!("../../target/release/statecraft.exe");
static SAMPLE_CSV: &[u8] = include_bytes!("../payload/samples/demo.csv");
static README_MD: &[u8] = include_bytes!("../payload/README.md");
static UNINSTALL_PS: &str = r#"$ErrorActionPreference = "SilentlyContinue"
$dest = Join-Path $env:LOCALAPPDATA "Programs\StateCraft"
$start = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
Remove-Item (Join-Path $start "StateCraft.lnk") -Force
Remove-Item (Join-Path $start "Uninstall StateCraft.lnk") -Force
Remove-Item -Recurse -Force $dest
"#;

const SHORTCUT_PS: &str = r#"$dest = Join-Path $env:LOCALAPPDATA "Programs\StateCraft"
$start = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$wsh = New-Object -ComObject WScript.Shell
$sc = $wsh.CreateShortcut((Join-Path $start "StateCraft.lnk"))
$sc.TargetPath = Join-Path $dest "statecraft.exe"
$sc.WorkingDirectory = $dest
$sc.Description = "StateCraft - lightweight SPSS-like statistical workbench"
$sc.Save()
$usc = $wsh.CreateShortcut((Join-Path $start "Uninstall StateCraft.lnk"))
$usc.TargetPath = "powershell.exe"
$usc.Arguments = "-ExecutionPolicy Bypass -File `"$dest\uninstall.ps1`""
$usc.Save()
"#;

fn install_dir() -> PathBuf {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(local).join("Programs").join("StateCraft")
}

fn main() {
    let dest = install_dir();
    let app_path = dest.join("statecraft.exe");

    // If the app is currently running, close it so the exe can be replaced.
    if app_path.exists() {
        let _ = Command::new("taskkill")
            .args(["/IM", "statecraft.exe", "/F"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
    }

    if let Err(e) = write_payload(&dest, "statecraft.exe", APP_EXE) {
        eprintln!("FAILED to install statecraft.exe: {}", e);
        eprintln!("Please close StateCraft and run the installer again.");
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

    println!("StateCraft installed successfully");
    println!("Location: {}", dest.display());
    println!("Start Menu shortcut: StateCraft");
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
