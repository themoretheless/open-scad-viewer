import type { GeometryQuality } from '../core/build'
import type { MeshData } from '../core/mesh'

/**
 * Increment this when the worker wire format changes incompatibly. Keeping the
 * version in every message makes an old, cached worker fail visibly instead of
 * accidentally publishing data into a newer application state.
 */
export const GEOMETRY_WORKER_PROTOCOL_VERSION = 2 as const

export type GeometryWorkerProtocolVersion = typeof GEOMETRY_WORKER_PROTOCOL_VERSION
export type GeometryJobId = number
export type DocumentRevision = number

export type GeometryBuildPhase = 'queued' | 'initializing' | 'compiling' | 'serializing' | 'complete'
export type GeometryCancelReason = 'user' | 'superseded' | 'disposed' | 'worker-restart'

export interface GeometryBuildError {
  name: string
  message: string
  line?: number
  column?: number
}

interface GeometryJobEnvelope {
  protocolVersion: GeometryWorkerProtocolVersion
  documentRevision: DocumentRevision
  jobId: GeometryJobId
  quality: GeometryQuality
}

export interface GeometryBuildRequest extends GeometryJobEnvelope {
  type: 'build'
  source: string
}

export interface GeometryCancelRequest extends Omit<GeometryJobEnvelope, 'quality'> {
  type: 'cancel'
  reason: GeometryCancelReason
}

export type GeometryWorkerRequest = GeometryBuildRequest | GeometryCancelRequest

export interface GeometryBuildAccepted extends GeometryJobEnvelope {
  status: 'accepted'
  phase: 'queued'
}

export interface GeometryBuildStarted extends GeometryJobEnvelope {
  status: 'started'
  phase: 'initializing' | 'compiling'
}

export interface GeometryBuildProgress extends GeometryJobEnvelope {
  status: 'progress'
  phase: GeometryBuildPhase
  /** Null means that the current kernel phase cannot report meaningful completion. */
  progress: number | null
}

export interface GeometryBuildSuccess extends GeometryJobEnvelope {
  status: 'succeeded'
  phase: 'complete'
  meshes: MeshData[]
  warnings: string[]
  volume: number
  surfaceArea: number
  durationMs: number
}

export interface GeometryBuildFailure extends GeometryJobEnvelope {
  status: 'failed'
  phase: GeometryBuildPhase
  error: GeometryBuildError
  durationMs: number
}

export interface GeometryBuildCancelled extends GeometryJobEnvelope {
  status: 'cancelled'
  phase: GeometryBuildPhase
  reason: GeometryCancelReason
  durationMs: number
}

export interface GeometryBuildStale extends GeometryJobEnvelope {
  status: 'stale'
  phase: GeometryBuildPhase
  supersededBy?: {
    documentRevision: DocumentRevision
    jobId: GeometryJobId
  }
  durationMs: number
}

export type GeometryBuildTerminal =
  | GeometryBuildSuccess
  | GeometryBuildFailure
  | GeometryBuildCancelled
  | GeometryBuildStale

export type GeometryWorkerEvent =
  | GeometryBuildAccepted
  | GeometryBuildStarted
  | GeometryBuildProgress
  | GeometryBuildTerminal

const QUALITIES = new Set<GeometryQuality>(['preview', 'full'])
const PHASES = new Set<GeometryBuildPhase>(['queued', 'initializing', 'compiling', 'serializing', 'complete'])
const CANCEL_REASONS = new Set<GeometryCancelReason>(['user', 'superseded', 'disposed', 'worker-restart'])
const MESH_VERTEX_STRIDE = 6
const BVH_LEAF_BIT = 0x80000000
const BVH_LEAF_COUNT_MASK = 0x7fffffff
const MAX_BVH_LEAF_SIZE = 64

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object'
}

function isNonNegativeSafeInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0
}

function isNonNegativeFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0
}

function isPhase(value: unknown): value is GeometryBuildPhase {
  return typeof value === 'string' && PHASES.has(value as GeometryBuildPhase)
}

function isQuality(value: unknown): value is GeometryQuality {
  return typeof value === 'string' && QUALITIES.has(value as GeometryQuality)
}

function isCancelReason(value: unknown): value is GeometryCancelReason {
  return typeof value === 'string' && CANCEL_REASONS.has(value as GeometryCancelReason)
}

