<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { BotEngine, type BotFrame } from '../bot/engine'
import { EXPRESSIONS, EXPRESSION_BY_ID, DEFAULT_EXPRESSION } from '../bot/expressions'
import { RAYON } from '../bot/repere'
import { COLORS, COLOR_BY_ID, SHAPES, SHAPE_BY_ID, DEFAULT_SHAPE, DEFAULT_COLOR } from '../bot/skins'
import { eyeFillFor } from '../contrast'
import { COLOR_NAMES, EXPRESSION_NAMES, SHAPE_NAMES, STYLE_NAMES, displayName } from '../display'
import { STYLE_IDS, DEFAULT_STYLE, isStyleId, type StyleId } from '../pet/materials'
import PetFigure from '../pet/PetFigure.vue'

/*
 * The picker is a bench: the pet is on it, alive, while you change it.
 *
 * The preview is not a drawing — it is the same `BotEngine` the desktop window
 * runs, sampled on the same clock-free `sample(t)`, drawn by the same
 * `PetFigure` component. So the thing on the bench is the pet, and it cannot
 * promise something the desktop window will not do.
 */

interface Config {
  shape: string
  color: string
  expression: string
  style: string
}

const R = RAYON

const config = ref<Config>({
  shape: DEFAULT_SHAPE,
  color: DEFAULT_COLOR,
  expression: DEFAULT_EXPRESSION,
  style: DEFAULT_STYLE
})
const error = ref('')
const saved = ref(false)

const shapeName = computed(() => displayName(SHAPE_NAMES, config.value.shape))
const colorName = computed(() => displayName(COLOR_NAMES, config.value.color))
const expressionName = computed(() => displayName(EXPRESSION_NAMES, config.value.expression))

const ink = computed(() => COLOR_BY_ID.get(config.value.color)?.hex ?? '#0a0a0c')
const eyeFill = computed(() => eyeFillFor(ink.value))

/** The stored id is a plain string; anything unknown paints as flat. */
const styleId = computed<StyleId>(() =>
  isStyleId(config.value.style) ? config.value.style : DEFAULT_STYLE
)

/* ----------------------------------------------------------------- clock */

let clock = 0
let last = 0
let raf = 0
let winkEndsAt = 0
let nextWinkAt = 9 + Math.random() * 6

function makeEngine() {
  const engine = new BotEngine(
    R,
    'idle',
    SHAPE_BY_ID.get(config.value.shape)?.radii ?? null,
    EXPRESSION_BY_ID.get(config.value.expression) ?? null
  )
  engine.reset('idle', clock)
  return engine
}

const engine = shallowRef(makeEngine())
const frame = shallowRef<BotFrame>(engine.value.sample(clock))

/** Shape and expression are baked into the engine at construction. */
watch(
  () => [config.value.shape, config.value.expression],
  () => {
    engine.value = makeEngine()
    winkEndsAt = 0
    // Re-arm the wink too: with the clock already past the old deadline, every
    // change used to trigger an immediate wink on top of the perk animation.
    nextWinkAt = clock + 9 + Math.random() * 9
    perk()
  }
)

function tick(ms: number) {
  raf = requestAnimationFrame(tick)

  // Bounded delta: an occluded window resumes where it left off.
  const dt = last ? Math.min((ms - last) / 1000, 0.064) : 0
  last = ms
  clock += dt

  if (winkEndsAt) {
    if (clock >= winkEndsAt) {
      winkEndsAt = 0
      engine.value.setState('idle', clock)
    }
  } else if (clock >= nextWinkAt) {
    // `wink` only: the roomier emotes throw orbits far outside the stage.
    engine.value.setState('wink', clock)
    winkEndsAt = clock + 1.5
    nextWinkAt = winkEndsAt + 9 + Math.random() * 9
  }

  frame.value = engine.value.sample(clock)
}

/** A small squash when the pet is redressed, so the change reads as physical. */
const perkKey = ref(0)
function perk() {
  perkKey.value++
}

/* ---------------------------------------------------------------- saving */

