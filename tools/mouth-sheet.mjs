/**
 * Design gate for a mouth on the pet: a throwaway contact sheet.
 *
 * Draws real frames out of `src-tauri/tests/engine-golden.json` (body, eyes), then
 * overlays candidate mouths so the shape can be judged in pixels instead of prose.
 * Nothing here feeds the app; it writes `tools/mouth-sheet.html` and stops.
 *
 *   node tools/mouth-sheet.mjs
 *
 * Every candidate is the same family — a filled lens defined by a width, a drop,
 * a lip thickness and an opening — so whichever wins can be expressed as numbers
 * in the frame contract and interpolated. `cat` is the exception (two lenses) and
 * is shown because it is the obvious alternative, not because it morphs as well.
 */

import { readFileSync, writeFileSync } from 'node:fs'

const golden = JSON.parse(readFileSync('src-tauri/tests/engine-golden.json', 'utf8'))
const byName = new Map(golden.cases.map((c) => [c.name, c]))

/** Contrast fill, ported from `src/contrast.ts`. */
function eyeFillFor(ink) {
  const n = parseInt(ink.replace('#', ''), 16)
  const r = ((n >> 16) & 0xff) / 255
  const g = ((n >> 8) & 0xff) / 255
  const b = (n & 0xff) / 255
  const l = 0.2126 * r + 0.7152 * g + 0.0722 * b
  return l > 0.55 ? '#111116' : '#fdfdfd'
}

function matrixOf(m) {
  const v = m.slice(m.indexOf('(') + 1, m.lastIndexOf(')')).split(',').map(Number)
  return { a: v[0], b: v[1], c: v[2], d: v[3], e: v[4], f: v[5] }
}

/** Face frame from the eyes the engine actually placed. */
function faceOf(frame) {
  const eyes = frame.eyes.map((e) => ({ d: e.d, m: matrixOf(e.matrix) }))
  if (eyes.length < 2) return null
  const ey = eyes.map((e) => e.m.f)
  const ex = (eyes[0].m.e + eyes[1].m.e) / 2
  const eyeY = (ey[0] + ey[1]) / 2
  const u = 2 * 9.3 * Math.abs((Math.abs(eyes[0].m.a) + Math.abs(eyes[1].m.a)) / 2)
  // A mouth does not follow the gaze as far as the eyes do — it drifts with the
  // head, about a third as much — and it sits low on the face, nearer the body
  // centre than the chin.
  const x = ex * 0.75
  const y = eyeY + 0.36 * (100 - eyeY)
  return { eyes, x, y, u }
}

/* ------------------------------------------------------------------ mouths */

const CANDIDATES = {
  // The resting face: barely there, so the expressions can still do the talking.
  line: { family: 'lens', w: 2.2, curve: 0.12, t: 0.3, open: 0 },
  // A soft closed smile.
  smile: { family: 'lens', w: 2.4, curve: 0.45, t: 0.3, open: 0 },
  // A wider, deeper grin, parted.
  grin: { family: 'lens', w: 2.6, curve: 0.5, t: 0.3, open: 0.45 },
  // The open extreme: a round mouth, same family, opening instead of a lip.
  open: { family: 'lens', w: 1.1, curve: 0.15, t: 0.28, open: 0.55 },
  // Two lobes — cute, but the one that blurs first when small.
  cat: { family: 'cat', w: 2.0, h: 0.5, t: 0.3 }
}

const LABELS = {
  line: 'Line — neutral, the resting mouth',
  smile: 'Smile — soft closed lens',
  grin: 'Grin — deeper, wider lens',
  open: 'Open — round, the open extreme',
  cat: 'Cat — two lobes'
}

/**
 * A mouth as a uniform lip with round caps, plus an optional opening below it.
 *
 * The lip is a stroked quadratic — constant thickness, so the ends stay clean at
 * every size. `open` fills the area between the lip and a lower curve, so a mouth
 * can part without switching to a different kind of shape: at `open = 0` the fill
 * is empty and only the lip shows. Both painters can draw this (a stroked path and
 * a filled path), and both numbers interpolate.
 */
