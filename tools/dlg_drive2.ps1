param(
  [string]$Out = "C:\Users\Saurav\AppData\Local\Temp\opencode",
  [string]$Proc = "statscraft"
)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WD {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
"@

function Get-Main {
  $p = Get-Process -Name $Proc -EA SilentlyContinue |
       Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
  if (-not $p) { throw "no window for $Proc" }
  return $p
}
function Click([int]$x,[int]$y) {
  [void][WD]::SetCursorPos($x,$y); Start-Sleep -Milliseconds 250
  [WD]::mouse_event(0x0002,0,0,0,[IntPtr]::Zero)
  [WD]::mouse_event(0x0004,0,0,0,[IntPtr]::Zero)
  Start-Sleep -Milliseconds 500
}
function Hover([int]$x,[int]$y) {
  [void][WD]::SetCursorPos($x,$y); Start-Sleep -Milliseconds 450
}
function FullShot([string]$name) {
  $vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bmp = New-Object System.Drawing.Bitmap $vs.Width, $vs.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($vs.Left,$vs.Top,0,0,(New-Object System.Drawing.Size $vs.Width,$vs.Height))
  $g.Dispose()
  $f = 1.0/1.6
  $small = New-Object System.Drawing.Bitmap ([int]($vs.Width*$f)), ([int]($vs.Height*$f))
  $g2 = [System.Drawing.Graphics]::FromImage($small)
  $g2.InterpolationMode = 'HighQualityBicubic'
  $g2.DrawImage($bmp,0,0,$small.Width,$small.Height); $g2.Dispose()
  $path = Join-Path $Out $name
  $small.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose(); $small.Dispose()
  Write-Output "SAVED $path"
}

$p = Get-Main
$h = $p.MainWindowHandle
[void][WD]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 800

$r = New-Object WD+RECT
[void][WD]::GetWindowRect($h, [ref]$r)
Write-Output "WINDOW $($r.L),$($r.T) - $($r.R),$($r.B)"

# Geometry measured from the running app (window-relative):
#   "Analyze" label   x 234..303, y 40..49
#   dropdown first row (Descriptive Statistics) y = 77
#   dropdown panel right edge x = 648
$analyzeX = $r.L + 268
$analyzeY = $r.T + 45
$descY    = $r.T + 77
$subX     = $r.L + 700
$subY     = $r.T + 77

# 1. Open the Analyze menu.
Click $analyzeX $analyzeY
FullShot "f1_analyze.png"

# 2. Hover "Descriptive Statistics" to open its submenu.
Hover $analyzeX $descY
Start-Sleep -Milliseconds 800
FullShot "f2_submenu.png"

# 3. Click the first leaf of the submenu ("Frequencies...").
Click $subX $subY
Start-Sleep -Milliseconds 1500
FullShot "f3_dialog.png"
Write-Output "DONE"
