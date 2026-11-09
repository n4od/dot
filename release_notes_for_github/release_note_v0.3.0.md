# Release v0.3.0 — A Native Pet

Dot no longer runs its pet in a browser. The pet is now a native Windows layered
window painted directly, and the change is not cosmetic: it is roughly **13× less
CPU and 22× less memory** while looking and behaving exactly the same.

## 📊 The numbers

Measured on one idle pet, release build, 40 s window:

| | v0.2.0 (WebView2) | v0.3.0 (native) |
|---|---|---|
| CPU | ~14 % of one core | **~1.1 %** |
| Memory (working) | 324 MB | **14.6 MB** |
| Memory (private) | 137 MB | **3.3 MB** |
| Processes | 7 | **1** |
| WebView2 processes | 6 | **0** |

Same shapes, same expressions, same three styles, same orbits and emotes, same
click-through. It just stopped carrying a browser to draw a blob.

## ✨ What changed

- **The pet is a native window.** A transparent, always-on-top Win32 layered
  window painted with tiny-skia. It owns its own message loop, cursor tracking,
  click-through, drag and emote scheduling.
- **The engine was ported to Rust.** `src/bot/` remains the TypeScript reference
  and the settings preview; `src-tauri/src/engine/` is the native engine the pet
  runs. A **142-case golden-frame parity suite** replays the same frames through
  both and matches every field — every path and matrix string comes out
  byte-for-byte identical.
- **Settings apply in place.** Picking a shape, colour, expression or style now
  morphs the running pet directly: no window reload, no flash, no reset of the
  animation.
- **The settings window is unchanged in use**, and is still a webview — but it is
  created on demand, so it costs nothing while closed.
- **All three styles were reproduced natively**: Flat, Clay and Plush, including
  the shade, sheen and eye-recession that give each one its character.
- **The browser pet view sleeps at idle** (it is now only a visual-development
  tool): ~15 fps while resting, 60 while moving or emoting, stopped while hidden.

## 🔧 Under the hood

- `docs/native-renderer.md` is the full case study: the measurements that found
  the WebView2 floor, the spike that proved a native window cheaper, the parity
  contract, and the loop fixes that took idle CPU from 2.55 % to 1.09 %.
- `tools/golden/dump.ts` regenerates the parity contract and the catalogue.
- The measured tables — radial profiles, orbit seeds, blink schedule and the
  `eyefit` solve — travel as `src-tauri/assets/engine-catalogue.json` rather than
  being retyped.

## 📦 Downloads

- **`Dot_0.3.0_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

## ⚠️ Notes

- `config.json` is unchanged; a 0.2.0 file loads as-is, and unknown ids still fall
  back to the catalogue defaults.
- Windows only, as before.
- The settings **preview** still uses the TypeScript engine while the desktop pet
  paints natively; the parity suite keeps the frames equal, but a new style is
  currently added in both `materials.ts` and `render.rs`.
- Tray → **Add a pet** is a no-op until the multi-pet list lands; Phase 3 of the
  roadmap.