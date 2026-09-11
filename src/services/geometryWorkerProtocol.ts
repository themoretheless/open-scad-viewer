import {isNativeGeometryArtifact,MAX_NATIVE_GEOMETRY_CHARACTERS} from '../core/nativeGeometry'
import type { GeometryEvaluationResult, GeometryPhaseTimings, GeometryQuality } from '../core/build'
import {
  GEOMETRY_MANIFEST_ARCHIVE,
  geometryEngineClassForLanguageContract,
  MAX_GEOMETRY_SOURCE_CHARACTERS,
  type GeometryExecutionDescriptor,
} from '../core/geometryExecution'
import type { MeshData } from '../core/mesh'
import { sha256Hex } from '../core/sha256'

/**
 * Increment this when the worker wire format changes incompatibly. Keeping the
 * version in every message makes an old, cached worker fail visibly instead of
 * accidentally publishing data into a newer application state.
 */
export const GEOMETRY_WORKER_PROTOCOL_VERSION = 6 as const

export type GeometryWorkerProtocolVersion = typeof GEOMETRY_WORKER_PROTOCOL_VERSION
export type GeometryJobId = number
export type DocumentRevision = number

export type GeometryBuildPhase = 'queued' | 'initializing' | 'compiling' | 'serializing' | 'complete'
export type GeometryCancelReason = 'user' | 'superseded' | 'disposed' | 'worker-restart'

export interface GeometryBuildError {
  name: string
  message: string
  code?: string
  start?: number
  end?: number
  line?: number
  column?: number
}

interface GeometryJobEnvelope {
  protocolVersion: GeometryWorkerProtocolVersion
  documentRevision: DocumentRevision
  jobId: GeometryJobId
  quality: GeometryQuality
  /** SHA-256 of the exact UTF-8 source bytes for this job. */
  sourceSha256: string
}

export interface GeometryBuildRequest extends GeometryJobEnvelope {
  type: 'build'
  source: string
}

export interface GeometryCancelRequest extends Omit<GeometryJobEnvelope, 'quality' | 'sourceSha256'> {
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
  execution: GeometryExecutionDescriptor
  meshes: MeshData[]
  warnings: string[]
  volume: number
  surfaceArea: number
  /**
   * True iff quality-based reduction altered any evaluated value. A preview
   * result with reduced=false has full-equivalent authoritative outputs for
   * the same exact source under the current equivalence policy, so the
   * application may publish it as full and skip the rebuild.
   */
  reduced: boolean
  timings: GeometryPhaseTimings
  durationMs: number
}

export interface GeometryBuildFailure extends GeometryJobEnvelope {
  status: 'failed'
  phase: GeometryBuildPhase
  /** Present once source routing selected an engine, even when execution failed. */
  execution?: GeometryExecutionDescriptor
  error: GeometryBuildError
  durationMs: number
}

export interface GeometryBuildCancelled extends GeometryJobEnvelope {
  status: 'cancelled'
  phase: GeometryBuildPhase
  execution?: GeometryExecutionDescriptor
  reason: GeometryCancelReason
  durationMs: number
}

export interface GeometryBuildStale extends GeometryJobEnvelope {
  status: 'stale'
  phase: GeometryBuildPhase
  execution?: GeometryExecutionDescriptor
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
export const GEOMETRY_WORKER_PAYLOAD_LIMITS = Object.freeze({
  meshes: 1_000,
  triangles: 750_000,
  bytes: 128 * 1024 * 1024,
  warnings: 32,
  warningCharacters: 512,
  errorNameCharacters: 128,
  errorMessageCharacters: 4_096,
  identityCharacters: 256,
  sourceLabelCharacters: 512,
  totalTextCharacters: 2 * 1024 * 1024,
})

function isRecord(value: unknown): value is Record<string, unknown> {
  if (value === null || Array.isArray(value) || typeof value !== 'object') return false
  const prototype = Object.getPrototypeOf(value)
  return prototype === Object.prototype || prototype === null
}

function isDenseExactArray(value: unknown, maximumLength = Number.MAX_SAFE_INTEGER): value is unknown[] {
  if (!Array.isArray(value) || value.length > maximumLength) return false
  const keys = Reflect.ownKeys(value)
  if (keys.length !== value.length + 1 || keys.some(key => typeof key !== 'string')) return false
  const lengthDescriptor = Object.getOwnPropertyDescriptor(value, 'length')
  if (!lengthDescriptor || !Object.hasOwn(lengthDescriptor, 'value') || lengthDescriptor.enumerable) return false
  for (let index = 0; index < value.length; index++) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index))
    if (!descriptor || !Object.hasOwn(descriptor, 'value') || !descriptor.enumerable) return false
  }
  return true
}

