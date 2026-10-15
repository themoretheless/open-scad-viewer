/** Matrix helpers and unit primitive meshes for the GPU CSG preview. */
import type { GpuCsgPrimitive, GpuCsgPrimitiveKind } from './gpuCsgTypes'

export function identityMat4(): Float32Array {
  const m = new Float32Array(16)
  m[0] = m[5] = m[10] = m[15] = 1
  return m
}

export function multiplyMat4(a: Float32Array, b: Float32Array, out?: Float32Array): Float32Array {
  const dst = out ?? new Float32Array(16)
  const a00 = a[0]!, a01 = a[1]!, a02 = a[2]!, a03 = a[3]!
  const a10 = a[4]!, a11 = a[5]!, a12 = a[6]!, a13 = a[7]!
  const a20 = a[8]!, a21 = a[9]!, a22 = a[10]!, a23 = a[11]!
  const a30 = a[12]!, a31 = a[13]!, a32 = a[14]!, a33 = a[15]!

  const b00 = b[0]!, b01 = b[1]!, b02 = b[2]!, b03 = b[3]!
  const b10 = b[4]!, b11 = b[5]!, b12 = b[6]!, b13 = b[7]!
  const b20 = b[8]!, b21 = b[9]!, b22 = b[10]!, b23 = b[11]!
  const b30 = b[12]!, b31 = b[13]!, b32 = b[14]!, b33 = b[15]!

  dst[0] = a00 * b00 + a01 * b10 + a02 * b20 + a03 * b30
  dst[1] = a00 * b01 + a01 * b11 + a02 * b21 + a03 * b31
  dst[2] = a00 * b02 + a01 * b12 + a02 * b22 + a03 * b32
  dst[3] = a00 * b03 + a01 * b13 + a02 * b23 + a03 * b33

  dst[4] = a10 * b00 + a11 * b10 + a12 * b20 + a13 * b30
  dst[5] = a10 * b01 + a11 * b11 + a12 * b21 + a13 * b31
  dst[6] = a10 * b02 + a11 * b12 + a12 * b22 + a13 * b32
  dst[7] = a10 * b03 + a11 * b13 + a12 * b23 + a13 * b33

  dst[8] = a20 * b00 + a21 * b10 + a22 * b20 + a23 * b30
  dst[9] = a20 * b01 + a21 * b11 + a22 * b21 + a23 * b31
  dst[10] = a20 * b02 + a21 * b12 + a22 * b22 + a23 * b32
  dst[11] = a20 * b03 + a21 * b13 + a22 * b23 + a23 * b33

  dst[12] = a30 * b00 + a31 * b10 + a32 * b20 + a33 * b30
  dst[13] = a30 * b01 + a31 * b11 + a32 * b21 + a33 * b31
  dst[14] = a30 * b02 + a31 * b12 + a32 * b22 + a33 * b32
  dst[15] = a30 * b03 + a31 * b13 + a32 * b23 + a33 * b33
  return dst
}


export function translationMat4(tx: number, ty: number, tz: number): Float32Array {
  const m = identityMat4()
  m[3] = tx
  m[7] = ty
  m[11] = tz
  return m
}

export function scaleMat4(sx: number, sy: number, sz: number): Float32Array {
  const m = identityMat4()
  m[0] = sx
  m[5] = sy
  m[10] = sz
  return m
}

export function rotationXMat4(rad: number): Float32Array {
  const c = Math.cos(rad)
  const s = Math.sin(rad)
  const m = identityMat4()
  m[5] = c
  m[6] = -s
  m[9] = s
  m[10] = c
  return m
}

export function rotationYMat4(rad: number): Float32Array {
  const c = Math.cos(rad)
  const s = Math.sin(rad)
  const m = identityMat4()
  m[0] = c
  m[2] = s
  m[8] = -s
  m[10] = c
  return m
}

