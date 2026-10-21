param([int]$SubRow = 3)
# One-shot: open Analyze, hover row $SubRow, screenshot, then screenshot again
# after moving into where the submenu should be.
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class H2 {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  public const uint LD=0x02, LU=0x04;
  public static void Click(int x, int y, int wait) {
    SetCursorPos(x, y); System.Threading.Thread.Sleep(250);
    mouse_event(LD,0,0,0,IntPtr.Zero); System.Threading.Thread.Sleep(90);
    mouse_event(LU,0,0,0,IntPtr.Zero); System.Threading.Thread.Sleep(wait);
  }
}
'@
function Shot([string]$t) {
  $vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bmp = New-Object System.Drawing.Bitmap $vs.Width, $vs.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($vs.X, $vs.Y, 0, 0, $bmp.Size)
  $bmp.Save("C:\Users\Saurav\AppData\Local\Temp\spss_ref\h_$t.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
}
$p = Get-Process stats -EA SilentlyContinue | Select-Object -First 1
[void][H2]::SetForegroundWindow($p.MainWindowHandle); Start-Sleep -Milliseconds 1200

[H2]::Click(285, 38, 1200)
Shot 'a_menu'
# walk down the menu one row at a time, capturing each, so the highlighted
# row and any opened submenu are visible
for ($i = 1; $i -le 8; $i++) {
  [void][H2]::SetCursorPos(300, 60 + ($i-1)*26)
  Start-Sleep -Milliseconds 1100
  Shot "b_row$i"
}
