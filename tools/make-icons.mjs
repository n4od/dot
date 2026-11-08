import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { deflateSync } from 'node:zlib'

/**
 * Generates the app + tray icons without pulling an image library in.
 *
 * The mark is the pet itself: a filled disc with two tilted white capsules for
 * eyes, drawn with a 4x4 supersample so it still reads at 16 px.
 */

const here = dirname(fileURLToPath(import.meta.url))
const out = resolve(here, '../src-tauri/icons')
mkdirSync(out, { recursive: true })

const CRC = new Int32Array(256)
for (let n = 0; n < 256; n++) {
  let c = n
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
  CRC[n] = c
}

function crc32(buf) {
  let c = -1
  for (const b of buf) c = CRC[(c ^ b) & 0xff] ^ (c >>> 8)
  return (c ^ -1) >>> 0
}

function chunk(type, data) {
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length)
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data])
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(body))
  return Buffer.concat([len, body, crc])
}

/** Squared distance from point to a segment, for the capsule eyes. */
function segDist(px, py, ax, ay, bx, by) {
  const dx = bx - ax
  const dy = by - ay
  const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy || 1)))
  const x = ax + dx * t
  const y = ay + dy * t
  return (px - x) ** 2 + (py - y) ** 2
}

function shade(size) {
  const SS = 4
  const px = new Float32Array(size * size * 4)
  const R = size * 0.47
  const c = size / 2
  const eyeW = size * 0.115
  const eyeH = size * 0.34

  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      let body = 0
      let eye = 0
      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const fx = x + (sx + 0.5) / SS
          const fy = y + (sy + 0.5) / SS
          const inBody = (fx - c) ** 2 + (fy - c) ** 2 <= R * R
          if (!inBody) continue
          body++
          // Two capsules leaning "\\", offset left and right of centre.
          const lean = size * 0.055
          const ex = fx < c ? c - size * 0.155 : c + size * 0.155
          const top = fy < c ? lean : -lean
          const d = segDist(fx, fy, ex + top, c - eyeH / 2, ex - top, c + eyeH / 2)
          if (d <= (eyeW / 2) ** 2) eye++
        }
      }
      const total = SS * SS
      const i = (y * size + x) * 4
      px[i] = 10
      px[i + 1] = 10
      px[i + 2] = 12
      const a = body / total
      // Blend towards white where the eye mask covers the body.
      const e = a > 0 ? Math.min(1, eye / body) : 0
      px[i] = px[i] * (1 - e) + 253 * e
      px[i + 1] = px[i + 1] * (1 - e) + 253 * e
      px[i + 2] = px[i + 2] * (1 - e) + 253 * e
      px[i + 3] = a
    }
  }
  return px
}

function encodePng(size) {
  const rgb = shade(size)
  const raw = Buffer.alloc(size * (size * 4 + 1))
  let o = 0
  for (let y = 0; y < size; y++) {
    raw[o++] = 0 // filter: none
    for (let x = 0; x < size; x++) {
      const i = (y * size + x) * 4
      raw[o++] = Math.round(rgb[i])
      raw[o++] = Math.round(rgb[i + 1])
      raw[o++] = Math.round(rgb[i + 2])
      raw[o++] = Math.round(rgb[i + 3] * 255)
    }
  }
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(size, 0)
  ihdr.writeUInt32BE(size, 4)
  ihdr[8] = 8 // bit depth
  ihdr[9] = 6 // RGBA
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0))
  ])
}

function encodeIco(png) {
  const header = Buffer.alloc(6)
  header.writeUInt16LE(0, 0)
  header.writeUInt16LE(1, 2) // type: icon
  header.writeUInt16LE(1, 4)
  const entry = Buffer.alloc(16)
  entry[0] = 0 // 0 means 256
  entry[1] = 0
  entry[4] = 1 // colour planes
  entry[6] = 32 // bits per pixel
  entry.writeUInt32LE(png.length, 8)
  entry.writeUInt32LE(22, 12)
  return Buffer.concat([header, entry, png])
}

const sizes = { '32x32.png': 32, '128x128.png': 128, '128x128@2x.png': 256, 'icon.png': 512, 'tray.png': 32 }
for (const [name, size] of Object.entries(sizes)) {
  writeFileSync(resolve(out, name), encodePng(size))
}
writeFileSync(resolve(out, 'icon.ico'), encodeIco(encodePng(256)))
console.log(`icons written to ${out}`)