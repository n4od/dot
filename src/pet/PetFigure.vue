<script setup lang="ts">
import { computed } from 'vue'
import type { BotFrame } from '../bot/engine'
import { NOTIF_BLUE } from '../bot/decor'
import { DEMI_VIEWBOX, RAYON } from '../bot/repere'
import { eyeShade, surfaceFor, type StyleId } from './materials'

/*
 * One drawing of a pet, shared by the desktop window and the settings stage.
 * If these were two templates they would drift, and the picker would start
 * promising something the pet does not do.
 *
 * `interactive` marks the shapes that accept a click. Only the body and the eyes
 * count: the orbits and the particles fly well outside the silhouette, and a pet
 * you can grab by its orbit is a pet that steals clicks from the desktop.
 */

const props = withDefaults(
  defineProps<{
    frame: BotFrame
    ink: string
    eyeFill: string
    interactive?: boolean
    /**
     * Half-size of the viewBox, in engine units. The default leaves room for the
     * orbits; a small value crops in on the body, which is what the settings
     * picker wants for a face thumbnail.
     */
    zoom?: number
    /** Body and face only — no orbits, particles or notification badge. */
    bare?: boolean
    /** How the same geometry is painted. `plat` is the engine's flat fill. */
    styleId?: StyleId
  }>(),
  { interactive: false, zoom: DEMI_VIEWBOX, bare: false, styleId: 'plat' }
)

/**
 * The style seam. `surfaceFor` turns the id and the ink into a `Surface`, or
 * `null` for `plat` — the flat fill, which is the absence of a surface.
 *
 * Everything below this is shared by every style: the template paints whatever
 * surface it is handed, so a new one of this kind is data in `./materials.ts` and
 * needs no change here. A branch is only owed when a style brings a new *kind of
 * element* — an outline, a second silhouette — not new numbers.
 */
const surface = computed(() => surfaceFor(props.styleId, props.ink))
const eyeLid = computed(() => eyeShade(props.eyeFill, surface.value?.eyeDepth ?? 0))

const R = RAYON
const uid = Math.random().toString(36).slice(2, 8)
const clipId = `pet-clip-${uid}`

/**
 * The mouth is two quadratics: the lip, then the opening back to the start. At
 * `open = 0` the two coincide, so only the stroke shows — a line with round ends.
 * Filled *and* stroked with the same path, an open mouth is a D, not a wedge.
 */
const mouthPath = computed(() => {
  const m = props.frame.mouth
  if (!m) return ''
  const hw = m.w / 2
  const top = `Q ${m.x} ${m.y + 2 * m.curve} ${m.x + hw} ${m.y}`
  const back = `Q ${m.x} ${m.y + 2 * (m.curve + m.open)} ${m.x - hw} ${m.y}`
  return `M ${m.x - hw} ${m.y} ${top} ${back} Z`
})

function dotAttrs(dot: BotFrame['dots'][number]) {
  // On a transparent window there is no paper colour to fade towards, so depth
  // is expressed as opacity rather than as a blend into the background.
  const common = { fill: dot.color ?? props.ink, opacity: dot.opacity * (dot.depth ?? 1) }
  return dot.d
    ? {
        ...common,
        d: dot.d,
        transform: `translate(${dot.x} ${dot.y}) rotate(${dot.rot ?? 0}) scale(${R})`
      }
    : { ...common, cx: dot.x, cy: dot.y, r: dot.r }
}
</script>

