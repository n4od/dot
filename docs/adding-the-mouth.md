# Adding the mouth

How the mouth was added to Dot's copy of the engine, written so the same change can
be replayed on upstream [`jeremy-prt/bloub`](https://github.com/jeremy-prt/bloub)
@ `b4bb3c1` — or on any later copy of it.

`src/bot/` is upstream's engine, one documented fork deep. Everything below is the
fork, nothing else: if you are porting to a clean upstream checkout, these edits are
the whole job, and the twelve code files at the end are every one that changes.

The result: one closed path per frame, filled **and** stroked, chosen by numbers
that interpolate, so an expression can part a mouth without swapping shapes.

## The shape, in one paragraph

The lip is a quadratic from the left corner to the right corner, bowing by `curve`;
the opening is a second quadratic from the right corner back to the left, bowing by
`curve + open`; the path closes on itself. Filling *and* stroking that single path
with round caps gives three behaviours from one shape: at `open = 0` the two curves
coincide, the fill is empty and only the stroke shows — a rounded line; at `open > 0`
the fill is a lens under the lip, and the stroke rounds its corners, so an open mouth
is a **D** rather than a wedge; and with `curve < 0` the whole thing turns over into a
frown. No second shape, no branch in a painter, nothing that cannot be linearly
interpolated.

```
M x-hw y            lip, left corner
Q x  y + 2*curve    control under the middle, so the middle dips by `curve`
  x+hw y            ...to the right corner
Q x  y + 2*(curve+open)     the opening, back to the left corner
  x-hw y
Z
```

Screen Y grows downwards, so a positive `curve` dips the middle of the lip *below*
its corners, which reads as a smile; negative reads as a frown.

One more degree of freedom sits on the frame rather than on the shape: the whole path
is **canted to the eye line** (`rot`, in degrees). A rolled head tilts its eyes, so a
level mouth underneath reads as a smirk rather than as a cocked head — see step 4.

The whole path is then **canted to the eye line** (see step 4): a head that is rolled
tilts its eyes, and a level mouth under a canted face reads as a smirk rather than as
a cocked head.

## Where the numbers came from

The mouth was chosen before it was coded, in `tools/mouth-sheet.mjs` — a throwaway
contact sheet that draws real engine frames (body and eyes) with five candidate
mouths over them, across the eight silhouettes at 180 px, at 180/96/42 px, and in
three inks:

| candidate | family | shape |
| --- | --- | --- |
| `line` | lens | barely there; the neutral resting mouth |
| `smile` | lens | soft closed lens |
| `grin` | lens | **chosen**: wider, deeper, parted |
| `open` | lens | the open extreme, round |
| `cat` | two lobes | cute, but the first thing to blur when small |

All five are the same lens family, which is why the winner could be written as four
numbers rather than as a new kind of element. `cat` is the exception and was dropped
precisely because it is not.

The mock measured mouths against the eye capsule it had just read out of a frame —
`u = 2 · 9.3 · mean(|a|) ≈ 13.76` viewBox units — and gave each candidate fractions
of `u`. To put a candidate into the engine, divide by the ball radius (`RAYON`, 100):

```
w_engine   = spec.w     · u / 100
curve      = spec.curve · u / 100
thickness  = spec.t     · u / 100
open       = spec.open  · u / 100
```

`grin` (`w 2.6u, curve 0.5u, t 0.3u, open 0.45u`) therefore lands on the resting
values below. Re-run `node tools/mouth-sheet.mjs` if you want to revisit the choice —
it reads `src-tauri/tests/engine-golden.json` for its frames, so on upstream you would
point `faceOf`/`cell` at a dump of your own.

## The contract

Two types and one rule. (Code below is quoted from the shipped TS engine; the French
comments are trimmed where they are not the point.)

**`MouthCfg` rides on the pose** (`states.ts`), in the same unit as the eye
configs: fractions of the ball radius. It is what a state or an expression writes.

```ts
export interface MouthCfg {
  /** largeur du trait de levre, en unites de rayon de boule */
  w: number
  /** cambrure : positif = sourire, negatif = moue */
  curve: number
  /** epaisseur de la levre */
  thickness: number
  /** ouverture sous la levre : 0 = bouche fermee */
  open: number
  /** opacite : 0 retire la bouche */
  alpha: number
}

/** La bouche au repos : le sourire entrouvert retenu a la revue de design. */
export const REST_MOUTH: MouthCfg = {
  w: 0.358,
  curve: 0.069,
  thickness: 0.041,
  open: 0.062,
  alpha: 1
}

/** Une bouche : largeur, cambrure, ouverture, epaisseur. */
export const mouth = (w: number, curve: number, open = 0, thickness = 0.041): MouthCfg => ({
  w,
  curve,
  thickness,
  open,
  alpha: 1
})
```

**`MouthFrame` rides on the frame** (`engine.ts`), already converted to viewBox
units and already placed on the body. It is what a painter consumes, and the only
thing a painter ever sees.

```ts
export interface MouthFrame {
  x: number
  y: number
  /** de combien la bouche est inclinee, en degres, sur la ligne des yeux */
  rot: number
  w: number
  curve: number
  thickness: number
  open: number
  alpha: number
}
```

**The rule:** `alpha` is the pose's mouth opacity and *nothing else*. The painter
multiplies it by `frame.bodyAlpha` exactly where it multiplies the eyes' — in the
SVG painter by living inside the `<g :opacity="frame.bodyAlpha">` group, in the
native painter by an explicit multiply. Bake `bodyAlpha` into `MouthFrame.alpha` as
well and the mouth dims twice as fast as the eyes on every fade.

## The port, in order

### 1. `states.ts` — the type, the resting values, the helper

Add `MouthCfg`, `REST_MOUTH` and `mouth()` as above, add the field to `Pose`:

```ts
export interface Pose {
  // ...
  /** la bouche, meme unite que les yeux : fractions du rayon du corps */
  mouth: MouthCfg
  /** opacite des yeux : sert aux etats sans visage */
  eyeAlpha: number
  // ...
}
```

and default it in `base()`, next to the eyes, so every state that does not care
inherits the resting mouth:

```ts
function base(over: Partial<Pose> = {}): Pose {
  return {
    sil: circle(1),
    // ...
    eyes: pair(EYE_W, EYE_H),
    mouth: { ...REST_MOUTH },   // <- here
    eyeAlpha: 1,
    // ...
  }
}
```

Spread the constant; do not share the object. A state that mutates its pose in place
would otherwise edit the resting mouth for every other state.

### 2. `engine.ts` — the frame field

```ts
export interface BotFrame {
  bodyPath: string
  bodyAlpha: number
  eyes: RenderedEye[]
  mouth: MouthFrame | null   // <- here
  dots: DotRender[]
  // ...
}
```

`null` means "nothing to draw": the engine returns it once `mouth.alpha` falls below
`0.01`, so a painter can `v-if` it and skip the work entirely.

### 3. `engine.ts` — interpolation

```ts
const lerpMouth = (a: Pose['mouth'], b: Pose['mouth'], t: number) => ({
  w: lerp(a.w, b.w, t),
  curve: lerp(a.curve, b.curve, t),
  thickness: lerp(a.thickness, b.thickness, t),
  open: lerp(a.open, b.open, t),
  alpha: lerp(a.alpha, b.alpha, t)
})
```

and in `blendPose`, on the line after `eyes`:

```ts
eyes: [lerpEye(a.eyes[0], b.eyes[0], t), lerpEye(a.eyes[1], b.eyes[1], t)],
mouth: lerpMouth(a.mouth, b.mouth, t),
eyeAlpha: lerp(a.eyeAlpha, b.eyeAlpha, t),
```

Every field interpolates, so no pair of expressions can produce a pose the painter
cannot draw.

### 4. `engine.ts` — the anchor

This is the only part with a design decision in it. The mouth is placed from the eyes
the engine **actually put on the body**, not from the gaze angles:

```ts
// Ancrée sur les yeux que le moteur a réellement placés : elle tourne avec la
// tête, mais trois quarts moins loin que les yeux, et se pose à 36 % du chemin
// entre la ligne des yeux et le bord.
const pair = eyePoses(gaze, R, pose.split)
const e0 = pair[0]!
const e1 = pair[1]!
const f0 = bodyRadius(e0.x, e0.y)   // the same fit the eyes are drawn with
const f1 = bodyRadius(e1.x, e1.y)
const ax = e0.x * f0
const ay = e0.y * f0
const bx = e1.x * f1
const by = e1.y * f1
const mx = (ax + bx) / 2
const my = (ay + by) / 2
// La pente de la ligne des yeux. Une tête qui roule incline ses yeux, donc la
// bouche doit s'incliner avec eux : à plat sous un visage penché, elle se lit
// comme un sourire en coin.
let rot = (Math.atan2(by - ay, bx - ax) * 180) / Math.PI
if (rot > 90) rot -= 180
else if (rot <= -90) rot += 180
const mouth =
  pose.mouth.alpha > 0.01
    ? {
        x: mx * 0.75 + (offX + decalage.x) * R,
        y: my + 0.36 * (R - my) + (offY + decalage.y) * R,
        rot,
        w: pose.mouth.w * R,
        curve: pose.mouth.curve * R,
        thickness: pose.mouth.thickness * R,
        open: pose.mouth.open * R,
        alpha: pose.mouth.alpha
      }
    : null
```

Four things are load-bearing:

- **the fit.** Eyes live on a unit sphere and are pulled onto the actual silhouette
  by `bodyRadius`. The mouth must be built from the *fitted* eye positions, or on
  non-circular silhouettes it drifts away from the face.
- **`0.75`.** The mouth follows the head's turn, but less than the eyes do — a mouth
  sits lower on the face, nearer the axis of rotation. At `1.0` it swings out with
  the eyes and separates from them on a steep gaze; lower and it stops turning.
- **`0.36`.** The mouth sits at 36 % of the way from the eye line down to the rim.
  Both constants held on all eight silhouettes, which is why there is **no per-shape
  table** — and why adding a shape does not require touching the mouth.
- **`rot`.** The eye line's own slope, taken from the two *fitted* eye centres and
  kept within a half turn. The head's roll is not part of the look (`Look` has yaw,
  pitch, spin and wander, no roll), so an expression with a roll — `neutre` is −13° —
  looks at you with its head canted. Place the mouth level under those eyes and it
  reads as a smirk; cant it with them and the whole face reads as a cocked head.
  Wrapped into `(−90, 90]` so a gaze that sends the eyes round the sphere cannot turn
  the mouth upside down.

