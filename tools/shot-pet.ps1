# Captures the native pet window to a PNG, so the tiny-skia paint can be looked at.
#   powershell -ExecutionPolicy Bypass -File tools/shot-pet.ps1 -Out shot.png
#   powershell -ExecutionPolicy Bypass -File tools/shot-pet.ps1 -Out look.png -CenterCursor
#
# Runs a private copy of the app for the length of the capture and stops it again,
# so quit a running pet first if you care about its window position.
#
# -CenterCursor puts the pointer in the middle of the pet and puts it back
# afterwards: that is the pose pet.rs asks for when the cursor is on the pet
# (`yaw 0, pitch +10, mix 1`), which is not the pose the settings stage draws.
param(
  [string]$Exe = 'src-tauri\target\release\dot.exe',
  [string]$Out = 'shot.png',
  [int]$SettleMs = 900,
  [switch]$CenterCursor
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Shot {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll", CharSet=CharSet.Unicode, EntryPoint="FindWindowW")] public static extern IntPtr FindWindow(string cls, string name);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
}
"@

Get-Process -Name dot -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 600
$proc = Start-Process -FilePath (Resolve-Path $Exe).Path -PassThru
$restore = $null
try {
  $hwnd = [IntPtr]::Zero
  for ($i = 0; $i -lt 25 -and $hwnd -eq [IntPtr]::Zero; $i++) {
    Start-Sleep -Milliseconds 200
    $hwnd = [Shot]::FindWindow('DotNativePet', $null)
  }
  if ($hwnd -eq [IntPtr]::Zero) { throw 'pet window not found' }

  $r = New-Object Shot+RECT
  [void][Shot]::GetWindowRect($hwnd, [ref]$r)
  if ($CenterCursor) {
    $was = New-Object Shot+POINT
    if ([Shot]::GetCursorPos([ref]$was)) { $restore = $was }
    [void][Shot]::SetCursorPos([int](($r.Left + $r.Right) / 2), [int](($r.Top + $r.Bottom) / 2))
  }
  Start-Sleep -Milliseconds $SettleMs

  $w = $r.Right - $r.Left
  $h = $r.Bottom - $r.Top
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
  $g.Dispose()
  $bmp.Save((Join-Path (Get-Location) $Out), [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  "saved $Out  ${w}x${h}  at $($r.Left),$($r.Top)"
}
finally {
  if ($restore) { [void][Shot]::SetCursorPos($restore.X, $restore.Y) }
  Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}