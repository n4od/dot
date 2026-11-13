# AGENTS.md

Dot is a desktop pet on Windows. The pet itself is a **native** transparent,
frameless, always-on-top Win32 layered window (`src-tauri/src/pet.rs`), driven by a
Rust port of the animation engine and painted with tiny-skia. A Tauri v2 shell
around a Vue 3 + Vite 8 frontend remains for the tray, the settings window and
`config.json`; WebView2 is only created while Settings is open.

## Before you start

- **The owner drives.** Do not scaffold, refactor, install, build or launch the app
  unless explicitly asked. Propose first; wait for a go-ahead.
- **Never write history without an explicit instruction.** No `git commit`, `amend`,
  `reset`, `rebase` or `tag` unless the owner says to commit. "go" for a plan is not
  authorisation to commit it. Do the work, verify it, report, stop, and wait.
- Shell is PowerShell, and `pnpm` resolves to `pnpm.ps1` while execution policy is
  Restricted — a bare `pnpm …` fails with a script-disabled error. Always use
  `cmd /c "pnpm …"`.

## Commands

```powershell
cmd /c "pnpm install"
cmd /c "pnpm app:dev"    # Tauri + Vite, hot reload. The real dev loop.
cmd /c "pnpm dev"        # Vite only, browser pet view (settings preview needs the shell)
cmd /c "pnpm build"      # vue-tsc --noEmit && vite build. The only typecheck gate.
cmd /c "pnpm test"       # vitest run: the engine suite, plain Node env (~15s)
cmd /c "pnpm icons"      # regenerate src-tauri/icons from tools/make-icons.mjs
```

- `pnpm test` runs everything under `src/**/*.test.ts`: the six upstream suites
  copied into `src/bot/`, plus the app's own — `contrast.test.ts`,
  `display.test.ts`, `pet/materials.test.ts` and `engine.test.ts`. 10 files, 109
  tests. Vitest runs with `globals: false`, so every test imports
  `{ describe, expect, it }` from `vitest` explicitly.
- **The Rust side is tested too**: `cargo test --manifest-path src-tauri` runs the
  config serde tests and the **142 engine-parity cases** in
  `src-tauri/tests/engine_parity.rs`. CI (`.github/workflows/ci.yml`) runs both
  sides, plus `cargo fmt --check` and `cargo clippy -D warnings`.
- `pnpm build` typechecks the test files too (tsconfig includes `src/**/*.ts`), so a
  type error in a test fails the build, not just the tests.
- Browser iteration: `http://localhost:1421/?window=pet&state=orbit`. `?shape=`,
  `?color=`, `?expression=` and `?style=` are also read from the URL. This is the
  **browser** pet view for visual work; the desktop pet is native and does not use
  it. The settings preview is not usable in a plain browser: it asks the Rust
  engine for its frames (`preview_frame`), so it needs the Tauri shell.

## Build traps

- **A standalone binary needs `custom-protocol`**, or the window appears with the
  right size and position and renders nothing:
  `cargo build --manifest-path src-tauri/Cargo.toml --features custom-protocol`.
  `pnpm app:build` passes it; a plain `cargo build` does not. The feature is
  declared in `src-tauri/Cargo.toml` — don't remove it.
- **Don't set `build.minify: 'esbuild'` in `vite.config.ts`.** Vite 8 no longer
  bundles esbuild, and the build dies with `Failed to load transformWithEsbuild`.
- Port **1421** is pinned on both sides (`vite.config.ts` ↔ `tauri.conf.json`
  `devUrl`). Changing one breaks `app:dev`.

## Wiring

- **The desktop pet is not a webview.** `src-tauri/src/pet.rs` owns a Win32
  layered window on its own thread: creation, message loop, cursor poll,
  click-through, drag, emotes and in-place config updates.
  `src-tauri/src/render.rs` paints frames with tiny-skia. `src-tauri/src/lib.rs`
  keeps the tray, `config.json` and the settings window and exposes three commands:
  `quit_app`, `get_config`, `set_config`. `capabilities/default.json` only needs
  `core:default`.
