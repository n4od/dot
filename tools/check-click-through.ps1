# Verifies the pet's click-through, end to end.
#
# The window must NOT be WS_EX_TRANSPARENT over the body (so it takes clicks and
# drags), and MUST be over a transparent corner (so clicks reach the desktop).
# This launches the built app, moves the real cursor to each point, and reads the
# window's extended style back — the check the roadmap owed.
#
#   powershell -ExecutionPolicy Bypass -File tools\check-click-through.ps1
#   powershell -ExecutionPolicy Bypass -File tools\check-click-through.ps1 -Exe src-tauri\target\debug\dot.exe
#
param([string]$Exe = '')

$ErrorActionPreference = 'Stop'

if (-not $Exe) {
  foreach ($candidate in @('src-tauri\target\release\dot.exe', 'src-tauri\target\debug\dot.exe')) {
    if (Test-Path $candidate) { $Exe = $candidate; break }
  }
}
if (-not $Exe -or -not (Test-Path $Exe)) {
  throw "Build the app first (cargo build --release --features custom-protocol), or pass -Exe <path>."
}
$Exe = (Resolve-Path $Exe).Path

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Ct {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll", CharSet=CharSet.Unicode, EntryPoint="FindWindowW")] public static extern IntPtr FindWindow(string cls, string name);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll", EntryPoint="GetWindowLongPtrW")] public static extern IntPtr GetWindowLongPtr(IntPtr h, int index);
}
"@

$GWL_EXSTYLE = -20
$WS_EX_TRANSPARENT = 0x20

Get-Process -Name dot -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 800

$saved = New-Object Ct+POINT
[void][Ct]::GetCursorPos([ref]$saved)

$proc = Start-Process -FilePath $Exe -PassThru
$ok = $false
try {
  $hwnd = [IntPtr]::Zero
  for ($i = 0; $i -lt 20 -and $hwnd -eq [IntPtr]::Zero; $i++) {
    Start-Sleep -Milliseconds 300
    $hwnd = [Ct]::FindWindow('DotNativePet', $null)
  }
  if ($hwnd -eq [IntPtr]::Zero) { throw 'pet window not found (is it hidden?)' }

  $r = New-Object Ct+RECT
  [void][Ct]::GetWindowRect($hwnd, [ref]$r)
  $cx = [int](($r.Left + $r.Right) / 2)
  $cy = [int](($r.Top + $r.Bottom) / 2)

  # Click-through is ON when the extended style carries WS_EX_TRANSPARENT.
  function Test-Through([int]$x, [int]$y) {
    [void][Ct]::SetCursorPos($x, $y)
    Start-Sleep -Milliseconds 500
    $ex = [Ct]::GetWindowLongPtr($hwnd, $GWL_EXSTYLE).ToInt64()
    return (($ex -band $WS_EX_TRANSPARENT) -ne 0)
  }

  $body = Test-Through $cx $cy
  $corner = Test-Through ($r.Left + 4) ($r.Top + 4)

  Write-Host ("body centre  {0},{1}  click-through = {2}" -f $cx, $cy, $body)
  Write-Host ("corner       {0},{1}  click-through = {2}" -f ($r.Left + 4), ($r.Top + 4), $corner)
  $ok = ((-not $body) -and $corner)
}
finally {
  [void][Ct]::SetCursorPos($saved.X, $saved.Y)
  Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}

if ($ok) {
  Write-Host 'PASS: the body takes clicks and the corner passes them through.' -ForegroundColor Green
  exit 0
}
Write-Host 'FAIL: expected body=interactive, corner=click-through.' -ForegroundColor Red
exit 1
