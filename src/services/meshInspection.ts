import {callGeometryRust} from './geometry/kernel'
import type {
  MeshData,
  MeshProvenanceRun,
  MeshTopologyDiagnostics,
} from '../core/mesh'
import { type Aabb3, type Vec3 } from './math3d'

const VERTEX_STRIDE = 6

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
  const geometry=callGeometryRust<Pick<MeshInspection,'bounds'|'dimensions'|'center'>>('mesh_display_inspect',{vertexBits:new Uint32Array(mesh.vertices.buffer,mesh.vertices.byteOffset,mesh.vertices.length),transform:mesh.transform})
  return {index,vertices:Math.floor(mesh.vertices.length/VERTEX_STRIDE),triangles:Math.floor(mesh.indices.length/3),...geometry,topology:(mesh as MeshWithTopology).topology??null}
}

/** Interpolate a triangle hit into world space using BVH barycentric weights. */
export function computeHitWorldPoint(
  mesh: MeshData,
  triangleIndex: number,
  barycentric: readonly [number, number, number],
): Vec3 | null {
  return callGeometryRust('mesh_hit_point',{vertexBits:new Uint32Array(mesh.vertices.buffer,mesh.vertices.byteOffset,mesh.vertices.length),indices:mesh.indices,transform:mesh.transform,triangleIndex,barycentric})
}

/** Compute a flat, unit-length world-space normal for the hit triangle. */
export function computeHitNormal(mesh: MeshData, triangleIndex: number): Vec3 | null {
  return callGeometryRust('mesh_hit_normal',{vertexBits:new Uint32Array(mesh.vertices.buffer,mesh.vertices.byteOffset,mesh.vertices.length),indices:mesh.indices,transform:mesh.transform,triangleIndex})
}

/** Summarize an exact two-point measurement in world coordinates. */
export function summarizeMeasurement(start: Vec3, end: Vec3): MeasurementSummary {
  if (![...start, ...end].every(Number.isFinite)) {
    throw new RangeError('Measurement points must contain finite coordinates')
  }
  return callGeometryRust('mesh_measure',{start,end})
}

export interface MeshManifoldDiagnostics {
  uniqueVertexCount: number
  uniqueEdgeCount: number
  validFaceCount: number
  degenerateTriangleCount: number
  boundaryEdgeCount: number
  nonManifoldEdgeCount: number
  isWatertight: boolean
  eulerCharacteristic: number
}

const SCRATCH_WORLD_PTS = new Float64Array(9)

function writeWorldTriangle(
  vertices: Float32Array,
  indices: Uint32Array,
  triangleIndex: number,
  transform: Float32Array | readonly number[],
  out: Float64Array,
): boolean {
  const base = triangleIndex * 3
  if (base + 2 >= indices.length) return false
  const m0 = transform[0]!, m1 = transform[1]!, m2 = transform[2]!, m3 = transform[3]!
  const m4 = transform[4]!, m5 = transform[5]!, m6 = transform[6]!, m7 = transform[7]!
  const m8 = transform[8]!, m9 = transform[9]!, m10 = transform[10]!, m11 = transform[11]!

  for (let k = 0; k < 3; k++) {
    const vIdx = indices[base + k]! * VERTEX_STRIDE
    if (vIdx + 2 >= vertices.length) return false
    const vx = vertices[vIdx]!, vy = vertices[vIdx + 1]!, vz = vertices[vIdx + 2]!
    const wx = m0 * vx + m1 * vy + m2 * vz + m3
    const wy = m4 * vx + m5 * vy + m6 * vz + m7
    const wz = m8 * vx + m9 * vy + m10 * vz + m11
    if (!Number.isFinite(wx) || !Number.isFinite(wy) || !Number.isFinite(wz)) return false
    const o = k * 3
    out[o] = wx
    out[o + 1] = wy
    out[o + 2] = wz
  }
  return true
}

/**
 * Inspects topological manifold diagnostics on a render mesh by welding coincident
 * world-space vertices (accounting for split per-face normal vertices) and analyzing
 * triangle areas, edge incidence counts, and the Euler characteristic (V - E + F).
 */
