# Release v0.4.0 — Settling In

Dot grows up a little: it remembers where you put it, it can start with Windows,
the tray item finally does something, and it behaves on a multi-monitor desk.

## ✨ New

- **It remembers where you left it.** Drag the pet somewhere, quit, relaunch — it
  comes back there, clamped to the monitor's work area so it can never reopen
  off-screen.
- **Start with Windows.** A **Startup** toggle in Settings registers Dot with
  Windows, or removes it, with no separate launcher.
- **The tray does something.** The old dead *Add a pet* is now a **Hide pet / Show
  pet** toggle.
- **Multi-monitor.** Gaze is normalised against the monitor the pet is on, and
  dragging it to a screen with a different scale resizes it so it keeps the same
  physical size.
- **Double-click to wink.** A second click within 320 ms plays a wink; a single
  click still drags.

## 🔧 Under the hood

- Autostart is a `HKCU\...\Run` value written by the app itself — no plugin.
- The remembered position lives in `window.json`, beside `config.json`.
- Still one native process, ~1 % of a core and ~15 MB at rest, no WebView2 for the
  pet.

## 📦 Downloads

- **`Dot_0.4.0_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

## ⚠️ Notes

- Windows only, as before.
- Exclusive-fullscreen games cover it, as they do any window.