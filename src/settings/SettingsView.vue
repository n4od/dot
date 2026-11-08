<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { BotEngine, type BotFrame } from '../bot/engine'
import { EXPRESSIONS, EXPRESSION_BY_ID, DEFAULT_EXPRESSION } from '../bot/expressions'
import { DEMI_VIEWBOX, RAYON } from '../bot/repere'
import { COLORS, COLOR_BY_ID, SHAPES, SHAPE_BY_ID, DEFAULT_SHAPE, DEFAULT_COLOR } from '../bot/skins'
import { eyeFillFor } from '../contrast'

/*
 * The picker renders its previews through the real engine rather than a
 * hand-drawn approximation: `sample(0)` on a fresh engine is the resting pose,
 * so what you see here is what the pet window draws. Nothing in this file knows
 * how an eye is shaped.
 */

interface Config {
  shape: string
  color: string
  expression: string
}

const R = RAYON
const VB = DEMI_VIEWBOX

const config = ref<Config>({ shape: DEFAULT_SHAPE, color: DEFAULT_COLOR, expression: DEFAULT_EXPRESSION })
const error = ref('')
const saved = ref(false)

/** Paint a posture: a throwaway engine asked for its first frame. */
function pose(shape: string, expression: string): BotFrame {
  const engine = new BotEngine(
    R,
    'idle',
    SHAPE_BY_ID.get(shape)?.radii ?? null,
    EXPRESSION_BY_ID.get(expression) ?? null
  )
  engine.reset('idle', 0)
  return engine.sample(0)
}

const preview = computed(() => pose(config.value.shape, config.value.expression))
const ink = computed(() => COLOR_BY_ID.get(config.value.color)?.hex ?? '#0a0a0c')
const eyeFill = computed(() => eyeFillFor(ink.value))

/**
 * One face per expression, posed once at startup. Expressions are independent of
 * shape and colour, so only the fill has to be reactive.
 */
const expressions = EXPRESSIONS.map((expression) => ({ id: expression.id, eyes: pose(DEFAULT_SHAPE, expression.id).eyes }))

/** A silhouette from the catalogue's own radii, so the button cannot lie either. */
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

function pick(key: keyof Config, value: string) {
  if (config.value[key] === value) return
  config.value = { ...config.value, [key]: value }
  schedule()
}

/*
 * Writes coalesce: clicking through a palette should leave one file on disk, not
 * one per swatch. Rust owns the file and applies the change to the pet window in
 * the same call, so there is nothing else to send.
 */
let timer = 0
function schedule() {
  saved.value = false
  error.value = ''
  window.clearTimeout(timer)
  timer = window.setTimeout(async () => {
    try {
      await invoke('set_config', { config: config.value })
      saved.value = true
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
  } catch (e) {
    error.value = String(e)
  }
})
</script>

<template>
  <main>
    <header>
      <div class="swatch" :style="{ background: ink }">
        <svg :viewBox="`${-VB} ${-VB} ${VB * 2} ${VB * 2}`" aria-hidden="true">
          <defs>
            <clipPath id="settings-clip">
              <path :d="preview.bodyPath" />
            </clipPath>
          </defs>
          <path :d="preview.bodyPath" :fill="ink" />
          <g clip-path="url(#settings-clip)">
            <path
              v-for="(eye, i) in preview.eyes"
              :key="i"
              :d="eye.d"
              :transform="eye.matrix"
              :opacity="eye.alpha"
              :fill="eyeFill"
            />
          </g>
        </svg>
      </div>
      <div>
        <h1>Dot</h1>
        <p class="lede">Your pet's shape, colour and face. Changes apply immediately.</p>
      </div>
    </header>

    <section>
      <h2>Shape</h2>
      <div class="grid shapes">
        <button
          v-for="shape in SHAPES"
          :key="shape.id"
          type="button"
          :class="{ on: shape.id === config.shape }"
          :aria-pressed="shape.id === config.shape"
          @click="pick('shape', shape.id)"
        >
          <svg viewBox="-1.25 -1.25 2.5 2.5" aria-hidden="true">
            <path :d="outline(shape.id)" :fill="ink" />
          </svg>
          <span>{{ shape.id }}</span>
        </button>
      </div>
    </section>

    <section>
      <h2>Colour</h2>
      <div class="grid colors">
        <button
          v-for="color in COLORS"
          :key="color.id"
          type="button"
          :class="{ on: color.id === config.color }"
          :aria-pressed="color.id === config.color"
          :aria-label="color.id"
          :title="color.id"
          @click="pick('color', color.id)"
        >
          <span class="dot" :style="{ background: color.hex }"></span>
        </button>
      </div>
    </section>

    <section>
      <h2>Expression</h2>
      <div class="grid expressions">
        <button
          v-for="expression in expressions"
          :key="expression.id"
          type="button"
          :class="{ on: expression.id === config.expression }"
          :aria-pressed="expression.id === config.expression"
          @click="pick('expression', expression.id)"
        >
          <svg viewBox="-50 -50 100 100" aria-hidden="true">
            <path
              v-for="(eye, i) in expression.eyes"
              :key="i"
              :d="eye.d"
              :transform="eye.matrix"
              :opacity="eye.alpha"
              :fill="ink"
            />
          </svg>
          <span>{{ expression.id }}</span>
        </button>
      </div>
    </section>

    <footer>
      <span class="status">
        <template v-if="error">Could not save: {{ error }}</template>
        <template v-else-if="saved">Saved</template>
      </span>
      <button type="button" class="ghost" @click="quit">Quit Dot</button>
    </footer>
  </main>
</template>

<style scoped>
main {
  padding: 22px 26px 26px;
  font-size: 13px;
  color: #2a2a33;
}

header {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 20px;
}

.swatch {
  flex: none;
  width: 64px;
  height: 64px;
  border-radius: 50%;
}

.swatch svg {
  display: block;
  width: 100%;
  height: 100%;
}

h1 {
  margin: 0 0 4px;
  font-size: 17px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.lede {
  margin: 0;
  color: #55555f;
}

h2 {
  margin: 0 0 8px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #7b7b87;
}

section {
  margin-bottom: 18px;
  padding: 14px 16px;
  border-radius: 12px;
  background: #f3f3f6;
}

.grid {
  display: grid;
  gap: 6px;
}

.shapes {
  grid-template-columns: repeat(auto-fill, minmax(72px, 1fr));
}

.shapes button {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px 4px 6px;
  color: #55555f;
}

.shapes svg {
  width: 30px;
  height: 30px;
}

.shapes span {
  font-size: 11px;
}

.colors {
  grid-template-columns: repeat(auto-fill, minmax(40px, 1fr));
}

.colors button {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 6px 0;
}

.dot {
  width: 20px;
  height: 20px;
  border-radius: 50%;
}

.expressions {
  grid-template-columns: repeat(auto-fill, minmax(88px, 1fr));
}

.expressions button {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  color: #55555f;
}

.expressions svg {
  flex: none;
  width: 26px;
  height: 26px;
}

.expressions span {
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
}

button {
  font: inherit;
  border: 1px solid #e2e2e9;
  border-radius: 9px;
  background: #fff;
  cursor: pointer;
}

button:hover {
  border-color: #c9c9d4;
}

button.on {
  border-color: #2a2a33;
  box-shadow: inset 0 0 0 1px #2a2a33;
  background: #fff;
  color: #2a2a33;
}

footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.status {
  color: #7b7b87;
  font-size: 12px;
}

.ghost {
  padding: 8px 14px;
  color: #2a2a33;
}

.ghost:hover {
  background: #f6f6f9;
}
</style>