Then add `mouth` to the returned frame, and keep the body offsets (`offX`, `offY`,
`decalage`) applied exactly as the eyes apply them.

### 5. `engine.ts` — who is allowed to override it

The mouth follows the expression wherever the expression's *face* is applied — the
same flag that already decides for `gaze`, `split` and `eyes`:

```ts
if (def.baseFace && expr) {
  pose = { ...pose, gaze: expr.gaze, split: expr.split, eyes: expr.eyes, mouth: expr.mouth }
}
```

States that own their face (`baseFace: false`) keep whatever their own pose says.
That single condition is the whole policy: `idle` and `swirl` wear the user's
expression, `wink`/`wide`/`notify`/`egg`/`hexagon`/`play`/`orbit` wear theirs.

### 6. `expressions.ts` — sixteen mouths

Add the field to `BotExpression`, lerp it in `blendExpression`:

```ts
mouth: lerpMouthCfg(a.mouth, b.mouth, t)
```

and give every expression one. Read as: width, bow (positive = smile), opening,
lip thickness. The last column is the default and is omitted in the source.

| id | w | curve | open | thickness |
| --- | --- | --- | --- | --- |
| `neutre` | 0.358 | 0.069 | 0.062 | 0.041 |
| `attentif` | 0.3 | 0.048 | 0.018 | 0.041 |
| `surpris` | 0.155 | 0.012 | 0.17 | 0.042 |
| `excite` | 0.375 | 0.072 | 0.13 | 0.041 |
| `heureux` | 0.345 | 0.062 | 0.045 | 0.041 |
| `hilare` | 0.4 | 0.082 | 0.155 | 0.046 |
| `colere` | 0.3 | −0.048 | 0 | 0.046 |
| `triste` | 0.245 | −0.036 | 0 | 0.041 |
| `effraye` | 0.17 | 0 | 0.2 | 0.042 |
| `mefiant` | 0.265 | −0.012 | 0 | 0.04 |
| `confus` | 0.225 | 0.012 | 0.03 | 0.04 |
| `curieux` | 0.15 | 0.008 | 0.125 | 0.041 |
| `fier` | 0.335 | 0.042 | 0.02 | 0.043 |
| `timide` | 0.195 | 0.036 | 0 | 0.038 |
| `blase` | 0.3 | −0.006 | 0 | 0.036 |
| `somnolent` | 0.185 | 0.008 | 0.055 | 0.042 |

