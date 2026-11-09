import { describe, expect, it } from 'vitest'
import { EXPRESSIONS } from './bot/expressions'
import { COLORS, SHAPES } from './bot/skins'
import { STYLE_IDS } from './pet/materials'
import { COLOR_NAMES, EXPRESSION_NAMES, SHAPE_NAMES, STYLE_NAMES, displayName } from './display'

/*
 * The picker shows `display.ts`, the config stores ids. If a catalogue grows
 * without a label, the interface falls back to a French id; if a label outlives
 * its id, the map is carrying a lie. Both are cheap to catch here.
 */

describe('displayName', () => {
  it('falls back to the id, so a hand-edited config still shows something', () => {
    expect(displayName(SHAPE_NAMES, 'nope')).toBe('nope')
    expect(displayName({}, 'argile')).toBe('argile')
  })
})

describe('the label maps', () => {
  it('cover every id the engine and the styles can produce', () => {
    for (const shape of SHAPES) expect(SHAPE_NAMES[shape.id]).toBeTruthy()
    for (const color of COLORS) expect(COLOR_NAMES[color.id]).toBeTruthy()
    for (const expression of EXPRESSIONS) expect(EXPRESSION_NAMES[expression.id]).toBeTruthy()
    for (const style of STYLE_IDS) expect(STYLE_NAMES[style]).toBeTruthy()
  })

  it('carry no label for an id that no longer exists', () => {
    const known = new Set<string>([
      ...SHAPES.map((shape) => shape.id),
      ...COLORS.map((color) => color.id),
      ...EXPRESSIONS.map((expression) => expression.id),
      ...STYLE_IDS
    ])
    const labelled = [
      ...Object.keys(SHAPE_NAMES),
      ...Object.keys(COLOR_NAMES),
      ...Object.keys(EXPRESSION_NAMES),
      ...Object.keys(STYLE_NAMES)
    ]
    for (const id of labelled) expect(known.has(id)).toBe(true)
  })
})
