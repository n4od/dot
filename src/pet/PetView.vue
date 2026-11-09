<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, triggerRef } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { BotEngine, type BotFrame } from '../bot/engine'
import { STATE_BY_ID, type StateId } from '../bot/states'
import { DEFAULT_EXPRESSION, EXPRESSION_BY_ID } from '../bot/expressions'
import { COLOR_BY_ID, DEFAULT_COLOR, DEFAULT_SHAPE, SHAPE_BY_ID } from '../bot/skins'
import { RAYON } from '../bot/repere'
import { clamp } from '../bot/math'
import { eyeFillFor } from '../contrast'
import { DEFAULT_STYLE, isStyleId, type StyleId } from './materials'
import PetFigure from './PetFigure.vue'

/*
 * A desktop pet is a client of the same clock-free engine the web demo uses:
 * `sample(t)` is a pure function of time, so everything below is just a clock,
 * an input (the global cursor) and a scheduler for emotes.
 */

const params = new URLSearchParams(window.location.search)
const shapeRadii = SHAPE_BY_ID.get(params.get('shape') ?? DEFAULT_SHAPE)?.radii ?? null
const expression = EXPRESSION_BY_ID.get(params.get('expression') ?? DEFAULT_EXPRESSION) ?? null
const inkColor = COLOR_BY_ID.get(params.get('color') ?? DEFAULT_COLOR)?.hex ?? '#0a0a0c'
const styleParam = params.get('style')
const styleId: StyleId = isStyleId(styleParam) ? styleParam : DEFAULT_STYLE

const ink = computed(() => inkColor)

/**
 * The web demo paints the eyes as holes and backs the body with `paper`, because
 * the page colour is the reference background. A transparent desktop window has
 * no such colour: a hole would show the wallpaper (and the orbit rings drawn
 * behind the body), so the eyes are filled and clipped to the body instead.
 * `eyeFillFor` is shared with the settings preview so the two cannot disagree.
 */
const eyeFill = eyeFillFor(inkColor)

const R = RAYON

const engine = new BotEngine(R, 'idle', shapeRadii, expression)
engine.reset('idle', 0)
const frame = shallowRef<BotFrame>(engine.sample(0))

/** Dev affordance: `?state=orbit` opens straight into one state. */
const forcedState = params.get('state') as StateId | null
const forced = forcedState && STATE_BY_ID.has(forcedState) ? forcedState : null

/* ------------------------------------------------------------- behaviour */

/** States a pet may play on its own. `idle` is the resting pose between them. */
const EMOTES: StateId[] = [
  'wink',
  'wide',
  'notify',
  'exclaim',
  'play',
  'orbit',
  'burst',
  'sleep',
  'thinking',
  'alert'
]

/** Seconds before the first emote, so the pet is calm when the app starts. */
let nextEmoteAt = 7 + Math.random() * 6
/** Absolute clock time at which the current emote ends. 0 = resting. */
let emoteEndsAt = 0

let clock = 0
let last = 0
let raf = 0

/* ------------------------------------------------------------ frame budget */

/**
 * The pet has two costs and they are not equal: drawing is cheap, keeping a
 * browser window awake is not. Measured with one idle pet, the WebView2 process
 * tree costs ~13 % of a core *before it draws anything* — a `requestAnimationFrame`
 * ticking at 60 Hz, the transparent always-on-top window being composited — and
 * our own sampling and patching adds a few points on top.
 *
 * So the loop does not chain `rAF` while it rests. Once nothing is happening —
 * not dragged, no emote, the cursor still for a moment — it draws every
 * `IDLE_FRAME_MS` and schedules the *next* frame with a timer instead of a vsync
 * callback, so the browser is not woken 60 times a second to do nothing. A cursor
 * movement or an emote puts it straight back on `rAF`, where smooth gaze and
 * animation live.
 *
 * None of this costs timing: the engine is clock-free, so `sample(clock)` reads
 * the wall clock however the frames arrive.
 */
/** ~15 fps while resting. 66 ms is a chosen floor: below it the breathing steps. */
const IDLE_FRAME_MS = 66
/** How long after the last cursor movement the pet stays at the full rate. */
const WAKE_MS = 900

let wakeUntil = 0
let lastDraw = 0
/** True while the pet is dragged, emoting or freshly woken: full rate. */
let busy = false
/** The idle `setTimeout`, when the loop is resting instead of on `rAF`. */
let timer = 0
/** Where the cursor was the last time the click-through test ran. */
let hitAtX = NaN
let hitAtY = NaN

/* ---------------------------------------------------------------- cursor */

