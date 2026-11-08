import { describe, expect, it } from 'vitest'
import { DEFAULT_STYLE, STYLE_IDS, eyeShade, isStyleId, surfaceFor } from './materials'

/*
 * These guard the style axis as a *storage key*. `config.json` writes these ids,
 * so the rules that matter are: the known ones are accepted, an unknown one
 * falls back rather than throwing, and the default is still flat. They are the
 * only rules a third style could quietly break.
 */

const INK = '#3b93f0'

/** Perceived brightness of a `#rrggbb`, enough to compare two tones. */
function luminance(hex: string): number {
  const n = parseInt(hex.slice(1), 16)
  return 0.2126 * ((n >> 16) & 0xff) + 0.7152 * ((n >> 8) & 0xff) + 0.0722 * (n & 0xff)
}

describe('isStyleId', () => {
  it('accepts every id in STYLE_IDS', () => {
    for (const id of STYLE_IDS) expect(isStyleId(id)).toBe(true)
  })

  it('rejects anything else, including null and the empty string', () => {
    for (const value of ['', 'clay', 'ARGILE', 'plat ', null, undefined, 'glass']) {
      expect(isStyleId(value)).toBe(false)
    }
  })
})

describe('surfaceFor', () => {
  it('draws no surface for the flat default', () => {
    expect(surfaceFor('plat', INK)).toBeNull()
  })

  it('gives every lit style a highlight, a mid tone and a shadow, in that order', () => {
    for (const id of STYLE_IDS.filter((style) => style !== 'plat')) {
      const surface = surfaceFor(id, INK)
      expect(surface).not.toBeNull()
      for (const tone of [surface!.light, surface!.base, surface!.deep]) {
        expect(tone).toMatch(/^#[0-9a-f]{6}$/)
      }
      // The whole point of a surface: the light side is brighter than the mid
      // tone, which is brighter than the side turning away.
      expect(luminance(surface!.light)).toBeGreaterThan(luminance(surface!.base))
      expect(luminance(surface!.base)).toBeGreaterThan(luminance(surface!.deep))
    }
  })
})

describe('the default', () => {
  it('is flat, so a config file written before styles existed opens unchanged', () => {
    expect(DEFAULT_STYLE).toBe('plat')
    expect(STYLE_IDS[0]).toBe('plat')
  })
})

describe('eyeShade', () => {
  it('leaves the fill alone when there is no depth', () => {
    expect(eyeShade('#fdfdfd', 0)).toBe('#fdfdfd')
  })

  it('darkens the top of a pressed-in eye', () => {
    expect(eyeShade('#ffffff', 0.28)).toBe('#b8b8b8')
  })
})