import { describe, expect, it } from 'vitest'
import { eyeFillFor } from './contrast'

/*
 * The eye fill is what makes the pet legible on a transparent window: there is no
 * page colour behind a hole, so the eyes are painted, and that paint has to
 * contrast with the ink. `contrast.ts` is shared by the pet and the settings
 * preview, so this is the one place the rule can be pinned.
 */

describe('eyeFillFor', () => {
  it('gives a light fill on a dark pet', () => {
    expect(eyeFillFor('#000000')).toBe('#fdfdfd')
    expect(eyeFillFor('#0a0a0c')).toBe('#fdfdfd')
  })

  it('gives a dark fill on a light pet', () => {
    expect(eyeFillFor('#ffffff')).toBe('#111116')
    expect(eyeFillFor('#f1efe9')).toBe('#111116') // Cream
  })

  it('decides on perceived luminance, not the mean channel', () => {
    // Green is the brightest channel by weight, blue the darkest, even though
    // their channel means are identical. A naive (r + g + b) / 3 would swap
    // these two and put light eyes on a bright yellow-green pet.
    expect(eyeFillFor('#00ff00')).toBe('#111116')
    expect(eyeFillFor('#0000ff')).toBe('#fdfdfd')
  })
})
