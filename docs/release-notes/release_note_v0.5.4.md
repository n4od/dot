# Release v0.5.4 — Right Here

Two small quality-of-life changes: the pet can be driven from where you are already
looking, and it can no longer be running twice at once.

## ✨ New

- **A menu on the pet itself.** Right-click it for **Emote now**, **Settings...** and
  **Quit** — no trip to the tray icon. **Emote now** asks for the same spontaneous
  emote the pet plays on its own; Settings and Quit are the same actions the tray
  carries.

## 🐛 Fixed

- **Only one pet, ever.** Launching Dot again used to add a second pet to the
  wallpaper — the window is native, so nothing stopped it. A named mutex now holds
  the claim: the second launch asks the Dot that is already running to bring its pet
  back on screen, then steps aside without building a tray or a window. If you hid
  the pet earlier, launching again is now a way to get it back.

## 🔧 Under the hood

- The menu runs on the pet's own thread, because a Win32 popup menu has to. The pet
  holds its last frame while the menu is open — that is how menus behave, not a
  stall.
- The pet itself still knows nothing about Tauri: Settings and Quit are closures the
  shell binds to its main thread and hands down.
- No engine, painter or golden change — the 148-case parity suite is untouched.

## 📦 Downloads

- **`Dot_0.5.4_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

## ⚠️ Notes

- Windows only, as before.
- The claim is per login session, so two people on one PC each get their own pet.
- Exclusive-fullscreen games cover it, as they do any window.
