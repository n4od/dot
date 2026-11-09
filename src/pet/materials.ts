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
 *   2. `SURFACE_RULES` — add the rule: the tint targets and amounts for the three
 *      tones, plus the light/shade/sheen numbers. Derive every colour from `ink`
 *      so the style works for all twelve colours, including the near-black and
 *      near-white ends.
 *   3. `STYLE_NAMES` in `src/display.ts` — the English label.
 *   4. Regenerate the catalogue:
 *      `pnpm exec vitest run --config vitest.golden.config.ts`.
 *   5. Nothing else. The preview paints any `Surface`; `render.rs` reads the same
 *      rule from the catalogue, so the native pet needs no change.
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
  const rule = SURFACE_RULES[style]
  if (!rule) return null // `plat` draws no surface at all
  const tone = (x: Tone) => mixHex(ink, x.to, x.t)
  return {
    light: tone(rule.light),
    base: rule.base ? tone(rule.base) : ink,
    deep: tone(rule.deep),
    lx: rule.lx,
    ly: rule.ly,
    reach: rule.reach,
    mid: rule.mid,
    shade: rule.shade,
    gloss: rule.gloss,
    glossR: rule.glossR,
    eyeDepth: rule.eyeDepth,
    orbits: rule.orbits
  }
}

/**
 * A tint: `to` is mixed into the ink by `t`. Keeping the *rule* rather than the
 * resolved colours is what lets `render.rs` derive the exact same tones natively
 * from the same numbers.
 */
export interface Tone {
  to: string
  t: number
}

/** The numbers of a lit style, with tint targets instead of resolved hex. */
export interface SurfaceRule {
  light: Tone
  /** `null` means the pet's own ink is the mid tone. */
  base: Tone | null
  deep: Tone
  lx: number
  ly: number
  reach: number
  mid: number
  shade: number
  gloss: number
  glossR: number
  eyeDepth: number
  orbits: number
}

/**
 * The one definition of every lit style. `tools/golden/dump.ts` copies this into
 * `src-tauri/assets/engine-catalogue.json`, and `render.rs` reads it from there —
 * so the preview and the native pet cannot disagree about a style's numbers.
 */
export const SURFACE_RULES: Partial<Record<StyleId, SurfaceRule>> = {
  /** Firm and cool-lit: the object is solid, with a clear terminator and a specular. */
  argile: {
    light: { to: '#ffffff', t: 0.5 },
    base: null,
    deep: { to: '#000000', t: 0.34 },
    lx: -0.3,
    ly: -0.32,
    reach: 1.5,
    mid: 0.34,
    shade: 0.22,
    gloss: 0.55,
    glossR: 0.26,
    eyeDepth: 0.28,
    orbits: 0.9
  },
  /** Warm and diffuse: a broad light, a big soft sheen, and a gentle edge. */
  peluche: {
    light: { to: '#fff3e6', t: 0.3 },
    base: { to: '#ffffff', t: 0.04 },
    deep: { to: '#241a12', t: 0.15 },
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
