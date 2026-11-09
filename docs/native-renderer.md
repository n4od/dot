# How Dot became native

A case study of taking one small desktop pet from **~14 % of a core and 324 MB**
to **~1.1 % of a core and 14.6 MB**, without changing how it looks or moves.

The short version: the expensive thing was never the animation. It was the
browser runtime wrapped around it. Measuring that first — instead of optimising
the drawing — is what made the result possible.

---

## TL;DR

| | WebView2 pet | native pet |
|---|---|---|
| idle CPU | ~14 % of one core | **~1 % (0.7–1.6 % across runs)** |
| memory | 324 MB working / 137 MB private | **14.6 MB / 3.3 MB** |
| processes | 7 | **1** |
| WebView2 processes | 6 | **0** |

The pet is now a Win32 **layered window** (`src-tauri/src/pet.rs`) painted with
**tiny-skia** (`src-tauri/src/render.rs`) from a **Rust port of the engine**
(`src-tauri/src/engine/`). The settings window is still a webview; it is created
on demand, so it costs nothing while closed.

The three things that made this safe rather than a rewrite-and-hope:

1. **Measure the floor before touching code.** A window that draws *nothing* still
   cost ~13 % of a core and ~300 MB. That number decided the architecture.
2. **Spike the risky part from the numbers.** A 150-line software-rendered window
   measured ~0.5 % / 6 MB before the real port began.
3. **Port with a contract, not with care.** 142 golden frames generated from the
   TypeScript engine, replayed by the Rust one; every path and matrix string came
   out byte-for-byte identical.

---

## 1. Measure the floor, not the average

The first instinct is to profile the animation and optimise the drawing. The
first measurement says otherwise.

One idle pet, per process, 30–40 s windows, nothing else running:

| state | total CPU | gpu-process | renderer | memory |
|---|---|---|---|---|
| 60 fps (original) | ~26 % | ~13 % | ~12 % | 324 MB |
| 30 fps (frame cap) | ~19 % | ~10 % | ~9 % | 324 MB |
| **no repaint at all** | **~12.7 %** | **6.7 %** | **5.2 %** | **324 MB** |

`dot.exe` — the actual application — was **21 MB and 0.5 % of a core**. Everything
else was WebView2: a browser, a GPU process, a renderer, two utilities and a
crash handler, all to draw one blob.

Two conclusions fall straight out:

- **A live WebView2 window costs ~13 % of a core before it draws anything.**
- **~300 MB of memory is the runtime, not the app.** No amount of page-side
  optimisation moves it.

That reframes the problem: the frame-rate work (60 → 30 fps, a 27 % cut) was real
but it was trimming a rounding error on top of a fixed floor.

### A note on measurement discipline

CPU time on Windows is noisy. Early runs ranged from 28 % to 69 % for the *same*
build, because a Vite dev server and a measurement browser tab were running. The
numbers above come from:

- release builds only,
- no dev server and no other webview processes,
- **interleaved A/B runs** of two otherwise identical binaries,
- 30–60 s windows, reporting the median and treating the minimum as the
  least-contaminated estimate.

If two runs of the same binary disagree by more than the effect you are measuring,
the measurement is the problem, not the code.

---

## 2. The decision

For a small, always-visible, mostly non-interactive surface, the webview is the
wrong engine. It is a general-purpose document renderer with a multi-process
architecture, and Dot needs a 180 px transparent bitmap.

The honest alternative was a native layered window. Its costs and guarantees:

| | WebView2 window | native layered window |
|---|---|---|
| runtime | ~300 MB, ~13 % floor | ~1 % total |
| GPU | compositor process | software raster; DWM composites on update |
| transparency | page background tricks | true per-pixel alpha |
| cross-platform | yes | Windows-only |
| reuse of the Vue app | full | none for the pet |

This is a trade that only makes sense for a surface like a pet. For a dashboard,
editor or form, the webview is the right engine and the work belongs inside it.

---

## 3. Prove it with a spike

Before porting 2,400 lines of engine, a throwaway binary answered one question:
*what does the cheapest possible native pet cost?*

- 180×180 winit-less Win32 window: `WS_EX_LAYERED | TOPMOST | TOOLWINDOW |
  NOACTIVATE`, `WS_POPUP`.
- A tiny-skia gradient circle, converted to premultiplied BGRA, blitted with
  `UpdateLayeredWindow`.