export function rotationZMat4(rad: number): Float32Array {
  const c = Math.cos(rad)
  const s = Math.sin(rad)
  const m = identityMat4()
  m[0] = c
  m[1] = -s
  m[4] = s
  m[5] = c
  return m
}

export function rotationEulerMat4(rxDeg: number, ryDeg: number, rzDeg: number): Float32Array {
  const deg2rad = Math.PI / 180
  const ax = rxDeg * deg2rad
  const ay = ryDeg * deg2rad
  const az = rzDeg * deg2rad

  const cx = Math.cos(ax), sx = Math.sin(ax)
  const cy = Math.cos(ay), sy = Math.sin(ay)
  const cz = Math.cos(az), sz = Math.sin(az)

  // Direct composition of Euler Z * Y * X
  const m = new Float32Array(16)
  m[0] = cz * cy
  m[1] = cz * sy * sx - sz * cx
  m[2] = cz * sy * cx + sz * sx
  m[4] = sz * cy
  m[5] = sz * sy * sx + cz * cx
  m[6] = sz * sy * cx - cz * sx
  m[8] = -sy
  m[9] = cy * sx
  m[10] = cy * cx
  m[15] = 1
  return m
}

export function normalMatrixFromModel(model: Float32Array, out?: Float32Array): Float32Array {
  // Transpose of inverse of 3x3 upper-left block, padded to 4x4
  const n = out ?? identityMat4()
  if (out) {
    n.fill(0)
    n[0] = n[5] = n[10] = n[15] = 1
  }
  const m00 = model[0]!, m01 = model[1]!, m02 = model[2]!
  const m10 = model[4]!, m11 = model[5]!, m12 = model[6]!
  const m20 = model[8]!, m21 = model[9]!, m22 = model[10]!

  const c00 = m11 * m22 - m12 * m21
  const c01 = -(m10 * m22 - m12 * m20)
  const c02 = m10 * m21 - m11 * m20

  const det = m00 * c00 + m01 * c01 + m02 * c02
  if (Math.abs(det) < 1e-8) return n

  const invDet = 1 / det
  n[0] = c00 * invDet
  n[1] = -(m01 * m22 - m02 * m21) * invDet
  n[2] = (m01 * m12 - m02 * m11) * invDet

  n[4] = c01 * invDet
  n[5] = (m00 * m22 - m02 * m20) * invDet
  n[6] = -(m00 * m12 - m02 * m10) * invDet

  n[8] = c02 * invDet
  n[9] = -(m00 * m21 - m01 * m20) * invDet
  n[10] = (m00 * m11 - m01 * m10) * invDet
  return n
}

/* ── Primitive Geometry Generation ────────────────────── */

export interface GeometryData {
  /** Interleaved Float32Array [x, y, z, nx, ny, nz] */
  vertices: Float32Array
  /** Triangles index buffer */
  indices: Uint32Array
}

let cachedUnitCubeCentered: GeometryData | null = null
let cachedUnitCubeUncentered: GeometryData | null = null
let cachedUnitCylinder32Centered: GeometryData | null = null
let cachedUnitSphere32x16: GeometryData | null = null

/**
 * Generates a unit cube [-0.5, 0.5]^3 or [0, 1]^3.
 * 24 vertices with sharp perpendicular face normals, 36 indices.
 */
