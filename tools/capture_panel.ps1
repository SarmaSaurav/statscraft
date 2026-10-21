# Capture one panel of the Output Viewer, by clicking an entry in the outline.
param(
    [Parameter(Mandatory = $true)][string]$Index,
    [string]$Out = "C:\Users\Saurav\AppData\Local\Temp\opencode\panel.png"
)

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class WinEnum2 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
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

$proc = Get-Process statscraft -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $proc) { Write-Output "no statscraft process"; exit 1 }
$hs = [WinEnum2]::WindowsFor([uint32]$proc.Id)
$viewer = $null
$rect = $null
foreach ($h in $hs) {
    $sb = New-Object System.Text.StringBuilder 256
    [void][WinEnum2]::GetWindowText($h, $sb, 256)
    $r = New-Object WinEnum2+RECT
    [void][WinEnum2]::GetWindowRect($h, [ref]$r)
    if ($sb.ToString() -like "Output1*") {
        $viewer = $h
        $rect = $r
        break
    }
}
if (-not $viewer) { Write-Output "no output window"; exit 1 }

[void][W]::SetForegroundWindow($viewer)
Start-Sleep -Milliseconds 300
[void][W]::SetWindowPos($viewer, [IntPtr](-1), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040)
Start-Sleep -Milliseconds 700

# The outline sits at a fixed offset inside the viewer window.
[void][W]::ShowWindow($viewer, 9)
Start-Sleep -Milliseconds 200

# The outline's rows sit at a fixed offset in the window: the first row
# is at y=131 and the spacing is 26 pixels.
$x = $rect.L + 60
$y = $rect.T + 131 + ([int]$Index - 1) * 26
[void][W]::SetCursorPos($x, $y)
Start-Sleep -Milliseconds 250
[W]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero)
[W]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 900

$w = 1096
$h = 759
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($rect.L, $rect.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
# Stop being topmost again.
[void][W]::SetWindowPos($viewer, [IntPtr](-2), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040)
Write-Output "SAVED $Out ($w x $h) index=$Index"
