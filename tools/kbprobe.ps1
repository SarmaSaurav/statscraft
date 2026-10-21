param([string]$Out = 'C:/Users/Saurav/AppData/Local/Temp/spss_ref/kb.png')
# Probe SPSS's menu behaviour with the keyboard, capturing the screen after
# each key so we can see exactly what happens.
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class B {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
'@
function Shot([string]$tag) {
  $vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bmp = New-Object System.Drawing.Bitmap $vs.Width, $vs.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($vs.X, $vs.Y, 0, 0, $bmp.Size)
  $bmp.Save("C:\Users\Saurav\AppData\Local\Temp\spss_ref\kb_$tag.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  Write-Output "SHOT $tag"
}
$p = Get-Process stats -EA SilentlyContinue | Select-Object -First 1
[void][B]::SetForegroundWindow($p.MainWindowHandle); Start-Sleep -Milliseconds 1000
$r = New-Object B+RECT
[void][B]::GetWindowRect($p.MainWindowHandle, [ref]$r)
Write-Output ("WINDOW {0},{1} {2}x{3}" -f $r.L, $r.T, ($r.R-$r.L), ($r.B-$r.T))

Shot 'a_start'
[System.Windows.Forms.SendKeys]::SendWait('{F10}')
Start-Sleep -Milliseconds 1000
Shot 'b_f10'
for ($i=0; $i -lt 5; $i++) { [System.Windows.Forms.SendKeys]::SendWait('{DOWN}'); Start-Sleep -Milliseconds 250 }
Shot 'c_down5'
[System.Windows.Forms.SendKeys]::SendWait('{RIGHT}')
Start-Sleep -Milliseconds 1200
Shot 'd_right'
