# Open a data file in the running SPSS Statistics 27 and capture the window
# that results. Uses the app's own File > Open path.
param(
  [Parameter(Mandatory=$true)][string]$Data,
  [string]$Out = ''
)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class SP {
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
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
$p = Get-Process stats -EA SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Output 'SPSS_NOT_RUNNING'; exit 1 }
$pid0 = [uint32]$p.Id
$main = [SP]::ForPid($pid0) | Where-Object { $_.T -like '*Data Editor*' } | Select-Object -First 1
if (-not $main) { Write-Output 'NO_MAIN'; exit 1 }
[void][SP]::SetForegroundWindow($main.H)
Start-Sleep -Milliseconds 900

# The app's own Open Data dialog is reached with Ctrl+O.
[System.Windows.Forms.SendKeys]::SendWait('^o')
Start-Sleep -Seconds 3

# A file-name field should now have focus; type the path and press Return.
[System.Windows.Forms.SendKeys]::SendWait($Data.Replace('(', '{(}').Replace(')', '{)}'))
Start-Sleep -Milliseconds 700
[System.Windows.Forms.SendKeys]::SendWait('{ENTER}')
Start-Sleep -Seconds 5

foreach ($w in [SP]::ForPid($pid0)) {
  Write-Output ("WINDOW: {0} | {1}x{2}" -f $w.T, $w.Wd, $w.Ht)
}

if ($Out -ne '') {
  $target = [SP]::ForPid($pid0) | Where-Object { $_.T -like '*Data Editor*' } | Select-Object -First 1
  if ($target) {
    [void][SP]::SetForegroundWindow($target.H); Start-Sleep -Milliseconds 500
    $bmp = New-Object System.Drawing.Bitmap $target.Wd, $target.Ht
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($target.L, $target.TP, 0, 0, $bmp.Size)
    $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    Write-Output ("SAVED {0}" -f $Out)
  }
}
