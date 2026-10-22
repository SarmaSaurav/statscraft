# Capture a set of StatsBench 27 reference surfaces.
# Each entry: menu path (menu X, submenu row Y, leaf Z) and a label.
param(
  [string]$OutDir = 'C:/Users/Saurav/AppData/Local/Temp/ref_shots',
  [string]$Only = ''
)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class R2 {
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
  public class W { public IntPtr H; public string T2; public int L,TP,R,B; public int Wd { get { return R-L; } } public int Ht { get { return B-TP; } } }
  public static W[] ForPid(uint pid) {
    var list = new List<W>();
    EnumWindows((h,l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h)) {
        var sb = new StringBuilder(512); GetWindowText(h, sb, 512);
        if (sb.Length > 0) { RECT r; GetWindowRect(h, out r); list.Add(new W { H=h, T2=sb.ToString(), L=r.L, TP=r.T, R=r.R, B=r.B }); }
      }
      return true;
    }, IntPtr.Zero);
    return list.ToArray();
  }
  public static void Click(int x, int y, int wait) {
    SetCursorPos(x, y); System.Threading.Thread.Sleep(220);
    mouse_event(LD,0,0,0,IntPtr.Zero); System.Threading.Thread.Sleep(90);
    mouse_event(LU,0,0,0,IntPtr.Zero); System.Threading.Thread.Sleep(wait);
  }
}
'@
# Measured geometry (StatsBench 27 window sits at -8,-8):
#   Analyze menu      -> (285, 38)
#   menu item rows   -> y = 63, 89, 115, 141, 167, 193 (26px apart)
#   Descriptive Statistics submenu leaf 'Frequencies' -> (572, 118)
#   submenu leaves for other parents start at x=572 too.
$SHOTS = @(
  @{ Name='freq';      Menu=285; Sub=115; Leaf=118 },
  @{ Name='desc';      Menu=285; Sub=115; Leaf=146 },
  @{ Name='crosstabs'; Menu=285; Sub=115; Leaf=202 }
)
$p = Get-Process stats -EA SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Output 'NOT_RUNNING'; exit 1 }
$pid0 = [uint32]$p.Id
$main = [R2]::ForPid($pid0) | Where-Object { $_.T2 -like '*Data Editor*' } | Select-Object -First 1
if (-not $main) { Write-Output 'NO_MAIN'; exit 1 }

foreach ($s in $SHOTS) {
  if ($Only -ne '' -and $s.Name -ne $Only) { continue }
  # close anything already open
  foreach ($w in [R2]::ForPid($pid0)) {
    if ($w.T2 -notlike '*Data Editor*' -and $w.Wd -gt 150 -and $w.Wd -lt 900) {
      [void][R2]::SetForegroundWindow($w.H); Start-Sleep -Milliseconds 300
      [System.Windows.Forms.SendKeys]::SendWait('%{F4}'); Start-Sleep -Milliseconds 800
    }
  }
  [void][R2]::SetForegroundWindow($main.H); Start-Sleep -Milliseconds 900
  [R2]::Click($s.Menu, 38, 1200)
  [void][R2]::SetCursorPos(300, $s.Sub); Start-Sleep -Milliseconds 1300
  [R2]::Click(572, $s.Leaf, 2600)

  $dlg = [R2]::ForPid($pid0) | Where-Object { $_.T2 -notlike '*Data Editor*' -and $_.T2 -notlike '*Viewer*' -and $_.Wd -gt 150 -and $_.Wd -lt 900 } | Select-Object -First 1
  if ($dlg) {
    [void][R2]::SetForegroundWindow($dlg.H); Start-Sleep -Milliseconds 700
    $bmp = New-Object System.Drawing.Bitmap $dlg.Wd, $dlg.Ht
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($dlg.L, $dlg.TP, 0, 0, $bmp.Size)
    $bmp.Save("C:\Users\Saurav\AppData\Local\Temp\ref_shots\dlg_$($s.Name).png", [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    Write-Output ("SAVED dlg_$($s.Name).png | $($dlg.T2) $($dlg.Wd)x$($dlg.Ht)")
  } else {
    Write-Output "NO_DIALOG for $($s.Name)"
  }
}