function lens(x, y, u, spec) {
  const w = spec.w * u
  const h = spec.curve * u
  const t = spec.t * u
  const open = (spec.open ?? 0) * u
  // One closed path for every candidate: the lip on top, the opening below it.
  // At `open = 0` the two curves coincide, so the fill is empty and only the
  // stroke shows — a line with round ends. Filling and stroking the same path
  // rounds the corners too, so an open mouth is a D rather than a wedge. Both
  // painters can do this, and `open` is just another number to interpolate.
  const d =
    `M ${x - w / 2} ${y} Q ${x} ${y + 2 * h} ${x + w / 2} ${y} ` +
    `Q ${x} ${y + 2 * (h + open)} ${x - w / 2} ${y} Z`
  return { d, stroke: t }
}

function catMouth(x, y, u, spec) {
  const w = spec.w * u
  const h = spec.h * u
  const t = spec.t * u
  const half = w / 2
  const d = [
    `M ${x - half} ${y}`,
    `Q ${x - half / 2} ${y + 2 * h} ${x} ${y}`,
    `Q ${x + half / 2} ${y + 2 * h} ${x + half} ${y}`
  ].join(' ')
  return { d, stroke: t }
}

function mouthMarkup(kind, face, ink) {
  const fill = eyeFillFor(ink)
  const spec = CANDIDATES[kind]
  if (spec.family === 'cat') {
    const m = catMouth(face.x, face.y, face.u, spec)
    return `<path d="${m.d}" fill="none" stroke="${fill}" stroke-width="${m.stroke}" stroke-linecap="round" stroke-linejoin="round"/>`
  }
  const m = lens(face.x, face.y, face.u, spec)
  return `<path d="${m.d}" fill="${fill}" stroke="${fill}" stroke-width="${m.stroke}" stroke-linejoin="round" stroke-linecap="round"/>`
}

/* -------------------------------------------------------------- rendering */

function cell(caseName, kind, ink, px) {
  const c = byName.get(caseName)
  if (!c) throw new Error(`missing case ${caseName}`)
  const f = c.frame
  const face = faceOf(f)
  if (!face) return `<div class="cell empty">no eyes</div>`
  const mouthY = face.y
  const bg = ink === '#f1efe9' ? '#2a2a30' : '#f4f4f6'
  return `<div class="cell" style="background:${bg}">
    <svg viewBox="-158 -158 316 316" width="${px}" height="${px}">
      <path d="${f.bodyPath}" fill="${ink}"/>
      ${f.eyes
        .map((e) => `<path d="${e.d}" transform="${e.matrix}" fill="${eyeFillFor(ink)}"/>`)
        .join('')}
      ${mouthMarkup(kind, { ...face, y: mouthY }, ink)}
    </svg>
  </div>`
}

const SHAPES = ['cercle', 'galet', 'squircle', 'capsule', 'triangle', 'hexagone', 'nuage', 'goutte']
const KINDS = Object.keys(CANDIDATES)

/** A crop around the face, so the mouth can be judged as a shape rather than a smudge. */
function detailCell(caseName, kind, ink, half, px) {
  const c = byName.get(caseName)
  const f = c.frame
  const face = faceOf(f)
  const bg = ink === '#f1efe9' ? '#2a2a30' : '#f4f4f6'
  return `<div class="cell" style="background:${bg}">
    <svg viewBox="${face.x - half} ${face.y - half - 30} ${half * 2} ${half * 2}" width="${px}" height="${px}">
      <path d="${f.bodyPath}" fill="${ink}"/>
      ${f.eyes
        .map((e) => `<path d="${e.d}" transform="${e.matrix}" fill="${eyeFillFor(ink)}"/>`)
        .join('')}
      ${mouthMarkup(kind, face, ink)}
    </svg>
  </div>`
}

const closeUp = `
<table>
  <thead><tr><th></th>${KINDS.map((k) => `<th>${LABELS[k]}</th>`).join('')}</tr></thead>
  <tbody>
    <tr><th class="row">full — 180 px</th>${KINDS.map(
      (k) => `<td>${cell('shape:cercle@0.7', k, '#0a0a0c', 180)}</td>`
    ).join('')}</tr>
    <tr><th class="row">face, zoomed</th>${KINDS.map(
      (k) => `<td>${detailCell('shape:cercle@0.7', k, '#0a0a0c', 62, 180)}</td>`
    ).join('')}</tr>
  </tbody>
</table>`

const shapeGrid = `
<table>
  <thead><tr><th></th>${SHAPES.map((s) => `<th>${s}</th>`).join('')}</tr></thead>
  <tbody>
    ${KINDS.map(
      (k) => `<tr><th class="row">${LABELS[k]}</th>${SHAPES.map(
        (s) => `<td>${cell(`shape:${s}@0.7`, k, '#0a0a0c', 180)}</td>`
      ).join('')}</tr>`
    ).join('')}
  </tbody>
</table>`

