param(
  [string]$Proc = "statscraft",
  [string]$Data = "samples\survey_demo.csv",
  [string]$Dialog = "frequencies",
  [string]$Out = "C:\Users\Saurav\AppData\Local\Temp\opencode\dlg_shot.png"
)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WG {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
"@

$dir = (Get-Location).Path
Get-Process -Name $Proc -EA SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

$p = Start-Process -FilePath ".\target\release\$Proc.exe" `
     -ArgumentList @($Data, "--dialog", $Dialog) `
     -WorkingDirectory $dir -PassThru
Start-Sleep -Seconds 9
$p.Refresh()
$h = $p.MainWindowHandle
if ($h -eq 0) { throw "no main window" }
[void][WG]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 900

$r = New-Object WG+RECT
[void][WG]::GetWindowRect($h, [ref]$r)
$w = $r.R - $r.L; $ht = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $ht
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $ht))
$g.Dispose()
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "SAVED $Out ($w x $ht) title=$($p.MainWindowTitle)"