The vocabulary the eye can read, in three rules: **sign** of `curve` is the mood
(down for the sad and the angry, up for everyone happy); **`open`** is arousal
(round and open for surprise and fear, closed for the flat expressions); **`w`** is
volume (widest for `hilare` at 0.4, narrowest for `curieux` at 0.15). Every
expression is a perturbation of the resting grin, never a different kind of object.

### 7. `states.ts` — the states that need one

States that own their face inherit `REST_MOUTH` from `base()` and are already right.
Four have no face at all — no eyes *and* no mouth — and three need an explicit value:

| state | mouth |
| --- | --- |
| `thinking` | `{ ...REST_MOUTH, alpha: 0 }` — the body *is* a dot |
| `alert` | `{ ...REST_MOUTH, alpha: 0 }` — the body *is* the `!` |
| `exclaim` | `{ ...REST_MOUTH, alpha: 0 }` |
| `sleep` | `{ ...REST_MOUTH, alpha: 0 }` — a bare breathing ball |
| `wide` | `mouth(0.165, 0.012, 0.18, 0.042)` — startled, small and round |
| `burst` | `{ ...REST_MOUTH, alpha: clamp((t - 1.85) / 0.4) }` |
| `comet` | `{ ...REST_MOUTH, alpha: clamp((t - 2) / 0.35) }` |

