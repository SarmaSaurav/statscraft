param(
  [string]$Proc = "statscraft",
  [string]$Out = "C:\Users\Saurav\AppData\Local\Temp\opencode"
)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Cap {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
"@

function Click([int]$x,[int]$y) {
  Add-Type -Namespace W -Name M -MemberDefinition '
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  ' -ErrorAction SilentlyContinue
  [void][W.M]::SetCursorPos($x,$y); Start-Sleep -Milliseconds 200
  [W.M]::mouse_event(0x0002,0,0,0,[IntPtr]::Zero)
  [W.M]::mouse_event(0x0004,0,0,0,[IntPtr]::Zero)
  Start-Sleep -Milliseconds 400
}
function Grab([IntPtr]$h,[string]$path) {
  [void][Cap]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040)
  Start-Sleep -Milliseconds 700
  $r = New-Object Cap+RECT
  [void][Cap]::GetWindowRect($h, [ref]$r)
  $w = $r.R - $r.L; $ht = $r.B - $r.T
  $bmp = New-Object System.Drawing.Bitmap $w, $ht
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $ht))
  $g.Dispose(); $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
  [void][Cap]::SetWindowPos($h, [IntPtr](-2), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040)
  Write-Output "SAVED $path"
}

$p = Get-Process -Name $Proc -EA SilentlyContinue | Select-Object -First 1
if (-not $p) { throw "no process" }
$hs = @()
$cb = [Cap+EnumProc]{
  param($h, $l)
  $out = $false
  return $true
}
# Find the Output window by title.
$outH = [IntPtr]::Zero
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class Find {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public static IntPtr ByTitle(uint want, string needle) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == want && IsWindowVisible(h)) {
        var sb = new StringBuilder(256);
        GetWindowText(h, sb, 256);
        if (sb.ToString().Contains(needle)) { found = h; return false; }
      }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
"@
$outH = [Find]::ByTitle([uint32]$p.Id, "Output1")
if ($outH -eq [IntPtr]::Zero) { throw "no output window" }
[void][Cap]::ShowWindow($outH, 9)
[void][Cap]::SetForegroundWindow($outH)
Start-Sleep -Milliseconds 800

$r = New-Object Cap+RECT
[void][Cap]::GetWindowRect($outH, [ref]$r)
Write-Output "OUTPUT WINDOW $($r.L),$($r.T) - $($r.R),$($r.B)"

# The outline entries, at the measured y positions.
$targets = @{
  "p3_reg"      = 345
  "p3_mw"       = 454
  "p3_explore"  = 397
  "p3_means"    = 371
}
foreach ($name in $targets.Keys) {
  Click ($r.L + 120) ($r.T + $targets[$name])
  Start-Sleep -Milliseconds 500
  Grab $outH (Join-Path $Out "$name.png")
}
Write-Output "DONE"
