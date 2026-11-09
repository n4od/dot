# Release v0.3.1 — Settings Can Be Closed Again

A small patch for a regression in 0.3.0: closing the Settings window also closed
the pet.

## 🐛 Fixed

- **Closing Settings no longer stops the pet.** Tauri exits when its last window
  closes, and the pet is a native Win32 window rather than a Tauri one, so the app
  now distinguishes an explicit **Quit** from the last webview closing. Closing
  Settings still destroys its webview and frees its processes — the pet just keeps
  running.

## 📦 Downloads

- **`Dot_0.3.1_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

Everything from 0.3.0 is unchanged: the native renderer, ~1 % of one core and
14.6 MB at rest, all three styles, and settings applied in place.