import type {
  MeshData,
  MeshProvenanceRun,
  MeshTopologyDiagnostics,
} from '../core/mesh'
import { type Aabb3, type Vec3 } from './math3d'

const VERTEX_STRIDE = 6
const SCRATCH_WORLD_PTS = new Float64Array(9)

function writeWorldVertex(
  vertices: Float32Array,
  m: Float32Array,
  vertexIndex: number,
  out: Float64Array,
  outOffset: number,
): boolean {
  if (!Number.isInteger(vertexIndex) || vertexIndex < 0) return false
  const o = vertexIndex * VERTEX_STRIDE
  if (o + 2 >= vertices.length) return false
  const x = vertices[o]
  const y = vertices[o + 1]
  const z = vertices[o + 2]
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) return false
  const w = m[12] * x + m[13] * y + m[14] * z + m[15]
  const iw = Number.isFinite(w) && Math.abs(w) > 1e-12 ? 1 / w : 1
  const px = (m[0] * x + m[1] * y + m[2] * z + m[3]) * iw
  const py = (m[4] * x + m[5] * y + m[6] * z + m[7]) * iw
  const pz = (m[8] * x + m[9] * y + m[10] * z + m[11]) * iw
  if (!Number.isFinite(px) || !Number.isFinite(py) || !Number.isFinite(pz)) return false
  out[outOffset] = px
  out[outOffset + 1] = py
  out[outOffset + 2] = pz
  return true
}

function writeWorldTriangle(
  vertices: Float32Array,
  indices: Uint32Array,
  triangleIndex: number,
  m: Float32Array,
  out: Float64Array,
): boolean {
  if (!Number.isInteger(triangleIndex) || triangleIndex < 0) return false
  const o = triangleIndex * 3
  if (o + 2 >= indices.length) return false
  return (
    writeWorldVertex(vertices, m, indices[o], out, 0) &&
    writeWorldVertex(vertices, m, indices[o + 1], out, 3) &&
    writeWorldVertex(vertices, m, indices[o + 2], out, 6)
  )
}

type MeshWithTopology = MeshData & {
  /** Optional while older worker responses are still in flight. */
  topology?: MeshTopologyDiagnostics
}

export interface MeshInspection {
  index: number
  /** Number of property vertices in the render mesh. */
  vertices: number
  triangles: number
  /** World-space bounds, or null when the mesh has no finite positions. */
  bounds: Aabb3 | null
  /** World-space width, depth and height (X, Y and Z). */
  dimensions: Vec3
  /** Center of the world-space AABB. */
  center: Vec3 | null
  /** Worker-computed topology diagnostics, when available. */
  topology: MeshTopologyDiagnostics | null
}

export interface MeasurementSummary {
  start: Vec3
  end: Vec3
  /** Signed end - start delta. */
  delta: Vec3
  deltaX: number
  deltaY: number
  deltaZ: number
  distance: number
}

export interface MeshReplacementMatchOptions {
  /** Positional same-name identities are trustworthy only within one source snapshot (preview → full). */
  sameSourceSnapshot?: boolean
}

const POSITIONAL_ENTITY_GROUP_RE = /%23\d+(?=\/|>|$)/gi

/**
 * Match replacement meshes to the previous scene by their source provenance.
 * Stable entity IDs are authoritative. Legacy provenance is only used when a
 * key is unique on both sides; ambiguous duplicates are deliberately left
 * unmatched instead of attaching selection/visibility to the wrong object.
 */