`burst` and `comet` fade their eyes back in on a ramp; their mouth has to ride the
**same expression**, or the mouth appears over a faceless body a beat before the eyes.
If you add a state that hides the eyes, hide the mouth with it — that is the one
invariant a reader of `states.ts` has to keep in mind.

### 8. The painter

Two lines of geometry, and it is the same two lines in either painter.

```ts
const mouthPath = computed(() => {
  const m = props.frame.mouth
  if (!m) return ''
  const hw = m.w / 2
  const top = `Q ${m.x} ${m.y + 2 * m.curve} ${m.x + hw} ${m.y}`
  const back = `Q ${m.x} ${m.y + 2 * (m.curve + m.open)} ${m.x - hw} ${m.y}`
  return `M ${m.x - hw} ${m.y} ${top} ${back} Z`
})
```

Fill **and** stroke it, with round caps and joins, and **rotate it about its own
centre** — the numbers are already viewBox units, and the rotation keeps the anchor
exactly where the engine put it:

```html
<path
  v-if="frame.mouth"
  :d="mouthPath"
  :opacity="frame.mouth.alpha"
  :transform="`rotate(${frame.mouth.rot} ${frame.mouth.x} ${frame.mouth.y})`"
  fill="..."
  stroke="..."
  :stroke-width="frame.mouth.thickness"
  stroke-linecap="round"
  stroke-linejoin="round"
/>
```

Fill alone gives a pointed lens; stroke alone loses the opening; both together is what
makes the D. Do **not** reach for `vector-effect="non-scaling-stroke"`: it would make
the lip a constant number of *screen* pixels, so the mouth would stop matching the
eyes at every size but one.

#### In upstream `BloubBot.vue`, the ink inverts

Upstream punches the face out of the body with an SVG `<mask>`: the body path is
white (opaque), and every eye and the notch are black (holes). Dot does the opposite
— it fills the eyes with the contrast colour and clips them to the body — because a
layered window has no paper behind it and a hole would show the wallpaper.

The mouth geometry is identical; only the ink differs. On upstream the mouth is a
**black element inside that same mask**, next to the eyes:

```html
<path
  v-if="frame.mouth"
  :d="mouthPath"
  :transform="`rotate(${frame.mouth.rot} ${frame.mouth.x} ${frame.mouth.y})`"
  fill="#000"
  stroke="#000"
  :stroke-width="frame.mouth.thickness"
  stroke-linecap="round"
  stroke-linejoin="round"
/>
```