- `src/main.ts` and `src/styles.css` now serve the **settings** window in the app.
  `?window=pet` remains for browser iteration only; it is not what runs on the
  desktop. A new webview window type still needs a branch in `main.ts`, a builder in
  `lib.rs`, and a block in `styles.css`.
- **Rust owns the pet's appearance.** `Config` is `#[serde(default)]`: a file
  written before a field existed still loads, so add new fields with a default,
  never make one required. The pet thread reads it at startup, and `set_config`
  hands later changes to it, applied in place. Don't move this state into the
  frontend.
- The pet remembers its position in `window.json` (same config dir): the pet thread
  writes it when a drag settles, and `setup` restores it clamped to the monitor work
  area.
- Autostart is a `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` value managed
  by `src-tauri/src/autostart.rs` — no plugin. The settings toggle calls
  `get_autostart` / `set_autostart`.
- **Closing Settings must not close the pet.** Tauri exits when its last window
  closes, and `AppHandle::exit` reports the same `ExitRequested` event. `lib.rs`
  keeps a `QUITTING` flag and calls `api.prevent_exit()` unless an explicit Quit
  set it. Without that, destroying the Settings webview takes the native pet down
  with it — the pet is not a Tauri window.
- The tray's **Hide pet / Show pet** toggles the native window's visibility
  (`PetHandle::set_visible`, applied on the pet thread, which stops drawing while
  hidden). Window labels are hardcoded (`settings` only).
- A **double-click** on the pet plays a wink (`DOUBLE_CLICK_MS` in `pet.rs`); a
  single click starts the OS drag.
- **Reactions live in `pet.rs`, on states the engine already has.** A drag that
  moved the window raises `WIDE_REQUESTED` from the window procedure (compare the
  rect across `WM_NCLBUTTONDOWN`) for a startled look; `SLEEP_AFTER_MS` of no cursor
  inside the pet's rect plays `Sleep` and holds it, and the cursor entering the rect
  or a click wakes it. Neither adds geometry, so neither touches the painters or the
  parity suite.
- `src/bot/` is a frozen copy of `jeremy-prt/bloub` @ `b4bb3c1` (MIT), tests
  included. `bloub/` is a reference snapshot that is gitignored and excluded from
  Vite's watcher — **editing it changes nothing**, including its copy of the test
  suite, which is never run. Re-copy files by hand if upstream is updated.
- **The engine exists twice on purpose, joined by a contract.** `src/bot/` is the
  frozen TypeScript original — now the reference and the browser pet view only.
  `src-tauri/src/engine/` is the Rust port the native pet *and the settings
  preview* use. Neither edits the other. `tools/golden/dump.ts` regenerates the
  golden frames and the catalogue
  (`pnpm exec vitest run --config vitest.golden.config.ts`); `cargo test` replays
  them in `src-tauri/tests/engine_parity.rs`. Change one engine and the parity test
  fails until the other follows — that is the point, do not weaken it to make a
  change pass.
- `src/bot/engine.ts` is clock-free: `sample(now)` is pure with `now` in seconds, and
  `reset`/`setState`/`setLook` all take a clock time. Never read `Date.now()` inside
  the engine — the pet relies on pausing and resuming without drifting out of pose.
- **`src/pet/PetFigure.vue` is the settings preview** (and the browser dev view). It
  gets its frames from the Rust engine through `preview_frame` and paints them with
  SVG; the desktop pet paints the same frames natively in `render.rs`. Its
  `interactive` prop puts `.hit` on the body and eyes for the browser view; `zoom`
  and `bare` crop the expression thumbnails; `styleId` selects the paint — see
  **Styles** below.
- `src/display.ts` maps the engine's French ids to English labels for the interface.
  **The ids are storage keys** (`config.json`, the upstream tests); rename a label,
  never an id.
- The settings window is a fixed-height document with one scrolling region
  (`.panel`): the page itself must not scroll, or the stage scrolls away from the
  controls.

## Styles (how the pet is painted)

- **A style paints geometry; it never moves it.** The engine emits one frame; the
  style decides the fills. `src/pet/materials.ts` owns the preview definitions and
  the checklist; `src-tauri/src/render.rs` owns the native paint. Keep the split.