function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

function hasExactKeys(
  value: Record<string, unknown>,
  required: readonly string[],
  optional: readonly string[] = [],
): boolean {
  const keys = Reflect.ownKeys(value)
  if (keys.some(key => typeof key !== 'string')) return false
  const allowed = new Set([...required, ...optional])
  return required.every(key => Object.hasOwn(value, key))
    && keys.every(key => {
      if (typeof key !== 'string' || !allowed.has(key)) return false
      const descriptor = Object.getOwnPropertyDescriptor(value, key)
      return descriptor !== undefined
        && Object.hasOwn(descriptor, 'value')
        && descriptor.enumerable
    })
}

function hasBoundedDataProperties(value: Record<string, unknown>, maximumKeys: number): boolean {
  const keys = Reflect.ownKeys(value)
  return keys.length <= maximumKeys && keys.every(key => {
    if (typeof key !== 'string') return false
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    return descriptor !== undefined
      && Object.hasOwn(descriptor, 'value')
      && descriptor.enumerable
  })
}

function hasExclusiveArrayBuffer(view: ArrayBufferView): boolean {
  return view.buffer instanceof ArrayBuffer
    && view.byteOffset === 0
    && view.byteLength === view.buffer.byteLength
}

function isFloat32Payload(value: unknown): value is Float32Array {
  return value instanceof Float32Array && hasExclusiveArrayBuffer(value)
}

function isUint32Payload(value: unknown): value is Uint32Array {
  return value instanceof Uint32Array && hasExclusiveArrayBuffer(value)
}

function optionalKey(
  value: Record<string, unknown>,
  key: string,
  predicate: (candidate: unknown) => boolean,
): boolean {
  return !Object.hasOwn(value, key) || predicate(value[key])
}

function isNonNegativeSafeInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0
}

function isNonNegativeFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0
}

function isPhaseTimings(value: unknown): value is GeometryPhaseTimings {
  if (!isRecord(value)
    || !hasExactKeys(value, ['parseMs', 'bindMs', 'initializeMs', 'evaluateMs', 'analyzeMs'])) return false
  const candidate = value as Partial<GeometryPhaseTimings>
  return isNonNegativeFiniteNumber(candidate.parseMs)
    && isNonNegativeFiniteNumber(candidate.bindMs)
    && isNonNegativeFiniteNumber(candidate.initializeMs)
    && isNonNegativeFiniteNumber(candidate.evaluateMs)
    && isNonNegativeFiniteNumber(candidate.analyzeMs)
}