const sizeGrid = `
<table>
  <thead><tr><th></th><th>180 px — the desktop</th><th>96 px</th><th>42 px — the expression thumbnails</th></tr></thead>
  <tbody>
    ${KINDS.map(
      (k) => `<tr><th class="row">${LABELS[k]}</th>${[180, 96, 42]
        .map((px) => `<td>${cell('state:idle@1', k, '#0a0a0c', px)}</td>`)
        .join('')}</tr>`
    ).join('')}
  </tbody>
</table>`

const contrastGrid = `
<table>
  <thead><tr><th></th><th>Ink (the default)</th><th>Cream — the fill flips</th><th>Red</th></tr></thead>
  <tbody>
    ${KINDS.map(
      (k) => `<tr><th class="row">${LABELS[k]}</th>${['#0a0a0c', '#f1efe9', '#e8483f']
        .map((ink) => `<td>${cell('state:idle@1', k, ink, 150)}</td>`)
        .join('')}</tr>`
    ).join('')}
  </tbody>
</table>`

const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Dot — mouth candidates</title>
<style>
  :root { color-scheme: light; }
  body { margin: 0; padding: 28px 32px 64px; background: #fbfbfc; color: #1b1b21;
         font: 14px/1.5 "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif; }
  h1 { font: 600 20px/1.3 Georgia, serif; margin: 0 0 4px; }
  h2 { font: 600 15px/1.4 "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
       margin: 34px 0 10px; padding-bottom: 6px; border-bottom: 1px solid #e3e3e8; }
  p { margin: 6px 0; color: #55555f; max-width: 74ch; }
  table { border-collapse: separate; border-spacing: 8px; margin-left: -8px; }
  th { font-weight: 600; font-size: 11.5px; color: #6b6b76; text-align: center;
       text-transform: uppercase; letter-spacing: .04em; }
  th.row { text-align: left; text-transform: none; letter-spacing: 0;
           font-size: 12.5px; color: #2c2c34; white-space: nowrap; padding-right: 6px; }
  td { padding: 0; }
  .cell { border-radius: 10px; overflow: hidden; line-height: 0; }
  .cell.empty { width: 150px; height: 150px; display: grid; place-items: center;
                color: #999; font-size: 12px; background: #f0f0f3; }
  svg { display: block; }
  .note { margin-top: 30px; padding: 12px 14px; background: #fff8e6; border: 1px solid #efe0b8;
          border-radius: 8px; color: #5a4a1c; font-size: 12.5px; }
  code { font: 12px ui-monospace, Consolas, monospace; }
</style>
</head>
<body>
  <h1>Dot — mouth candidates</h1>
  <p>Real engine frames (body and eyes from the golden set), with a candidate mouth
  overlaid. Nothing here is implemented; this is the design gate.</p>

  <h2>The candidates, on the default shape</h2>
  <p>Top row: at the desktop's real 180 px. Bottom row: the same frames cropped to
  the face, to judge the mouth as a shape.</p>
  ${closeUp}

  <h2>Across the eight shapes</h2>
  <p>The mouth is placed from the eyes the engine actually placed, so it follows the
  face on each silhouette rather than sitting at a fixed point.</p>
  ${shapeGrid}

  <h2>Across sizes</h2>
  <p>The desktop pet is 180 px. The settings' expression thumbnails are 42 px, where a
  mouth is under a pixel thick — this is the honest view of what survives.</p>
  ${sizeGrid}

  <h2>Across colours</h2>
  <p>The mouth takes the same contrast fill as the eyes, so it has to work when that
  fill flips.</p>
  ${contrastGrid}

  <div class="note">
    <strong>How these are built.</strong> Three are one family — a filled lens with a
    width, a drop, a lip thickness and an opening — so they interpolate like the eyes.
    <code>cat</code> is two lenses and is shown as the alternative, not because it
    morphs as well. Tips are round caps (<code>t/2</code> discs), which both painters
    can draw.
  </div>
</body>
</html>
`

writeFileSync('tools/mouth-sheet.html', html)
console.log(`wrote tools/mouth-sheet.html  (${KINDS.length} candidates x ${SHAPES.length} shapes)`)