export function inspectMeshManifoldDiagnostics(
  mesh: Pick<MeshData, 'vertices' | 'indices' | 'transform'>,
): MeshManifoldDiagnostics {
  if (mesh.transform.length !== 16) throw new Error('Expected 16-element matrix')
  const triangleCount = Math.floor(mesh.indices.length / 3)
  const maxPossibleVertices = Math.max(1, triangleCount * 3 + 1)
  const vertexMap = new Map<string, number>()
  const edgeCounts = new Map<number, number>()
  let degenerateTriangleCount = 0
  let validFaceCount = 0

  const getWeldedVertexId = (x: number, y: number, z: number): number => {
    const qx = Math.round(x * 1e6)
    const qy = Math.round(y * 1e6)
    const qz = Math.round(z * 1e6)
    const key = `${qx},${qy},${qz}`
    let id = vertexMap.get(key)
    if (id === undefined) {
      id = vertexMap.size
      vertexMap.set(key, id)
    }
    return id
  }

  const recordEdge = (a: number, b: number) => {
    const u = a < b ? a : b
    const v = a < b ? b : a
    const edgeKey = u * maxPossibleVertices + v
    edgeCounts.set(edgeKey, (edgeCounts.get(edgeKey) ?? 0) + 1)
  }

  for (let t = 0; t < triangleCount; t++) {
    if (!writeWorldTriangle(mesh.vertices, mesh.indices, t, mesh.transform, SCRATCH_WORLD_PTS)) {
      degenerateTriangleCount++
      continue
    }
    const ax = SCRATCH_WORLD_PTS[0], ay = SCRATCH_WORLD_PTS[1], az = SCRATCH_WORLD_PTS[2]
    const bx = SCRATCH_WORLD_PTS[3], by = SCRATCH_WORLD_PTS[4], bz = SCRATCH_WORLD_PTS[5]
    const cx = SCRATCH_WORLD_PTS[6], cy = SCRATCH_WORLD_PTS[7], cz = SCRATCH_WORLD_PTS[8]

    const abx = bx - ax, aby = by - ay, abz = bz - az
    const acx = cx - ax, acy = cy - ay, acz = cz - az
    const nx = aby * acz - abz * acy
    const ny = abz * acx - abx * acz
    const nz = abx * acy - aby * acx
    const doubleAreaSq = nx * nx + ny * ny + nz * nz

    const v0 = getWeldedVertexId(ax, ay, az)
    const v1 = getWeldedVertexId(bx, by, bz)
    const v2 = getWeldedVertexId(cx, cy, cz)

    if (v0 === v1 || v1 === v2 || v2 === v0 || !Number.isFinite(doubleAreaSq) || doubleAreaSq <= 1e-20) {
      degenerateTriangleCount++
      continue
    }

    validFaceCount++
    recordEdge(v0, v1)
    recordEdge(v1, v2)
    recordEdge(v2, v0)
  }

  let boundaryEdgeCount = 0
  let nonManifoldEdgeCount = 0
  for (const count of edgeCounts.values()) {
    if (count === 1) boundaryEdgeCount++
    else if (count > 2) nonManifoldEdgeCount++
  }

  const uniqueVertexCount = vertexMap.size
  const uniqueEdgeCount = edgeCounts.size
  const eulerCharacteristic = uniqueVertexCount - uniqueEdgeCount + validFaceCount
  const isWatertight =
    validFaceCount > 0 &&
    degenerateTriangleCount === 0 &&
    boundaryEdgeCount === 0 &&
    nonManifoldEdgeCount === 0

  return {
    uniqueVertexCount,
    uniqueEdgeCount,
    validFaceCount,
    degenerateTriangleCount,
    boundaryEdgeCount,
    nonManifoldEdgeCount,
    isWatertight,
    eulerCharacteristic,
  }
}

export interface MeshTriangleQualitySummary {
  triangleCount: number
  minAspectRatio: number
  maxAspectRatio: number
  meanAspectRatio: number
  degenerateTriangleCount: number
  minAreaMm2: number
  maxAreaMm2: number
  totalAreaMm2: number
}

const EQUILATERAL_ALTITUDE_FACTOR = Math.sqrt(3) / 2

/**
 * Computes triangle aspect ratio and area quality statistics in a single zero-allocation
 * pass over mesh triangles. Supports both tightly packed 3-float position buffers and
 * 6-float interleaved `[x, y, z, nx, ny, nz]` render mesh vertex buffers.
 *
 * Aspect ratio is defined as the ratio of the longest edge to the corresponding altitude,
 * normalized by $\frac{\sqrt{3}}{2}$ so that an equilateral triangle has an aspect ratio of $1.0$.
 */