function isExecutionDescriptor(
  value: unknown,
  quality: GeometryQuality,
  allowedEvidence: 'runtime' | 'runtime-or-planned' = 'runtime',
): value is GeometryExecutionDescriptor {
  if (!isRecord(value)
    || !hasExactKeys(value, [
      'languageContract', 'requiredCapabilities', 'engineClass', 'engineKey',
      'kernelFingerprint', 'semanticProgramVersion', 'capabilityManifestVersion',
      'manifestDigest', 'purpose', 'quality', 'representation', 'evidence',
      'effectiveLimits', 'automaticFallback',
    ])
    || (value.languageContract !== 'legacy/current'
      && value.languageContract !== 'openscad-viewer/brep-1')
    || (value.engineClass !== 'mesh' && value.engineClass !== 'brep')
    || value.engineClass !== geometryEngineClassForLanguageContract(value.languageContract)
    || typeof value.engineKey !== 'string' || !value.engineKey
    || typeof value.kernelFingerprint !== 'string' || !value.kernelFingerprint
    || typeof value.semanticProgramVersion !== 'string' || !value.semanticProgramVersion
    || typeof value.capabilityManifestVersion !== 'string' || !value.capabilityManifestVersion
    || typeof value.manifestDigest !== 'string' || !/^[a-f0-9]{64}$/.test(value.manifestDigest)
    || value.purpose !== quality
    || value.quality !== quality
    || (value.representation !== 'mesh' && value.representation !== 'brep')
    || (value.engineClass === 'mesh' && value.representation !== 'mesh')
    || (allowedEvidence === 'runtime'
      ? value.evidence !== 'runtime'
      : value.evidence !== 'runtime' && value.evidence !== 'planned')
    || value.automaticFallback !== false
    || !isDenseExactArray(value.requiredCapabilities, 32)
    || value.requiredCapabilities.some(capability => typeof capability !== 'string'
      || !/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(capability))
    || new Set(value.requiredCapabilities).size !== value.requiredCapabilities.length
    || !isRecord(value.effectiveLimits)
    || !hasBoundedDataProperties(value.effectiveLimits, 32)
    || Object.values(value.effectiveLimits).some(limit => !isNonNegativeFiniteNumber(limit))) return false

  const manifest = GEOMETRY_MANIFEST_ARCHIVE[
    value.capabilityManifestVersion as keyof typeof GEOMETRY_MANIFEST_ARCHIVE
  ]
  if (!manifest
    || value.engineClass !== manifest.engineClass
    || value.engineKey !== manifest.engineKey
    || value.kernelFingerprint !== manifest.kernelFingerprint
    || value.semanticProgramVersion !== manifest.semanticProgramVersion
    || value.manifestDigest !== manifest.manifestDigest
    || !(manifest.languageContracts as readonly string[]).includes(value.languageContract as string)
    || !(manifest.qualities as readonly string[]).includes(quality)) return false

  const representations = value.evidence === 'runtime'
    ? manifest.representations as readonly string[]
    : [...manifest.representations, ...manifest.plannedRepresentations] as readonly string[]
  if (!representations.includes(value.representation as string)) return false
  if (value.evidence === 'runtime'
    && (value.requiredCapabilities as string[]).some(capability => (
      !(manifest.capabilities as readonly string[]).includes(capability)
    ))) return false

  const effectiveLimits = value.effectiveLimits as Record<string, number>
  const manifestLimits = manifest.limits as Readonly<Record<string, number>>
  const limitKeys = Object.keys(effectiveLimits).sort()
  const manifestLimitKeys = Object.keys(manifestLimits).sort()
  return limitKeys.length === manifestLimitKeys.length
    && limitKeys.every((key, index) => key === manifestLimitKeys[index]
      && effectiveLimits[key] === manifestLimits[key])
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
  if (!isRecord(value)
    || !hasExactKeys(value, ['name', 'message'], ['code', 'start', 'end', 'line', 'column'])
    || typeof value.name !== 'string' || typeof value.message !== 'string') return false
  const hasStart = Object.hasOwn(value, 'start')
  const hasEnd = Object.hasOwn(value, 'end')
  return value.name.length > 0
    && value.name.length <= GEOMETRY_WORKER_PAYLOAD_LIMITS.errorNameCharacters
    && value.message.length <= GEOMETRY_WORKER_PAYLOAD_LIMITS.errorMessageCharacters
    && optionalKey(value, 'line', candidate => isNonNegativeSafeInteger(candidate) && candidate > 0)
    && optionalKey(value, 'column', candidate => isNonNegativeSafeInteger(candidate) && candidate > 0)
    && optionalKey(value, 'code', candidate => typeof candidate === 'string' && candidate.length <= 80)
    && optionalKey(value, 'start', isNonNegativeSafeInteger)
    && optionalKey(value, 'end', isNonNegativeSafeInteger)
    && hasStart === hasEnd
    && (!hasStart || !hasEnd || (typeof value.start === 'number' && typeof value.end === 'number' && value.end >= value.start))
}

function isMeshSourceReference(value: unknown): boolean {
  if (!isRecord(value)
    || !hasExactKeys(value, ['id', 'originalId', 'start', 'end', 'label'], ['operationId', 'instanceId'])) return false
  return isNonNegativeSafeInteger(value.id)
    && isNonNegativeSafeInteger(value.originalId)
    && isNonNegativeSafeInteger(value.start)
    && isNonNegativeSafeInteger(value.end)
    && value.end >= value.start
    && typeof value.label === 'string'
    && value.label.length <= GEOMETRY_WORKER_PAYLOAD_LIMITS.sourceLabelCharacters
    && optionalKey(value, 'operationId', candidate => (typeof candidate === 'string'
      && candidate.length <= GEOMETRY_WORKER_PAYLOAD_LIMITS.identityCharacters
      && candidate.startsWith('op:')))
    && optionalKey(value, 'instanceId', candidate => (typeof candidate === 'string'
      && candidate.length <= GEOMETRY_WORKER_PAYLOAD_LIMITS.identityCharacters
      && candidate.startsWith('entity:')))
}

