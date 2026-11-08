import { mixHex } from '../bot/skins'

/*
 * HOW A PET IS PAINTED — the style axis.
 *
 * The engine hands over geometry — a body path, eyes, orbits, particles — and
 * knows nothing about fill. A "style" is a way of spending that geometry:
 *
 *   plat    one flat hex per shape, the way the upstream web demo does it.
 *   argile  a lit surface: a highlight towards the light, a shaded edge turning
 *           away, a specular, and eyes that read as pressed in.
 *
 * This module owns the *definitions* (which styles exist, the ids, and the
 * colours a style derives from the ink). `src/pet/PetFigure.vue` owns the
 * *drawing*: it branches on the id and emits the SVG. Keep the split: nothing
 * here draws, and PetFigure never hard-codes a colour a style could compute.
 *
 * ------------------------------------------------------------------ ADDING A STYLE
 *
 *   1. `StyleId` and `STYLE_IDS` below — add the id. These are storage keys:
 *      `config.json` writes them, so NEVER rename or reorder an existing one,
 *      only add. `argile` is French to match the engine's other ids; a new one
 *      should follow suit.
 *   2. Add a paint function here (`glassMaterial(ink)`, ...). Take the ink hex
 *      and return the colours the drawing needs. Derive everything from `ink`
 *      so the style works for all twelve colours, including the near-black and
 *      near-white ends.
 *   3. `STYLE_NAMES` in `src/display.ts` — the English label.
 *   4. `src/pet/PetFigure.vue` — a `computed` flag plus the draw branch. This is
 *      the only file that grows per style.
 *   5. Nothing in Rust. `Config` accepts any string, `pet_query` passes it
 *      through, the settings pill is generated from `STYLE_IDS`, and the pet
 *      resolves an unknown id back to `plat`. Touch Rust only to change the
 *      *default* style.
 *
 * A style must respect the constraints the desktop window imposes:
 *
 *   - Fills and gradients only. A per-frame SVG filter (`feGaussianBlur`, ...)
 *     multiplies by however many elements are painted, every animation frame.
 *   - No opaque backdrop. The window is transparent; the body is opaque and
 *     everything behind it is genuinely behind it (see README).
 *   - Keep `.hit` on the body path and the eye group. `interactive` puts it
 *     there and the click-through test is `closest('.hit')`.
 *   - Eyes stay *filled* (a mask hole would show the wallpaper and the orbits)
 *     and take their contrast from `src/contrast.ts`. Do not add a second
 *     contrast rule.
 *   - It must read at 42 px (the expression thumbnails, `bare` + `zoom`) as well
 *     as at 180 px, and on irregular silhouettes, not just the circle.
 */

export type StyleId = 'plat' | 'argile'

export const STYLE_IDS: StyleId[] = ['plat', 'argile']

export const DEFAULT_STYLE: StyleId = 'plat'

export function isStyleId(value: string | null | undefined): value is StyleId {
  return value === 'plat' || value === 'argile'
}

export interface ClayMaterial {
  /** Facing the light, top-left. */
  light: string
  /** The pet's own colour, at the mid tone. */
  base: string
  /** Turning away from the light. */
  deep: string
}

/**
 * The light comes from the top-left, the conventional read for a lit object and
 * the one that agrees with the stage's own gradient in the settings window.
 */
export function clayMaterial(ink: string): ClayMaterial {
  return {
    light: mixHex(ink, '#ffffff', 0.5),
    base: ink,
    deep: mixHex(ink, '#000000', 0.34)
  }
}

/**
 * The top of an eye sits in the shadow of its own rim, so it is a touch darker
 * than the fill below it. Toward black rather than toward the ink, so the shadow
 * reads the same on a dark pet's light eyes and a light pet's dark ones.
 */
export function clayEyeLid(eyeFill: string): string {
  return mixHex(eyeFill, '#000000', 0.28)
}
