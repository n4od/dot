import { mixHex } from '../bot/skins'

/*
 * HOW A PET IS PAINTED — the style axis.
 *
 * The engine hands over geometry — a body path, eyes, orbits, particles — and
 * knows nothing about fill. A "style" is a way of spending that geometry:
 *
 *   plat     one flat hex per shape, the way the upstream web demo does it.
 *   argile   a lit surface: a firm highlight, a specular, a shaded edge, and eyes
 *            that read as pressed in. Reads as a solid, slightly glossy object.
 *   peluche  the same idea, warm and soft: a broad diffuse light, a whisper of a
 *            sheen, a gentle edge and barely-recessed eyes. Reads as a toy.
 *
 * A style is DATA, not code. `Surface` below is the whole vocabulary — the three
 * tones, where the light sits, how far it reaches, how strong the shade, the
 * sheen and the eye recession are. `surfaceFor` maps a style id to one of those.
 * `src/pet/PetFigure.vue` paints whatever `Surface` it is handed, so adding a
 * style of this kind touches no drawing code at all.
 *
 * ------------------------------------------------------------------ ADDING A STYLE
 *
 *   1. `StyleId` and `STYLE_IDS` below — add the id. These are storage keys:
 *      `config.json` writes them, so NEVER rename or reorder an existing one,
 *      only add. The ids are French to match the engine's others.
 *   2. `surfaceFor` — return a `Surface`. Derive every colour from `ink` so the
 *      style works for all twelve colours, including the near-black and
 *      near-white ends.
 *   3. `STYLE_NAMES` in `src/display.ts` — the English label.
 *   4. Nothing else. `PetFigure.vue` paints any `Surface`; it needs a change only
 *      when a style introduces a *new kind of element* — an outline, a second
 *      silhouette, a tinted orbit — rather than new numbers.
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

export type StyleId = 'plat' | 'argile' | 'peluche'

export const STYLE_IDS: StyleId[] = ['plat', 'argile', 'peluche']

export const DEFAULT_STYLE: StyleId = 'plat'

export function isStyleId(value: string | null | undefined): value is StyleId {
  return value === 'plat' || value === 'argile' || value === 'peluche'
}

/**
 * A lit surface. Every field is a number or a colour, so the drawing code never
 * learns a style's name — it just paints these.
 */
export interface Surface {
  /** Facing the light. */
  light: string
  /** The pet's own colour, at the mid tone. */
  base: string
  /** Turning away from the light. */
  deep: string
  /** Light position, in radius units (negative = up and left). */
  lx: number
  ly: number
  /** How far the gradient reaches, in radii. Larger = softer falloff. */
  reach: number
  /** Where `base` takes over from `light`, as a fraction of `reach`. */
  mid: number
  /** Opacity of the lower-edge shade wash. */
  shade: number
  /** Peak opacity of the sheen; 0 is perfectly matte. */
  gloss: number
  /** Sheen size, in radii. A small one is a specular; a large one is a soft sheen. */
  glossR: number
  /** How deeply the eyes read as pressed into the surface, 0 to 1. */
  eyeDepth: number
  /** Orbits are dimmed to this while a surface is on, so they do not fight it. */
  orbits: number
}

export function surfaceFor(style: StyleId, ink: string): Surface | null {
  if (style === 'argile') return clay(ink)
  if (style === 'peluche') return plush(ink)
  return null // `plat` draws no surface at all
}

/** Firm and cool-lit: the object is solid, with a clear terminator and a specular. */
function clay(ink: string): Surface {
  return {
    light: mixHex(ink, '#ffffff', 0.5),
    base: ink,
    deep: mixHex(ink, '#000000', 0.34),
    lx: -0.3,
    ly: -0.32,
    reach: 1.5,
    mid: 0.34,
    shade: 0.22,
    gloss: 0.55,
    glossR: 0.26,
    eyeDepth: 0.28,
    orbits: 0.9
  }
}

/** Warm and diffuse: a broad light, a big soft sheen, and a gentle edge. */
function plush(ink: string): Surface {
  return {
    light: mixHex(ink, '#fff3e6', 0.3),
    base: mixHex(ink, '#ffffff', 0.04),
    deep: mixHex(ink, '#241a12', 0.15),
    lx: -0.2,
    ly: -0.28,
    reach: 2,
    mid: 0.45,
    shade: 0.1,
    gloss: 0.18,
    glossR: 0.5,
    eyeDepth: 0.12,
    orbits: 0.85
  }
}

/**
 * The top of an eye sits in the shadow of its own rim, so it is a touch darker
 * than the fill below it. Toward black rather than toward the ink, so the shadow
 * reads the same on a dark pet's light eyes and a light pet's dark ones.
 */
export function eyeShade(eyeFill: string, depth: number): string {
  return depth <= 0 ? eyeFill : mixHex(eyeFill, '#000000', depth)
}