<template>
  <svg :viewBox="`${-zoom} ${-zoom} ${zoom * 2} ${zoom * 2}`" role="img" aria-label="Dot">
    <defs>
      <!-- Eyes are clipped to the body instead of punched through it. -->
      <clipPath :id="clipId">
        <path :d="frame.bodyPath" />
      </clipPath>

      <linearGradient
        v-for="arc in frame.arcs"
        :id="`${uid}-${arc.id}`"
        :key="arc.id"
        gradientUnits="userSpaceOnUse"
        :x1="arc.grad.x1"
        :y1="arc.grad.y1"
        :x2="arc.grad.x2"
        :y2="arc.grad.y2"
      >
        <stop
          v-for="(c, i) in arc.grad.stops"
          :key="i"
          :offset="i / (arc.grad.stops.length - 1)"
          :stop-color="c"
        />
      </linearGradient>

      <!-- A lit style: the body is a radial gradient, a vertical wash darkens the
           lower edge, and a sheen sits where the light lands. `plat` uses none. -->
      <template v-if="surface">
        <radialGradient
          :id="`${uid}-body`"
          gradientUnits="userSpaceOnUse"
          :cx="surface.lx * R"
          :cy="surface.ly * R"
          :r="surface.reach * R"
          :fx="(surface.lx - 0.14) * R"
          :fy="(surface.ly - 0.16) * R"
        >
          <stop offset="0" :stop-color="surface.light" />
          <stop :offset="surface.mid" :stop-color="surface.base" />
          <stop offset="1" :stop-color="surface.deep" />
        </radialGradient>

        <linearGradient :id="`${uid}-shade`" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0.45" stop-color="#000" stop-opacity="0" />
          <stop offset="1" stop-color="#000" :stop-opacity="surface.shade" />
        </linearGradient>

        <radialGradient :id="`${uid}-spec`">
          <stop offset="0" stop-color="#fff" stop-opacity="1" />
          <stop offset="0.6" stop-color="#fff" stop-opacity="0.22" />
          <stop offset="1" stop-color="#fff" stop-opacity="0" />
        </radialGradient>

        <!-- Eyes are pressed in: the top sits in the rim's shadow. -->
        <linearGradient :id="`${uid}-eye`" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" :stop-color="eyeLid" />
          <stop offset="1" :stop-color="eyeFill" />
        </linearGradient>
      </template>
    </defs>

    <!-- Back half of the orbits, behind the body. -->
    <g v-if="!bare" fill="none" stroke-linecap="round" :opacity="surface ? surface.orbits : 1">
      <path
        v-for="arc in frame.arcs"
        :key="`b${arc.id}`"
        :d="arc.back"
        :stroke="`url(#${uid}-${arc.id})`"
        :stroke-width="arc.width"
        :opacity="arc.opacity"
      />
    </g>

    <g v-if="frame.dotsBehind">
      <component
        :is="dot.d ? 'path' : 'circle'"
        v-for="(dot, i) in frame.dots"
        :key="`pb${i}`"
        v-bind="dotAttrs(dot)"
      />
    </g>

    <g :opacity="frame.bodyAlpha">
      <path
        :class="{ hit: interactive }"
        :d="frame.bodyPath"
        :fill="surface ? `url(#${uid}-body)` : ink"
      />

      <g v-if="surface" :clip-path="`url(#${clipId})`">
        <path :d="frame.bodyPath" :fill="`url(#${uid}-shade)`" />
        <ellipse
          v-if="surface.gloss > 0"
          :cx="(surface.lx - 0.02) * R"
          :cy="(surface.ly - 0.04) * R"
          :rx="surface.glossR * R"
          :ry="surface.glossR * 0.58 * R"
          :transform="`rotate(-32 ${(surface.lx - 0.02) * R} ${(surface.ly - 0.04) * R})`"
          :opacity="surface.gloss"
          :fill="`url(#${uid}-spec)`"
        />
      </g>

      <g :class="{ hit: interactive }" :clip-path="`url(#${clipId})`">
        <path
          v-for="(eye, i) in frame.eyes"
          :key="i"
          :d="eye.d"
          :transform="eye.matrix"
          :opacity="eye.alpha"
          :fill="surface ? `url(#${uid}-eye)` : eyeFill"
        />
        <circle
          v-if="frame.notch"
          :cx="frame.notch.x"
          :cy="frame.notch.y"
          :r="frame.notch.r"
          :fill="eyeFill"
        />
        <path
          v-if="frame.mouth"
          :d="mouthPath"
          :opacity="frame.mouth.alpha"
          :transform="`rotate(${frame.mouth.rot} ${frame.mouth.x} ${frame.mouth.y})`"
          :fill="eyeFill"
          :stroke="eyeFill"
          :stroke-width="frame.mouth.thickness"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </g>
    </g>

    <g v-if="!bare && !frame.dotsBehind">
      <component
        :is="dot.d ? 'path' : 'circle'"
        v-for="(dot, i) in frame.dots"
        :key="`pf${i}`"
        v-bind="dotAttrs(dot)"
      />
    </g>

    <circle
      v-if="!bare && frame.notif"
      :cx="frame.notif.x"
      :cy="frame.notif.y"
      :r="frame.notif.r"
      :fill="NOTIF_BLUE"
    />

    <!-- Front half of the orbits. -->
    <g v-if="!bare" fill="none" stroke-linecap="round" :opacity="surface ? surface.orbits : 1">
      <path
        v-for="arc in frame.arcs"
        :key="`f${arc.id}`"
        :d="arc.front"
        :stroke="`url(#${uid}-${arc.id})`"
        :stroke-width="arc.width"
        :opacity="arc.opacity"
      />
    </g>
  </svg>
</template>

<style scoped>
svg {
  display: block;
  width: 100%;
  height: 100%;
  overflow: visible;
}
</style>