export function generateUnitCube(center = true): GeometryData {
  if (center && cachedUnitCubeCentered) return cachedUnitCubeCentered
  if (!center && cachedUnitCubeUncentered) return cachedUnitCubeUncentered

  const min = center ? -0.5 : 0
  const max = center ? 0.5 : 1

  // 6 faces * 4 vertices = 24 vertices
  // Format: [px, py, pz, nx, ny, nz]
  const vertices = new Float32Array([
    // Front face (Z+)
    min, min, max,  0, 0, 1,
    max, min, max,  0, 0, 1,
    max, max, max,  0, 0, 1,
    min, max, max,  0, 0, 1,
    // Back face (Z-)
    max, min, min,  0, 0, -1,
    min, min, min,  0, 0, -1,
    min, max, min,  0, 0, -1,
    max, max, min,  0, 0, -1,
    // Right face (X+)
    max, min, min,  1, 0, 0,
    max, max, min,  1, 0, 0,
    max, max, max,  1, 0, 0,
    max, min, max,  1, 0, 0,
    // Left face (X-)
    min, min, max, -1, 0, 0,
    min, max, max, -1, 0, 0,
    min, max, min, -1, 0, 0,
    min, min, min, -1, 0, 0,
    // Top face (Y+)
    min, max, max,  0, 1, 0,
    max, max, max,  0, 1, 0,
    max, max, min,  0, 1, 0,
    min, max, min,  0, 1, 0,
    // Bottom face (Y-)
    min, min, min,  0, -1, 0,
    max, min, min,  0, -1, 0,
    max, min, max,  0, -1, 0,
    min, min, max,  0, -1, 0,
  ])

  const indices = new Uint32Array(36)
  for (let f = 0; f < 6; f++) {
    const base = f * 4
    const i = f * 6
    indices[i] = base
    indices[i + 1] = base + 1
    indices[i + 2] = base + 2
    indices[i + 3] = base
    indices[i + 4] = base + 2
    indices[i + 5] = base + 3
  }

  const data: GeometryData = { vertices, indices }
  if (center) cachedUnitCubeCentered = data
  else cachedUnitCubeUncentered = data
  return data
}

/**
 * Generates a unit cylinder (radius = 1, height = 1, along Z-axis).
 */
export function generateUnitCylinder(segments = 32, center = true): GeometryData {
  if (segments === 32 && center && cachedUnitCylinder32Centered) {
    return cachedUnitCylinder32Centered
  }

  const zMin = center ? -0.5 : 0
  const zMax = center ? 0.5 : 1

  const totalVerts = 2 + segments * 2 + (segments + 1) * 2
  const vertices = new Float32Array(totalVerts * 6)
  const indices = new Uint32Array(segments * 12)
  let vPtr = 0
  let iPtr = 0

  // Top cap center
  const topCenterIdx = 0
  vertices[vPtr++] = 0; vertices[vPtr++] = 0; vertices[vPtr++] = zMax
  vertices[vPtr++] = 0; vertices[vPtr++] = 0; vertices[vPtr++] = 1

  // Top cap perimeter
  const topStartIdx = 1
  const twoPiOverSegs = (Math.PI * 2) / segments
  for (let i = 0; i < segments; i++) {
    const theta = i * twoPiOverSegs
    const x = Math.cos(theta)
    const y = Math.sin(theta)
    vertices[vPtr++] = x; vertices[vPtr++] = y; vertices[vPtr++] = zMax
    vertices[vPtr++] = 0; vertices[vPtr++] = 0; vertices[vPtr++] = 1
  }
  for (let i = 0; i < segments; i++) {
    const next = (i + 1) % segments
    indices[iPtr++] = topCenterIdx
    indices[iPtr++] = topStartIdx + i
    indices[iPtr++] = topStartIdx + next
  }

  // Bottom cap center
  const bottomCenterIdx = vPtr / 6
  vertices[vPtr++] = 0; vertices[vPtr++] = 0; vertices[vPtr++] = zMin
  vertices[vPtr++] = 0; vertices[vPtr++] = 0; vertices[vPtr++] = -1

  // Bottom cap perimeter
  const bottomStartIdx = bottomCenterIdx + 1
  for (let i = 0; i < segments; i++) {
    const theta = i * twoPiOverSegs
    const x = Math.cos(theta)
    const y = Math.sin(theta)
    vertices[vPtr++] = x; vertices[vPtr++] = y; vertices[vPtr++] = zMin
    vertices[vPtr++] = 0; vertices[vPtr++] = 0; vertices[vPtr++] = -1
  }
  for (let i = 0; i < segments; i++) {
    const next = (i + 1) % segments
    indices[iPtr++] = bottomCenterIdx
    indices[iPtr++] = bottomStartIdx + next
    indices[iPtr++] = bottomStartIdx + i
  }

  // Side wall (quads with radial normals)
  const sideStartIdx = vPtr / 6
  for (let i = 0; i <= segments; i++) {
    const theta = i * twoPiOverSegs
    const x = Math.cos(theta)
    const y = Math.sin(theta)
    // Top vertex
    vertices[vPtr++] = x; vertices[vPtr++] = y; vertices[vPtr++] = zMax
    vertices[vPtr++] = x; vertices[vPtr++] = y; vertices[vPtr++] = 0
    // Bottom vertex
    vertices[vPtr++] = x; vertices[vPtr++] = y; vertices[vPtr++] = zMin
    vertices[vPtr++] = x; vertices[vPtr++] = y; vertices[vPtr++] = 0
  }
  for (let i = 0; i < segments; i++) {
    const iTop = sideStartIdx + i * 2
    const iBot = iTop + 1
    const nTop = sideStartIdx + (i + 1) * 2
    const nBot = nTop + 1
    indices[iPtr++] = iTop; indices[iPtr++] = nTop; indices[iPtr++] = iBot
    indices[iPtr++] = iBot; indices[iPtr++] = nTop; indices[iPtr++] = nBot
  }

  const data: GeometryData = { vertices, indices }
  if (segments === 32 && center) cachedUnitCylinder32Centered = data
  return data
}

