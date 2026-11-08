# Dot

A desktop pet that lives in a transparent, frameless window: it turns its head
towards your real cursor anywhere on the screen, emotes on its own, and lets
clicks through wherever it is not painted.

Built on the morph engine from [`jeremy-prt/bloub`](bloub) (MIT), pinned at
commit [`b4bb3c1b5f93c7b87a2e8d620f667c4093d97749`](https://github.com/jeremy-prt/bloub/commit/b4bb3c1b5f93c7b87a2e8d620f667c4093d97749).
`src/bot/` is a copy of that engine, plus its test suite: it is framework-free,
DOM-free and clock-free, so `sample(t)` is a pure function of time. That is what
lets the pet pause, resume and sleep without ever drifting out of pose.

`bloub/` is a vendored snapshot of that commit, kept for its tests, its
customiser and its docs. It is deliberately **not** a git repository — the
history was stripped, so it cannot turn this project into a nested repo, and
`.gitignore` keeps it out of ours. To restore full history:

```bash
git clone https://github.com/jeremy-prt/bloub.git /tmp/bloub
git -C /tmp/bloub checkout b4bb3c1b5f93c7b87a2e8d620f667c4093d97749
```

## Running it

```bash
pnpm install
pnpm app:dev      # Tauri + Vite with hot reload (what you want while iterating)
pnpm dev          # Vite only: the animation runs in a browser, with no shell
pnpm build        # vue-tsc --noEmit && vite build
pnpm test         # vitest run: the engine suite, in a plain Node environment
```

A standalone binary needs the frontend **embedded**, and that only happens with
the `custom-protocol` feature:

```bash
pnpm build
cargo build --manifest-path src-tauri/Cargo.toml --features custom-protocol
# -> src-tauri/target/debug/dot.exe
```

A bare `cargo build` compiles and runs, but the window comes up blank: Tauri
loads a dev server URL that is not there and embeds nothing. `tauri build` adds
the feature for you; a plain `cargo` invocation does not.

The tray icon is the only way to quit while the pet is on screen: **Quit**.

## Windows and how they are made

| Window | Created in | Shape |
|---|---|---|
| `pet-1` | `src-tauri/src/lib.rs` | 180×180, transparent, undecorated, always on top, skipped from the taskbar |
| `settings` | tray → Settings | normal decorated window |

Both load the same `index.html` and pick a view from `?window=`.

For visual work there is also `?window=pet&state=orbit`: it opens straight into
one state instead of the idle loop.

## Configuring it

**Tray → Settings** opens the picker: eight shapes, twelve colours and sixteen
expressions, each previewed by the engine itself. Every click applies immediately.

The choice lives in one small JSON file:

```json
{ "shape": "triangle", "color": "violet", "expression": "somnolent" }
```

at `%APPDATA%\dev.dotscode.dot\config.json`. Delete it to go back to defaults
(`cercle` / `encre` / `neutre`).

**Rust owns that file, not the frontend.** `create_pet` runs in `setup()`, before
any webview exists to be asked what to draw, so the config has to be readable
without one. The pet window is opened with the values as query parameters, and a
change re-opens it with the new ones — which is why a colour applies without
restarting anything. A missing, unreadable or half-written file is not an error:
unknown ids fall back to the catalogue defaults, so a hand-edited file degrades
instead of breaking the pet.

## How the desktop-specific parts work

**Global cursor.** A page can only see the pointer inside its own box, which is
useless for a 180 px pet. A Rust thread polls `AppHandle::cursor_position()` every
33 ms and emits `pet://cursor` in physical pixels. The pet window converts that
into a look direction, normalised against the monitor so the gaze saturates at
the screen edge rather than at the window edge.

**Click-through.** A transparent window still swallows clicks over its whole
rectangle. `set_ignore_cursor_events` is all-or-nothing per window, so the pet
flips it based on a hit test: `elementFromPoint` against the painted shapes
(`.hit`), probed at five points for hysteresis. Transparent corners pass clicks
to the desktop; the body takes them.

**Eyes.** The original demo draws the eyes as holes in a mask so they reveal the
page colour. A transparent window has no page colour — a hole would show the
wallpaper and the orbit rings drawn behind the body. So the eyes are filled with
a contrasting colour and clipped to the body path instead. Same geometry, same
morphs, desktop-safe.

**No paper backing.** Same reason: the opaque path the demo places under the body
to hide the back half of the rings would show as a disc. Here the body is simply
opaque and everything behind it is genuinely behind it.

## What is not here yet

- More than one pet: the tray's *Add a pet* currently re-focuses `pet-1`, and the
  config file describes a single appearance. Both need to become a list keyed by
  window label (`pet-2`, `pet-3`, ...).
- Autostart, always-on-top over fullscreen apps, multi-monitor placement,
  per-monitor DPI changes.
- Tests for anything outside `src/bot/`: the engine suite is the only coverage,
  and the pet's own behaviour (hit testing, gaze, drag) is still unproven by
  anything but hand-checking.

## Licence

Dot is MIT, © 2026 [N4OD](https://github.com/N4OD) — see [LICENSE](LICENSE).

The engine in `src/bot/` is separately MIT, © Jérémy Perret, copied from
`jeremy-prt/bloub` at commit `b4bb3c1b5f93c7b87a2e8d620f667c4093d97749`. That
notice is reproduced in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md), as MIT
requires.

One caveat on the licence, not the code: the visual design is an imitation of
x.ai's bot avatar, as the upstream README says. Fine for a personal pet; worth
replacing the silhouettes before shipping it commercially.
