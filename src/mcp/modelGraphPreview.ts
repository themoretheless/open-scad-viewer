import { deflateSync } from 'node:zlib'
import type { MeshData } from '../core/mesh'
const SIZE = 192
function crc32(bytes: Uint8Array) {
  let crc = 0xffffffff
  for (const byte of bytes) { crc ^= byte; for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1)) }
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
/** Orthographic CPU snapshots of actual triangles, independent of a browser/GPU. */
export function renderModelGraphPreviews(meshes: Pick<MeshData, 'vertices' | 'indices' | 'transform'>[]) {
  const count = meshes.reduce((n, mesh) => n + mesh.indices.length / 3, 0)
  if (count > 20000) throw new Error('Preview triangle limit (20000) exceeded.')
  const triangles: P[][] = []
  for (const mesh of meshes) for (let i = 0; i < mesh.indices.length; i += 3) {
    const t = mesh.transform
    triangles.push([0, 1, 2].map(j => {
      const n = mesh.indices[i + j]! * 6, x = mesh.vertices[n]!, y = mesh.vertices[n + 1]!, z = mesh.vertices[n + 2]!
      return [t[0]! * x + t[1]! * y + t[2]! * z + t[3]!, t[4]! * x + t[5]! * y + t[6]! * z + t[7]!, t[8]! * x + t[9]! * y + t[10]! * z + t[11]!] as P
    }))
  }
  const views = [
    { name: 'front', project: ([x, y, z]: P): P => [x, z, -y] },
    { name: 'top', project: ([x, y, z]: P): P => [x, y, z] },
    { name: 'isometric', project: ([x, y, z]: P): P => [(x - y) / Math.sqrt(2), (-x - y + 2 * z) / Math.sqrt(6), (x + y + z) / Math.sqrt(3)] },
  ]
  let work = 0
  return views.map(view => {
    const projected = triangles.map(tri => tri.map(view.project))
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
    for (const triangle of projected) for (const [x, y] of triangle) { minX = Math.min(minX, x); maxX = Math.max(maxX, x); minY = Math.min(minY, y); maxY = Math.max(maxY, y) }
    const scale = (SIZE - 24) / Math.max(maxX - minX, maxY - minY, 1e-9)
    const pixels = Buffer.alloc(SIZE * SIZE * 3, 245), depths = new Float64Array(SIZE * SIZE).fill(-Infinity)
    for (const triangle of projected) {
      const [a, b, c] = triangle.map(([x, y, z]) => [(x - (minX + maxX) / 2) * scale + SIZE / 2, SIZE / 2 - (y - (minY + maxY) / 2) * scale, z] as P) as [P, P, P]
      const area = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
      if (Math.abs(area) < 1e-10) continue
      const loX = Math.max(0, Math.floor(Math.min(a[0], b[0], c[0]))), hiX = Math.min(SIZE - 1, Math.ceil(Math.max(a[0], b[0], c[0])))
      const loY = Math.max(0, Math.floor(Math.min(a[1], b[1], c[1]))), hiY = Math.min(SIZE - 1, Math.ceil(Math.max(a[1], b[1], c[1])))
      work += (hiX - loX + 1) * (hiY - loY + 1)
      if (work > 8_000_000) throw new Error('Preview raster work budget exceeded.')
      const [p, q, r] = triangle as [P, P, P]
      const normal = [(q[1]-p[1])*(r[2]-p[2])-(q[2]-p[2])*(r[1]-p[1]), (q[2]-p[2])*(r[0]-p[0])-(q[0]-p[0])*(r[2]-p[2]), (q[0]-p[0])*(r[1]-p[1])-(q[1]-p[1])*(r[0]-p[0])]
      const length = Math.hypot(...normal), facing = normal[2]! < 0 ? -1 : 1
      const light = Math.max(0, facing * (normal[0]! * -0.3 + normal[1]! * 0.6 + normal[2]! * 0.74) / length)
      const shade = Math.round(75 + 130 * light)
      for (let y = loY; y <= hiY; y++) for (let x = loX; x <= hiX; x++) {
        const u = ((b[1] - c[1]) * (x + 0.5 - c[0]) + (c[0] - b[0]) * (y + 0.5 - c[1])) / area
        const v = ((c[1] - a[1]) * (x + 0.5 - c[0]) + (a[0] - c[0]) * (y + 0.5 - c[1])) / area
        if (u < 0 || v < 0 || u + v > 1) continue
        const depth = u * a[2] + v * b[2] + (1 - u - v) * c[2], index = y * SIZE + x
        if (depth <= depths[index]!) continue
        depths[index] = depth; pixels[index * 3] = shade - 30; pixels[index * 3 + 1] = shade; pixels[index * 3 + 2] = Math.min(255, shade + 30)
      }
    }
    return { view: view.name, png: png(pixels) }
  })
}