- Measured exactly like the app.

| | 60 fps | 15 fps | static |
|---|---|---|---|
| CPU | 0.47 % | 0.078 % | 0.000 % |
| memory | 6.9 MB | 6.3 MB | 6.3 MB |

The executable was **0.37 MB**. That was enough evidence to commit to the rewrite.

The spike lived in the temp directory and never entered the tree.

---

## 4. Port the engine without breaking it

The animation engine (`src/bot/`) is pure TypeScript: no DOM, no clock, no
randomness. `sample(t)` is a function of time alone. That makes it portable — and
testable — but a hand port still drifts.

### The contract

`tools/golden/dump.ts` generates two artefacts:

- **`src-tauri/tests/engine-golden.json`** — 142 cases, each a *replay script*
  (`init`, ordered `ops`, sample time `t`) plus the exact frame the TypeScript
  engine produced:
  - every state across its own duration,
  - plain and chained transitions,
  - gaze targets including spin and wander,
  - every catalogue shape and expression,
  - shape and expression morphs mid-flight,
  - a combined shape + expression + look case.

- **`src-tauri/assets/engine-catalogue.json`** — every measured table: radial
  profiles, shape radii, expressions, orbit seeds, the blink schedule, and the
  expensive `eyefit` solve.

The catalogue is the important trick. `eyefit.ts` builds a table by solving a
geometry problem once at import. Retyping or re-porting it would be a source of
silent drift, so the generator **dumps the solved table** and the Rust engine only
looks entries up. The same applies to the numerically measured profiles: they are
data, so they travel as data.

### The test

`src-tauri/tests/engine_parity.rs` replays all 142 scripts through the Rust
engine and compares every field:

- numbers with a 1e-6 tolerance,
- path and matrix strings by command skeleton plus numeric values at 1e-3,
- colours exactly.

Result: **142/142 pass, and 0 strings needed the relaxed numeric comparison.**
Every `d` and `matrix` came out byte-for-byte identical, because the port keeps
the same operation order and uses JS `Math.round` semantics in `r2`.

That test is the reason the two engines can coexist without lying to each other.
Change one and it fails until the other follows.

### Structure

```
src-tauri/src/engine/
  mod.rs        BotEngine, ids, catalogue, morph logic
  math.rs       clamp, lerp, easings, loopNoise, r2
  shape.rs      Silhouette, blend, toPoints, closedPath, radiusAtAngle
  face.rs       eyePoses, liveliness, blinkScale
  decor.rs      arcRender, wheel, particles
  states.rs     every pose(t)
  expressions.rs
```

The engine is still clock-free in Rust. `sample(now)` reads no system time; the
window owns the clock.

---

## 5. The native window

`src-tauri/src/pet.rs` runs a dedicated thread with its own Win32 message loop.
The Tauri event loop is untouched, which means the tray, config and settings
window keep working normally.

### Transparency

The window is `WS_EX_LAYERED` and has no background brush. Each frame is a
32-bit top-down DIB filled by tiny-skia, converted to premultiplied BGRA, and
presented with:

```rust
UpdateLayeredWindow(hwnd, null, null, &size, memdc, &src, 0, &blend, ULW_ALPHA)
```

Passing a null destination point is deliberate: it keeps the position the OS drag
gave the window instead of snapping it back every frame.

### Painting

`src-tauri/src/render.rs` reproduces `PetFigure.vue`'s order and the three styles
from `materials.ts` (`plat`, `argile`, `peluche`): body gradient or flat fill,
lower-edge shade, specular/sheen, eyes clipped to the body, back/front orbit arcs,
particles, notification badge. Gradients, strokes and clip masks are tiny-skia
primitives; the style vocabulary (`Surface`) is the same numbers on both sides.

tiny-skia has no SVG path parser, so the native side does not re-parse the
engine's strings. The engine exposes the structured geometry it already has
(`body_points`, eye size, arc polylines) alongside the strings the parity test
pins; the renderer builds paths from those.

### Behaviour

- **Hit testing**: point-in-polygon over `frame.body_points`, five probes for
  hysteresis, in viewBox units.
- **Click-through**: `WS_EX_TRANSPARENT` is toggled from that test, so transparent
  corners pass clicks to the desktop and the body takes them.
- **Drag**: `WM_LBUTTONDOWN` on the body sends `WM_NCLBUTTONDOWN`/`HTCAPTION`,
  the standard OS move loop.
