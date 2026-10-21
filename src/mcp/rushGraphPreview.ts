import { deflateSync } from 'node:zlib'
import type { MeshData } from '../core/mesh'
const SIZE = 192
export const RUSH_GRAPH_PREVIEW_TRIANGLE_LIMIT = 20000
const CRC32_TABLE = (() => {
  const table = new Uint32Array(256)
  for (let n = 0; n < 256; n++) {
    let c = n
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1))
    table[n] = c >>> 0
  }
  return table
})()

function crc32(bytes: Uint8Array) {
  let crc = 0xffffffff
  for (let i = 0; i < bytes.length; i++) crc = (crc >>> 8) ^ CRC32_TABLE[(crc ^ bytes[i]!) & 0xff]!
  return (crc ^ 0xffffffff) >>> 0
}
function chunk(name: string, data: Buffer) {
  const type = Buffer.from(name), size = Buffer.alloc(4), crc = Buffer.alloc(4)
  size.writeUInt32BE(data.length); crc.writeUInt32BE(crc32(Buffer.concat([type, data])))
  return Buffer.concat([size, type, data, crc])
}
function png(pixels: Buffer) {
  const header = Buffer.alloc(13); header.writeUInt32BE(SIZE, 0); header.writeUInt32BE(SIZE, 4); header[8] = 8; header[9] = 2
  const rows = Buffer.alloc(SIZE * (1 + SIZE * 3))
  for (let y = 0; y < SIZE; y++) pixels.copy(rows, y * (1 + SIZE * 3) + 1, y * SIZE * 3, (y + 1) * SIZE * 3)
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', header), chunk('IDAT', deflateSync(rows)), chunk('IEND', Buffer.alloc(0))])
}
type P = [number, number, number]
const INV_SQRT2 = 1 / Math.sqrt(2), INV_SQRT6 = 1 / Math.sqrt(6), INV_SQRT3 = 1 / Math.sqrt(3)
/** Orthographic CPU snapshots of actual triangles, independent of a browser/GPU. */
export function renderRushGraphPreviews(meshes: Pick<MeshData, 'vertices' | 'indices' | 'transform'>[]) {
  const count = meshes.reduce((n, mesh) => n + mesh.indices.length / 3, 0)
  if (count > RUSH_GRAPH_PREVIEW_TRIANGLE_LIMIT) throw new Error(`Preview triangle limit (${RUSH_GRAPH_PREVIEW_TRIANGLE_LIMIT}) exceeded.`)
  const triangles = new Float64Array(count * 9)
  let triOffset = 0
  for (const mesh of meshes) {
    const t = mesh.transform
    const t0 = t[0]!, t1 = t[1]!, t2 = t[2]!, t3 = t[3]!
    const t4 = t[4]!, t5 = t[5]!, t6 = t[6]!, t7 = t[7]!
    const t8 = t[8]!, t9 = t[9]!, t10 = t[10]!, t11 = t[11]!
    const verts = mesh.vertices, idx = mesh.indices
    for (let i = 0; i < idx.length; i += 3) {
      const n0 = idx[i]! * 6, x0 = verts[n0]!, y0 = verts[n0 + 1]!, z0 = verts[n0 + 2]!
      const n1 = idx[i + 1]! * 6, x1 = verts[n1]!, y1 = verts[n1 + 1]!, z1 = verts[n1 + 2]!
      const n2 = idx[i + 2]! * 6, x2 = verts[n2]!, y2 = verts[n2 + 1]!, z2 = verts[n2 + 2]!
      triangles[triOffset] = t0 * x0 + t1 * y0 + t2 * z0 + t3
      triangles[triOffset + 1] = t4 * x0 + t5 * y0 + t6 * z0 + t7
      triangles[triOffset + 2] = t8 * x0 + t9 * y0 + t10 * z0 + t11
      triangles[triOffset + 3] = t0 * x1 + t1 * y1 + t2 * z1 + t3
      triangles[triOffset + 4] = t4 * x1 + t5 * y1 + t6 * z1 + t7
      triangles[triOffset + 5] = t8 * x1 + t9 * y1 + t10 * z1 + t11
      triangles[triOffset + 6] = t0 * x2 + t1 * y2 + t2 * z2 + t3
      triangles[triOffset + 7] = t4 * x2 + t5 * y2 + t6 * z2 + t7
      triangles[triOffset + 8] = t8 * x2 + t9 * y2 + t10 * z2 + t11
      triOffset += 9
    }
  }
  const views = [
    { name: 'front', kind: 0 },
    { name: 'top', kind: 1 },
    { name: 'isometric', kind: 2 },
  ] as const
  const projected = new Float64Array(count * 9)
  let work = 0
  return views.map(view => {
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
    const kind = view.kind
    for (let base = 0; base < count * 9; base += 3) {
      const x = triangles[base]!, y = triangles[base + 1]!, z = triangles[base + 2]!
      let px: number, py: number, pz: number
      if (kind === 0) {
        px = x; py = z; pz = -y
      } else if (kind === 1) {
        px = x; py = y; pz = z
      } else {
        px = (x - y) * INV_SQRT2
        py = (-x - y + 2 * z) * INV_SQRT6
        pz = (x + y + z) * INV_SQRT3
      }
      projected[base] = px; projected[base + 1] = py; projected[base + 2] = pz
      if (px < minX) minX = px; if (px > maxX) maxX = px
      if (py < minY) minY = py; if (py > maxY) maxY = py
    }
    const scale = (SIZE - 24) / Math.max(maxX - minX, maxY - minY, 1e-9)
    const midX = (minX + maxX) / 2, midY = (minY + maxY) / 2, halfSize = SIZE / 2
    const pixels = Buffer.alloc(SIZE * SIZE * 3, 245), depths = new Float64Array(SIZE * SIZE).fill(-Infinity)
    for (let base = 0; base < count * 9; base += 9) {
      const p0 = projected[base]!, p1 = projected[base + 1]!, p2 = projected[base + 2]!
      const q0 = projected[base + 3]!, q1 = projected[base + 4]!, q2 = projected[base + 5]!
      const r0 = projected[base + 6]!, r1 = projected[base + 7]!, r2 = projected[base + 8]!
      const a0 = (p0 - midX) * scale + halfSize, a1 = halfSize - (p1 - midY) * scale, a2 = p2
      const b0 = (q0 - midX) * scale + halfSize, b1 = halfSize - (q1 - midY) * scale, b2 = q2
      const c0 = (r0 - midX) * scale + halfSize, c1 = halfSize - (r1 - midY) * scale, c2 = r2
      const area = (b1 - c1) * (a0 - c0) + (c0 - b0) * (a1 - c1)
      if (Math.abs(area) < 1e-10) continue
      const loX = Math.max(0, Math.floor(Math.min(a0, b0, c0))), hiX = Math.min(SIZE - 1, Math.ceil(Math.max(a0, b0, c0)))
      const loY = Math.max(0, Math.floor(Math.min(a1, b1, c1))), hiY = Math.min(SIZE - 1, Math.ceil(Math.max(a1, b1, c1)))
      work += (hiX - loX + 1) * (hiY - loY + 1)
      if (work > 8_000_000) throw new Error('Preview raster work budget exceeded.')
      const nx = (q1 - p1) * (r2 - p2) - (q2 - p2) * (r1 - p1)
      const ny = (q2 - p2) * (r0 - p0) - (q0 - p0) * (r2 - p2)
      const nz = (q0 - p0) * (r1 - p1) - (q1 - p1) * (r0 - p0)
      const length = Math.hypot(nx, ny, nz), facing = nz < 0 ? -1 : 1
      const light = Math.max(0, facing * (nx * -0.3 + ny * 0.6 + nz * 0.74) / length)
      const shade = Math.round(75 + 130 * light)
      for (let y = loY; y <= hiY; y++) for (let x = loX; x <= hiX; x++) {
        const u = ((b1 - c1) * (x + 0.5 - c0) + (c0 - b0) * (y + 0.5 - c1)) / area
        const v = ((c1 - a1) * (x + 0.5 - c0) + (a0 - c0) * (y + 0.5 - c1)) / area
        if (u < 0 || v < 0 || u + v > 1) continue
        const depth = u * a2 + v * b2 + (1 - u - v) * c2, index = y * SIZE + x
        if (depth <= depths[index]!) continue
        depths[index] = depth; pixels[index * 3] = shade - 30; pixels[index * 3 + 1] = shade; pixels[index * 3 + 2] = Math.min(255, shade + 30)
      }
    }
    return { view: view.name, png: png(pixels) }
  })
}
