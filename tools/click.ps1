param(
  [Parameter(Mandatory=$true)][int]$X,
  [Parameter(Mandatory=$true)][int]$Y,
  [int]$Wait = 800
)
# Click at absolute screen coordinates.
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class Click {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  public const uint LD=0x02, LU=0x04;
}
'@
[void][Click]::SetCursorPos($X, $Y)
Start-Sleep -Milliseconds 120
[Click]::mouse_event([Click]::LD,0,0,0,[IntPtr]::Zero)
Start-Sleep -Milliseconds 60
[Click]::mouse_event([Click]::LU,0,0,0,[IntPtr]::Zero)
Start-Sleep -Milliseconds $Wait
Write-Output "CLICKED $X,$Y"
