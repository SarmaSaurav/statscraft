$ErrorActionPreference = "SilentlyContinue"
$dest = Join-Path $env:LOCALAPPDATA "Programs\StateCraft"
$start = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
Remove-Item (Join-Path $start "StateCraft.lnk") -Force
Remove-Item (Join-Path $start "Uninstall StateCraft.lnk") -Force
Remove-Item -Recurse -Force $dest
Write-Host "StateCraft uninstalled"