function pick(key: keyof Config, value: string) {
  if (config.value[key] === value) return
  config.value = { ...config.value, [key]: value }
  schedule()
}

/*
 * Writes coalesce, so clicking through a palette leaves one file on disk rather
 * than one per swatch. Rust owns the file and re-opens the pet window with the
 * new values in the same call.
 */
let timer = 0
let savedTimer = 0
function schedule() {
  saved.value = false
  error.value = ''
  window.clearTimeout(timer)
  timer = window.setTimeout(async () => {
    try {
      await invoke('set_config', { config: config.value })
      saved.value = true
      window.clearTimeout(savedTimer)
      savedTimer = window.setTimeout(() => (saved.value = false), 1800)
    } catch (e) {
      error.value = String(e)
    }
  }, 150)
}

function quit() {
  void invoke('quit_app')
}

onMounted(async () => {
  try {
    config.value = await invoke<Config>('get_config')
    engine.value = makeEngine()
  } catch (e) {
    error.value = String(e)
  }
  raf = requestAnimationFrame(tick)
})

onBeforeUnmount(() => {
  cancelAnimationFrame(raf)
  window.clearTimeout(timer)
  window.clearTimeout(savedTimer)
})

/* -------------------------------------------------------------- silhouettes */

/** A chip glyph from the catalogue's own radii, so the button cannot lie. */
function outline(shape: string): string {
  const radii = SHAPE_BY_ID.get(shape)?.radii
  if (!radii) return ''
  const n = radii.length
  return (
    radii
      .map((r, i) => {
        const a = (i / n) * Math.PI * 2
        // Same convention as the engine: angle 0 is +x, y grows downwards.
        return `${i === 0 ? 'M' : 'L'}${(Math.cos(a) * r).toFixed(3)} ${(Math.sin(a) * r).toFixed(3)}`
      })
      .join('') + 'Z'
  )
}

/**
 * Each chip shows a whole head, not a pair of floating eyes.
 *
 * An expression is mostly *eye shape* — a squint is two flat lines, a sleepy
 * face is two dashes. On their own those read as broken glyphs; the same marks
 * inside a silhouette read as a face. So the chip is a miniature pet, in the
 * pet's own colour and with the real eye fill: sixteen faces of the thing you
 * are dressing, one per expression.
 */
const expressions = EXPRESSIONS.map((expression) => {
  const engine = new BotEngine(R, 'idle', SHAPE_BY_ID.get(DEFAULT_SHAPE)?.radii ?? null, expression)
  engine.reset('idle', 0)
  return {
    id: expression.id,
    name: displayName(EXPRESSION_NAMES, expression.id),
    frame: straightAhead(engine.sample(0))
  }
})

/**
 * Cancels the expression's head turn in a thumbnail.
 *
 * The engine's resting pose looks up-left (the reference video's pose), and a few
 * expressions turn the head further. That is right on the desktop, but in a row
 * of sixteen faces it reads as a misaligned glyph. Shifting every eye by the same
 * amount puts the pair back on the face's axis while keeping the asymmetries that
 * carry the expression — one eye higher, one narrower, mirrored tilts.
 */
function straightAhead(frame: BotFrame): BotFrame {
  const parsed = frame.eyes.map((eye) => {
    const match = /^matrix\(([^)]+)\)$/.exec(eye.matrix)
    return match ? match[1].split(',').map(Number) : null
  })
  if (frame.eyes.length === 0 || parsed.some((p) => !p)) return frame

  const cx = parsed.reduce((sum, p) => sum + p![4], 0) / parsed.length
  const cy = parsed.reduce((sum, p) => sum + p![5], 0) / parsed.length

  return {
    ...frame,
    eyes: frame.eyes.map((eye, i) => {
      const [a, b, c, d, e, f] = parsed[i]!
      return { ...eye, matrix: `matrix(${a},${b},${c},${d},${e - cx},${f - cy})` }
    })
  }
}

/* The chip crops in on the body and drops the orbits (`zoom` + `bare`), so the
   face fills the thumbnail instead of floating in a mostly-empty box. */
</script>

