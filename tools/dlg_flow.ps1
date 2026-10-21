param(
  [string]$Dialog = "descriptives",
  [string]$Out = "C:\Users\Saurav\AppData\Local\Temp\opencode"
)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WI {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
"@

function Click([int]$x,[int]$y) {
  [void][WI]::SetCursorPos($x,$y); Start-Sleep -Milliseconds 250
  [WI]::mouse_event(0x0002,0,0,0,[IntPtr]::Zero)
  [WI]::mouse_event(0x0004,0,0,0,[IntPtr]::Zero)
  Start-Sleep -Milliseconds 450
}
Add-Type @"
using System;
using System.Drawing;
public class WP {
  [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr h);
  [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr h, IntPtr dc);
  [DllImport("gdi32.dll")] public static extern uint GetPixel(IntPtr dc, int x, int y);
  public static System.Drawing.Color Pixel(int x, int y) {
    IntPtr dc = GetDC(IntPtr.Zero);
    uint c = GetPixel(dc, x, y);
    ReleaseDC(IntPtr.Zero, dc);
    return System.Drawing.Color.FromArgb((int)(c & 0xFF), (int)((c >> 8) & 0xFF), (int)((c >> 16) & 0xFF));
  }
}
"@
function Get-Pixel([int]$x,[int]$y) { return [WP]::Pixel($x, $y) }

function Shot([string]$name) {
  $p = Get-Process statscraft -EA SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
  $r = New-Object WI+RECT
  [void][WI]::GetWindowRect($p.MainWindowHandle, [ref]$r)
  $w = $r.R - $r.L; $ht = $r.B - $r.T
  $bmp = New-Object System.Drawing.Bitmap $w, $ht
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $ht))
  $g.Dispose()
  $f = Join-Path $Out $name
  $bmp.Save($f, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
  Write-Output "SAVED $f"
}

# Launch straight into the dialog.
$dir = (Get-Location).Path
Get-Process statscraft -EA SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1
$p = Start-Process -FilePath ".\target\release\statscraft.exe" `
     -ArgumentList @("samples\survey_demo.csv", "--dialog", $Dialog) `
     -WorkingDirectory $dir -PassThru
Start-Sleep -Seconds 9
$p.Refresh()
$h = $p.MainWindowHandle
[void][WI]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 900
$r = New-Object WI+RECT
[void][WI]::GetWindowRect($h, [ref]$r)
Write-Output "WINDOW $($r.L),$($r.T) - $($r.R),$($r.B)"

# The Frequencies dialog: source list on the left, its first row at
# window-relative y=142; the move arrow at x=493, y=134; OK at x=71,y=510.
# These are measured from the dialog capture.
$src1X = $r.L + 52
$src1Y = $r.T + 142
$arrowX = $r.L + 493
$arrowY = $r.T + 134

# 1. Click "score" (row 6 of the source list) and move it.
Click ($r.L + 52) ($r.T + 271)
Shot "g1_selected.png"
Click $arrowX $arrowY
Shot "g2_moved.png"

# 2. Click "age" (row 4) and move it too.
Click ($r.L + 52) ($r.T + 219)
Click $arrowX $arrowY
Shot "g3_two.png"

# 3. Open the Statistics/Options sub-dialog.
Click ($r.L + 697) ($r.T + 383)
Start-Sleep -Milliseconds 600
Shot "g4_statistics.png"

# 4. Tick "Mean" and "Std. Deviation" in the sub-dialog.
Click ($r.L + 672) ($r.T + 460)
Click ($r.L + 672) ($r.T + 541)
Start-Sleep -Milliseconds 400
Shot "g5_ticked.png"

# 5. Press OK. The button row moves as the sub-dialog opens and closes, so
#    find the accent-coloured OK button by scanning for it.
$okX = $r.L + 71
$okY = $r.T + 660
foreach ($dy in 0, 18, 36, 54, 72, -18, -36) {
  $c = Get-Pixel ($okX + 25) ($okY + $dy + 14)
  if ($c.B -gt 140 -and $c.B -gt ($c.R + 40) -and $c.R -lt 160) {
    $okY = $okY + $dy
    Write-Output "OK button found at y=$okY"
    break
  }
}
Click $okX $okY
Start-Sleep -Seconds 3
Shot "g6_after_ok.png"

# 6. The Output Viewer opens in its own window - find and capture it.
$ov = Get-Process statscraft -EA SilentlyContinue |
      Where-Object { $_.MainWindowTitle -like "Output1*" } | Select-Object -First 1
if ($ov) {
  [void][WI]::SetForegroundWindow($ov.MainWindowHandle)
  Start-Sleep -Milliseconds 800
  $r2 = New-Object WI+RECT
  [void][WI]::GetWindowRect($ov.MainWindowHandle, [ref]$r2)
  $w = $r2.R - $r2.L; $ht = $r2.B - $r2.T
  $bmp = New-Object System.Drawing.Bitmap $w, $ht
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r2.L, $r2.T, 0, 0, (New-Object System.Drawing.Size $w, $ht))
  $g.Dispose()
  $f = Join-Path $Out "g7_output.png"
  $bmp.Save($f, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
  Write-Output "SAVED $f"
} else {
  Write-Output "NO OUTPUT WINDOW FOUND"
}

Write-Output "DONE"
