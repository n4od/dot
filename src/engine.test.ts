import { describe, expect, it } from 'vitest'
import { BotEngine, type BotFrame } from './bot/engine'
import { RAYON } from './bot/repere'

/*
 * The suite in `src/bot/` is a copy of the upstream tests, so it cannot pin
 * anything this app relies on that upstream does not exercise — `LOOK_MORPH` is
 * the one we know about. These are contract tests from the outside: they drive
 * the public setter and read the rendered eyes, so they protect the app without
 * editing the frozen fork.
 */

const LOOK = { yaw: 30, pitch: 0, mix: 1, spin: 0, wander: 0 }

function engineWithMorph(morph: number): BotEngine {
  const engine = new BotEngine(RAYON, 'idle', null, null)
  engine.reset('idle', 0)
  engine.setLook(LOOK, 0, morph)
  return engine
}

/** Mean horizontal position of the two eyes, in viewBox units. */
function eyeMidX(frame: BotFrame): number {
  const xs = frame.eyes.map((eye) => {
    const match = /matrix\(([^)]+)\)/.exec(eye.matrix)
    const e = match?.[1]?.split(',')[4]
    return e === undefined ? NaN : Number(e)
  })
  return xs.reduce((sum, x) => sum + x, 0) / xs.length
}

/** Where the eyes sit `t` seconds after the look was set, for a given morph. */
function eyesAt(morph: number, t: number): number {
  return eyeMidX(engineWithMorph(morph).sample(t))
}

/**
 * How far the eyes have travelled towards the look at time `t`, 0 at rest and 1
 * fully arrived. A near-infinite morph is "has not moved" and a near-zero one is
 * "already there"; the default sits between. All three are sampled at the same
 * instant, so the engine's own idle motion — blink, breath — cancels out.
 */
function progressAt(t: number): number {
  const rest = eyesAt(1e9, t)
  const arrived = eyesAt(1e-3, t)
  return (eyesAt(BotEngine.LOOK_MORPH, t) - rest) / (arrived - rest)
}

describe('BotEngine.sample', () => {
  it('is a pure function of time', () => {
    const engine = new BotEngine(RAYON, 'idle', null, null)
    engine.reset('idle', 0)
    expect(engine.sample(3.5)).toEqual(engine.sample(3.5))
  })
})

describe('the gaze morph', () => {
  it('turns the eyes, and eases rather than snapping', () => {
    /*
     * Absolute times on purpose: they are what pin `LOOK_MORPH` at 0.24 s. The
     * eyes must not have arrived a third of the way in, and must have arrived by
     * the end — change the constant and one of those flips.
     */
    const beforeDone = 0.12
    const done = 0.24

    // The gaze really does move the eyes; otherwise the rest is vacuous.
    const travel = eyesAt(1e-3, beforeDone) - eyesAt(1e9, beforeDone)
    expect(Math.abs(travel)).toBeGreaterThan(0.5)

    const midway = progressAt(beforeDone)
    expect(midway).toBeGreaterThan(0)
    expect(midway).toBeLessThan(1)

    expect(progressAt(done)).toBeGreaterThan(0.98)
  })
})