function isMeshProvenanceRun(value: unknown, triangleCount: number): boolean {
  if (!isRecord(value)
    || !hasExactKeys(value, ['triangleStart', 'triangleEnd', 'source', 'backside'])) return false
  return isNonNegativeSafeInteger(value.triangleStart)
    && isNonNegativeSafeInteger(value.triangleEnd)
    && value.triangleEnd > value.triangleStart
    && value.triangleEnd <= triangleCount
    && typeof value.backside === 'boolean'
    && (value.source === null || isMeshSourceReference(value.source))
}

function hasOrderedNonOverlappingProvenanceRuns(
  value: readonly unknown[],
  triangleCount: number,
): boolean {
  let previousEnd = 0
  for (const run of value) {
    if (!isMeshProvenanceRun(run, triangleCount)) return false
    const current = run as { triangleStart: number; triangleEnd: number }
    if (current.triangleStart < previousEnd) return false
    previousEnd = current.triangleEnd
  }
  return true
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
  if (!hasExactKeys(value, ['version', 'vertexStride', 'leafSize', 'nodeCount', 'bounds', 'nodes', 'triangles'])
    || value.version !== 1
    || value.vertexStride !== MESH_VERTEX_STRIDE
    || !isNonNegativeSafeInteger(value.leafSize)
    || value.leafSize === 0
    || value.leafSize > MAX_BVH_LEAF_SIZE
    || !isNonNegativeSafeInteger(value.nodeCount)
    || !isFloat32Payload(value.bounds)
    || !isUint32Payload(value.nodes)
    || !isUint32Payload(value.triangles)) return false

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
  if (!hasExactKeys(value, [
    'vertices', 'indices', 'bvh', 'edgeIndices', 'color', 'transform',
    'faceIds', 'provenance', 'topology',
  ], ['entityId', 'geometryAssetId', 'faceIdsAuthoritative', 'nativeGeometry'])
    || !isFloat32Payload(value.vertices)
    || !isUint32Payload(value.indices)
    || !isUint32Payload(value.edgeIndices)
    || (value.faceIdsAuthoritative !== undefined && typeof value.faceIdsAuthoritative !== 'boolean')
    || !isUint32Payload(value.faceIds)
    || !isFloat32Payload(value.transform)
    || !isRecord(value.bvh)
    || !isRecord(value.topology)
    || !hasExactKeys(value.topology, ['boundary', 'crease', 'nonManifold', 'degenerate'])) return false

  const triangleCount = value.indices.length / 3
  const bvh = value.bvh
  const topology = value.topology
  const vertexCount = value.vertices.length / MESH_VERTEX_STRIDE
  return optionalKey(value, 'entityId', candidate => typeof candidate === 'string'
      && candidate.length <= GEOMETRY_WORKER_PAYLOAD_LIMITS.identityCharacters
      && candidate.startsWith('entity:'))
    && optionalKey(value, 'geometryAssetId', candidate => typeof candidate === 'string'
        && candidate.length <= GEOMETRY_WORKER_PAYLOAD_LIMITS.identityCharacters
        && candidate.startsWith('asset:'))
    && optionalKey(value, 'nativeGeometry', isNativeGeometryArtifact)
    && Number.isInteger(vertexCount)
    && Number.isInteger(triangleCount)
    && value.vertices.every(Number.isFinite)
    && value.edgeIndices.length % 2 === 0
    && hasOnlyVertexReferences(value.indices, vertexCount)
    && hasOnlyVertexReferences(value.edgeIndices, vertexCount)
    && value.faceIds.length === triangleCount
    && value.transform.length === 16
    && value.transform.every(Number.isFinite)
    && isDenseExactArray(value.color, 4)
    && value.color.length === 4
    && value.color.every(component => typeof component === 'number' && Number.isFinite(component)
      && component >= 0 && component <= 1)
    && isMeshBvh(bvh, triangleCount)
    && isDenseExactArray(value.provenance, triangleCount)
    && hasOrderedNonOverlappingProvenanceRuns(value.provenance, triangleCount)
    && isNonNegativeSafeInteger(topology.boundary)
    && isNonNegativeSafeInteger(topology.crease)
    && isNonNegativeSafeInteger(topology.nonManifold)
    && isNonNegativeSafeInteger(topology.degenerate)
}