/**
 * Generates a unit UV sphere (radius = 1).
 */
export function generateUnitSphere(segments = 32, rings = 16): GeometryData {
  if (segments === 32 && rings === 16 && cachedUnitSphere32x16) {
    return cachedUnitSphere32x16
  }

  const totalVerts = (rings + 1) * (segments + 1)
  const vertices = new Float32Array(totalVerts * 6)
  const indices = new Uint32Array(rings * segments * 6)
  let vPtr = 0
  let iPtr = 0

  const piOverRings = Math.PI / rings
  const twoPiOverSegs = (Math.PI * 2) / segments

  for (let r = 0; r <= rings; r++) {
    const phi = r * piOverRings
    const sinPhi = Math.sin(phi)
    const cosPhi = Math.cos(phi)

    for (let s = 0; s <= segments; s++) {
      const theta = s * twoPiOverSegs
      const x = Math.cos(theta) * sinPhi
      const y = Math.sin(theta) * sinPhi
      const z = cosPhi
      // Normalized position doubles as normal
      vertices[vPtr++] = x; vertices[vPtr++] = y; vertices[vPtr++] = z
      vertices[vPtr++] = x; vertices[vPtr++] = y; vertices[vPtr++] = z
    }
  }

  for (let r = 0; r < rings; r++) {
    for (let s = 0; s < segments; s++) {
      const first = r * (segments + 1) + s
      const second = first + segments + 1
      indices[iPtr++] = first; indices[iPtr++] = second; indices[iPtr++] = first + 1
      indices[iPtr++] = second; indices[iPtr++] = second + 1; indices[iPtr++] = first + 1
    }
  }

  const data: GeometryData = { vertices, indices }
  if (segments === 32 && rings === 16) cachedUnitSphere32x16 = data
  return data
}

/* ── CSG Tree Flattening & Normalization ──────────────── */

let nextCsgId = 1
export function resetCsgIdCounter() {
  nextCsgId = 1
}

export function createCsgPrimitive(
  kind: GpuCsgPrimitiveKind,
  transform: Float32Array = identityMat4(),
  color: [number, number, number, number] = [0.85, 0.7, 0.2, 1.0],
  customGeometry?: GeometryData,
): GpuCsgPrimitive {
  return {
    id: `csg_prim_${nextCsgId++}`,
    kind,
    transform,
    color,
    vertices: customGeometry?.vertices,
    indices: customGeometry?.indices,
  }
}
