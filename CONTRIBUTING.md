# Contributing

Thanks for looking. Dot is a small project, and the notes below are the whole
process.

## Before you start

- **The engine exists twice, on purpose.** `src/bot/` is a frozen copy of
  [`jeremy-prt/bloub`](https://github.com/jeremy-prt/bloub) and is the reference;
  `src-tauri/src/engine/` is the Rust port the app actually runs. They are joined
  by a contract: `tools/golden/dump.ts` generates 142 golden frames, and
  `cargo test` replays them in `src-tauri/tests/engine_parity.rs`. **Do not weaken
  the parity test to make a change pass** — change the other engine too.
- **Rust owns the state.** `config.json`, the native pet window
  (`src-tauri/src/pet.rs`) and the settings window (`src-tauri/src/lib.rs`) are all
  Rust. The pet is a Win32 layered window painted with tiny-skia
  (`src-tauri/src/render.rs`); it is not a webview.
- **Ids are storage keys.** The French ids in `src/bot/` are written to
  `config.json`. Rename a label, never an id, and never reorder them.

## Setting up

```powershell
cmd /c "pnpm install"
cmd /c "pnpm app:dev"    # Tauri + Vite, hot reload — the real dev loop
```

`pnpm dev` (Vite only) serves the browser pet view. The settings preview asks the
Rust engine for its frames, so it needs the Tauri shell.

## Before opening a pull request

Run what CI runs:

```powershell
cmd /c "pnpm build"                                      # typecheck (vue-tsc)
cmd /c "pnpm test"                                       # web tests
cargo test --manifest-path src-tauri/Cargo.toml          # config + 142 parity cases
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

If you touch the TypeScript engine or the catalogue, regenerate the contract and
commit the result:

```powershell
cmd /c "pnpm exec vitest run --config vitest.golden.config.ts"
```

## Style

- Match the surrounding code, and its comments: they explain *why*, not *what*.
- Keep the settings preview and the desktop pet in step. They share one engine and
  one style definition; a new style is a rule in `SURFACE_RULES` plus a label, then
  a regenerate.
- Nothing in `src/bot/` or `bloub/` should be edited to make the app's behaviour
  pass a test.