export function computeMeshTriangleQualitySummary(mesh: {
  vertices: Float32Array | number[]
  indices: Uint32Array | number[]
}): MeshTriangleQualitySummary {
  const { vertices, indices } = mesh
  const triangleCount = Math.floor(indices.length / 3)
  if (triangleCount === 0 || vertices.length < 3) {
    return {
      triangleCount: 0,
      minAspectRatio: 0,
      maxAspectRatio: 0,
      meanAspectRatio: 0,
      degenerateTriangleCount: 0,
      minAreaMm2: 0,
      maxAreaMm2: 0,
      totalAreaMm2: 0,
    }
  }

  let maxIndex = 0
  for (let i = 0; i < triangleCount * 3; i++) {
    const idx = indices[i]!
    if (idx > maxIndex) maxIndex = idx
  }
  const stride =
    (maxIndex + 1) * 3 > vertices.length
      ? 3
      : vertices.length % 6 === 0 && (maxIndex + 1) * 6 === vertices.length
        ? 6
        : 3

  let minAspectRatio = Infinity
  let maxAspectRatio = 0
  let sumAspectRatio = 0
  let validCount = 0
  let degenerateTriangleCount = 0
  let minAreaMm2 = Infinity
  let maxAreaMm2 = 0
  let totalAreaMm2 = 0

  for (let t = 0; t < triangleCount; t++) {
    const base = t * 3
    const i0 = indices[base]! * stride
    const i1 = indices[base + 1]! * stride
    const i2 = indices[base + 2]! * stride

    if (i0 + 2 >= vertices.length || i1 + 2 >= vertices.length || i2 + 2 >= vertices.length) {
      degenerateTriangleCount++
      continue
    }

    const ax = vertices[i0]!, ay = vertices[i0 + 1]!, az = vertices[i0 + 2]!
    const bx = vertices[i1]!, by = vertices[i1 + 1]!, bz = vertices[i1 + 2]!
    const cx = vertices[i2]!, cy = vertices[i2 + 1]!, cz = vertices[i2 + 2]!

    const abx = bx - ax, aby = by - ay, abz = bz - az
    const bcx = cx - bx, bcy = cy - by, bcz = cz - bz
    const cax = ax - cx, cay = ay - cy, caz = az - cz

    const e0Sq = abx * abx + aby * aby + abz * abz
    const e1Sq = bcx * bcx + bcy * bcy + bcz * bcz
    const e2Sq = cax * cax + cay * cay + caz * caz
    const maxEdgeSq = Math.max(e0Sq, e1Sq, e2Sq)

    // Cross product AB x (-CA) = AB x AC
    const acx = -cax, acy = -cay, acz = -caz
    const nx = aby * acz - abz * acy
    const ny = abz * acx - abx * acz
    const nz = abx * acy - aby * acx
    const doubleArea = Math.hypot(nx, ny, nz)
    const area = 0.5 * doubleArea

    if (!Number.isFinite(area) || area < 1e-10 || maxEdgeSq <= 0) {
      degenerateTriangleCount++
      continue
    }

    totalAreaMm2 += area
    if (area < minAreaMm2) minAreaMm2 = area
    if (area > maxAreaMm2) maxAreaMm2 = area

    // L_max / h_min = L_max / (2 * area / L_max) = L_max^2 / (2 * area)
    const aspectRatio = ( EQUILATERAL_ALTITUDE_FACTOR * maxEdgeSq ) / doubleArea
    if (aspectRatio < minAspectRatio) minAspectRatio = aspectRatio
    if (aspectRatio > maxAspectRatio) maxAspectRatio = aspectRatio
    sumAspectRatio += aspectRatio
    validCount++
  }

  return {
    triangleCount,
    minAspectRatio: validCount > 0 ? minAspectRatio : 0,
    maxAspectRatio: validCount > 0 ? maxAspectRatio : 0,
    meanAspectRatio: validCount > 0 ? sumAspectRatio / validCount : 0,
    degenerateTriangleCount,
    minAreaMm2: validCount > 0 ? minAreaMm2 : 0,
    maxAreaMm2: validCount > 0 ? maxAreaMm2 : 0,
    totalAreaMm2,
  }
}

