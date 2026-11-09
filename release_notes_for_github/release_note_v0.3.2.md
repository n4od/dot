# Release v0.3.2 — One Engine

The Settings preview now asks the **same Rust engine the desktop pet runs** for
its frames, and the styles are defined in one place. Nothing changes on screen;
this release removes the second engine from the app and the duplicated style
definitions.

## 🔧 Under the hood

- **The settings preview is drawn by the Rust engine.** A new `preview_frame`
  command replays the same stateless script the parity fixtures use, and
  `SettingsView` no longer constructs the TypeScript engine. One runtime engine now
  backs both the bench and the desktop pet.
- **Styles are defined once.** `SURFACE_RULES` in `src/pet/materials.ts` is the only
  definition of Clay and Plush; the catalogue generator copies it into
  `engine-catalogue.json`, and the native painter reads it. Adding a style no
  longer needs a matching Rust edit.
- The TypeScript engine stays as the parity reference (142 golden frames) and for
  the browser-only pet view.

## 🎨 Fixed

- **A pale pet no longer disappears on the stage.** A Cream or white pet now has a
  thin edge halo against the paper, so the preview reads at a glance.

## 📦 Downloads

- **`Dot_0.3.2_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

## ⚠️ Note

`pnpm dev` now serves the browser pet view only. The settings preview asks the
Rust engine for frames, so working on the settings UI uses `pnpm app:dev` (the
real dev loop).