function isBuildError(value: unknown): value is GeometryBuildError {
  if (!isRecord(value) || typeof value.name !== 'string' || typeof value.message !== 'string') return false
  return (value.line === undefined || (isNonNegativeSafeInteger(value.line) && value.line > 0))
    && (value.column === undefined || (isNonNegativeSafeInteger(value.column) && value.column > 0))
}

function isMeshSourceReference(value: unknown): boolean {
  if (!isRecord(value)) return false
  return isNonNegativeSafeInteger(value.id)
    && isNonNegativeSafeInteger(value.originalId)
    && isNonNegativeSafeInteger(value.start)
    && isNonNegativeSafeInteger(value.end)
    && value.end >= value.start
    && typeof value.label === 'string'
    && (value.operationId === undefined || (typeof value.operationId === 'string' && value.operationId.startsWith('op:')))
    && (value.instanceId === undefined || (typeof value.instanceId === 'string' && value.instanceId.startsWith('entity:')))
}

function isMeshProvenanceRun(value: unknown, triangleCount: number): boolean {
  if (!isRecord(value)) return false
  return isNonNegativeSafeInteger(value.triangleStart)
    && isNonNegativeSafeInteger(value.triangleEnd)
    && value.triangleEnd >= value.triangleStart
    && value.triangleEnd <= triangleCount
    && typeof value.backside === 'boolean'
    && (value.source === null || isMeshSourceReference(value.source))
}

function hasOnlyVertexReferences(indices: Uint32Array, vertexCount: number): boolean {
  for (const index of indices) {
    if (index >= vertexCount) return false
  }
  return true
}

/**
 * Validate the complete BVH graph before it reaches main-thread picking.
 * Traversal is iterative and each node may be visited only once, which rejects
 * cycles and shared children while keeping validation bounded by nodeCount.
 */
function isMeshBvh(
  value: Record<string, unknown>,
  triangleCount: number,
): boolean {
  if (value.version !== 1
    || value.vertexStride !== MESH_VERTEX_STRIDE
    || !isNonNegativeSafeInteger(value.leafSize)
    || value.leafSize === 0
    || value.leafSize > MAX_BVH_LEAF_SIZE
    || !isNonNegativeSafeInteger(value.nodeCount)
    || !(value.bounds instanceof Float32Array)
    || !(value.nodes instanceof Uint32Array)
    || !(value.triangles instanceof Uint32Array)) return false

  const nodeCount = value.nodeCount
  const bounds = value.bounds
  const nodes = value.nodes
  const triangles = value.triangles
  if (bounds.length !== nodeCount * 6 || nodes.length !== nodeCount * 2) return false
  if (triangles.length > triangleCount) return false
  if (nodeCount === 0) return triangles.length === 0
  if (triangles.length === 0 || nodeCount > triangles.length * 2 - 1) return false

  const referencedTriangles = new Uint8Array(triangleCount)
  for (const triangle of triangles) {
    if (triangle >= triangleCount || referencedTriangles[triangle]) return false
    referencedTriangles[triangle] = 1
  }

  const reachedNodes = new Uint8Array(nodeCount)
  const coveredTriangleSlots = new Uint8Array(triangles.length)
  const stack: number[] = [0]
  let reachedCount = 0
  let coveredCount = 0

  while (stack.length) {
    const node = stack.pop()!
    if (node >= nodeCount || reachedNodes[node]) return false
    reachedNodes[node] = 1
    reachedCount++

    const boundsOffset = node * 6
    for (let axis = 0; axis < 3; axis++) {
      const minimum = bounds[boundsOffset + axis]
      const maximum = bounds[boundsOffset + axis + 3]
      if (!Number.isFinite(minimum) || !Number.isFinite(maximum) || minimum > maximum) return false
    }

    const dataOffset = node * 2
    const firstOrLeft = nodes[dataOffset]
    const metadata = nodes[dataOffset + 1]
    if ((metadata & BVH_LEAF_BIT) !== 0) {
      const count = metadata & BVH_LEAF_COUNT_MASK
      if (count === 0 || count > value.leafSize
        || firstOrLeft > triangles.length
        || count > triangles.length - firstOrLeft) return false
      const end = firstOrLeft + count
      for (let slot = firstOrLeft; slot < end; slot++) {
        if (coveredTriangleSlots[slot]) return false
        coveredTriangleSlots[slot] = 1
        coveredCount++
      }
      continue
    }

    const left = firstOrLeft
    const right = metadata
    if (left >= nodeCount || right >= nodeCount || left === right) return false
    stack.push(right, left)
  }

  return reachedCount === nodeCount && coveredCount === triangles.length
}

