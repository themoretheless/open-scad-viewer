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
  const matches = replacement.map(() => -1)
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
  return entityId.replace(/%23\d+(?=\/|>|$)/gi, '%23*')
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
