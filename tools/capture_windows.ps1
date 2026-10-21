param(
  [string]$Proc = "statscraft",
  [string]$Out = "C:\Users\Saurav\AppData\Local\Temp\opencode"
)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class WinEnum {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  public static List<IntPtr> WindowsFor(uint want) {
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

$p = Get-Process -Name $Proc -EA SilentlyContinue | Select-Object -First 1
if (-not $p) { throw "no process" }
$hs = [WinEnum]::WindowsFor([uint32]$p.Id)
Write-Output "windows for pid $($p.Id): $($hs.Count)"

$i = 0
foreach ($h in $hs) {
  $sb = New-Object System.Text.StringBuilder 256
  [void][WinEnum]::GetWindowText($h, $sb, 256)
  $r = New-Object WinEnum+RECT
  [void][WinEnum]::GetWindowRect($h, [ref]$r)
  $title = $sb.ToString()
  Write-Output "  [$i] '$title'  $($r.L),$($r.T) - $($r.R),$($r.B)"

  if ($title -like "Output1*") {
    # Bring it to the front properly. SetForegroundWindow alone is
    # refused when another process owns the foreground, so the window is
    # also made topmost for the duration of the capture.
    [void][WinEnum]::ShowWindow($h, 9)
    [void][WinEnum]::SetForegroundWindow($h)
    Start-Sleep -Milliseconds 300
    [void][WinEnum]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040)
    Start-Sleep -Milliseconds 900
    $r2 = New-Object WinEnum+RECT
    [void][WinEnum]::GetWindowRect($h, [ref]$r2)
    $w = $r2.R - $r2.L; $ht = $r2.B - $r2.T
    $bmp = New-Object System.Drawing.Bitmap $w, $ht
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r2.L, $r2.T, 0, 0, (New-Object System.Drawing.Size $w, $ht))
    $g.Dispose()
    $f = Join-Path $Out "p3_output.png"
    $bmp.Save($f, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
    # Stop being topmost again.
    [void][WinEnum]::SetWindowPos($h, [IntPtr](-2), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040)
    Write-Output "    SAVED $f ($w x $ht)"
  }
  $i++
}
Write-Output "DONE"
