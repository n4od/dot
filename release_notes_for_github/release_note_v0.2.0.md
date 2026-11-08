# Release v0.2.0 — Dress Your Pet

Dot becomes yours: pick a shape, a colour, a face and a finish, watch it change live, and it remembers. This release turns the settings window from a form into a workbench, with the pet itself sitting on the stage while you dress it.

## 🚀 Key Improvements

### 🚀 New Features

- **A pet you can design**: Tray → Settings now offers eight shapes, twelve colours, sixteen expressions and three finishes. Every click applies to the pet on your desktop immediately.
- **It remembers**: the choice lives in one small `config.json` in the app config directory. Rust owns that file, because the pet window is created before any webview exists that could be asked what to draw.
- **Degrades instead of breaking**: a missing, unreadable or hand-edited config file falls back to the catalogue defaults, and an unknown id resolves to a default rather than failing to start.
- **Three paint styles**: Flat (the original one-colour fill), Clay (a firm, glossy lit surface with a specular and eyes that read as pressed in) and Plush (warm and diffuse, with a broad highlight and a soft sheen). Style defaults to Flat, so a config written before the field existed opens on the pet it always was.
- **English everywhere**: Circle, Pebble, Cloud / Ink, Blue, Cream / Neutral, Sleepy. The stored ids stay French, so existing config files and the upstream engine tests are untouched.

### 🎨 UI / UX

- **The settings window is a workbench**: a live, animated pet on a paper stage at the left — the real engine, not a still — with quiet controls at the right. Change the expression and the face changes in front of you.
- **Expressions are faces, not floating eyes**: each of the sixteen chips is a miniature of the real pet, cropped to the body and painted in your current colour. Eyes on their own read as broken glyphs; a squint is two marks, a sleepy face two dashes.
- **The head turn is cancelled in the thumbnails** so the row reads straight-on, while the per-eye asymmetries that carry each expression are kept.
- **One selection language**: the mark is the content, and selection is a ring around it — no bordered cards, no boxes with dots in them.
- **The palette is bare swatches**, the shapes a compact chip strip, and the whole picker fits 780×600 without a scrollbar.

### 🐛 Bug Fixes

- **Fixed the settings window being unscrollable**, with stretched previews: `styles.css` applied its pet-window rules to every window, so `overflow: hidden` clipped the expression grid and `#app svg { width: 100% }` overrode the scoped sizes. Both are now scoped to the pet window only.
- **Fixed the preview winking on every change**: the wink deadline outlived the engine rebuild, so picking an option fired a wink on top of the perk animation.
- **Fixed a pale pet vanishing against the paper**: Cream had no edge, so shape silhouettes now carry a hairline stroke and expression thumbnails a soft drop shadow.

### 🛠 Developer / Core

- **One drawing for every pet**: `src/pet/PetFigure.vue` is shared by the desktop window and the settings stage, so the picker cannot promise something the pet will not do.
- **Styles are data**: a style is a `Surface` — three tones, where the light sits and how far it reaches, and the strength of the shade, sheen and eye recession. The figure paints any `Surface` it is handed, so a new style of this kind is a `Surface` plus a label and no drawing code at all.
- **The engine suite runs in this repo**: 94 tests over `src/bot/`, plus the first tests outside it, pinning the style ids as storage keys and the shading order of every lit style. `pnpm test` is now 101 tests in 7 files.
- **Cost measured, not guessed**: styles are fills and gradients only, no per-frame filters. Clay adds roughly 0.3–0.6 ms per frame against a 16.7 ms budget.

## 📦 Downloads

- **`Dot_0.2.0_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

Windows 10/11. Dot uses the WebView2 runtime, which ships with Windows by default.

---

## 📅 Recent Version History

- **v0.2.0**: configurable shape, colour, expression and style; the settings workbench; three paint styles.
- **v0.1.0**: first release — a transparent desktop pet on the bloub morph engine.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/dot/blob/main/CHANGELOG.md)