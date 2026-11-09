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
pnpm test         # vitest run: the engine suite and the app's own tests
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
| `settings` | tray → Settings | 780×600, normal decorated window |

Both load the same `index.html` and pick a view from `?window=`.

For visual work there is also `?window=pet&state=orbit`: it opens straight into
one state instead of the idle loop.

## Configuring it

**Tray → Settings** is a workbench: the pet sits on a stage at the left, alive —
blinking, and winking every so often — while shape, colour and expression are
picked at the right. Every click applies to the pet on your desktop immediately.

The pet on the stage is not an illustration. It is the same engine the desktop
window runs, sampled on the same clock-free `sample(t)` and drawn by the same
component (`src/pet/PetFigure.vue`), so the bench cannot promise something the
pet will not do.

Option names are English (`Circle`, `Ink`, `Sleepy`); the ids behind them stay
French, because those are what the engine, its tests and the config file use.
`src/display.ts` holds the mapping.

The choice lives in one small JSON file:

```json
{ "shape": "triangle", "color": "violet", "expression": "somnolent", "style": "argile" }
```

at `%APPDATA%\dev.dotscode.dot\config.json`. Delete it to go back to defaults
(`cercle` / `encre` / `neutre` / `plat`).

**Rust owns that file, not the frontend.** `create_pet` runs in `setup()`, before
any webview exists to be asked what to draw, so the config has to be readable
without one. The pet window is opened with the values as query parameters, and a
change re-opens it with the new ones — which is why a colour applies without
restarting anything. A missing, unreadable or half-written file is not an error:
unknown ids fall back to the catalogue defaults, so a hand-edited file degrades
instead of breaking the pet.

## Styles, and how the pet is painted

`style` is the fourth axis, chosen with a pill in the settings header:

| id | label | what it does |
|---|---|---|
| `plat` | Flat | one flat fill — the default, and what the upstream demo does |
| `argile` | Clay | a lit surface: a top-left highlight, a shaded lower edge, a specular, and eyes that read as pressed in |
| `peluche` | Plush | the same light, warm and diffuse: a broad highlight, a big soft sheen, a gentle edge — a toy |

Both spend the *same* geometry. The style changes no point the engine emits, only
how the frame is painted. `plat` is the default precisely so that a `config.json`
written before styles existed still opens on the pet it always was — the field is
`#[serde(default)]` on the Rust side, and an unknown id resolves back to `plat`
in `src/pet/materials.ts`.

A style is **data, not code**: `Surface` in `src/pet/materials.ts` is the whole
vocabulary — three tones, where the light sits and how far it reaches, the strength
of the shade, the sheen and the eye recession — and `PetFigure.vue` paints whatever
`Surface` it is handed. Every colour is derived from the one ink hex, so all twelve
colours work including the near-black and near-white ends, and a lit style is built
from **fills and gradients only** — no SVG filters, which would cost a blur per
element on every animation frame.

### Adding a style

1. **`src/pet/materials.ts`** — add the id to `StyleId` and `STYLE_IDS`, and return a
   `Surface` from `surfaceFor`, deriving every colour from the ink.
2. **`src/display.ts`** — add the English label to `STYLE_NAMES`.
3. **Done.** `PetFigure.vue` paints any `Surface`. It needs a change only when a
   style brings a *new kind of element* — an outline, a second silhouette, a tinted
   orbit — rather than new numbers.
4. **Rust — nothing.** `Config` accepts any string, `pet_query` passes it through,
   the settings pill is generated from `STYLE_IDS`, and the pet validates against
   `isStyleId`. Rust only changes if the *default* style changes.

The ids are storage keys: **add, never rename** — `config.json` on disk and the
upstream engine tests both depend on the ones already written.

Whatever goes in must keep four promises: fills only (no per-frame filters), no
opaque backdrop (the window is transparent), `.hit` stays on the body path and the
eye group (or the pet stops accepting clicks), and the eyes stay filled and take
their contrast from `src/contrast.ts`. It also has to read at 42 px — the
expression thumbnails draw the same style through `bare` + `zoom` — and on the
irregular silhouettes, not just the circle.

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

The plan is [ROADMAP.md](ROADMAP.md). In short:

- **Cost**: the pet window runs at 60 fps whenever it is visible, about 36 % of one
  core. Cutting that when nothing is happening is the next thing worth doing.
- **More than one pet**: the tray's *Add a pet* currently re-focuses `pet-1`, and the
  config file describes a single appearance. Both need to become a list keyed by
  window label (`pet-2`, `pet-3`, ...).
- **Platform**: autostart, always-on-top over fullscreen apps, multi-monitor
  placement, per-monitor DPI changes.
- **Coverage**: the engine has the upstream suite and the app's pure code is
  covered too — but the pet's live behaviour (hit testing, gaze, drag) is still
  checked by hand.

## Licence

Dot is MIT, © 2026 [N4OD](https://github.com/N4OD) — see [LICENSE](LICENSE).

The engine in `src/bot/` is separately MIT, © Jérémy Perret, copied from
`jeremy-prt/bloub` at commit `b4bb3c1b5f93c7b87a2e8d620f667c4093d97749`. That
notice is reproduced in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md), as MIT
requires.

One caveat on the licence, not the code: the visual design is an imitation of
x.ai's bot avatar, as the upstream README says. Fine for a personal pet; worth
replacing the silhouettes before shipping it commercially.
