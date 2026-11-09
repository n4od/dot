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

The window costs ~36 % of one core just to sit there at 60 fps, and the pet is
meant to run all day.

- [ ] **Cut the idle cost.** Reduce the frame rate when nothing is happening, or
      pause `rAF` while the cursor is away and no emote is playing, and slow the
      Rust cursor poll to match. Measure before and after.
- [ ] **Apply a config change in place** instead of reopening the pet window, so
      picking a colour does not flash the pet or reset its animation.
- [ ] **Tune and re-verify click-through.** It is load-bearing and the current
      hand probe is flaky; make the check repeatable.
- [ ] **Idle CPU budget in CI.** Once the loop is throttled, a regression test
      that samples process CPU would keep it throttled.

## Phase 3 — More than one pet (0.4.0)

The feature the README has never delivered.

- [ ] **`config.json` becomes a list** keyed by window label, with a migration
      from the current single-appearance file.
- [ ] **The tray adds a pet**, creating `pet-2`, `pet-3`…
- [ ] **Settings edits the focused pet**, and a closed window forgets it.
- [ ] **Per-pet styling**: two pets can be different shapes, colours and styles.

## Phase 4 — Polish and platform

- [ ] Remember the pet's position across restarts.
- [ ] Multi-monitor placement and per-monitor DPI changes.
- [ ] Autostart on login; stay on top over fullscreen apps.
- [ ] A bundled interface font instead of the Georgia/Segoe fallbacks.
- [ ] Reduced-motion support.
- [ ] Optional: more styles. The `Surface` seam makes each one numbers plus a
      label — see the checklist in `src/pet/materials.ts`.

## Phase 5 — Distribution

- [ ] **Updater** (Tauri) and **code signing**, so an installed Dot can update
      itself and the installer is not "unknown publisher".
- [ ] **Release automation**: on a `v*` tag, build and attach the NSIS installer
      and the portable exe to a GitHub Release automatically.
- [ ] **Decide the artwork licence.** The silhouettes imitate x.ai's avatar; fine
      for a personal pet, worth replacing before shipping commercially.

## Not planned

- **macOS and Linux.** The global-cursor poll and the transparent, click-through
  window are Windows-shaped; that is a rewrite, not a port.
- **More styles for their own sake.** Three proves the seam. A new one is a
  `Surface` and a label whenever it is actually wanted.