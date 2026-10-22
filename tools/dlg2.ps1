# Open a procedure dialog in StatsBench 27 and capture it.
# Strategy: click the menu, hover the submenu, click the leaf - all in one
# process so no window switch can dismiss the menu. Captures after each step
# so the geometry can be diagnosed if it fails.
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class D2 {
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  public const uint LD=0x02, LU=0x04;
  public class W { public IntPtr H; public string T; public int L,TP,R,B; public int Wd { get { return R-L; } } public int Ht { get { return B-TP; } } }
  public static W[] ForPid(uint pid) {
    var list = new List<W>();
    EnumWindows((h,l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h)) {
        var sb = new StringBuilder(512); GetWindowText(h, sb, 512);
        if (sb.Length > 0) { RECT r; GetWindowRect(h, out r); list.Add(new W { H=h, T=sb.ToString(), L=r.L, TP=r.T, R=r.R, B=r.B }); }
      }
      return true;
    }, IntPtr.Zero);
    return list.ToArray();
  }
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
  $bmp.Save("C:\Users\Saurav\AppData\Local\Temp\ref_shots\d_$t.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
}
$p = Get-Process stats -EA SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Output 'NOT_RUNNING'; exit 1 }
$pid0 = [uint32]$p.Id
$main = [D2]::ForPid($pid0) | Where-Object { $_.T -like '*Data Editor*' } | Select-Object -First 1
if (-not $main) { Write-Output 'NO_MAIN'; exit 1 }
[void][D2]::SetForegroundWindow($main.H)
Start-Sleep -Milliseconds 1200
Write-Output ("MAIN at {0},{1}" -f $main.L, $main.TP)

# The window sits at -8,-8, so the menu text row is screen y = 34..42.
[D2]::Click(285, 38, 1400)
Shot '1_menu'

# Descriptive Statistics is the 3rd item: y=115. Its submenu opens at x=850.
[void][D2]::SetCursorPos(300, 115)
Start-Sleep -Milliseconds 1400
Shot '2_submenu'
# Frequencies: measured from a real capture - the submenu's first leaf sits
# at screen (572, 118).
[D2]::Click(572, 118, 2600)
Shot '3_dialog'

foreach ($w in [D2]::ForPid($pid0)) {
  Write-Output ("WINDOW: {0} | {1}x{2}" -f $w.T, $w.Wd, $w.Ht)
}
# capture the dialog window on its own
$dlg = [D2]::ForPid($pid0) | Where-Object { $_.T -notlike '*Data Editor*' -and $_.T -ne '' -and $_.Wd -gt 150 } | Select-Object -First 1
if ($dlg) {
  [void][D2]::SetForegroundWindow($dlg.H); Start-Sleep -Milliseconds 600
  $bmp = New-Object System.Drawing.Bitmap $dlg.Wd, $dlg.Ht
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($dlg.L, $dlg.TP, 0, 0, $bmp.Size)
  $bmp.Save('C:\Users\Saurav\AppData\Local\Temp\ref_shots\ref_freq_dialog.png', [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  Write-Output ("SAVED dialog {0} {1}x{2}" -f $dlg.T, $dlg.Wd, $dlg.Ht)
} else { Write-Output 'NO_DIALOG' }
