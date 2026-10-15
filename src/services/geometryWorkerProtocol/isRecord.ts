import { BVH_LEAF_BIT, BVH_LEAF_COUNT_MASK, CANCEL_REASONS, GEOMETRY_WORKER_PAYLOAD_LIMITS, MAX_BVH_LEAF_SIZE, MESH_VERTEX_STRIDE, PHASES, QUALITIES } from '../geometryWorkerProtocol'
import type { GeometryBuildError, GeometryBuildPhase, GeometryCancelReason } from '../geometryWorkerProtocol'
import { GEOMETRY_MANIFEST_ARCHIVE, geometryEngineClassForLanguageContract } from '../../core/geometryExecution'
import type { GeometryExecutionDescriptor } from '../../core/geometryExecution'
import type { GeometryPhaseTimings, GeometryQuality } from '../../core/build'
import type { MeshData } from '../../core/mesh'
import { isNativeGeometryArtifact } from '../../core/nativeGeometry'
import { isNonNegativeFiniteNumber, isNonNegativeSafeInteger } from '../workerPayloadPrimitives'

export function isRecord(value: unknown): value is Record<string, unknown> {
  if (value === null || Array.isArray(value) || typeof value !== 'object') return false
  const prototype = Object.getPrototypeOf(value)
  return prototype === Object.prototype || prototype === null
}

export function isDenseExactArray(value: unknown, maximumLength = Number.MAX_SAFE_INTEGER): value is unknown[] {
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

export function hasExactKeys(
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

export function hasBoundedDataProperties(value: Record<string, unknown>, maximumKeys: number): boolean {
  const keys = Reflect.ownKeys(value)
  return keys.length <= maximumKeys && keys.every(key => {
    if (typeof key !== 'string') return false
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    return descriptor !== undefined
      && Object.hasOwn(descriptor, 'value')
      && descriptor.enumerable
  })
}

export function hasExclusiveArrayBuffer(view: ArrayBufferView): boolean {
  return view.buffer instanceof ArrayBuffer
    && view.byteOffset === 0
    && view.byteLength === view.buffer.byteLength
}

export function isFloat32Payload(value: unknown): value is Float32Array {
  return value instanceof Float32Array && hasExclusiveArrayBuffer(value)
}

export function isUint32Payload(value: unknown): value is Uint32Array {
  return value instanceof Uint32Array && hasExclusiveArrayBuffer(value)
}

export function optionalKey(
  value: Record<string, unknown>,
  key: string,
  predicate: (candidate: unknown) => boolean,
): boolean {
  return !Object.hasOwn(value, key) || predicate(value[key])
}

export function isPhaseTimings(value: unknown): value is GeometryPhaseTimings {
  if (!isRecord(value)
    || !hasExactKeys(value, ['parseMs', 'bindMs', 'initializeMs', 'evaluateMs', 'analyzeMs'])) return false
  const candidate = value as Partial<GeometryPhaseTimings>
  return isNonNegativeFiniteNumber(candidate.parseMs)
    && isNonNegativeFiniteNumber(candidate.bindMs)
    && isNonNegativeFiniteNumber(candidate.initializeMs)
    && isNonNegativeFiniteNumber(candidate.evaluateMs)
    && isNonNegativeFiniteNumber(candidate.analyzeMs)
}

export function isExecutionDescriptor(
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

export function isPhase(value: unknown): value is GeometryBuildPhase {
  return typeof value === 'string' && PHASES.has(value as GeometryBuildPhase)
}

export function isQuality(value: unknown): value is GeometryQuality {
  return typeof value === 'string' && QUALITIES.has(value as GeometryQuality)
}

export function isCancelReason(value: unknown): value is GeometryCancelReason {
  return typeof value === 'string' && CANCEL_REASONS.has(value as GeometryCancelReason)
}

export function isBuildError(value: unknown): value is GeometryBuildError {
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

export function isMeshSourceReference(value: unknown): boolean {
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

export function isMeshProvenanceRun(value: unknown, triangleCount: number): boolean {
  if (!isRecord(value)
    || !hasExactKeys(value, ['triangleStart', 'triangleEnd', 'source', 'backside'])) return false
  return isNonNegativeSafeInteger(value.triangleStart)
    && isNonNegativeSafeInteger(value.triangleEnd)
    && value.triangleEnd > value.triangleStart
    && value.triangleEnd <= triangleCount
    && typeof value.backside === 'boolean'
    && (value.source === null || isMeshSourceReference(value.source))
}

export function hasOrderedNonOverlappingProvenanceRuns(
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

export function hasOnlyVertexReferences(indices: Uint32Array, vertexCount: number): boolean {
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
export function isMeshBvh(
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

export function isMeshData(value: unknown): value is MeshData {
  if (!isRecord(value)) return false
  if (!hasExactKeys(value, [
    'vertices', 'indices', 'bvh', 'edgeIndices', 'color', 'transform',
    'faceIds', 'provenance', 'topology',
  ], ['entityId', 'geometryAssetId', 'faceIdsAuthoritative', 'faceIdsInferred', 'nativeGeometry'])
    || !isFloat32Payload(value.vertices)
    || !isUint32Payload(value.indices)
    || !isUint32Payload(value.edgeIndices)
    || (value.faceIdsInferred !== undefined && typeof value.faceIdsInferred !== 'boolean')
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