- **Gaze**: `GetCursorPos` in the loop, normalised against the monitor centre.
- **Emotes**: the same scheduler and state list as the old webview view.
- **Config in place**: a version counter is bumped by `set_config`; the pet thread
  only takes the lock when it moved. Shape and expression morph, colour and style
  switch, with no reload and no flash.

---

## 6. Cut the loop's own cost

The first native build measured **2.55 %** — better than 14 %, but far worse than
the 0.08 % spike. The painting was not the problem; the loop was:

- `sleep(4 ms)` → 250 wake-ups per second,
- a mutex lock and two engine setters on every wake-up,
- `GetWindowRect` and gaze maths on every wake-up.

The fix was to make the loop sleep at the draw cadence and do the paperwork only
when it draws:

- ~60 fps while dragged/emoting/woken, ~15 fps at rest (`BUSY_MS`/`IDLE_MS`),
- config applied only when the atomic version counter changes,
- window rect, gaze and `set_look` inside the draw branch.

**2.55 % → ~1.1 %.** The remaining cost is mostly the real painting (gradients and
six stroked orbit arcs at 15 fps) plus the cursor poll. That is a reasonable place
to stop. Repeated 60–120 s runs land between 0.7 % and 1.6 %, because an emote runs
at 60 fps for a couple of seconds and the machine's background load moves; the
figure is a range, not a constant.

---

## 7. Results

One idle pet, release build, 40 s window, no other load:

| | WebView2 (start) | native (now) |
|---|---|---|
| CPU | ~14 % of one core | **~1.1 %** (0.7–1.6 % across repeated runs) |
| working set | 324 MB | **14.6 MB** |
| private | 137 MB | **3.3 MB** |
| processes | 7 | **1** |
| WebView2 processes | 6 | **0** |
| installer/binary | 9.3 MB | 9.78 MB (includes the settings UI) |

The pet still renders exactly as before — same shapes, expressions, styles,
orbits, emotes and click-through — which the screenshots and the parity suite
confirm.

---

## 8. Rules that made this work

1. **Measure the static floor.** A window that draws nothing still has a cost.
   If that floor is the runtime, page-side optimisations cannot beat it.
2. **Separate your code from the platform.** `dot.exe` was 21 MB and 0.5 %; the
   rest was the browser. Naming that precisely is what justifies the rewrite.
3. **Spike the riskiest assumption first.** One cheap binary answered "is a
   native window actually cheaper?" before any port began.
4. **Make ports verifiable, not careful.** Generate golden outputs from the
   reference implementation and replay them. "Byte-for-byte identical" is a much
   stronger guarantee than review.
5. **Dump data, don't retype it.** Measured tables and solved geometry are data;
   moving them as data removes a whole class of drift.
6. **The last percent is loop paperwork.** Cadence-aware sleeping and a version
   counter did more than any rendering change.
7. **Keep the safety net.** The parity suite is what lets the TypeScript and Rust
   engines coexist without silently diverging.

---

## 9. What is still owed

Being explicit about the trade-offs:

- **The settings preview and the pet use the same engine and style data, but not
  the same painter.** The preview asks Rust for its frames (`preview_frame`) and
  paints them with SVG; the desktop pet paints the same frames with tiny-skia. The
  styles are defined once (`SURFACE_RULES`), so the *data* cannot drift; the two
  painters are generic over that data and must stay behaviourally in step.
- **Drag, click-through and the live Settings→pet update** are implemented but
  were verified by hand, not automated. The rendering and idle cost were measured.
- **Multi-monitor gaze** uses the primary monitor; per-monitor DPI is a later
  phase.
- **Windows only.** The layered window and global cursor are Windows-shaped.

---

## 10. Reproducing

```powershell
# web tests and typecheck
cmd /c "pnpm test"
cmd /c "pnpm build"

# regenerate the parity contract (only after changing the TS engine)
cmd /c "pnpm exec vitest run --config vitest.golden.config.ts"

# Rust: config tests + 142 parity cases, then the style gates
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# release binary (custom-protocol embeds the settings frontend)
cargo build --release --manifest-path src-tauri/Cargo.toml --features custom-protocol
```

Then run `src-tauri/target/release/dot.exe` and watch one process, no WebView2,
and a pet that costs about one percent of a core to live on your desktop.