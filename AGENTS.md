# AGENTS.md

Dot is a desktop pet: a Tauri v2 shell (Rust) around a Vue 3 + Vite 8 frontend on
Windows. One transparent, frameless, always-on-top window per pet.

## Before you start

- **The owner drives.** Do not scaffold, refactor, install, build or launch the app
  unless explicitly asked. Propose first; wait for a go-ahead.
- Shell is PowerShell, and `pnpm` resolves to `pnpm.ps1` while execution policy is
  Restricted — a bare `pnpm …` fails with a script-disabled error. Always use
  `cmd /c "pnpm …"`.

## Commands

```powershell
cmd /c "pnpm install"
cmd /c "pnpm app:dev"    # Tauri + Vite, hot reload. The real dev loop.
cmd /c "pnpm dev"        # Vite only, in a browser, no shell — for visual work
cmd /c "pnpm build"      # vue-tsc --noEmit && vite build. The only typecheck gate.
cmd /c "pnpm test"       # vitest run: the engine suite, plain Node env (~15s)
cmd /c "pnpm icons"      # regenerate src-tauri/icons from tools/make-icons.mjs
```

- `pnpm test` covers **`src/bot/*.test.ts` only** — 6 files, 94 tests. They are a
  copy of the upstream suites and are the only automated coverage in the repo.
  Vitest runs with `globals: false`, so every test imports
  `{ describe, expect, it }` from `vitest` explicitly.
- `pnpm build` typechecks the test files too (tsconfig includes `src/**/*.ts`), so a
  type error in a test fails the build, not just the tests.
- Browser iteration: `http://localhost:1421/?window=pet&state=orbit`. `?shape=`,
  `?color=` and `?expression=` are also read from the URL.

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

- `src/main.ts` chooses the view from `?window=` (default `pet`). A new window type
  needs a branch there **and** a builder in `src-tauri/src/lib.rs`, **and** a block
  in `src/styles.css`.
- `src-tauri/src/lib.rs` owns window creation, the tray, and six plain commands:
  `pet_geometry`, `set_click_through`, `start_drag`, `quit_app`, `get_config`,
  `set_config`. They are not a plugin, so `capabilities/default.json` only needs
  `core:default`.
- **Rust owns the pet's appearance** (`config.json` in the app config dir), because
  `create_pet` runs in `setup()` before any webview exists. The values travel to the
  pet as query parameters, and `set_config` re-opens the window at the same place to
  apply them. Don't move that state into the frontend — Rust would have nothing to
  draw with at startup.
- Window labels are hardcoded (`pet-1`, `settings`); "Add a pet" currently just
  re-focuses `pet-1`.
- `src/bot/` is a frozen copy of `jeremy-prt/bloub` @ `b4bb3c1` (MIT), tests
  included. `bloub/` is a reference snapshot that is gitignored and excluded from
  Vite's watcher — **editing it changes nothing**, including its copy of the test
  suite, which is never run. Re-copy files by hand if upstream is updated.
- `src/bot/engine.ts` is clock-free: `sample(now)` is pure with `now` in seconds, and
  `reset`/`setState`/`setLook` all take a clock time. Never read `Date.now()` inside
  the engine — the pet relies on pausing and resuming without drifting out of pose.

## Rendering constraints that look like style choices

- **`src/styles.css` is shared by both windows and gates its rules on
  `html[data-window=...]`**, set by `main.ts` from `?window=`. The pet block makes
  the document transparent and non-interactive; the settings block makes it an
  ordinary scrolling document. A rule written unscoped leaks into the other window —
  that is how `#app svg { width: 100% }` silently overrode the picker's own sizes
  and how `overflow: hidden` made the picker unscrollable.
- Transparency depends on the pet block keeping `html`, `body` and `#app`
  transparent. Any opaque background there turns the window into a white square.
- Eyes are **filled and clipped** to the body path, not punched out as mask holes,
  and there is no paper backing disc. Both deliberately differ from the upstream
  demo: on a transparent window a hole shows the wallpaper and the orbit rings
  drawn behind the body. `src/contrast.ts` holds the eye fill, shared with the
  settings preview so the picker cannot misrepresent the pet.
- `class="hit"` is load-bearing. The click-through test is
  `elementFromPoint(…).closest('.hit')`; renaming or dropping those classes silently
  makes the pet unclickable.
- The pet window starts with `set_ignore_cursor_events(true)`. If the per-frame hit
  test stops flipping it, the pet is visible but invisible to the mouse.

## Bookkeeping

- The version lives in three places that must agree: `package.json`,
  `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`. Bump all three and add a
  `CHANGELOG.md` entry (Keep a Changelog + SemVer).
- `tauri.conf.json` carries two CSPs: `csp` for the built app and `devCsp` for the
  Vite dev server (HMR needs `ws://localhost:1421` and inline styles). Tightening
  the production one is fine; dropping `devCsp` silently applies the production
  policy in dev too.
- Dot is MIT (© N4OD). The engine is MIT (© Jérémy Perret); its notice must stay in
  `THIRD-PARTY-NOTICES.md` as long as `src/bot/` contains it.
