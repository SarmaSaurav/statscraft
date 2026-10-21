param(
  [Parameter(Mandatory=$true)][int]$X,
  [Parameter(Mandatory=$true)][int]$Y,
  [int]$Wait = 700
)
# Click at absolute screen coordinates, then optionally move the mouse away.
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class M {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  public const uint LD=0x02, LU=0x04;
}
'@
[void][M]::SetCursorPos($X, $Y)
Start-Sleep -Milliseconds 150
[M]::mouse_event([M]::LD,0,0,0,[IntPtr]::Zero)
Start-Sleep -Milliseconds 70
[M]::mouse_event([M]::LU,0,0,0,[IntPtr]::Zero)
Start-Sleep -Milliseconds $Wait
Write-Output "CLICK $X,$Y"