interface Geometry {
  x: number
  y: number
  w: number
  h: number
  scale: number
  monW: number
  monH: number
}

let geometry: Geometry | null = null
let cursor: { x: number; y: number } | null = null

/** How far the head turns at the edge of the screen. Chosen, not measured. */
const YAW_MAX = 18
const PITCH_BASE = 10
const PITCH_MAX = 13

let look = { yaw: 0, pitch: PITCH_BASE, mix: 0, spin: 0, wander: 1 }
let lookApplied = false

function updateLook() {
  if (!cursor || !geometry) {
    look = { yaw: 0, pitch: PITCH_BASE, mix: 0, spin: 0, wander: 1 }
    return
  }
  // Normalised against the monitor, not the window: a cursor at the edge of the
  // screen must saturate the gaze whatever size the pet happens to be.
  const cx = geometry.x + geometry.w / 2
  const cy = geometry.y + geometry.h / 2
  const nx = clamp((cursor.x - cx) / Math.max(geometry.monW / 2, 1), -1, 1)
  const ny = clamp((cursor.y - cy) / Math.max(geometry.monH / 2, 1), -1, 1)
  look = {
    yaw: nx * YAW_MAX,
    // engine: pitch positive = looking up, screen y grows downwards
    pitch: PITCH_BASE - ny * PITCH_MAX,
    mix: 1,
    spin: 0,
    wander: 0
  }
}

/* --------------------------------------------------- click-through + drag */

let through = true
let dragging = false
let dragGuard = 0
let dragStartedAt = 0
let downAt = 0

function hitAt(lx: number, ly: number): boolean {
  // The painted shapes are children of the `.hit` groups, so the element under
  // the cursor is a <path>/<circle>, not the group itself.
  return !!document.elementFromPoint(lx, ly)?.closest('.hit')
}

/**
 * Transparent windows still swallow clicks across their whole rectangle, so the
 * window is switched between "click-through" and "interactive" every frame
 * according to what is actually painted under the cursor.
 *
 * The five-point probe is the hysteresis: sampling only the exact point made the
 * window flicker between both states on the anti-aliased edge of the silhouette,
 * and a flicker during a drag cancels the drag.
 */
function updateHitTest() {
  if (!geometry || !cursor || dragging) {
    // Safety net: if the OS steals the pointer-up during a drag, the pet must
    // not stay permanently uninteractive.
    if (dragging && performance.now() - dragStartedAt > 8000) dragging = false
    return
  }
  if (performance.now() < dragGuard) return

  const lx = (cursor.x - geometry.x) / geometry.scale
  const ly = (cursor.y - geometry.y) / geometry.scale
  const inBounds = lx > -8 && ly > -8 && lx < geometry.w / geometry.scale + 8 && ly < geometry.h / geometry.scale + 8

  let over = false
  if (inBounds) {
    const pad = 5
    over = hitAt(lx, ly) || hitAt(lx + pad, ly) || hitAt(lx - pad, ly) || hitAt(lx, ly + pad) || hitAt(lx, ly - pad)
  }

  if (over === !through) return
  through = !over
  void invoke('set_click_through', { through }).catch(() => {})
}

async function beginDrag() {
  dragging = true
  dragStartedAt = performance.now()
  dragGuard = performance.now() + 120
  try {
    await invoke('start_drag')
  } catch {
    // the window may already be gone; nothing to do
  }
  dragGuard = performance.now() + 250
  void refreshGeometry()
}

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0) return
  const now = performance.now()
  if (now - downAt < 320) {
    // double-click: a small reaction instead of the scheduled emote
    playEmote('wink')
    downAt = 0
    return
  }
  downAt = now
  void beginDrag()
}

function onPointerUp() {
  dragging = false
  dragGuard = performance.now() + 150
  void refreshGeometry()
}

/* --------------------------------------------------------------- the loop */

function playEmote(id: StateId) {
  engine.setState(id, clock)
  emoteEndsAt = clock + Math.max(1.6, STATE_BY_ID.get(id)?.duration ?? 2)
  nextEmoteAt = emoteEndsAt + 7 + Math.random() * 14
}

