/**
 * Binary STL export from MeshData[].
 * Format: 80-byte header, 4-byte triangle count (uint32),
 * then per triangle: normal (3×f32), v1 (3×f32), v2 (3×f32), v3 (3×f32), 2-byte attribute.
 */
import type { MeshData } from '../core/mesh'
import type { Mat4 } from './math3d'

/** Apply a row-major 4x4 transform to a 3D point, writing into scratch locals. */
// The transform and degeneracy math is inlined into both passes below via these
// scratch variables, so no per-triangle [x, y, z] arrays are allocated.
let tx0 = 0, ty0 = 0, tz0 = 0, tx1 = 0, ty1 = 0, tz1 = 0, tx2 = 0, ty2 = 0, tz2 = 0

function transformInto(t: Mat4, x0: number, y0: number, z0: number, x1: number, y1: number, z1: number, x2: number, y2: number, z2: number) {
  tx0 = t[0] * x0 + t[1] * y0 + t[2] * z0 + t[3]
  ty0 = t[4] * x0 + t[5] * y0 + t[6] * z0 + t[7]
  tz0 = t[8] * x0 + t[9] * y0 + t[10] * z0 + t[11]
  tx1 = t[0] * x1 + t[1] * y1 + t[2] * z1 + t[3]
  ty1 = t[4] * x1 + t[5] * y1 + t[6] * z1 + t[7]
  tz1 = t[8] * x1 + t[9] * y1 + t[10] * z1 + t[11]
  tx2 = t[0] * x2 + t[1] * y2 + t[2] * z2 + t[3]
  ty2 = t[4] * x2 + t[5] * y2 + t[6] * z2 + t[7]
  tz2 = t[8] * x2 + t[9] * y2 + t[10] * z2 + t[11]
}

/** True if the three transformed scratch points form a zero-area triangle. */
function scratchIsDegenerate(): boolean {
  const ux = tx1 - tx0, uy = ty1 - ty0, uz = tz1 - tz0
  const vx = tx2 - tx0, vy = ty2 - ty0, vz = tz2 - tz0
  const cx = uy * vz - uz * vy
  const cy = uz * vx - ux * vz
  const cz = ux * vy - uy * vx
  return Math.sqrt(cx * cx + cy * cy + cz * cz) < 1e-10
}

/** Determinant of the upper-left 3x3 of a row-major 4x4 transform. */
function determinant3(t: Mat4): number {
  const a = t[0], b = t[1], c = t[2]
  const d = t[4], e = t[5], f = t[6]
  const g = t[8], h = t[9], i = t[10]
  return a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
}

/**
 * Build a binary STL file from mesh data and return the raw ArrayBuffer.
 * (Pure / no DOM — safe to call in any environment.)
 *
 * Mirrored meshes (negative-determinant transform) have their winding reversed
 * so facets stay outward-facing. Degenerate (zero-area) triangles, incomplete
 * trailing triangles, and triangles with out-of-range indices are skipped.
 */