function isMeshData(value: unknown): value is MeshData {
  if (!isRecord(value)) return false
  if (!(value.vertices instanceof Float32Array)
    || !(value.indices instanceof Uint32Array)
    || !(value.edgeIndices instanceof Uint32Array)
    || !(value.faceIds instanceof Uint32Array)
    || !(value.transform instanceof Float32Array)
    || !isRecord(value.bvh)
    || !isRecord(value.topology)) return false

  const triangleCount = value.indices.length / 3
  const bvh = value.bvh
  const topology = value.topology
  const vertexCount = value.vertices.length / MESH_VERTEX_STRIDE
  return (value.entityId === undefined || (typeof value.entityId === 'string' && value.entityId.startsWith('entity:')))
    && Number.isInteger(vertexCount)
    && Number.isInteger(triangleCount)
    && value.edgeIndices.length % 2 === 0
    && hasOnlyVertexReferences(value.indices, vertexCount)
    && hasOnlyVertexReferences(value.edgeIndices, vertexCount)
    && value.faceIds.length === triangleCount
    && value.transform.length === 16
    && Array.isArray(value.color)
    && value.color.length === 4
    && value.color.every(component => typeof component === 'number' && Number.isFinite(component))
    && isMeshBvh(bvh, triangleCount)
    && Array.isArray(value.provenance)
    && value.provenance.every(run => isMeshProvenanceRun(run, triangleCount))
    && isNonNegativeSafeInteger(topology.boundary)
    && isNonNegativeSafeInteger(topology.crease)
    && isNonNegativeSafeInteger(topology.nonManifold)
    && isNonNegativeSafeInteger(topology.degenerate)
}

function hasValidEventEnvelope(candidate: Record<string, unknown>): boolean {
  return candidate.protocolVersion === GEOMETRY_WORKER_PROTOCOL_VERSION
    && isNonNegativeSafeInteger(candidate.documentRevision)
    && isNonNegativeSafeInteger(candidate.jobId)
    && isQuality(candidate.quality)
    && isPhase(candidate.phase)
}

export function isGeometryWorkerRequest(value: unknown): value is GeometryWorkerRequest {
  if (!isRecord(value)) return false
  const candidate = value as Partial<GeometryWorkerRequest>
  const envelopeIsValid = candidate.protocolVersion === GEOMETRY_WORKER_PROTOCOL_VERSION
    && (candidate.type === 'build' || candidate.type === 'cancel')
    && isNonNegativeSafeInteger(candidate.documentRevision)
    && isNonNegativeSafeInteger(candidate.jobId)
  if (!envelopeIsValid) return false
  if (candidate.type === 'build') {
    const build = candidate as Partial<GeometryBuildRequest>
    return typeof build.source === 'string'
      && isQuality(build.quality)
  }
  const cancel = candidate as Partial<GeometryCancelRequest>
  return isCancelReason(cancel.reason)
}

export function isGeometryWorkerEvent(value: unknown): value is GeometryWorkerEvent {
  if (!isRecord(value) || !hasValidEventEnvelope(value)) return false

  switch (value.status) {
    case 'accepted':
      return value.phase === 'queued'
    case 'started':
      return value.phase === 'initializing' || value.phase === 'compiling'
    case 'progress':
      return value.progress === null
        || (typeof value.progress === 'number'
          && Number.isFinite(value.progress)
          && value.progress >= 0
          && value.progress <= 1)
    case 'succeeded':
      return value.phase === 'complete'
        && Array.isArray(value.meshes)
        && value.meshes.every(isMeshData)
        && Array.isArray(value.warnings)
        && value.warnings.every(warning => typeof warning === 'string')
        && isNonNegativeFiniteNumber(value.volume)
        && isNonNegativeFiniteNumber(value.surfaceArea)
        && isNonNegativeFiniteNumber(value.durationMs)
    case 'failed':
      return isBuildError(value.error) && isNonNegativeFiniteNumber(value.durationMs)
    case 'cancelled':
      return isCancelReason(value.reason) && isNonNegativeFiniteNumber(value.durationMs)
    case 'stale':
      return (value.supersededBy === undefined
          || (isRecord(value.supersededBy)
            && isNonNegativeSafeInteger(value.supersededBy.documentRevision)
            && isNonNegativeSafeInteger(value.supersededBy.jobId)))
        && isNonNegativeFiniteNumber(value.durationMs)
    default:
      return false
  }
}