export function matchMeshesByProvenance(
  previous: readonly MeshData[],
  replacement: readonly MeshData[],
  options: MeshReplacementMatchOptions = {},
): number[] {
  const matches = new Array<number>(replacement.length).fill(-1)
  const previousByEntity = new Map<string, number[]>()
  const replacementByEntity = new Map<string, number[]>()
  const previousByPositionalGroup = new Map<string, number>()
  const replacementByPositionalGroup = new Map<string, number>()

  previous.forEach((mesh, index) => {
    if (!mesh.entityId) return
    const indices = previousByEntity.get(mesh.entityId)
    if (indices) indices.push(index)
    else previousByEntity.set(mesh.entityId, [index])
    const group = positionalEntityGroup(mesh.entityId)
    previousByPositionalGroup.set(group, (previousByPositionalGroup.get(group) ?? 0) + 1)
  })
  replacement.forEach((mesh, index) => {
    if (!mesh.entityId) return
    const indices = replacementByEntity.get(mesh.entityId)
    if (indices) indices.push(index)
    else replacementByEntity.set(mesh.entityId, [index])
    const group = positionalEntityGroup(mesh.entityId)
    replacementByPositionalGroup.set(group, (replacementByPositionalGroup.get(group) ?? 0) + 1)
  })

  for (const [entityId, replacementIndices] of replacementByEntity) {
    const previousIndices = previousByEntity.get(entityId)
    const group = positionalEntityGroup(entityId)
    const positionalIdentityIsSafe = options.sameSourceSnapshot
      || (previousByPositionalGroup.get(group) === 1 && replacementByPositionalGroup.get(group) === 1)
    if (positionalIdentityIsSafe && replacementIndices.length === 1 && previousIndices?.length === 1) {
      matches[replacementIndices[0]] = previousIndices[0]
    }
  }

  // Compatibility for meshes produced before entity IDs were introduced.
  // Restricting this to identity-less meshes prevents a genuinely new entity
  // from inheriting state merely because it shares the same source operation.
  const previousByProvenance = new Map<string, number[]>()
  const replacementByProvenance = new Map<string, number[]>()
  previous.forEach((mesh, index) => {
    if (mesh.entityId) return
    const key = meshProvenanceKey(mesh)
    const indices = previousByProvenance.get(key)
    if (indices) indices.push(index)
    else previousByProvenance.set(key, [index])
  })
  replacement.forEach((mesh, index) => {
    if (mesh.entityId) return
    const key = meshProvenanceKey(mesh)
    const indices = replacementByProvenance.get(key)
    if (indices) indices.push(index)
    else replacementByProvenance.set(key, [index])
  })

  for (const [key, replacementIndices] of replacementByProvenance) {
    const previousIndices = previousByProvenance.get(key)
    if (replacementIndices.length === 1 && previousIndices?.length === 1) {
      matches[replacementIndices[0]] = previousIndices[0]
    }
  }

  return matches
}

/**
 * Operation paths encode their same-name sibling ordinal as `%23N`. Removing
 * only those encoded ordinals exposes identities that would otherwise look
 * stable after a same-name reorder. Dynamic loop occurrence suffixes are left
 * intact because they distinguish evaluated instances rather than AST slots.
 */
function positionalEntityGroup(entityId: string): string {
  return entityId.replace(POSITIONAL_ENTITY_GROUP_RE, '%23*')
}

function meshProvenanceKey(mesh: MeshData): string {
  const instances = new Set<string>()
  const operations = new Set<string>()
  const legacySources = new Set<string>()
  for (const run of mesh.provenance) {
    const source = run.source
    if (!source) continue
    if (source.instanceId) instances.add(source.instanceId)
    if (source.operationId) operations.add(source.operationId)
    legacySources.add(`${source.id}:${source.label}`)
  }
  if (instances.size) return `instance:${[...instances].sort().join('|')}`
  if (operations.size) return `operation:${[...operations].sort().join('|')}`
  if (legacySources.size) return `source:${[...legacySources].sort().join('|')}`
  // A single anonymous mesh can still use the compatibility path safely.
  return 'source:anonymous'
}

/** Resolve a triangle to its compact Manifold/OpenSCAD provenance run. */
export function resolveProvenance(
  mesh: MeshData,
  triangleIndex: number,
): MeshProvenanceRun | null {
  const triangleCount = Math.floor(mesh.indices.length / 3)
  if (!Number.isInteger(triangleIndex) || triangleIndex < 0 || triangleIndex >= triangleCount) {
    return null
  }

  // Runs normally arrive sorted, but a linear scan remains deterministic and
  // safe for partial/older worker messages without assuming that invariant.
  for (const run of mesh.provenance) {
    if (triangleIndex >= run.triangleStart && triangleIndex < run.triangleEnd) return run
  }
  return null
}

/** Build the object-level facts displayed by an Inspect panel. */
export function inspectMesh(mesh: MeshData, index: number): MeshInspection {
  const vertices = mesh.vertices
  const m = mesh.transform
  if (m.length !== 16) throw new Error('Expected 16-element matrix')
  let minX = Infinity, minY = Infinity, minZ = Infinity
  let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity
  let found = false
  const vertexSlots = Math.ceil(vertices.length / VERTEX_STRIDE)
  for (let i = 0; i < vertexSlots; i++) {
    if (writeWorldVertex(vertices, m, i, SCRATCH_WORLD_PTS, 0)) {
      found = true
      const px = SCRATCH_WORLD_PTS[0]
      const py = SCRATCH_WORLD_PTS[1]
      const pz = SCRATCH_WORLD_PTS[2]
      if (px < minX) minX = px
      if (py < minY) minY = py
      if (pz < minZ) minZ = pz
      if (px > maxX) maxX = px
      if (py > maxY) maxY = py
      if (pz > maxZ) maxZ = pz
    }
  }
  return {
    index,
    vertices: Math.floor(vertices.length / VERTEX_STRIDE),
    triangles: Math.floor(mesh.indices.length / 3),
    bounds: found ? { min: [minX, minY, minZ], max: [maxX, maxY, maxZ] } : null,
    dimensions: found ? [maxX - minX, maxY - minY, maxZ - minZ] : [0, 0, 0],
    center: found ? [(minX + maxX) / 2, (minY + maxY) / 2, (minZ + maxZ) / 2] : null,
    topology: (mesh as MeshWithTopology).topology ?? null,
  }
}

