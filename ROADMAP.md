# Roadmap

What is planned for Dot, in the order it is worth doing. The order is *risk
reduction first, then user-facing value* — not what is most fun to build.

`[x]` shipped · `[ ]` planned. [CHANGELOG.md](CHANGELOG.md) is what actually
shipped; this file is the intention, and the two should not disagree.

## Phase 1 — A safety net

Everything later is verified by hand today. Fix that first.

- [x] **Continuous integration.** `.github/workflows/ci.yml` runs the web job
      (typecheck, build, tests) on Linux and the Rust job (`cargo fmt --check`,
      `cargo clippy -D warnings`, `cargo test`) on Windows.
- [x] **Tests for the app's own pure code.** `src/contrast.test.ts` pins the eye
      fill to perceived luminance; `src/display.test.ts` checks that every
      catalogue id has a label and that no label outlives its id.
- [x] **Rust config tests.** An empty file gets the default pet, a file written
      before `style` existed loads flat, an unknown id is kept for the frontend to
      resolve, and a hand-edited value cannot inject a query parameter.
- [x] **A gaze-morph contract test.** `src/engine.test.ts` pins `LOOK_MORPH`, the
      one engine constant the upstream suite cannot protect, without editing the
      frozen fork.

## Phase 2 — Living with it (0.3.0)

A visible WebView2 window used to cost about 13 % of a core before it drew
anything, and the pet is meant to run all day.

- [x] **Cut the idle cost.** The loop sleeps when nothing is happening — no drag,
      no emote, cursor still — drawing at ~15 fps and waking to full rate on a
      cursor movement, a drag or an emote. The native pet goes further: one process
      and no browser at all — see Phase 5.
- [x] **Apply a config change in place.** The native pet thread applies shape,
      expression, colour and style to the running window; no reload, no flash.
- [ ] **Tune and re-verify click-through.** The native window flips
      `WS_EX_TRANSPARENT` from a point-in-polygon hit test; the repeatable check is
      still owed.
- [ ] **Idle CPU budget in CI.** A regression test that samples process CPU would
      keep the loop throttled.

## Phase 3 — Polish and platform

- [x] Remember the pet's position across restarts.
- [ ] Multi-monitor placement and per-monitor DPI changes.
- [x] Autostart on login.
- [ ] Stay on top over fullscreen apps.
- [ ] A bundled interface font instead of the Georgia/Segoe fallbacks.
- [ ] Reduced-motion support.
- [ ] Optional: more styles. The `Surface` seam makes each one numbers plus a
      label — see the checklist in `src/pet/materials.ts`.

## Phase 4 — Distribution

- [ ] **Updater** (Tauri) and **code signing**, so an installed Dot can update
      itself and the installer is not "unknown publisher".
- [ ] **Release automation**: on a `v*` tag, build and attach the NSIS installer
      and the portable exe to a GitHub Release automatically.
- [ ] **Decide the artwork licence.** The silhouettes imitate x.ai's avatar; fine
      for a personal pet, worth replacing before shipping commercially.

## Phase 5 — A renderer with no browser

Phase 2 measured the ceiling of the old design: ~13 % of one core and ~300 MB,
almost all of it the WebView2 runtime rather than Dot. This phase removed it for
the pet window, which is what "low resource" actually requires.

- [x] **Spike done — the numbers justify the rewrite.** A software-rasterised blob
      (`tiny-skia`) blitted to a transparent layered window measured **~0.5 % of a
      core at 60 fps, ~0.08 % at 15 fps, in 6.3 MB across a single process** —
      against the WebView2 pet's ~14 % and ~300 MB. The spike is a throwaway in the
      temp directory, not in the tree.
- [x] **Get the engine out of the browser.** Ported to `src-tauri/src/engine/`,
      driven by `assets/engine-catalogue.json` — the measured tables, including the
      `eyefit` solve, are dumped rather than retyped. A Rust integration test
      replays 142 golden scripts generated from the TypeScript engine and matches
      every frame, so the two cannot drift silently.
- [x] **Native pet window is shipping.** `src-tauri/src/pet.rs` creates a
      transparent, layered, always-on-top Win32 window on its own thread and paints
      it with `src-tauri/src/render.rs` (tiny-skia, all three styles). It hit-tests
      the silhouette to toggle click-through, starts an OS drag from the body, and
      tracks the global cursor and emotes. One process, **no WebView2**: measured
      **~1.1 % of one core and 14.6 MB at rest**, against ~14 % and ~324 MB before.
      Config changes apply to the running pet in place.
- [x] **Keep the settings picker honest.** The preview now gets its frames from
      the Rust engine through `preview_frame` — the same engine the desktop pet
      runs — and paints them with the same SVG `PetFigure`. Styles are defined
      once, in `materials.ts`'s `SURFACE_RULES`; the catalogue generator copies them
      into `engine-catalogue.json`, which `render.rs` reads. A new style is a rule,
      a label, and a regenerate.

## Not planned

- **macOS and Linux.** The global-cursor poll and the transparent, click-through
  window are Windows-shaped; that is a rewrite, not a port.
- **More styles for their own sake.** Three proves the seam. A new one is a
  `Surface` and a label whenever it is actually wanted.