<template>
  <main>
    <header>
      <h1>Dot</h1>
      <p class="tagline">Dress your pet. It saves as you go.</p>
      <div class="styles" role="group" aria-label="Style">
        <button
          v-for="id in STYLE_IDS"
          :key="id"
          type="button"
          class="stylebtn"
          :class="{ on: id === config.style }"
          :aria-pressed="id === config.style"
          @click="pick('style', id)"
        >
          {{ displayName(STYLE_NAMES, id) }}
        </button>
      </div>
    </header>

    <div class="bench">
      <section class="stage" aria-label="Preview">
        <PetFigure
          :key="perkKey"
          class="figure"
          :frame="frame"
          :ink="ink"
          :eye-fill="eyeFill"
          :style-id="styleId"
        />
        <p class="plate">{{ shapeName }} · {{ colorName }} · {{ expressionName }}</p>
      </section>

      <div class="panel">
        <section class="row">
          <div class="rowhead">
            <h2>Shape</h2>
            <span class="value">{{ shapeName }}</span>
          </div>
          <div class="chips">
            <button
              v-for="shape in SHAPES"
              :key="shape.id"
              type="button"
              class="chip shape"
              :class="{ on: shape.id === config.shape }"
              :aria-pressed="shape.id === config.shape"
              :aria-label="displayName(SHAPE_NAMES, shape.id)"
              :title="displayName(SHAPE_NAMES, shape.id)"
              @click="pick('shape', shape.id)"
            >
              <svg viewBox="-1.3 -1.3 2.6 2.6" aria-hidden="true">
                <path
                  :d="outline(shape.id)"
                  :fill="ink"
                  stroke="rgba(90, 70, 45, 0.3)"
                  stroke-width="0.045"
                />
              </svg>
            </button>
          </div>
        </section>

        <section class="row">
          <div class="rowhead">
            <h2>Colour</h2>
            <span class="value">{{ colorName }}</span>
          </div>
          <div class="swatches">
            <button
              v-for="color in COLORS"
              :key="color.id"
              type="button"
              class="swatch"
              :class="{ on: color.id === config.color }"
              :aria-pressed="color.id === config.color"
              :aria-label="displayName(COLOR_NAMES, color.id)"
              :title="displayName(COLOR_NAMES, color.id)"
              :style="{ '--swatch': color.hex }"
              @click="pick('color', color.id)"
            ></button>
          </div>
        </section>

        <section class="row">
          <div class="rowhead">
            <h2>Expression</h2>
            <span class="value">{{ expressionName }}</span>
          </div>
          <div class="chips faces">
            <button
              v-for="expression in expressions"
              :key="expression.id"
              type="button"
              class="chip face"
              :class="{ on: expression.id === config.expression }"
              :aria-pressed="expression.id === config.expression"
              :aria-label="expression.name"
              :title="expression.name"
              @click="pick('expression', expression.id)"
            >
              <PetFigure
                class="face-figure"
                :frame="expression.frame"
                :ink="ink"
                :eye-fill="eyeFill"
                :style-id="styleId"
                :zoom="110"
                bare
                aria-hidden="true"
              />
              <span>{{ expression.name }}</span>
            </button>
          </div>
        </section>
      </div>
    </div>

    <footer>
      <span class="status" :class="{ bad: error }">
        <template v-if="error">Could not save — {{ error }}</template>
        <template v-else-if="saved">Saved</template>
        <template v-else>Changes land on the pet right away</template>
      </span>
      <button type="button" class="quit" @click="quit">Quit Dot</button>
    </footer>
  </main>
</template>

<style scoped>
/*
 * A workbench: warm paper, one stage for the pet, controls kept quiet beside it.
 * The stage never scrolls; only the controls do, so the pet stays in view while
 * you page through the expression grid.
 */
main {
  --ink: #241d15;
  --muted: #8a7d6a;
  --hairline: rgba(90, 70, 45, 0.14);
  --paper: #f7f3ec;

  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  gap: 18px;
  box-sizing: border-box;
  height: 100%;
  padding: 20px 24px 16px;
  color: var(--ink);
}

