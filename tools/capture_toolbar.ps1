# Capture the main Data Editor window and crop the toolbar, so the
# icons can be looked at closely.
param(
  [string]$Data = "samples\survey_demo.csv",
  [string]$Out = "C:\Users\Saurav\AppData\Local\Temp\opencode\ui_icons.png",
  [string]$Extra = "",
  [int]$CropHeight = 150
)

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class WI {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int t, bool r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  public static List<IntPtr> ForPid(uint want) {
    var list = new List<IntPtr>();
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == want && IsWindowVisible(h)) list.Add(h);
      return true;
    }, IntPtr.Zero);
    return list;
  }
}
"@

$dir = (Get-Location).Path
Get-Process -Name statscraft -EA SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1
$args = @($Data)
if ($Extra -ne "") { $args += $Extra.Split(" ") }
$p = Start-Process -FilePath ".\target\release\statscraft.exe" -ArgumentList $args -WorkingDirectory $dir -PassThru
Start-Sleep -Seconds 7

$handle = [IntPtr]::Zero
foreach ($h in [WI]::ForPid([uint32]$p.Id)) {
  $sb = New-Object System.Text.StringBuilder 256
  [void][WI]::GetWindowText($h, $sb, 256)
  if ($sb.ToString() -like "*DataSet*") { $handle = $h; break }
}
if ($handle -eq [IntPtr]::Zero) { Write-Output "no main window"; exit 1 }

[void][WI]::MoveWindow($handle, 60, 60, 1500, 900, $true)
Start-Sleep -Milliseconds 900
[void][WI]::SetForegroundWindow($handle)
Start-Sleep -Milliseconds 600

$r = New-Object WI+RECT
[void][WI]::GetWindowRect($handle, [ref]$r)
$w = $r.R - $r.L; $h = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
$g.Dispose()

# The toolbar is the strip under the menu bar.
$ch = [Math]::Min($CropHeight, $h - 30)
$crop = New-Object System.Drawing.Rectangle 0, 30, $w, $ch
$part = $bmp.Clone($crop, $bmp.PixelFormat)
$part.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$part.Dispose(); $bmp.Dispose()
Write-Output "SAVED $Out ($w x $ch)"