- **A style is data.** `materials.ts` maps an id to a `Surface` — three tones, the
  light position and reach, and the strength of the shade, sheen and eye recession.
  Both painters consume that vocabulary, so a new style of this kind is numbers, not
  a template branch; a branch is only owed for a new *kind of element*.
- Three styles today: `plat` (default, one flat fill), `argile` (firm, glossy) and
  `peluche` (warm, diffuse). The id is a **storage key** in `config.json`: add
  styles, never rename or reorder ids.
- **To add one:** id + a rule in `SURFACE_RULES` (`materials.ts`), the label in
  `src/display.ts`, then regenerate the catalogue. `render.rs` reads the rule from
  the catalogue, so there is no matching Rust edit. The settings pill is generated
  from `STYLE_IDS`; the preview validates with `isStyleId`. The default lives in
  Rust only.
- **`plat` must stay the default, and an unknown id must stay flat.** Config files
  written before the field existed have no `style` key; `#[serde(default)]` in Rust
  and `isStyleId`/`surface_for` are what keep them opening on the pet they were.
- **A style is bounded by the desktop window**: fills and gradients only, no opaque
  backdrop, eyes filled with the contrast colour (mirrored in `render.rs`), and
  legible at 42 px as well as 180 px, on irregular silhouettes too.

## Rendering constraints that look like style choices

- **`src/styles.css` is the settings document** (and the browser pet view), gated on
  `html[data-window=...]` set by `main.ts`. The settings block is an ordinary
  scrolling document; the browser `pet` block makes it transparent and
  non-interactive. The desktop pet has no document at all.
- **The native window is transparent because it is a layered window** painted with
  per-pixel alpha via `UpdateLayeredWindow`. It has no background brush and no page.
- Eyes are **filled and clipped** to the body, not punched out as holes, and there is
  no paper backing disc — on a transparent window a hole shows the wallpaper and the
  orbits behind the body. `src/contrast.ts` holds the preview's eye fill;
  `src-tauri/src/render.rs` mirrors it for the native pet.
- **In the browser view**, `class="hit"` is load-bearing: the click-through test is
  `elementFromPoint(…).closest('.hit')`; renaming or dropping those classes silently
  makes the pet unclickable. They come from `PetFigure`'s `interactive` prop.
- The native window starts with `WS_EX_TRANSPARENT` and flips it from
  `render::body_contains` on `frame.body_points` (five probes). If that stops
  happening the pet is visible but invisible to the mouse.
- `render::body_contains` has unit tests, and `tools/check-click-through.ps1`
  drives the real cursor and reads `WS_EX_TRANSPARENT` to prove the body takes
  clicks and the corners pass them through.
- **The native frame loop sleeps at the draw cadence** (`BUSY_MS`/`IDLE_MS` in
  `src-tauri/src/pet.rs`): ~60 fps while dragged, emoting or freshly woken, ~15 fps
  at rest, and it only takes the config lock when the version counter moves. The
  browser view (`PetView.vue`) has its own idle sleep, but it is not the desktop pet.
- The pet adapts to its monitor: gaze uses that monitor's size, and a DPI change
  resizes the window and rebuilds its DIB.

## Bookkeeping

- `ROADMAP.md` is the plan and `CHANGELOG.md` is what shipped. Tick a roadmap item
  when it lands; the two must not disagree.
- `docs/native-renderer.md` is the engineering case study for the native renderer:
  the measurements, the decision, the parity contract and the loop fixes. If you
  re-measure, keep its numbers honest.

- The version lives in three places that must agree: `package.json`,
  `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`. Bump all three and add a
  `CHANGELOG.md` entry (Keep a Changelog + SemVer).
- `tauri.conf.json` carries two CSPs: `csp` for the built app and `devCsp` for the
  Vite dev server (HMR needs `ws://localhost:1421` and inline styles). Tightening
  the production one is fine; dropping `devCsp` silently applies the production
  policy in dev too.
- Dot is MIT (© N4OD). The engine is MIT (© Jérémy Perret); its notice must stay in
  `THIRD-PARTY-NOTICES.md` as long as `src/bot/` contains it.