/**
 * Reject oversized publications before the deep element/BVH graph scan. This
 * loop is O(mesh count), never O(vertex/triangle count), and allocates no
 * scratch buffers.
 */
function hasSafeSuccessPayload(value: Record<string, unknown>): boolean {
  if (!isDenseExactArray(value.meshes, GEOMETRY_WORKER_PAYLOAD_LIMITS.meshes)
    || !isDenseExactArray(value.warnings, GEOMETRY_WORKER_PAYLOAD_LIMITS.warnings)) return false
  if (!value.warnings.every(warning => typeof warning === 'string'
    && warning.length <= GEOMETRY_WORKER_PAYLOAD_LIMITS.warningCharacters)) return false

  let nativeCharacters = 0
  let triangles = 0
  let bytes = 0
  let provenanceRuns = 0
  let textCharacters = (value.warnings as string[]).reduce((total, warning) => total + warning.length, 0)
  const entityIds = new Set<string>()
  const payloadBuffers = new Set<ArrayBuffer>()
  for (const candidate of value.meshes) {
    if (!isRecord(candidate)
      || !hasExactKeys(candidate, [
        'vertices', 'indices', 'bvh', 'edgeIndices', 'color', 'transform',
        'faceIds', 'provenance', 'topology',
      ], ['entityId', 'geometryAssetId', 'faceIdsAuthoritative', 'nativeGeometry'])
      || !isFloat32Payload(candidate.vertices)
      || !isUint32Payload(candidate.indices)
      || !isUint32Payload(candidate.edgeIndices)
      || (candidate.faceIdsAuthoritative !== undefined && typeof candidate.faceIdsAuthoritative !== 'boolean')
      || !isUint32Payload(candidate.faceIds)
      || !isFloat32Payload(candidate.transform)
      || !isRecord(candidate.bvh)
      || !hasExactKeys(candidate.bvh, [
        'version', 'vertexStride', 'leafSize', 'nodeCount', 'bounds', 'nodes', 'triangles',
      ])
      || !isFloat32Payload(candidate.bvh.bounds)
      || !isUint32Payload(candidate.bvh.nodes)
      || !isUint32Payload(candidate.bvh.triangles)
      || candidate.indices.length % 3 !== 0
      || !isDenseExactArray(candidate.provenance, candidate.indices.length / 3)) return false
    if(candidate.nativeGeometry!==undefined) {
      if(!isRecord(candidate.nativeGeometry)||!hasExactKeys(candidate.nativeGeometry,['version','nodeId','kind','revision','geometryJson','documentJson','documentRevision']))return false
      const native=candidate.nativeGeometry
      if(typeof native.geometryJson!=='string'||typeof native.documentJson!=='string')return false
      nativeCharacters+=native.geometryJson.length+native.documentJson.length
      if(nativeCharacters>MAX_NATIVE_GEOMETRY_CHARACTERS)return false
      bytes+=2*(native.geometryJson.length+native.documentJson.length)
    }
    const views = [
      candidate.vertices, candidate.indices, candidate.edgeIndices, candidate.faceIds,
      candidate.transform, candidate.bvh.bounds, candidate.bvh.nodes, candidate.bvh.triangles,
    ] as const
    for (const view of views) {
      const buffer = view.buffer as ArrayBuffer
      if (payloadBuffers.has(buffer)) return false
      payloadBuffers.add(buffer)
    }
    if (candidate.entityId !== undefined) {
      if (typeof candidate.entityId !== 'string' || entityIds.has(candidate.entityId)) return false
      entityIds.add(candidate.entityId)
      textCharacters += candidate.entityId.length
    }
    if (candidate.geometryAssetId !== undefined) {
      if (typeof candidate.geometryAssetId !== 'string') return false
      textCharacters += candidate.geometryAssetId.length
    }
    for (const run of candidate.provenance) {
      if (!isRecord(run)
        || !hasExactKeys(run, ['triangleStart', 'triangleEnd', 'source', 'backside'])
        || (run.source !== null && (!isRecord(run.source)
          || !hasExactKeys(run.source, [
            'id', 'originalId', 'start', 'end', 'label',
          ], ['operationId', 'instanceId'])))) return false
      if (isRecord(run.source)) {
        if (typeof run.source.label !== 'string') return false
        textCharacters += run.source.label.length
        if (run.source.operationId !== undefined) {
          if (typeof run.source.operationId !== 'string') return false
          textCharacters += run.source.operationId.length
        }
        if (run.source.instanceId !== undefined) {
          if (typeof run.source.instanceId !== 'string') return false
          textCharacters += run.source.instanceId.length
        }
      }
    }
    triangles += candidate.indices.length / 3
    provenanceRuns += candidate.provenance.length
    bytes += candidate.vertices.byteLength + candidate.indices.byteLength
      + candidate.edgeIndices.byteLength + candidate.faceIds.byteLength + candidate.transform.byteLength
      + candidate.bvh.bounds.byteLength + candidate.bvh.nodes.byteLength + candidate.bvh.triangles.byteLength
    if (triangles > GEOMETRY_WORKER_PAYLOAD_LIMITS.triangles
      || bytes > GEOMETRY_WORKER_PAYLOAD_LIMITS.bytes
      || provenanceRuns > triangles + value.meshes.length
      || textCharacters > GEOMETRY_WORKER_PAYLOAD_LIMITS.totalTextCharacters) return false
  }
  return true
}

