$ErrorActionPreference = "Stop"
$src = Split-Path -Parent $MyInvocation.MyCommand.Path
$dest = Join-Path $env:LOCALAPPDATA "Programs\StatsCraft"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item -Path (Join-Path $src "*") -Destination $dest -Recurse -Force
$start = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$wsh = New-Object -ComObject WScript.Shell
$sc = $wsh.CreateShortcut((Join-Path $start "StatsCraft.lnk"))
$sc.TargetPath = Join-Path $dest "statscraft.exe"
$sc.WorkingDirectory = $dest
$sc.Description = "StatsCraft - lightweight SPSS-like statistical workbench"
$sc.Save()
$usc = $wsh.CreateShortcut((Join-Path $start "Uninstall StatsCraft.lnk"))
$usc.TargetPath = "powershell.exe"
$usc.Arguments = "-ExecutionPolicy Bypass -File `"$dest\uninstall.ps1`""
$usc.Save()
Write-Host "StatsCraft installed to $dest"