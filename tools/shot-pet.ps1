# Captures the native pet window to a PNG, so the tiny-skia paint can be looked at.
#   powershell -ExecutionPolicy Bypass -File tools/shot-pet.ps1 -Out shot.png
param([string]$Exe = 'src-tauri\target\release\dot.exe', [string]$Out = 'shot.png', [int]$SettleMs = 900)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Shot {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll", CharSet=CharSet.Unicode, EntryPoint="FindWindowW")] public static extern IntPtr FindWindow(string cls, string name);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
}
"@

Get-Process -Name dot -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 600
$proc = Start-Process -FilePath (Resolve-Path $Exe).Path -PassThru
try {
  $hwnd = [IntPtr]::Zero
  for ($i = 0; $i -lt 25 -and $hwnd -eq [IntPtr]::Zero; $i++) {
    Start-Sleep -Milliseconds 200
    $hwnd = [Shot]::FindWindow('DotNativePet', $null)
  }
  if ($hwnd -eq [IntPtr]::Zero) { throw 'pet window not found' }
  Start-Sleep -Milliseconds $SettleMs
  $r = New-Object Shot+RECT
  [void][Shot]::GetWindowRect($hwnd, [ref]$r)
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
  Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}
