# Changelog

All notable changes to Dot are recorded here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-11-09

### Added

- **The pet is configurable, and remembers it.** Tray → Settings opens a picker for
  shape, colour, expression and style, and every click applies to the running pet
  immediately. The choice is a small `config.json` in the app config directory.

  Rust owns that file rather than the frontend: the pet window is created in
  `setup()`, before any webview exists that could be asked what to draw. The values
  travel to the pet as query parameters, and a change re-opens the window at the
  same position. A missing or unreadable file falls back to the catalogue defaults,
  and unknown ids resolve to defaults rather than failing to start.

- **`src/pet/PetFigure.vue`**, the one drawing of a pet, used by both the desktop
  window and the settings stage so the two cannot drift. Its `interactive` prop is
  what marks the body and the eyes as click targets; `zoom` crops the viewBox in on
  the body and `bare` drops the orbits and particles, which is how the expression
  thumbnails stay legible at 42 px.

- **`src/display.ts`**, English labels for the engine's French catalogue ids. The
  ids stay as storage keys, so existing config files and the upstream tests are
  unaffected.

- **`src/contrast.ts`**, the eye-fill rule lifted out of the pet view so the preview
  and the pet cannot disagree about what the eyes look like.

- **The engine's test suite runs in this repo.** `pnpm test` executes the six Vitest
  suites from `bloub` — 94 tests over `src/bot/`, in a plain Node environment in
  about 15 seconds. `pnpm build` typechecks them as well, so a type error in a test
  fails the build rather than only the tests. Vitest runs with `globals: false`, so
  each test imports `describe`, `expect` and `it` from `vitest` explicitly.

- **A `style` axis: Flat and Clay.** `style` joins shape, colour and expression in
  `config.json`, chosen with a pill in the settings header. `plat` is the flat fill
  and the default, so a file written before the field existed still opens on the pet
  it always was; `argile` paints the same geometry as a lit surface — a top-left
  highlight, a shaded lower edge, a specular, and eyes that read as pressed in.
  Everything is derived from the one ink hex, so all twelve colours work, and it is
  built from fills and gradients rather than filters, so it costs nothing per frame.

  The definitions live in `src/pet/materials.ts` and the drawing in
  `src/pet/PetFigure.vue`. The module header documents the four-step recipe for a
  third style; adding one needs no Rust change, because the settings pill is
  generated from the style ids and the pet validates against them.

- **`peluche` (Plush), and a style seam that is data rather than code.** Plush is
  the same light, warm and diffuse — a broad highlight, a big soft sheen, a gentle
  edge — where Clay is firm and glossy. Both are now described as a `Surface` in
  `src/pet/materials.ts`: three tones, the light's position and reach, and the
  strength of the shade, sheen and eye recession. `PetFigure.vue` paints any
  `Surface` without knowing a style's name, so adding one of this kind is a
  `Surface` and a label and **no drawing code at all**. It grows only for a new
  *kind* of element — an outline, a second silhouette — not for new numbers.

### Changed

- **The settings window is a workbench instead of a form.** A live, animated pet on
  a paper stage at the left — the real engine, not a still — with quiet controls at
  the right: a single row of eight shape chips, the palette as bare swatches rather
  than twelve boxes with dots in them, and the sixteen expressions as legible faces
  with names, all fitting without a scrollbar at 780×600. All three rows speak one
  language: the mark is the content, and selection is a ring around it rather than a
  box around the tile. Previously it was four identical grey cards, 86 px tiles for
  the options and 26 px eye glyphs with truncated French labels.

- **The expression picker shows faces, not floating eyes.** Each of the sixteen
  chips is now a miniature of the real pet — the engine's own frame, in the pet's
  current colour and with its real eye fill, cropped in on the body. Eyes on their
  own read as broken glyphs: a squint is two marks, a sleepy face is two dashes. The
  head turn is cancelled in the thumbnails so the row reads straight-on, while the
  per-eye asymmetries that carry the expression are kept. The short last row is
  centred, and the panel no longer overflows at 780×600.

### Fixed