/** Interpolate a triangle hit into world space using BVH barycentric weights. */
export function computeHitWorldPoint(
  mesh: MeshData,
  triangleIndex: number,
  barycentric: readonly [number, number, number],
): Vec3 | null {
  if (mesh.transform.length !== 16) throw new Error('Expected 16-element matrix')
  const w0 = barycentric[0], w1 = barycentric[1], w2 = barycentric[2]
  if (barycentric.length !== 3 || !Number.isFinite(w0) || !Number.isFinite(w1) || !Number.isFinite(w2)) {
    return null
  }
  if (!writeWorldTriangle(mesh.vertices, mesh.indices, triangleIndex, mesh.transform, SCRATCH_WORLD_PTS)) {
    return null
  }
  const px = SCRATCH_WORLD_PTS[0] * w0 + SCRATCH_WORLD_PTS[3] * w1 + SCRATCH_WORLD_PTS[6] * w2
  const py = SCRATCH_WORLD_PTS[1] * w0 + SCRATCH_WORLD_PTS[4] * w1 + SCRATCH_WORLD_PTS[7] * w2
  const pz = SCRATCH_WORLD_PTS[2] * w0 + SCRATCH_WORLD_PTS[5] * w1 + SCRATCH_WORLD_PTS[8] * w2
  return Number.isFinite(px) && Number.isFinite(py) && Number.isFinite(pz) ? [px, py, pz] : null
}

/** Compute a flat, unit-length world-space normal for the hit triangle. */
export function computeHitNormal(mesh: MeshData, triangleIndex: number): Vec3 | null {
  if (mesh.transform.length !== 16) throw new Error('Expected 16-element matrix')
  if (!writeWorldTriangle(mesh.vertices, mesh.indices, triangleIndex, mesh.transform, SCRATCH_WORLD_PTS)) {
    return null
  }
  const abx = SCRATCH_WORLD_PTS[3] - SCRATCH_WORLD_PTS[0]
  const aby = SCRATCH_WORLD_PTS[4] - SCRATCH_WORLD_PTS[1]
  const abz = SCRATCH_WORLD_PTS[5] - SCRATCH_WORLD_PTS[2]
  const acx = SCRATCH_WORLD_PTS[6] - SCRATCH_WORLD_PTS[0]
  const acy = SCRATCH_WORLD_PTS[7] - SCRATCH_WORLD_PTS[1]
  const acz = SCRATCH_WORLD_PTS[8] - SCRATCH_WORLD_PTS[2]
  const nx = aby * acz - abz * acy
  const ny = abz * acx - abx * acz
  const nz = abx * acy - aby * acx
  const length = Math.hypot(Math.hypot(nx, ny), nz)
  const scale = abx * abx + aby * aby + abz * abz + acx * acx + acy * acy + acz * acz
  if (!Number.isFinite(length) || length <= scale * Number.EPSILON * 16) {
    return null
  }
  return [nx / length, ny / length, nz / length]
}

/** Summarize an exact two-point measurement in world coordinates. */
export function summarizeMeasurement(start: Vec3, end: Vec3): MeasurementSummary {
  const s0 = start[0], s1 = start[1], s2 = start[2]
  const e0 = end[0], e1 = end[1], e2 = end[2]
  if (
    start.length !== 3 ||
    end.length !== 3 ||
    !Number.isFinite(s0) ||
    !Number.isFinite(s1) ||
    !Number.isFinite(s2) ||
    !Number.isFinite(e0) ||
    !Number.isFinite(e1) ||
    !Number.isFinite(e2)
  ) {
    throw new RangeError('Measurement points must contain finite coordinates')
  }
  const dx = e0 - s0
  const dy = e1 - s1
  const dz = e2 - s2
  const distance = Math.hypot(Math.hypot(dx, dy), dz)
  return {
    start: [s0, s1, s2],
    end: [e0, e1, e2],
    delta: [dx, dy, dz],
    deltaX: dx,
    deltaY: dy,
    deltaZ: dz,
    distance,
  }
}
