// Rasterizes the PWA icon set into public/icons without an image toolchain.
// Run `node scripts/generate-pwa-icons.mjs` after changing the artwork below.
import { mkdirSync, writeFileSync } from 'node:fs'
import { deflateSync } from 'node:zlib'

const BACKGROUND = [0x14, 0x14, 0x16]
const ACCENT = [0x4a, 0x9e, 0xff]
// Artwork in a 512-unit square: an isometric cube outline plus its three inner edges.
const OUTLINE = [
  [256, 96],
  [396, 176],
  [396, 336],
  [256, 416],
  [116, 336],
  [116, 176],
]
const INNER = [
  [256, 96],
  [396, 176],
  [116, 176],
]

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
  return c >>> 0
})

function crc32(bytes) {
  let c = 0xffffffff
  for (const byte of bytes) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8)
  return (c ^ 0xffffffff) >>> 0
}

function chunk(type, data) {
  const out = Buffer.alloc(12 + data.length)
  out.writeUInt32BE(data.length, 0)
  out.write(type, 4, 'ascii')
  data.copy(out, 8)
  out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length)
  return out
}

function encodePng(size, rgba) {
  const header = Buffer.alloc(13)
  header.writeUInt32BE(size, 0)
  header.writeUInt32BE(size, 4)
  header.set([8, 6, 0, 0, 0], 8)
  const raw = Buffer.alloc(size * (size * 4 + 1))
  for (let y = 0; y < size; y++) rgba.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4)
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
  return Buffer.concat([
    signature,
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

function segmentDistance(px, py, [ax, ay], [bx, by]) {
  const dx = bx - ax
  const dy = by - ay
  const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
  return Math.hypot(px - (ax + t * dx), py - (ay + t * dy))
}

function insideRoundedRect(x, y, radius) {
  const cx = Math.min(Math.max(x, radius), 512 - radius)
  const cy = Math.min(Math.max(y, radius), 512 - radius)
  return Math.hypot(x - cx, y - cy) <= radius
}

/** Coverage-sampled icon; `artScale` shrinks the cube toward the centre (maskable safe zone). */
function render(size, { cornerRadius, artScale }) {
  const samples = 4
  const rgba = Buffer.alloc(size * size * 4)
  const art = point => [256 + (point[0] - 256) * artScale, 256 + (point[1] - 256) * artScale]
  const outline = OUTLINE.map(art)
  const inner = INNER.map(art)
  const centre = art([256, 256])
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      let alpha = 0
      let accent = 0
      for (let sy = 0; sy < samples; sy++) {
        for (let sx = 0; sx < samples; sx++) {
          const ux = ((x + (sx + 0.5) / samples) / size) * 512
          const uy = ((y + (sy + 0.5) / samples) / size) * 512
          if (!insideRoundedRect(ux, uy, cornerRadius)) continue
          alpha++
          const edge = outline.some(
            (p, i) => segmentDistance(ux, uy, p, outline[(i + 1) % outline.length]) <= 12 * artScale,
          )
          if (edge) accent += 1
          else if (inner.some(p => segmentDistance(ux, uy, centre, p) <= 8 * artScale)) accent += 0.6
        }
      }
      const mix = alpha ? accent / alpha : 0
      const offset = (y * size + x) * 4
      for (let c = 0; c < 3; c++) rgba[offset + c] = Math.round(BACKGROUND[c] + (ACCENT[c] - BACKGROUND[c]) * mix)
      rgba[offset + 3] = Math.round((alpha / (samples * samples)) * 255)
    }
  }
  return encodePng(size, rgba)
}

const outDir = new URL('../public/icons/', import.meta.url)
mkdirSync(outDir, { recursive: true })
const icons = {
  'icon-192.png': render(192, { cornerRadius: 64, artScale: 1 }),
  'icon-512.png': render(512, { cornerRadius: 64, artScale: 1 }),
  // Maskable and Apple icons are full-bleed; the art stays inside the 80% safe-zone circle.
  'icon-maskable-512.png': render(512, { cornerRadius: 0, artScale: 0.72 }),
  'apple-touch-icon.png': render(180, { cornerRadius: 0, artScale: 0.82 }),
}
for (const [name, bytes] of Object.entries(icons)) {
  writeFileSync(new URL(name, outDir), bytes)
  console.log(`${name}: ${bytes.length} bytes`)
}