- **The settings window could not scroll, and its previews were the wrong size.**
  `src/styles.css` applied its pet-window rules to every window: `overflow: hidden`
  on `html`, `body` and `#app` clipped the picker's expression grid out of reach,
  and `#app svg { width: 100%; height: 100% }` overrode the scoped sizes, so the
  shape previews and expression faces were being stretched. Both are now scoped to
  `html[data-window='pet']`.

- **The settings preview winked on every change.** The wink deadline outlived the
  engine rebuild, so with the clock already past it the pet winked the instant you
  picked something, on top of the perk animation.

- **A pale pet vanished against the paper.** Shape silhouettes and expression
  thumbnails were painted in the pet's colour with nothing to separate Cream from
  the background. The silhouettes now carry a hairline stroke and the thumbnails a
  soft drop shadow.

## [0.1.0] — 2026-11-08

First release. A desktop pet in a transparent window, with no page behind it: the
bloub morph engine from `jeremy-prt/bloub` at `b4bb3c1`, driven by a Tauri v2
shell instead of a browser tab.

### Added

- **One window per pet.** A 180 × 180 frameless window that is transparent, always
  on top, absent from the taskbar and without a shadow. It loads the same bundle as
  the rest of the app and picks its view from `?window=pet`.
- **The pet watches the real cursor.** A Rust thread polls the global cursor every
  33 ms and emits `pet://cursor`; the window turns that into a look direction. The
  gaze is normalised against the monitor rather than the window, so the pet keeps
  looking at the pointer at the edge of the screen instead of saturating a few
  pixels outside its own box — something a web page cannot do at all.
- **Click-through everywhere it is not painted.** Transparent windows still swallow
  clicks across their whole rectangle, so the window flips
  `set_ignore_cursor_events` per frame based on a hit test of the painted shapes,
  probed at five points for hysteresis. Corners pass clicks to the desktop; the
  body takes them.
- **Dragging and a double-click reaction.** Press and hold to move the pet; the
  window re-reads its geometry when the drag ends. Double-click plays a wink.
- **A resting loop with ten emotes.** The pet idles and plays one of `wink`, `wide`,
  `notify`, `exclaim`, `play`, `orbit`, `burst`, `sleep`, `thinking` or `alert` at a
  randomised interval, then returns to idle. Gaze is only applied to states that
  carry the resting face, so an animation is never blurred by a look target.
- **Tray menu.** *Add a pet*, *Settings…* and *Quit* — the tray is the only way to
  quit, since a frameless pet has no window chrome.
- **A settings window.** A normal decorated window proving the second-window and
  IPC round trip; the real one will reuse the upstream customiser.
- **Generated icons.** `tools/make-icons.mjs` writes every PNG and the `.ico` with
  no image dependencies.

### Changed

- **Eyes are filled and clipped to the body, not punched through it.** The web demo
  masks the eyes out of the body so they reveal the page colour. A transparent
  window has no page colour — a hole would show the wallpaper, and the orbit rings
  drawn behind the body would reappear inside the eyes. Same geometry, same morphs,
  desktop-safe.
- **No paper backing.** The opaque disc the demo places under the body to hide the
  back half of the rings would show as a disc here, so the body is simply opaque
  and everything behind it is genuinely behind it.
- **Depth is opacity, not a blend.** Dot particles fade with opacity instead of
  blending towards a background colour that does not exist.

### Fixed

- **A standalone binary rendered a blank window.** Tauri only embeds the frontend
  when the `custom-protocol` feature is on, and only the `tauri` CLI passes it. A
  plain `cargo build` produced an app that started, sized itself correctly and drew
  nothing. The feature is now declared in `src-tauri/Cargo.toml`.

### Notes

- The engine in `src/bot/` is MIT, © Jérémy Perret, copied from
  `jeremy-prt/bloub` at `b4bb3c1b5f93c7b87a2e8d620f667c4093d97749`; see
  [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md). Dot itself is MIT, © N4OD.
- Not in this release: per-pet configuration, more than one simultaneous pet,
  autostart, multi-monitor and DPI handling, and the ported engine test suite. The
  README tracks these.