Do not add a separate white element over the body: the mouth would sit on top rather
than cut in, and it would lose the mask's edge behaviour. One mouth, one element, one
meaning.

## Traps

Each of these was paid for once.

- **`open = 0` is a degenerate path.** The fill contributes nothing (the winding is
  zero area) and only the stroke draws. That is intended, but it means a painter that
  fills *or* strokes — not both — renders a different mouth at rest. Keep the two
  calls adjacent and unconditional.
- **`alpha` is not `bodyAlpha`.** See the contract above; this is the one bug that
  looks like a style choice.
- **The stroke width is in viewBox units.** Forgetting that (or adding
  `non-scaling-stroke`) silently decouples the lip from the eyes.
- **Clip the mouth to the body.** The mouth is placed by formula, not fitted inside
  the silhouette, so an extreme one (`hilare`, `effraye`) can reach the rim of a short
  body. Both painters clip to the body — in Dot the mouth is drawn inside the same
  clip group as the eyes — and that clip is what makes the formula safe on every
  shape.
- **Anchor from the placed eyes, never from the gaze.** `eyePoses` plus the
  silhouette fit — see step 4.
- **Place it *and* orient it.** The mouth is anchored to the eye line, so it also has
  to be canted to it. `Look` has no roll, so an expression with a roll faces you with
  its head canted; a level mouth under those eyes reads as a smirk, and it is the
  mouth — not the eyes — that people blame. Rotate about the mouth's own centre: the
  anchor is a hand-tuned formula and rotating about the eye midpoint would move it.
- **Do not scale the lip up at small sizes.** The tempting "minimum 1 px thickness"
  fix breaks the relationship with the eyes; in Dot the parted resting mouth is what
  survives at 42 px (the settings chip), and nothing survives at 24 px. Small sizes
  are solved by *the mouth being open*, not by hinting the stroke.
- **Do not add a per-shape table.** `0.75` and `0.36` held on all eight silhouettes;
  a table would be eight numbers to keep honest against a formula that already works.
- **A new faceless state needs `alpha: 0`.** Nothing catches this for you: the mouth
  would simply linger on a body that has no eyes.

## Switching it off (Dot only)

Settings can turn the mouth off, and the switch is one gate in the engine rather
than a painter flag — the frame is the only place a fourth painter cannot forget.

```rust
// engine/mod.rs
pub fn set_mouth(&mut self, on: bool) { self.mouth = on; }

// engine/mod.rs, in sample()
let mouth = if self.mouth && p.mouth.alpha > 0.01 { Some(MouthFrame { .. }) } else { None };
```

`false` therefore produces exactly what a faceless state produces — `mouth: null` —
so every state, the settings stage and the sixteen expression thumbnails agree by
construction, and no painter needs to know the setting exists. The TS side is the
same flag on `BotEngine` (`private mouth = true` / `setMouth`).

Three things to get right when wiring it up:

- **the default is on, and `bool`'s serde default is `false`.** `Config` carries
  `#[serde(default)]` at the *struct* level with a `Default` impl, so put
  `mouth: true` there. A `#[serde(default)]` on the field would quietly take the
  mouth off every pet that existed before the field did — pin it with a test that
  parses `{}` and a file from before the field.
- **the same trap in `PreviewInit`**, which comes from the frontend: use
  `#[serde(default = "yes")]`, not `default`.
- **the thumbnails have to follow.** They are frames from the same engine, so pass
  the flag into the preview init and repaint them when it changes — a bench that
  keeps grinning while the pet cannot is a lie about the pet.
- and one piece of painter-side housekeeping that is easy to miss: the settings
  chips cancel the expression's head turn by shifting the eye matrices
  (`straightAhead`). The mouth is placed from those eyes, so it has to be shifted by
  the same amount, or every thumbnail draws its mouth a few units off its face.

## Porting the Rust twin (Dot only)

Upstream has one engine, so the port ends at step 8. Dot runs the engine twice and
the parity suite is what keeps them honest, so a Rust mirror is owed — the same
edits, one for one:

