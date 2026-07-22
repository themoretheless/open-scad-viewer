/**
 * Wavefront OBJ export from MeshData[].
 * Format: v x y z (vertices), vn nx ny nz (normals), f v//vn v//vn v//vn (faces)
 */
import type { MeshData } from '../core/mesh'
import type { Mat4 } from './math3d'
import { invert, transpose } from './math3d'

/** Apply a row-major 4x4 transform to a 3D point. */
function transformPoint(t: Mat4, x: number, y: number, z: number): [number, number, number] {
  return [
    t[0] * x + t[1] * y + t[2] * z + t[3],
    t[4] * x + t[5] * y + t[6] * z + t[7],
    t[8] * x + t[9] * y + t[10] * z + t[11],
  ]
}

/**
 * Transform a normal by the inverse-transpose of the model matrix's linear part,
 * which keeps normals perpendicular to faces under non-uniform scale / shear.
 * `nt` is the inverse-transpose Mat4 (computed once per mesh).
 */
function transformNormal(nt: Mat4, nx: number, ny: number, nz: number): [number, number, number] {
  // Direction vector (w = 0): only the upper-left 3x3 contributes.
  let rx = nt[0] * nx + nt[1] * ny + nt[2] * nz
  let ry = nt[4] * nx + nt[5] * ny + nt[6] * nz
  let rz = nt[8] * nx + nt[9] * ny + nt[10] * nz
  const len = Math.sqrt(rx * rx + ry * ry + rz * rz)
  if (len > 1e-10) { rx /= len; ry /= len; rz /= len }
  return [rx, ry, rz]
}

/** Determinant of the upper-left 3x3 of a row-major 4x4 transform. */
function determinant3(t: Mat4): number {
  const a = t[0], b = t[1], c = t[2]
  const d = t[4], e = t[5], f = t[6]
  const g = t[8], h = t[9], i = t[10]
  return a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
}

/**
 * Generate a Wavefront OBJ file from mesh data and trigger a download.
 */
export function exportOBJ(meshes: MeshData[], filename = 'model.obj') {
  const lines: string[] = []
  lines.push('# Wavefront OBJ exported from OpenSCAD 3D Viewer')
  lines.push('')

  let vertexOffset = 0
  let normalOffset = 0

  for (let mi = 0; mi < meshes.length; mi++) {
    const m = meshes[mi]
    const verts = m.vertices // interleaved: pos(3) + normal(3)
    const indices = m.indices
    const t = m.transform
    const vertCount = verts.length / 6
    // Inverse-transpose of the model matrix for correct normal transformation
    // under non-uniform scale / shear.
    const normalMatrix = transpose(invert(t))
    // Negative-determinant transforms (mirrors) flip winding; reverse face
    // vertex order so faces stay outward-facing.
    const flip = determinant3(t) < 0

    lines.push(`o mesh_${mi}`)

    // Write vertices
    for (let i = 0; i < vertCount; i++) {
      const x = verts[i * 6], y = verts[i * 6 + 1], z = verts[i * 6 + 2]
      const p = transformPoint(t, x, y, z)
      lines.push(`v ${p[0].toFixed(6)} ${p[1].toFixed(6)} ${p[2].toFixed(6)}`)
    }

    // Write normals
    for (let i = 0; i < vertCount; i++) {
      const nx = verts[i * 6 + 3], ny = verts[i * 6 + 4], nz = verts[i * 6 + 5]
      const n = transformNormal(normalMatrix, nx, ny, nz)
      lines.push(`vn ${n[0].toFixed(6)} ${n[1].toFixed(6)} ${n[2].toFixed(6)}`)
    }

    // Write faces (OBJ indices are 1-based)
    for (let i = 0; i < indices.length; i += 3) {
      let a = indices[i], b = indices[i + 1], c = indices[i + 2]
      if (flip) { const tmp = b; b = c; c = tmp }
      const i0 = a + 1 + vertexOffset
      const i1 = b + 1 + vertexOffset
      const i2 = c + 1 + vertexOffset
      const n0 = a + 1 + normalOffset
      const n1 = b + 1 + normalOffset
      const n2 = c + 1 + normalOffset
      lines.push(`f ${i0}//${n0} ${i1}//${n1} ${i2}//${n2}`)
    }

    vertexOffset += vertCount
    normalOffset += vertCount
    lines.push('')
  }

  const content = lines.join('\n')
  const blob = new Blob([content], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}
