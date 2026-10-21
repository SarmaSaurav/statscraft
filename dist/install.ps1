$ErrorActionPreference = "Stop"
$src = Split-Path -Parent $MyInvocation.MyCommand.Path
$dest = Join-Path $env:LOCALAPPDATA "Programs\StateCraft"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item -Path (Join-Path $src "*") -Destination $dest -Recurse -Force
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
Write-Host "StateCraft installed to $dest"