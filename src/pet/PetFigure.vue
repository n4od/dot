<script setup lang="ts">
import { computed } from 'vue'
import type { BotFrame } from '../bot/engine'
import { NOTIF_BLUE } from '../bot/decor'
import { DEMI_VIEWBOX, RAYON } from '../bot/repere'
import { clayEyeLid, clayMaterial, type StyleId } from './materials'

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
 * `argile` lights the body and recesses the eyes; everything else is unchanged.
 *
 * The style seam: everything outside this branch is shared by every style. A new
 * style is a flag here plus a branch in the template below — never a fork of the
 * component, or the picker would start promising something the pet will not do.
 * The definitions and the full checklist live in `./materials.ts`.
 */
const clay = computed(() => props.styleId === 'argile')
const mat = computed(() => clayMaterial(props.ink))
const eyeLid = computed(() => clayEyeLid(props.eyeFill))

const R = RAYON
const uid = Math.random().toString(36).slice(2, 8)
const clipId = `pet-clip-${uid}`

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

      <!-- Clay: one lit body. The light sits up-left, the specular is a second,
           smaller highlight, and the shade is a vertical wash inside the body. -->
      <template v-if="clay">
        <radialGradient
          :id="`${uid}-body`"
          gradientUnits="userSpaceOnUse"
          :cx="-0.3 * R"
          :cy="-0.32 * R"
          :r="1.5 * R"
          :fx="-0.44 * R"
          :fy="-0.48 * R"
        >
          <stop offset="0" :stop-color="mat.light" />
          <stop offset="0.34" :stop-color="mat.base" />
          <stop offset="1" :stop-color="mat.deep" />
        </radialGradient>

        <linearGradient :id="`${uid}-shade`" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0.45" stop-color="#000" stop-opacity="0" />
          <stop offset="1" stop-color="#000" stop-opacity="0.22" />
        </linearGradient>

        <radialGradient :id="`${uid}-spec`">
          <stop offset="0" stop-color="#fff" stop-opacity="0.55" />
          <stop offset="0.6" stop-color="#fff" stop-opacity="0.12" />
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
    <g v-if="!bare" fill="none" stroke-linecap="round" :opacity="clay ? 0.9 : 1">
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
        :fill="clay ? `url(#${uid}-body)` : ink"
      />

      <g v-if="clay" :clip-path="`url(#${clipId})`">
        <path :d="frame.bodyPath" :fill="`url(#${uid}-shade)`" />
        <ellipse
          :cx="-0.32 * R"
          :cy="-0.36 * R"
          :rx="0.26 * R"
          :ry="0.15 * R"
          :transform="`rotate(-32 ${-0.32 * R} ${-0.36 * R})`"
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
          :fill="clay ? `url(#${uid}-eye)` : eyeFill"
        />
        <circle
          v-if="frame.notch"
          :cx="frame.notch.x"
          :cy="frame.notch.y"
          :r="frame.notch.r"
          :fill="eyeFill"
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
    <g v-if="!bare" fill="none" stroke-linecap="round" :opacity="clay ? 0.9 : 1">
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
