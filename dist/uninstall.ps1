$ErrorActionPreference = "SilentlyContinue"
$dest = Join-Path $env:LOCALAPPDATA "Programs\StatsCraft"
$start = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
Remove-Item (Join-Path $start "StatsCraft.lnk") -Force
Remove-Item (Join-Path $start "Uninstall StatsCraft.lnk") -Force
Remove-Item -Recurse -Force $dest
Write-Host "StatsCraft uninstalled"