header {
  display: flex;
  align-items: baseline;
  gap: 12px;
}

h1 {
  margin: 0;
  font-family: Georgia, 'Iowan Old Style', 'Times New Roman', serif;
  font-size: 27px;
  font-weight: 400;
  letter-spacing: -0.015em;
}

.tagline {
  margin: 0;
  font-size: 12.5px;
  color: var(--muted);
}

/* A mode switch, not a value: two words in a pill, out of the panel's way. */
.styles {
  display: flex;
  align-self: center;
  margin-left: auto;
  padding: 2px;
  border: 1px solid var(--hairline);
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.55);
}

.stylebtn {
  padding: 4px 12px;
  border-radius: 999px;
  font-size: 11px;
  letter-spacing: 0.01em;
  color: var(--muted);
  transition:
    background 0.15s ease,
    color 0.15s ease;
}

.stylebtn:hover {
  color: var(--ink);
}

.stylebtn.on {
  background: var(--ink);
  color: #f7f3ec;
}

/* ------------------------------------------------------------------ bench */

.bench {
  display: grid;
  grid-template-columns: minmax(220px, 300px) minmax(0, 1fr);
  gap: 26px;
  min-height: 0;
}

.stage {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 14px;
  min-height: 0;
  padding: 18px;
  border-radius: 20px;
  overflow: hidden;
  background: radial-gradient(circle at 50% 34%, #fdfbf7 0%, #f1eade 60%, #e6ddcd 100%);
  box-shadow:
    inset 0 1px 0 #fff,
    inset 0 -20px 46px rgba(120, 100, 70, 0.07);
}

/* Dotted grid, faded out towards the edges so the stage has a centre. */
.stage::before {
  content: '';
  position: absolute;
  inset: 0;
  background-image: radial-gradient(rgba(90, 75, 55, 0.17) 1px, transparent 1px);
  background-size: 14px 14px;
  -webkit-mask-image: radial-gradient(circle at 50% 42%, #000 38%, transparent 76%);
  mask-image: radial-gradient(circle at 50% 42%, #000 38%, transparent 76%);
  pointer-events: none;
}

.figure {
  position: relative;
  align-self: center;
  width: min(100%, 262px);
  aspect-ratio: 1;
  filter: drop-shadow(0 14px 16px rgba(70, 52, 30, 0.22));
  animation: perk 0.42s cubic-bezier(0.2, 0.8, 0.3, 1.3);
}

@keyframes perk {
  0% {
    transform: scale(0.9) translateY(6px);
  }
  62% {
    transform: scale(1.035);
  }
  100% {
    transform: scale(1);
  }
}

.plate {
  position: relative;
  margin: 0;
  text-align: center;
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.13em;
  text-transform: uppercase;
  color: var(--muted);
}

/* ---------------------------------------------------------------- controls */

.panel {
  min-height: 0;
  padding-right: 6px;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(90, 70, 45, 0.28) transparent;
}

/* Chromium ignores scrollbar-width styling often enough that it is worth
   spelling out: a wide grey track would be the loudest thing on this panel. */
.panel::-webkit-scrollbar {
  width: 8px;
}

.panel::-webkit-scrollbar-thumb {
  border-radius: 4px;
  background: rgba(90, 70, 45, 0.24);
}

.panel::-webkit-scrollbar-track {
  background: transparent;
}

.row {
  padding: 11px 0;
  border-bottom: 1px solid var(--hairline);
}

.row:first-child {
  padding-top: 0;
}

.row:last-child {
  padding-bottom: 0;
  border-bottom: 0;
}

.rowhead {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}

h2 {
  margin: 0;
  font-size: 10.5px;
  font-weight: 700;
  letter-spacing: 0.15em;
  text-transform: uppercase;
  color: var(--muted);
}

.value {
  font-size: 12px;
  color: #5c5142;
}

button {
  font: inherit;
  color: inherit;
  border: 0;
  background: none;
  cursor: pointer;
}

button:focus-visible {
  outline: 2px solid #b0603a;
  outline-offset: 2px;
}

/* Shape and expression chips share a frame; only the glyph differs. */
.chips {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(42px, 1fr));
  gap: 7px;
}

/* Bare glyphs, like the palette: the marks are the content, so the tiles stay
   out of the way. Sixteen boxes around sixteen faces read as a spreadsheet. */
.chip {
  display: flex;
  align-items: center;
  justify-content: center;
  box-sizing: border-box;
  border-radius: 12px;
  border: 0;
  background: transparent;
  transition:
    background 0.15s ease,
    box-shadow 0.15s ease,
    transform 0.15s ease;
}

.chip:hover {
  background: rgba(255, 255, 255, 0.8);
}

.chip.on {
  background: #fff;
  box-shadow: inset 0 0 0 1.5px var(--ink);
}

.shape {
  aspect-ratio: 1;
  padding: 9px;
}

.shape svg {
  display: block;
  width: 100%;
  height: 100%;
  border-radius: 50%;
  /* Same lift as the expression faces, so the rows read as one instrument. */
  filter: drop-shadow(0 1px 1px rgba(70, 52, 30, 0.2));
  transition:
    transform 0.15s ease,
    box-shadow 0.15s ease;
}

.shape:hover svg {
  transform: scale(1.05);
}

/* Selection is a ring around the mark, not a box around the tile. */
.shape.on {
  background: transparent;
  box-shadow: none;
}

.shape.on svg {
  transform: scale(1.06);
  box-shadow:
    0 0 0 2px #f7f3ec,
    0 0 0 3.5px var(--ink);
}

.faces {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 6px;
}

/* Six to a row, but a short last row stays centred instead of hanging left. */
.faces .face {
  flex: 0 0 calc((100% - 5 * 6px) / 6);
}

.face {
  flex-direction: column;
  gap: 2px;
  padding: 6px 4px 5px;
}

.face-figure {
  display: block;
  width: 42px;
  height: 42px;
  border-radius: 50%;
  /* A pale pet (Cream) would otherwise disappear into the paper. */
  filter: drop-shadow(0 1px 1px rgba(70, 52, 30, 0.2));
  transition:
    transform 0.15s ease,
    box-shadow 0.15s ease;
}

.face:hover .face-figure {
  transform: scale(1.05);
}

/* The selection is a ring around the face itself, not a box around the tile. */
.face.on {
  background: transparent;
  box-shadow: none;
}

.face.on .face-figure {
  transform: scale(1.08);
  box-shadow:
    0 0 0 2px #f7f3ec,
    0 0 0 3.5px var(--ink);
}

.face span {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 18px;
  font-size: 10px;
  line-height: 1.2;
  text-align: center;
  color: #5c5142;
}

.face.on span {
  color: var(--ink);
  font-weight: 600;
}

/* Bare swatches: the palette is the content, so the chrome gets out of the way. */
.swatches {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(24px, 1fr));
  gap: 6px;
}

.swatch {
  aspect-ratio: 1;
  height: auto;
  border-radius: 50%;
  background: var(--swatch);
  box-shadow: inset 0 0 0 1px rgba(40, 32, 20, 0.14);
  transition:
    transform 0.15s ease,
    box-shadow 0.15s ease;
}

.swatch:hover {
  transform: scale(1.08);
}

.swatch.on {
  transform: scale(1.06);
  box-shadow:
    inset 0 0 0 1px rgba(40, 32, 20, 0.14),
    0 0 0 2px #f7f3ec,
    0 0 0 3.5px var(--ink);
}

/* ----------------------------------------------------------------- footer */

footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--hairline);
}

.status {
  flex: 1;
  font-size: 11.5px;
  color: var(--muted);
}

.status.bad {
  color: #a3341f;
}

.quit {
  padding: 7px 13px;
  border-radius: 9px;
  border: 1px solid var(--hairline);
  background: rgba(255, 255, 255, 0.6);
  font-size: 12px;
}

.quit:hover {
  background: #fff;
  border-color: rgba(90, 70, 45, 0.4);
}
</style>
