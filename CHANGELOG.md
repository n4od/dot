# Changelog

All notable changes to Dot are recorded here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **The engine's test suite runs in this repo.** `pnpm test` executes the six Vitest
  suites from `bloub` — 94 tests over `src/bot/`, in a plain Node environment in
  about 15 seconds. `pnpm build` typechecks them as well, so a type error in a test
  fails the build rather than only the tests. Vitest runs with `globals: false`, so
  each test imports `describe`, `expect` and `it` from `vitest` explicitly.

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