/**
 * Validate a standalone kernel result before an MCP qualification supervisor
 * accepts data from a disposable child realm. This deliberately applies the
 * same bounded mesh checks as protocol-v5 success publication without adding a
 * Worker-v5 envelope or claiming that the qualification adapter is a v5
 * provider.
 */
export function isGeometryEvaluationResultPayload(value: unknown): value is GeometryEvaluationResult {
  if (!isRecord(value)
    || !hasExactKeys(value, [
      'meshes', 'warnings', 'volume', 'surfaceArea', 'quality', 'reduced', 'timings',
    ])
    || !hasSafeSuccessPayload(value)
    || !isDenseExactArray(value.meshes, GEOMETRY_WORKER_PAYLOAD_LIMITS.meshes)
    || !value.meshes.every(isMeshData)
    || !isDenseExactArray(value.warnings, GEOMETRY_WORKER_PAYLOAD_LIMITS.warnings)
    || !value.warnings.every(warning => typeof warning === 'string'
      && warning.length <= GEOMETRY_WORKER_PAYLOAD_LIMITS.warningCharacters)
    || !isQuality(value.quality)
    || !isNonNegativeFiniteNumber(value.volume)
    || !isNonNegativeFiniteNumber(value.surfaceArea)
    || typeof value.reduced !== 'boolean'
    || !isPhaseTimings(value.timings)) return false
  return true
}

function hasValidEventEnvelope(candidate: Record<string, unknown>): boolean {
  return candidate.protocolVersion === GEOMETRY_WORKER_PROTOCOL_VERSION
    && isNonNegativeSafeInteger(candidate.documentRevision)
    && isNonNegativeSafeInteger(candidate.jobId)
    && isQuality(candidate.quality)
    && typeof candidate.sourceSha256 === 'string'
    && /^[a-f0-9]{64}$/.test(candidate.sourceSha256)
    && isPhase(candidate.phase)
}

export function isGeometryWorkerRequest(value: unknown): value is GeometryWorkerRequest {
  try {
    if (!isRecord(value) || !hasBoundedDataProperties(value, 12)) return false
    const candidate = value as Partial<GeometryWorkerRequest>
    const envelopeIsValid = candidate.protocolVersion === GEOMETRY_WORKER_PROTOCOL_VERSION
      && (candidate.type === 'build' || candidate.type === 'cancel')
      && isNonNegativeSafeInteger(candidate.documentRevision)
      && isNonNegativeSafeInteger(candidate.jobId)
    if (!envelopeIsValid) return false
    if (candidate.type === 'build') {
      const build = candidate as Partial<GeometryBuildRequest>
      return hasExactKeys(value, [
        'protocolVersion', 'type', 'documentRevision', 'jobId', 'source',
        'sourceSha256', 'quality',
      ])
        && typeof build.source === 'string'
        && build.source.length <= MAX_GEOMETRY_SOURCE_CHARACTERS
        && isWellFormedUnicode(build.source)
        && typeof build.sourceSha256 === 'string'
        && /^[a-f0-9]{64}$/.test(build.sourceSha256)
        && sha256Hex(build.source) === build.sourceSha256
        && isQuality(build.quality)
    }
    const cancel = candidate as Partial<GeometryCancelRequest>
    return hasExactKeys(value, [
      'protocolVersion', 'type', 'documentRevision', 'jobId', 'reason',
    ]) && isCancelReason(cancel.reason)
  } catch {
    return false
  }
}