function tick(ms: number) {
  raf = 0
  timer = 0

  // Bounded delta: a suspended window resumes where it left off instead of
  // jumping forward by however long the machine was asleep.
  const dt = last ? Math.min((ms - last) / 1000, 0.064) : 0
  last = ms
  clock += dt

  if (emoteEndsAt) {
    if (clock >= emoteEndsAt) {
      emoteEndsAt = 0
      engine.setState('idle', clock)
    }
  } else if (clock >= nextEmoteAt) {
    playEmote(EMOTES[(Math.random() * EMOTES.length) | 0]!)
  }

  busy = dragging || emoteEndsAt !== 0 || ms < wakeUntil
  if (!busy && ms - lastDraw < IDLE_FRAME_MS) {
    schedule()
    return
  }
  lastDraw = ms

  updateLook()

  // Gaze only commands states that carry the resting face. Elsewhere the eyes
  // *are* the animation and overlaying a target would blur it.
  const baseFace = STATE_BY_ID.get(engine.state)?.baseFace ?? false
  if (baseFace) {
    engine.setLook(look, clock)
    lookApplied = true
  } else if (lookApplied) {
    engine.setLook(null, clock)
    lookApplied = false
  }

  frame.value = engine.sample(clock)
  triggerRef(frame)

  /*
   * Five `elementFromPoint` probes answer a question whose answer cannot change
   * while neither the cursor nor the silhouette moves: re-test only after a move,
   * or while an emote or a drag is changing the shape under the pointer.
   */
  if (busy || (cursor && (cursor.x !== hitAtX || cursor.y !== hitAtY))) {
    hitAtX = cursor?.x ?? NaN
    hitAtY = cursor?.y ?? NaN
    updateHitTest()
  }

  schedule()
}

/**
 * Queue the next frame: on `rAF` while the pet has something to do, on a timer
 * while it rests, and not at all while the window is hidden. `busy` is the flag
 * the frame just computed.
 */
function schedule() {
  if (document.hidden) return
  if (busy) raf = requestAnimationFrame(tick)
  else timer = window.setTimeout(() => tick(performance.now()), IDLE_FRAME_MS)
}

/** Resume at once — a moving cursor or a reappearing window must not wait a beat. */
function wake() {
  if (document.hidden) return
  if (timer) {
    window.clearTimeout(timer)
    timer = 0
  }
  if (!raf) raf = requestAnimationFrame(tick)
}

/* ------------------------------------------------------------------ wiring */

let unlisten: UnlistenFn | null = null
let geoPoll = 0

async function refreshGeometry() {
  try {
    geometry = await invoke<Geometry>('pet_geometry')
  } catch {
    geometry = null
  }
}

/** A hidden window is not composited, so the loop stops until it is shown again. */
function onVisibility() {
  if (document.hidden) {
    if (timer) {
      window.clearTimeout(timer)
      timer = 0
    }
    if (raf) {
      cancelAnimationFrame(raf)
      raf = 0
    }
  } else {
    wake()
  }
}

onMounted(async () => {
  if (forced) {
    engine.setState(forced, 0)
    emoteEndsAt = 0
    nextEmoteAt = Infinity
  }

  await refreshGeometry()
  geoPoll = window.setInterval(() => void refreshGeometry(), 1000)

  window.addEventListener('pointerup', onPointerUp)
  window.addEventListener('pointercancel', onPointerUp)
  window.addEventListener('blur', onPointerUp)
  document.addEventListener('visibilitychange', onVisibility)

  // Guarded so `pnpm dev` also works in a plain browser for visual iteration:
  // there is no Tauri runtime there, only the animation.
  try {
    unlisten = await listen<[number, number]>('pet://cursor', (event) => {
      const [x, y] = event.payload
      cursor = { x, y }
      // A moving cursor needs the full frame rate for a smooth gaze.
      wakeUntil = performance.now() + WAKE_MS
      wake()
    })
  } catch {
    unlisten = null
  }

  raf = requestAnimationFrame(tick)
})

onBeforeUnmount(() => {
  cancelAnimationFrame(raf)
  window.clearTimeout(timer)
  window.clearInterval(geoPoll)
  window.removeEventListener('pointerup', onPointerUp)
  window.removeEventListener('pointercancel', onPointerUp)
  window.removeEventListener('blur', onPointerUp)
  document.removeEventListener('visibilitychange', onVisibility)
  unlisten?.()
})

/* --------------------------------------------------------------- template */

function onFigurePointerDown(event: PointerEvent) {
  onPointerDown(event)
}
</script>

<template>
  <!-- The figure is shared with the settings stage; `interactive` is what makes
       the body and the eyes the only clickable parts. -->
  <PetFigure
    :frame="frame"
    :ink="ink"
    :eye-fill="eyeFill"
    :style-id="styleId"
    interactive
    class="pet"
    @pointerdown="onFigurePointerDown"
  />
</template>

<style scoped>
.pet {
  cursor: grab;
}
</style>