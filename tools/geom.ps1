param(
  [int]$SubRow = 3,
  [int]$Leaf = 1,
  [string]$Tag = 'probe'
)
# Open Analyze > <SubRow> and dump a screenshot at each stage so the exact
# submenu geometry can be read off.
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class G {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  public const uint LD=0x02, LU=0x04;
  public static void Click(int x, int y, int wait) {
    SetCursorPos(x, y); System.Threading.Thread.Sleep(220);
    mouse_event(LD,0,0,0,IntPtr.Zero); System.Threading.Thread.Sleep(80);
    mouse_event(LU,0,0,0,IntPtr.Zero); System.Threading.Thread.Sleep(wait);
  }
}
'@
function Shot([string]$t) {
  $vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bmp = New-Object System.Drawing.Bitmap $vs.Width, $vs.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($vs.X, $vs.Y, 0, 0, $bmp.Size)
  $bmp.Save("C:\Users\Saurav\AppData\Local\Temp\ref_shots\g_$Tag`_$t.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
}
$p = Get-Process stats -EA SilentlyContinue | Select-Object -First 1
[void][G]::SetForegroundWindow($p.MainWindowHandle); Start-Sleep -Milliseconds 1000

[G]::Click(312, 45, 1100)
Shot '1_open'
# hover a few candidate rows to find where the submenu appears
foreach ($y in 60, 86, 112, 138, 164) {
  [void][G]::SetCursorPos(340, $y)
  Start-Sleep -Milliseconds 1000
  Shot "2_hover$y"
}
