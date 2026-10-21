# Drive the StatsCraft app through its showcase surfaces and capture each.
# Windows are re-resolved by title before every action: egui recreates the
# native window handle when a viewport opens, so a cached handle goes stale.
param(
  [string]$OutDir = 'C:/Users/Saurav/AppData/Local/Temp/opencode',
  [switch]$SkipSyntax
)

Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class D {
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int t, bool r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  public const uint LD=0x02, LU=0x04;
  public static List<IntPtr> ForPid(uint target) {
    var list = new List<IntPtr>();
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == target && IsWindowVisible(h)) list.Add(h);
      return true;
    }, IntPtr.Zero);
    return list;
  }
  public class WinInfo {
    public IntPtr Handle; public int L, T, R, B;
    public int W { get { return R - L; } }
    public int H { get { return B - T; } }
  }
  public static WinInfo Info(IntPtr h) {
    RECT r; GetWindowRect(h, out r);
    return new WinInfo { Handle = h, L = r.L, T = r.T, R = r.R, B = r.B };
  }
  public static string Title(IntPtr h) {
    var sb = new StringBuilder(256); GetWindowText(h, sb, 256); return sb.ToString();
  }
  public static void Click(int x, int y, int wait = 600) {
    SetCursorPos(x, y);
    System.Threading.Thread.Sleep(140);
    mouse_event(LD, 0, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(70);
    mouse_event(LU, 0, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(wait);
  }
}
'@

function Get-Win([string]$match) {
  $p = Get-Process statscraft -EA SilentlyContinue | Select-Object -First 1
  if (-not $p) { return [IntPtr]::Zero }
  foreach ($h in [D]::ForPid([uint32]$p.Id)) {
    $t = [D]::Title($h)
    if ($t -and $t -like "*$match*" -and $t -notlike '*statscraft.exe') {
      return [IntPtr]$h
    }
  }
  return [IntPtr]::Zero
}

function Get-Rect([string]$match) {
  $h = Get-Win $match
  if ($h -eq [IntPtr]::Zero) { return $null }
  return [D]::GetWindowRect($h)
}

function Get-Info([string]$match) {
  $h = Get-Win $match
  if ($h -eq [IntPtr]::Zero) { return $null }
  return [D]::Info($h)
}

function Save-Win([string]$match, [string]$path) {
  $i = Get-Info $match
  if (-not $i) { Write-Output "NO_WINDOW '$match'"; return }
  if ($i.W -le 0 -or $i.H -le 0) { Write-Output "BAD_RECT '$match'"; return }
  $bmp = New-Object System.Drawing.Bitmap $i.W, $i.H
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($i.L, $i.T, 0, 0, $bmp.Size)
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  Write-Output ("SAVED {0} ({1}x{2})" -f (Split-Path $path -Leaf), $i.W, $i.H)
}

# ---------------------------------------------------------------- 1. Data Editor
$main = Get-Win 'DataSet'
if (-not $main) { Write-Output 'NO_MAIN'; exit 1 }
[void][D]::MoveWindow($main, 60, 60, 1400, 900, $true)
Start-Sleep -Milliseconds 900
[void][D]::SetForegroundWindow($main); Start-Sleep -Milliseconds 600
Save-Win 'DataSet' "$OutDir/shot1_data.png"

# ---------------------------------------------------------------- 2. Variable View
$i = Get-Info 'DataSet'
[D]::Click(($i.L + 195), ($i.B - 24), 1400)
Save-Win 'DataSet' "$OutDir/shot2_vars.png"

# back to Data View
$i = Get-Info 'DataSet'
[D]::Click(($i.L + 72), ($i.B - 24), 900)

if (-not $SkipSyntax) {
  # ---------------------------------------------------------------- 4. Syntax Editor
  $i = Get-Info 'DataSet'
  # last toolbar button (the <> syntax glyph)
  [D]::Click(($i.L + 678), ($i.T + 84), 2000)
  $sx = Get-Win 'Syntax1'
  if ($sx -ne [IntPtr]::Zero) {
    [void][D]::MoveWindow($sx, 640, 150, 740, 520, $true)
    Start-Sleep -Milliseconds 900

    # Type a real analysis script into the editor.
    [void][D]::SetForegroundWindow($sx); Start-Sleep -Milliseconds 600
    [D]::Click((640 + 200), (150 + 160), 500)
    [System.Windows.Forms.SendKeys]::SendWait('^a')
    Start-Sleep -Milliseconds 250
    [System.Windows.Forms.SendKeys]::SendWait(
      'FREQUENCIES region group satisfaction{ENTER}DESCRIPTIVES score age response_time{ENTER}T-TEST score BY group{ENTER}CROSSTABS region BY group{ENTER}CORRELATIONS score age response_time')
    Start-Sleep -Milliseconds 900
    Save-Win 'Syntax1' "$OutDir/shot4_syntax.png"

    # Run it -> the Output Viewer opens with real tables.
    [D]::Click((640 + 75), (150 + 362), 3500)
    $ov = Get-Win 'Output1'
    if ($ov -ne [IntPtr]::Zero) {
      [void][D]::MoveWindow($ov, 120, 90, 1160, 860, $true)
      Start-Sleep -Milliseconds 1100
      Save-Win 'Output1' "$OutDir/shot3_output.png"
    } else {
      Write-Output 'NO_OUTPUT_WINDOW'
    }
  } else {
    Write-Output 'NO_SYNTAX_WINDOW'
  }
}
Write-Output 'DONE'