export function isGeometryWorkerEvent(value: unknown): value is GeometryWorkerEvent {
  try {
    if (!isRecord(value)
      || !hasBoundedDataProperties(value, 24)
      || !hasValidEventEnvelope(value)) return false

    switch (value.status) {
      case 'accepted':
        return hasExactKeys(value, [
          'protocolVersion', 'documentRevision', 'jobId', 'quality', 'sourceSha256',
          'status', 'phase',
        ]) && value.phase === 'queued'
      case 'started':
        return hasExactKeys(value, [
          'protocolVersion', 'documentRevision', 'jobId', 'quality', 'sourceSha256',
          'status', 'phase',
        ]) && (value.phase === 'initializing' || value.phase === 'compiling')
      case 'progress':
        return hasExactKeys(value, [
          'protocolVersion', 'documentRevision', 'jobId', 'quality', 'sourceSha256',
          'status', 'phase', 'progress',
        ]) && (value.progress === null
          || (typeof value.progress === 'number'
            && Number.isFinite(value.progress)
            && value.progress >= 0
            && value.progress <= 1))
      case 'succeeded':
        return hasExactKeys(value, [
          'protocolVersion', 'documentRevision', 'jobId', 'quality', 'sourceSha256',
          'status', 'phase', 'execution', 'meshes', 'warnings', 'volume', 'surfaceArea',
          'reduced', 'timings', 'durationMs',
        ])
          && value.phase === 'complete'
          && isQuality(value.quality)
          && isExecutionDescriptor(value.execution, value.quality)
          && hasSafeSuccessPayload(value)
          && isDenseExactArray(value.meshes, GEOMETRY_WORKER_PAYLOAD_LIMITS.meshes)
          && value.meshes.every(isMeshData)
          && isNonNegativeFiniteNumber(value.volume)
          && isNonNegativeFiniteNumber(value.surfaceArea)
          && typeof value.reduced === 'boolean'
          && isPhaseTimings(value.timings)
          && isNonNegativeFiniteNumber(value.durationMs)
      case 'failed':
        return hasExactKeys(value, [
          'protocolVersion', 'documentRevision', 'jobId', 'quality', 'sourceSha256',
          'status', 'phase', 'error', 'durationMs',
        ], ['execution'])
          && optionalKey(value, 'execution', candidate => isQuality(value.quality)
            && isExecutionDescriptor(candidate, value.quality, 'runtime-or-planned'))
          && isBuildError(value.error)
          && isNonNegativeFiniteNumber(value.durationMs)
      case 'cancelled':
        return hasExactKeys(value, [
          'protocolVersion', 'documentRevision', 'jobId', 'quality', 'sourceSha256',
          'status', 'phase', 'reason', 'durationMs',
        ], ['execution'])
          && optionalKey(value, 'execution', candidate => isQuality(value.quality)
            && isExecutionDescriptor(candidate, value.quality, 'runtime-or-planned'))
          && isCancelReason(value.reason)
          && isNonNegativeFiniteNumber(value.durationMs)
      case 'stale':
        return hasExactKeys(value, [
          'protocolVersion', 'documentRevision', 'jobId', 'quality', 'sourceSha256',
          'status', 'phase', 'durationMs',
        ], ['execution', 'supersededBy'])
          && optionalKey(value, 'execution', candidate => isQuality(value.quality)
            && isExecutionDescriptor(candidate, value.quality, 'runtime-or-planned'))
          && optionalKey(value, 'supersededBy', candidate => isRecord(candidate)
            && hasExactKeys(candidate, ['documentRevision', 'jobId'])
            && isNonNegativeSafeInteger(candidate.documentRevision)
            && isNonNegativeSafeInteger(candidate.jobId))
          && isNonNegativeFiniteNumber(value.durationMs)
      default:
        return false
    }
  } catch {
      return false
  }
}
