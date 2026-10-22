# Walk the StatsBench Text Import Wizard to the end, loading the CSV with a header row.
# The wizard is at a known screen position; coordinates are computed from the
# captured geometry so they survive small layout differences.
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class T {
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
    SetCursorPos(x, y); System.Threading.Thread.Sleep(200);
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
  $bmp.Save("C:\Users\Saurav\AppData\Local\Temp\ref_shots\w_$t.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
}
$p = Get-Process stats -EA SilentlyContinue | Select-Object -First 1
$pid0 = [uint32]$p.Id
$wiz = [T]::ForPid($pid0) | Where-Object { $_.T2 -like '*Import Wizard*' } | Select-Object -First 1
if (-not $wiz) { Write-Output 'NO_WIZARD'; exit 1 }
[void][T]::SetForegroundWindow($wiz.H); Start-Sleep -Milliseconds 600
Write-Output ("WIZARD at {0},{1} {2}x{3}" -f $wiz.L, $wiz.TP, $wiz.Wd, $wiz.Ht)

# Measured from a real capture of the wizard (550x500 at 685,266):
# the button row is at y=+467 and Next sits at x=+213.
$nextX = $wiz.L + 213
$nextY = $wiz.TP + 467

for ($step = 2; $step -le 6; $step++) {
  [T]::Click($nextX, $nextY, 1800)
  Shot "step$step"
  $cur = [T]::ForPid($pid0) | Where-Object { $_.T2 -like '*Import Wizard*' } | Select-Object -First 1
  if (-not $cur) { Write-Output "WIZARD_CLOSED at step $step"; break }
  $title = $cur.T2
  Write-Output ("STEP $step -> $title")
  if ($title -like '*Step 3*') {
    # Step 3 is "Header" - ensure "Yes" (variable names in first row) is picked.
    # Its radio sits at the top-left of the options panel.
    [T]::Click(($cur.L + [int]($cur.Wd * 0.30)), ($cur.TP + [int]($cur.Ht * 0.40)), 900)
    Shot 'step3_hdr'
  }
}
Start-Sleep -Seconds 2
foreach ($w in [T]::ForPid($pid0)) { Write-Output ("WINDOW: {0}" -f $w.T2) }
