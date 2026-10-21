param(
  [Parameter(Mandatory=$true)][string]$Out,
  [int]$Delay = 2
)
# Capture a specific window of the running StatsCraft app to a PNG.
# Usage: .\capture.ps1 -Out C:\path\file.png
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class Cap {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
'@
$p = Get-Process statscraft -EA SilentlyContinue |
     Where-Object { $_.MainWindowTitle -ne '' } |
     Sort-Object { $_.MainWindowTitle -notlike '*Output*' } |
     Select-Object -First 1
if (-not $p) { Write-Output 'NO_WINDOW'; exit 1 }
$r = New-Object Cap+RECT
[void][Cap]::GetWindowRect($p.MainWindowHandle, [ref]$r)
$w = $r.R - $r.L
$h = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output ("SAVED {0} {1}x{2} | {3}" -f $Out, $w, $h, $p.MainWindowTitle)
