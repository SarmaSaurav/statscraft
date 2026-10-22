param(
  [Parameter(Mandatory=$true)][string]$Out,
  [string]$Match = '',
  [switch]$All,
  [int]$Settle = 900
)
# Capture a native window by title substring, resolved fresh at capture time.
# Usage: .\snap.ps1 -Out shot.png -Match "StatsBench"
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class Snap {
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  public static List<IntPtr> All() {
    var list = new List<IntPtr>();
    EnumWindows((h, l) => { if (IsWindowVisible(h)) list.Add(h); return true; }, IntPtr.Zero);
    return list;
  }
  public static uint Pid(IntPtr h) { uint p; GetWindowThreadProcessId(h, out p); return p; }
  public static string Title(IntPtr h) {
    var sb = new StringBuilder(512); GetWindowText(h, sb, 512); return sb.ToString();
  }
  public class Win { public IntPtr H; public string T; public int L,TP,R,B; public int W { get { return R-L; } } public int H2 { get { return B-TP; } } }
  public static Win[] Windows(uint pid) {
    var list = new List<Win>();
    foreach (var h in All()) {
      if (Pid(h) != pid) continue;
      var t = Title(h);
      if (t.Length == 0) continue;
      RECT r; GetWindowRect(h, out r);
      list.Add(new Win { H=h, T=t, L=r.L, TP=r.T, R=r.R, B=r.B });
    }
    return list.ToArray();
  }
}
'@
Start-Sleep -Milliseconds $Settle

if ($All) {
  Write-Output "--- all visible windows ---"
  foreach ($w in [Snap]::Windows([uint32]0)) {
    if ($w.W -le 0) { continue }
    Write-Output ("[{0}] {1} | {2}x{3} at {4},{5}" -f $w.H, $w.T, $w.W, $w.H2, $w.L, $w.TP)
  }
  exit 0
}

$pid0 = [uint32]$args.Count
$cands = @()
foreach ($proc in Get-Process -EA SilentlyContinue) {
  foreach ($w in [Snap]::Windows([uint32]$proc.Id)) {
    if ($w.W -le 0 -or $w.H2 -le 0) { continue }
    if ($Match -eq '' -or $w.T -like "*$Match*") {
      $cands += [pscustomobject]@{ W=$w; Proc=$proc }
    }
  }
}
if ($cands.Count -eq 0) { Write-Output "NO_MATCH '$Match'"; exit 1 }

$pick = $cands | Sort-Object { $_.W.W * $_.W.H2 } -Descending | Select-Object -First 1
$w = $pick.W
[void][Snap]::SetForegroundWindow($w.H)
Start-Sleep -Milliseconds 400

$bmp = New-Object System.Drawing.Bitmap $w.W, $w.H2
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($w.L, $w.TP, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output ("SAVED {0} {1}x{2} | {3}" -f $Out, $w.W, $w.H2, $w.T)
