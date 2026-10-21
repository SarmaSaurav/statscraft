param(
  [Parameter(Mandatory=$true)][string]$Out,
  [int]$MenuX = 285,      # 'Analyze' on the menu bar
  [int]$MenuY = 38,      # the window is at -8,-8 so the menu text row is y=34..42
  [int]$SubRow = 3,       # which item in the Analyze menu to hover (1-based)
  [int]$Leaf = 1          # which item in the submenu to click (1-based)
)
# Open a menu in IBM SPSS Statistics and capture the resulting dialog.
# The whole interaction runs in ONE PowerShell process so no menu can be
# dismissed by a window switch between steps.
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class S {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  public const uint LD=0x02, LU=0x04;
  public static void Click(int x, int y, int wait) {
    SetCursorPos(x, y);
    System.Threading.Thread.Sleep(220);
    mouse_event(LD,0,0,0,IntPtr.Zero);
    System.Threading.Thread.Sleep(80);
    mouse_event(LU,0,0,0,IntPtr.Zero);
    System.Threading.Thread.Sleep(wait);
  }
}
'@
$p = Get-Process stats -EA SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Output 'SPSS_NOT_RUNNING'; exit 1 }
[void][S]::SetForegroundWindow($p.MainWindowHandle)
Start-Sleep -Milliseconds 1000

# 1. open the Analyze menu
[S]::Click($MenuX, $MenuY, 1000)

# 2. hover the target submenu. Analyze items start at screen y=63 and are
#    ~26px apart (Power Analysis 63, Reports 90, Descriptive Statistics 118,
#    Bayesian 142, Tables 166, Compare Means 190 ...).
$subY = 63 + ($SubRow - 1) * 26
[void][S]::SetCursorPos(340, $subY)
Start-Sleep -Milliseconds 1400

# 3. the submenu opens to the RIGHT of x=520; click the leaf item.
[S]::Click(560, 92 + ($Leaf - 1) * 28, 2500)

# capture whatever window is now on top
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class W2 {
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
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
}
'@
$pid0 = [uint32]$p.Id
$cands = [W2]::ForPid($pid0) | Where-Object { $_.Wd -gt 200 -and $_.Ht -gt 150 }
if ($cands.Count -eq 0) { Write-Output 'NO_DIALOG'; exit 1 }
# the dialog is the foreground window that is NOT the main data editor
$pick = $cands | Where-Object { $_.T -notlike '*Data Editor*' } | Select-Object -First 1
if (-not $pick) { $pick = $cands | Select-Object -First 1 }
[void][W2]::SetForegroundWindow($pick.H)
Start-Sleep -Milliseconds 500
$bmp = New-Object System.Drawing.Bitmap $pick.Wd, $pick.Ht
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($pick.L, $pick.TP, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output ("SAVED {0} {1}x{2} | {3}" -f $Out, $pick.Wd, $pick.Ht, $pick.T)