| TS | Rust |
| --- | --- |
| `MouthCfg`, `REST_MOUTH`, `mouth()`, `Pose.mouth` | `states.rs`: `MouthCfg` (+ `Deserialize`, `Default = REST`, `with_alpha`), `mouth()`, `Pose.mouth` |
| `lerpMouth`, `blendPose` | `states.rs::lerp_mouth_cfg`, `mod.rs::blend_pose` |
| `MouthFrame`, the anchor | `mod.rs`: `MouthFrame` (serialized, unlike `body_points`) and the anchor block in `sample()` |
| `expressions.ts` values | the catalogue JSON (`tools/golden/dump.ts` emits them; `ExprDef.mouth` is `#[serde(default)]` so an older catalogue still loads) |
| `posed()` (`baseFace`) | `mod.rs::posed()` |
| the painter | `render.rs::draw_mouth` |

Rust-specific differences, both mechanical:

- **`tiny_skia` wants `f32`.** `PathBuilder::move_to`/`quad_to` and `Stroke::width`
  take `f32`; cast once at the top of `draw_mouth` rather than letting `f64` leak
  into the calls.
- **`tiny_skia` has no CSS.** Fill and stroke are two calls on the same `Path`: draw
  with `fill_path`, then `path.stroke(&stroke, 1.0)` and `fill_path` the outline, both
  passing the body `Mask`. `LineCap::Round` and `LineJoin::Round` are the SVG
  attributes.

Then regenerate and let the suite judge: `pnpm exec vitest run --config
vitest.golden.config.ts` rewrites the golden frames and the catalogue from the TS
engine, and `cargo test --manifest-path src-tauri` replays all 142 cases against the
Rust one. Change one side only and it fails — that is the point.

## Seeing it

In Dot:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --example faces-sheet
```

writes every silhouette at 512/180/96/42/24 px, the sixteen expressions and the twelve
states into `%TEMP%\dots-mouth\`, painted by the shipped native painter in both
contrast directions. It renders every expression **twice**: at rest, and in the pose
the desktop actually wears (a cursor on the pet is `yaw 0, pitch +10, mix 1`). Judge a
face on both — the look is the only pose where the tracking overrides the expression's
yaw and pitch, and the only one where a roll has nothing to hide behind.

`tools/shot-pet.ps1` captures the real layered window instead, if you want to see what
the desktop actually shows. Between them they are how the mouth was judged — the
numbers above are the ones that survived looking at them.

## Files touched

Sixteen files in Dot: seven engine files (three TypeScript, four Rust), one painter,
one generator, two generated artefacts, one contact sheet, and four for the switch
(`lib.rs`, `pet.rs`, the settings view and the TS contract test).

| file | what |
| --- | --- |
| `src/bot/states.ts` | `MouthCfg`, `REST_MOUTH`, `mouth()`, `Pose.mouth`, `base()`, the state overrides |
| `src/bot/engine.ts` | `MouthFrame`, `BotFrame.mouth`, `lerpMouth`, `blendPose`, the anchor, `posed()` |
| `src/bot/expressions.ts` | `BotExpression.mouth`, the sixteen values, `blendExpression` |
| `src/pet/PetFigure.vue` | `mouthPath`, the clipped `<path>` |
| `src-tauri/src/engine/states.rs` | the Rust `MouthCfg`, `lerp_mouth_cfg`, `pose()` overrides |
| `src-tauri/src/engine/mod.rs` | `MouthFrame`, `BotFrame.mouth`, `blend_pose`, the anchor, `posed()`, `ExprDef.mouth` |
| `src-tauri/src/engine/expressions.rs` | `Expression.mouth`, `blend_expression` |
| `src-tauri/src/render.rs` | `draw_mouth` |
| `tools/golden/dump.ts` | emits each expression's mouth into the catalogue |
| `src-tauri/assets/engine-catalogue.json` | regenerated |
| `src-tauri/tests/engine-golden.json` | regenerated (142 frames) |
| `src-tauri/examples/faces-sheet.rs` | the contact sheet, for looking |
| `src-tauri/src/lib.rs` | `Config.mouth`, `PreviewInit.mouth` |
| `src-tauri/src/pet.rs` | `PetConfig.mouth`, applied in place with the rest of the config |
| `src/settings/SettingsView.vue` | the `Mouth` row, the preview init, `straightAhead` moves the mouth too |
| `src/engine.test.ts` | the switch is on by default, and off through a state with its own face |