export function buildSTLBuffer(meshes: MeshData[]): ArrayBuffer {
  // First pass: count valid (writable) triangles so the buffer and header
  // count match exactly what we emit.
  let totalTriangles = 0
  for (const m of meshes) {
    const verts = m.vertices
    const indices = m.indices
    const t = m.transform
    const vertCount = verts.length / 6
    const flip = determinant3(t) < 0

    for (let i = 0; i + 2 < indices.length; i += 3) {
      let i0 = indices[i], i1 = indices[i + 1], i2 = indices[i + 2]
      if (i0 >= vertCount || i1 >= vertCount || i2 >= vertCount) continue
      if (flip) { const tmp = i1; i1 = i2; i2 = tmp }

      const x0 = verts[i0 * 6], y0 = verts[i0 * 6 + 1], z0 = verts[i0 * 6 + 2]
      const x1 = verts[i1 * 6], y1 = verts[i1 * 6 + 1], z1 = verts[i1 * 6 + 2]
      const x2 = verts[i2 * 6], y2 = verts[i2 * 6 + 1], z2 = verts[i2 * 6 + 2]
      transformInto(t, x0, y0, z0, x1, y1, z1, x2, y2, z2)
      if (scratchIsDegenerate()) continue
      totalTriangles++
    }
  }

  // Binary STL: 80-byte header + 4-byte count + (50 bytes per triangle)
  const bufferSize = 84 + totalTriangles * 50
  const buffer = new ArrayBuffer(bufferSize)
  const view = new DataView(buffer)

  // Write header (80 bytes, ASCII text)
  const header = 'Binary STL exported from OpenSCAD 3D Viewer'
  for (let i = 0; i < 80; i++) {
    view.setUint8(i, i < header.length ? header.charCodeAt(i) : 0)
  }

  // Write triangle count
  view.setUint32(80, totalTriangles, true) // little-endian

  let offset = 84
  for (const m of meshes) {
    const verts = m.vertices // interleaved: pos(3) + normal(3)
    const indices = m.indices
    const t = m.transform
    const vertCount = verts.length / 6
    // A negative-determinant transform (mirror / scale(-1)) flips winding, so
    // reverse vertex order to keep facets outward-facing.
    const flip = determinant3(t) < 0

    for (let i = 0; i + 2 < indices.length; i += 3) {
      let i0 = indices[i], i1 = indices[i + 1], i2 = indices[i + 2]

      // Skip triangles referencing out-of-range vertices.
      if (i0 >= vertCount || i1 >= vertCount || i2 >= vertCount) continue

      if (flip) { const tmp = i1; i1 = i2; i2 = tmp }

      // Read raw vertex positions
      const x0 = verts[i0 * 6], y0 = verts[i0 * 6 + 1], z0 = verts[i0 * 6 + 2]
      const x1 = verts[i1 * 6], y1 = verts[i1 * 6 + 1], z1 = verts[i1 * 6 + 2]
      const x2 = verts[i2 * 6], y2 = verts[i2 * 6 + 1], z2 = verts[i2 * 6 + 2]

      // Apply transform into scratch locals (no per-triangle allocations).
      transformInto(t, x0, y0, z0, x1, y1, z1, x2, y2, z2)

      // Skip degenerate (zero-area) triangles rather than emitting a zero normal.
      if (scratchIsDegenerate()) continue

      // Compute face normal from (possibly reversed) transformed vertices.
      const ux = tx1 - tx0, uy = ty1 - ty0, uz = tz1 - tz0
      const vx = tx2 - tx0, vy = ty2 - ty0, vz = tz2 - tz0
      let nx = uy * vz - uz * vy
      let ny = uz * vx - ux * vz
      let nz = ux * vy - uy * vx
      const len = Math.sqrt(nx * nx + ny * ny + nz * nz)
      if (len > 1e-10) { nx /= len; ny /= len; nz /= len }

      // Write normal
      view.setFloat32(offset, nx, true); offset += 4
      view.setFloat32(offset, ny, true); offset += 4
      view.setFloat32(offset, nz, true); offset += 4

      // Write vertex 1
      view.setFloat32(offset, tx0, true); offset += 4
      view.setFloat32(offset, ty0, true); offset += 4
      view.setFloat32(offset, tz0, true); offset += 4

      // Write vertex 2
      view.setFloat32(offset, tx1, true); offset += 4
      view.setFloat32(offset, ty1, true); offset += 4
      view.setFloat32(offset, tz1, true); offset += 4

      // Write vertex 3
      view.setFloat32(offset, tx2, true); offset += 4
      view.setFloat32(offset, ty2, true); offset += 4
      view.setFloat32(offset, tz2, true); offset += 4

      // Attribute byte count (unused, set to 0)
      view.setUint16(offset, 0, true); offset += 2
    }
  }

  return buffer
}

/**
 * Generate a binary STL file from mesh data and trigger a download.
 */
export function exportSTL(meshes: MeshData[], filename = 'model.stl') {
  const buffer = buildSTLBuffer(meshes)

  // Create download
  const blob = new Blob([buffer], { type: 'application/octet-stream' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}
