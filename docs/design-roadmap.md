# Pet design roadmap — more movements, more things

Ideas for growing the pet's *appearance and behaviour*. This is deliberately **not**
part of [ROADMAP.md](ROADMAP.md), which tracks the app; this file tracks the pet's
design. Ticked items have shipped; the rest are ideas, not commitments.

The cost of any idea comes down to one question: does it **reuse the engine's
vocabulary**, or does it introduce a **new kind of element**?

## What constrains this

- **The engine exists twice.** `src/bot/` is the TypeScript reference;
  `src-tauri/src/engine/` is the Rust port the pet runs. A 142-case golden-frame
  parity suite (`tools/golden/dump.ts` → `src-tauri/tests/engine_parity.rs`) joins
  them, so changing one fails `cargo test` until the other follows. That is the
  safety net, not an obstacle.
- **The drawing exists twice.** `src/pet/PetFigure.vue` paints the frame as SVG
  (settings preview + browser); `src-tauri/src/render.rs` paints it with tiny-skia
  (the desktop pet). Reusing an existing element *kind* is free; adding a new kind
  means a branch in both.
- **The vocabulary today:** one silhouette, two eye capsules, `dots` (particles,
  which may carry any SVG path via `DotRender.d`), `arcs` (orbits — rings, swooshes,
  comets), and `notif`/`notch`. Plus the per-style `Surface` (a style is data).

Where things live:

| | files |
|---|---|
| Movements (states) | `src/bot/states.ts` → `src-tauri/src/engine/states.rs` |
| The frame contract | `BotFrame` in `src/bot/engine.ts` / `src-tauri/src/engine/mod.rs` |
| Painting | `src/pet/PetFigure.vue` (SVG) and `src-tauri/src/render.rs` (tiny-skia) |
| Spontaneous emotes | `EMOTES` in `src-tauri/src/pet.rs` |
| Shapes / colours | `src/bot/skins.ts` (+ the Rust catalogue) |
| Golden contract | `tools/golden/dump.ts` |

## Phase A — Reactions (behaviours on existing states)

Cheapest, and the most personality per hour: no new geometry, only `pet.rs` firing
a state that already exists in response to input.

- [x] A startled look (`Wide`) when a drag puts the pet down. The window
      procedure compares the window rect across the OS move loop, so a stray click
      that did not move the pet stays a click.
- [x] Sleep (`Sleep`) after `SLEEP_AFTER_MS` (2 min) with no cursor in the pet's
      rect; the pose breathes while held, and the cursor entering the rect or a
      click wakes it back to `Idle`.
- [ ] A "peek" when the cursor approaches, a shy recoil when it is close.
- [ ] A yawn or a stretch on waking.
- [ ] Double-click already winks; more gestures (hover, right-click, a second
      double).

## Phase B — New movements (states)

Write a `pose(t)` in `states.ts`, port it to `states.rs`, regenerate the golden, and
add it to `EMOTES` (and `SEQUENCE`/`POSES` if it should play in the montage).

- Yawn, sneeze, hiccup, stretch, shiver.
- A spin or a somersault; a bounce on landing.
- A "?" or "…" thinking bubble; a heart; a small celebration.

Uses the existing vocabulary, so no painter changes — but both engines and a
regenerate.

## Phase C — New particles and orbits (within the vocabulary)

`DotRender.d` already accepts an arbitrary path (that is how the `!` tear works),
and `ArcSeed` covers orbits. So these are mostly data:

- Sparkles or a twinkle burst; a heart or star particle.
- A new orbit configuration (more rings, a different sweep) as a state's decor.

## Phase D — New kinds of things (the real feature work)

An accessory (hat, scarf, glasses), an outline, a drop shadow, or *tinted* orbits.
Each needs `Pose`/`BotFrame` extended in **both** engines and a branch in **both**
painters. This is the branch the style seam avoids on purpose — and why the extra
styles ended up looking similar.

- [x] **A mouth.** The first element built through that seam, and the template for
      the rest: `MouthCfg` on the pose (both engines, interpolated), one `MouthFrame`
      in the contract, one `draw_mouth` per painter. It is data before it is a
      branch — width, bow, lip thickness and opening — so every expression and
      emote moves the mouth by numbers alone. Designed in `tools/mouth-sheet.mjs`:
      five candidates, eight silhouettes, three sizes, and `line`/`grin`/`open` won.
- [ ] An accessory that hangs off the same anchor (glasses, a scarf).
- [ ] An outline or drop shadow, which the style seam cannot express (it paints
      fills, not silhouettes).

## Phase E — New shapes and colours

- **Colour:** one row in each catalogue; trivial.
- **Shape:** radial geometry in `skins.ts` mirrored in the Rust catalogue, *and* the
  `eyefit` decalage table regenerated (it is solved per shape). Not a one-liner.

## Phase F — Screen behaviour

The engine morphs in place. Walking, gravity, edge-bouncing or wandering across
monitors means moving the *window* in `pet.rs` alongside new states — more behaviour
than art, and the largest of these.

## Suggested order

**A → B → C.** Those three deliver the most visible change for the least risk and
touch neither the frame contract nor the painters. Do **D** or **E** only for a
deliberate single addition; leave **F** for last.
