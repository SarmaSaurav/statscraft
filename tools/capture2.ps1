param(
  [Parameter(Mandatory=$true)][string]$Out,
  [string]$Match = '',
  [switch]$Move,
  [int]$MoveX = 0, [int]$MoveY = 0, [int]$MoveW = 0, [int]$MoveH = 0
)
# Capture a StateCraft window by title substring. Optionally move/resize it first.
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class VC {
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int t, bool r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  public static List<IntPtr> ForPid(uint target) {
    var list = new List<IntPtr>();
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == target && IsWindowVisible(h)) list.Add(h);
      return true;
    }, IntPtr.Zero);
    return list;
  }
  public static string Title(IntPtr h) {
    var sb = new StringBuilder(256); GetWindowText(h, sb, 256); return sb.ToString();
  }
  public static RECT Rect(IntPtr h) { RECT r; GetWindowRect(h, out r); return r; }
}
'@
$p = Get-Process statecraft -EA SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Output 'NOT_RUNNING'; exit 1 }

$target = $null
foreach ($h in [VC]::ForPid([uint32]$p.Id)) {
  $t = [VC]::Title($h)
  if ($t -and $t -notlike '*statecraft.exe' -and ($Match -eq '' -or $t -like "*$Match*")) {
    $target = $h; $title = $t; break
  }
}
if (-not $target) { Write-Output "NO_MATCH '$Match'"; exit 1 }

if ($Move) {
  [void][VC]::MoveWindow($target, $MoveX, $MoveY, $MoveW, $MoveH, $true)
  Start-Sleep -Milliseconds 700
}
[void][VC]::SetForegroundWindow($target)
Start-Sleep -Milliseconds 400

$r = [VC]::Rect($target)
$bmp = New-Object System.Drawing.Bitmap ($r.R-$r.L), ($r.B-$r.T)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output ("SAVED {0} {1}x{2} | {3}" -f $Out, ($r.R-$r.L), ($r.B-$r.T), $title)
