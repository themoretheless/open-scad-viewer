import { ACTIVE_GEOMETRY_MANIFEST_VERSIONS, GEOMETRY_ENGINE_ROUTES, GEOMETRY_MANIFEST_ARCHIVE, MAX_GEOMETRY_SOURCE_CHARACTERS, canonicalJson, planGeometrySourceExecution } from '../../core/geometryExecution'
import type { GeometryBuildPurpose, GeometryEngineRegistrySnapshot, GeometryExecutionDescriptor } from '../../core/geometryExecution'
import { AVAILABILITY_CAUSES, BUILD_REQUEST_KEYS, CANCEL_KEYS, CAPABILITIES_REQUEST_KEYS, DIRECT_GEOMETRY_IDENTITY, DIRECT_GEOMETRY_PROTOCOL_VERSION, ERROR_KEYS, EVENT_KEYS, EXECUTION_KEYS, IDENTITY_KEYS, MAX_CAPABILITIES_DATA_NODES, MAX_CAPABILITIES_TEXT_CHARACTERS, MAX_ERROR_MESSAGE_CHARACTERS, PURPOSES } from '../directGeometryProtocol'
import type { DirectGeometryBuildRequest, DirectGeometryCancel, DirectGeometryCapabilitiesRequest, DirectGeometryError, DirectGeometryIdentity, DirectGeometryRequest, DirectGeometryStarted, DirectGeometryTerminal } from '../directGeometryProtocol'
import { GEOMETRY_WORKER_PAYLOAD_LIMITS, isGeometryEvaluationResultPayload } from '../../services/geometryWorkerProtocol'
import type { GeometryEngineAvailabilityCause } from '../../services/geometryBuildEngine'
import type { GeometryQuality } from '../../core/build'
import { exactDataKeys, ownData, positiveSafeInteger, record } from '../../core/plainData'
import { sha256Hex } from '../../core/sha256'
import { truncateWellFormed, isWellFormedUnicode as wellFormed } from '../../core/unicode'

export function denseDataArray(value: unknown, maximum: number): value is unknown[] {
  if (!Array.isArray(value) || value.length > maximum) return false
  const keys = Reflect.ownKeys(value)
  if (keys.length !== value.length + 1 || !keys.includes('length')) return false
  for (let index = 0; index < value.length; index++) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index))
    if (descriptor === undefined || !descriptor.enumerable
      || !Object.hasOwn(descriptor, 'value')) return false
  }
  return true
}

export function nullablePositiveInteger(value: unknown): value is number | null {
  return value === null || positiveSafeInteger(value)
}

export function nullableNonNegativeInteger(value: unknown): value is number | null {
  return value === null || (Number.isSafeInteger(value) && (value as number) >= 0)
}

export function compactMessage(value: string): string {
  return truncateWellFormed(
    value.replace(/[\r\n\u2028\u2029]+/g, ' ').trim(),
    MAX_ERROR_MESSAGE_CHARACTERS,
  )
}

export function isQuality(value: unknown): value is GeometryQuality {
  return value === 'preview' || value === 'full'
}

export function isPurpose(value: unknown): value is GeometryBuildPurpose {
  return typeof value === 'string' && PURPOSES.has(value as GeometryBuildPurpose)
}

export function isIdentity(value: unknown): value is DirectGeometryIdentity {
  const candidate = record(value)
  return candidate !== null && exactDataKeys(candidate, IDENTITY_KEYS)
    && IDENTITY_KEYS.every(key => Object.is(
      candidate[key],
      DIRECT_GEOMETRY_IDENTITY[key as keyof DirectGeometryIdentity],
    ))
}

export function hasBaseRequest(value: Record<string, unknown>): boolean {
  return value.protocolVersion === DIRECT_GEOMETRY_PROTOCOL_VERSION
    && positiveSafeInteger(value.workerEpoch)
    && positiveSafeInteger(value.jobId)
    && isIdentity(value.identity)
}

export function isDirectGeometryBuildRequest(
  value: unknown,
): value is DirectGeometryBuildRequest {
  try {
    const candidate = record(value)
    return candidate !== null && exactDataKeys(candidate, BUILD_REQUEST_KEYS)
      && hasBaseRequest(candidate) && candidate.type === 'build'
      && typeof candidate.source === 'string'
      && candidate.source.length <= MAX_GEOMETRY_SOURCE_CHARACTERS
      && wellFormed(candidate.source)
      && typeof candidate.sourceSha256 === 'string'
      && /^[a-f0-9]{64}$/.test(candidate.sourceSha256)
      && sha256Hex(candidate.source) === candidate.sourceSha256
      && isQuality(candidate.quality) && isPurpose(candidate.purpose)
  } catch {
    return false
  }
}

export function isDirectGeometryCapabilitiesRequest(
  value: unknown,
): value is DirectGeometryCapabilitiesRequest {
  try {
    const candidate = record(value)
    return candidate !== null && exactDataKeys(candidate, CAPABILITIES_REQUEST_KEYS)
      && hasBaseRequest(candidate) && candidate.type === 'capabilities'
      && candidate.sourceSha256 === null
      && candidate.quality === null && candidate.purpose === null
  } catch {
    return false
  }
}

export function isDirectGeometryRequest(value: unknown): value is DirectGeometryRequest {
  return isDirectGeometryBuildRequest(value) || isDirectGeometryCapabilitiesRequest(value)
}

export function correlationMatches(
  candidate: Record<string, unknown>,
  request: DirectGeometryRequest,
): boolean {
  return candidate.protocolVersion === request.protocolVersion
    && candidate.workerEpoch === request.workerEpoch
    && candidate.jobId === request.jobId
    && candidate.sourceSha256 === request.sourceSha256
    && candidate.quality === request.quality
    && candidate.purpose === request.purpose
    && candidate.kind === request.type
    && isIdentity(candidate.identity)
}

export function isDirectGeometryCancel(
  value: unknown,
  request: DirectGeometryRequest,
): value is DirectGeometryCancel {
  try {
    const candidate = record(value)
    return candidate !== null && exactDataKeys(candidate, CANCEL_KEYS)
      && candidate.type === 'cancel'
      && candidate.protocolVersion === request.protocolVersion
      && candidate.workerEpoch === request.workerEpoch
      && candidate.jobId === request.jobId
      && candidate.sourceSha256 === request.sourceSha256
      && candidate.quality === request.quality
      && candidate.purpose === request.purpose
      && (candidate.reason === 'cancelled' || candidate.reason === 'deadline'
        || candidate.reason === 'closed')
      && isIdentity(candidate.identity)
  } catch {
    return false
  }
}

export function isDirectGeometryStarted(
  value: unknown,
  request: DirectGeometryRequest,
): value is DirectGeometryStarted {
  try {
    const candidate = record(value)
    return candidate !== null && exactDataKeys(candidate, EVENT_KEYS)
      && candidate.status === 'started' && correlationMatches(candidate, request)
  } catch {
    return false
  }
}

export function isExecutionForRequest(
  value: unknown,
  request: DirectGeometryBuildRequest,
  allowedEvidence: 'runtime' | 'runtime-or-planned',
): value is GeometryExecutionDescriptor {
  if (!isBoundedPlainData(value, { nodes: 256, text: 16 * 1024 })) return false
  const candidate = record(value)
  if (candidate === null || !exactDataKeys(candidate, EXECUTION_KEYS)) return false
  let planned: GeometryExecutionDescriptor
  try {
    planned = planGeometrySourceExecution(request.source, {
      quality: request.quality,
      purpose: request.purpose,
    })
  } catch {
    return false
  }
  if (candidate.evidence !== 'runtime'
    && (allowedEvidence !== 'runtime-or-planned' || candidate.evidence !== 'planned')) return false
  return canonicalJson(candidate) === canonicalJson({ ...planned, evidence: candidate.evidence })
}

export function isDirectGeometryError(value: unknown): value is DirectGeometryError {
  const candidate = record(value)
  if (candidate === null || !exactDataKeys(candidate, ERROR_KEYS)
    || ![
      'openscad-parse', 'aborted', 'language-contract', 'engine-unavailable',
      'capability-unavailable', 'provider-contract', 'internal',
    ].includes(String(candidate.category))
    || typeof candidate.name !== 'string' || candidate.name.length < 1
    || candidate.name.length > 128 || !wellFormed(candidate.name)
    || typeof candidate.message !== 'string'
    || candidate.message.length > MAX_ERROR_MESSAGE_CHARACTERS
    || !wellFormed(candidate.message) || /[\r\n\u2028\u2029]/.test(candidate.message)
    || !(candidate.code === null || (typeof candidate.code === 'string'
      && candidate.code.length <= 80 && wellFormed(candidate.code)))
    || !nullablePositiveInteger(candidate.line)
    || !nullablePositiveInteger(candidate.column)
    || !nullableNonNegativeInteger(candidate.start)
    || !nullableNonNegativeInteger(candidate.end)
    || !(candidate.reportedContract === null
      || (typeof candidate.reportedContract === 'string'
        && candidate.reportedContract.length <= 128
        && wellFormed(candidate.reportedContract)))
    || !(candidate.availabilityCause === null
      || (typeof candidate.availabilityCause === 'string'
        && AVAILABILITY_CAUSES.has(candidate.availabilityCause as GeometryEngineAvailabilityCause)))
    || !denseDataArray(candidate.missingCapabilities, 32)
    || candidate.missingCapabilities.some(capability => typeof capability !== 'string'
      || !/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(capability))) return false

  const start = candidate.start as number | null
  const end = candidate.end as number | null
  if ((start === null) !== (end === null) || (start !== null && end! < start)) return false
  const noPositions = candidate.line === null && candidate.column === null
    && start === null && end === null
  const noRouting = candidate.reportedContract === null
    && candidate.availabilityCause === null
    && candidate.missingCapabilities.length === 0

  switch (candidate.category) {
    case 'openscad-parse':
      return candidate.name === 'OpenSCADParseError'
        && candidate.line !== null && candidate.column !== null
        && start !== null && end !== null && noRouting
    case 'aborted':
      return candidate.name === 'AbortedError' && noPositions && noRouting
    case 'language-contract':
      return candidate.name === 'GeometryLanguageContractError'
        && candidate.column === null && start === null && end === null
        && candidate.availabilityCause === null
        && candidate.missingCapabilities.length === 0
    case 'engine-unavailable':
      return candidate.name === 'GeometryEngineUnavailableError'
        && noPositions && candidate.reportedContract === null
        && candidate.availabilityCause !== null
        && candidate.missingCapabilities.length === 0
    case 'capability-unavailable':
      return candidate.name === 'GeometryCapabilityUnavailableError'
        && noPositions && candidate.reportedContract === null
        && candidate.availabilityCause === null
        && candidate.missingCapabilities.length > 0
    case 'provider-contract':
      return candidate.name === 'GeometryProviderContractError' && noPositions && noRouting
    case 'internal':
      return candidate.name === 'DirectGeometryWorkerError'
        && noPositions && noRouting && candidate.code === null
    default:
      return false
  }
}

/** Cheap typed-view budget before the shared deep result validator. */
export function isWithinDirectIpcLimits(value: unknown): boolean {
  const terminal = record(value)
  if (terminal === null || ownData(terminal, 'status') !== 'succeeded'
    || ownData(terminal, 'kind') !== 'build') return true
  const built = record(ownData(terminal, 'built'))
  const result = built === null ? null : record(ownData(built, 'result'))
  const meshes = result === null ? null : ownData(result, 'meshes')
  if (!denseDataArray(meshes, GEOMETRY_WORKER_PAYLOAD_LIMITS.meshes)) return false

  let triangles = 0
  let bytes = 0
  for (const meshValue of meshes) {
    const mesh = record(meshValue)
    const indices = mesh === null ? null : ownData(mesh, 'indices')
    const bvh = mesh === null ? null : record(ownData(mesh, 'bvh'))
    if (mesh === null || !(indices instanceof Uint32Array) || bvh === null) return false
    triangles += indices.length / 3
    const views = [
      ownData(mesh, 'vertices'), indices, ownData(mesh, 'edgeIndices'),
      ownData(mesh, 'faceIds'), ownData(mesh, 'transform'), ownData(bvh, 'bounds'),
      ownData(bvh, 'nodes'), ownData(bvh, 'triangles'),
    ]
    for (const view of views) {
      if (!ArrayBuffer.isView(view)) return false
      bytes += view.byteLength
    }
    if (triangles > GEOMETRY_WORKER_PAYLOAD_LIMITS.triangles
      || bytes > GEOMETRY_WORKER_PAYLOAD_LIMITS.bytes) return false
  }
  return true
}

export function isBoundedPlainData(
  value: unknown,
  budget: { nodes: number; text: number },
  depth = 0,
): boolean {
  if (--budget.nodes < 0 || depth > 12) return false
  if (value === null || typeof value === 'boolean') return true
  if (typeof value === 'number') return Number.isFinite(value)
  if (typeof value === 'string') {
    budget.text -= value.length
    return budget.text >= 0 && wellFormed(value)
  }
  if (Array.isArray(value)) {
    return denseDataArray(value, 256)
      && value.every(item => isBoundedPlainData(item, budget, depth + 1))
  }
  const candidate = record(value)
  if (candidate === null) return false
  const keys = Reflect.ownKeys(candidate)
  if (keys.length > 128) return false
  return keys.every(key => {
    if (typeof key !== 'string') return false
    const descriptor = Object.getOwnPropertyDescriptor(candidate, key)
    if (descriptor === undefined || !descriptor.enumerable
      || !Object.hasOwn(descriptor, 'value')) return false
    budget.text -= key.length
    return budget.text >= 0 && isBoundedPlainData(descriptor.value, budget, depth + 1)
  })
}

/** Exact, bounded snapshot from a disposable capabilities worker. */
export function isDirectGeometryCapabilities(
  value: unknown,
): value is GeometryEngineRegistrySnapshot {
  try {
    if (!isBoundedPlainData(value, {
      nodes: MAX_CAPABILITIES_DATA_NODES,
      text: MAX_CAPABILITIES_TEXT_CHARACTERS,
    })) return false
    const candidate = record(value)
    if (candidate === null || !exactDataKeys(candidate, [
      'contractVersion', 'sourceDirectedRouting', 'automaticFallback', 'routes', 'engines',
    ]) || candidate.contractVersion !== 1
      || candidate.sourceDirectedRouting !== true
      || candidate.automaticFallback !== false
      || canonicalJson(candidate.routes) !== canonicalJson(GEOMETRY_ENGINE_ROUTES)
      || !denseDataArray(candidate.engines, 2)
      || candidate.engines.length !== 2) return false

    const manifests = [
      GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS.mesh],
      GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS.brep],
    ] as const
    return candidate.engines.every((engineValue, index) => {
      const engine = record(engineValue)
      const manifest = manifests[index]
      if (engine === null
        || !exactDataKeys(engine, [...Object.keys(manifest), 'availability', 'unavailableReason'])
        || (engine.availability !== 'available' && engine.availability !== 'unavailable')
        || (engine.availability === 'available'
          ? engine.unavailableReason !== null
          : !(typeof engine.unavailableReason === 'string'
            && engine.unavailableReason.length > 0
            && engine.unavailableReason.length <= 4_096
            && wellFormed(engine.unavailableReason)))) return false
      const staticPart = Object.fromEntries(Object.keys(manifest).map(key => [key, engine[key]]))
      return canonicalJson(staticPart) === canonicalJson(manifest)
    })
  } catch {
    return false
  }
}

export function failureExecutionIsConsistent(
  execution: GeometryExecutionDescriptor | null,
  error: DirectGeometryError,
  request: DirectGeometryRequest,
): boolean {
  if (request.type === 'capabilities') {
    return execution === null && error.category === 'internal'
  }
  if (error.category === 'language-contract') return execution === null
  if (error.category === 'engine-unavailable' || error.category === 'capability-unavailable') {
    return execution !== null
      && isExecutionForRequest(execution, request, 'runtime-or-planned')
      && execution.evidence === 'planned'
  }
  if (error.category === 'openscad-parse' || error.category === 'aborted'
    || error.category === 'provider-contract') {
    return execution !== null && isExecutionForRequest(execution, request, 'runtime')
      && (error.category !== 'openscad-parse'
        || (error.start !== null && error.end !== null
          && error.start <= request.source.length && error.end <= request.source.length))
  }
  return execution === null || isExecutionForRequest(execution, request, 'runtime-or-planned')
}

export function isDirectGeometryTerminal(
  value: unknown,
  request: DirectGeometryRequest,
): value is DirectGeometryTerminal {
  try {
    const candidate = record(value)
    if (candidate === null || !correlationMatches(candidate, request)) return false
    if (candidate.status === 'succeeded' && candidate.kind === 'build') {
      if (request.type !== 'build'
        || !exactDataKeys(candidate, [...EVENT_KEYS, 'built'])) return false
      const built = record(candidate.built)
      return built !== null && exactDataKeys(built, ['result', 'execution'])
        && isGeometryEvaluationResultPayload(built.result)
        && built.result.quality === request.quality
        && isExecutionForRequest(built.execution, request, 'runtime')
    }
    if (candidate.status === 'succeeded' && candidate.kind === 'capabilities') {
      return request.type === 'capabilities'
        && exactDataKeys(candidate, [...EVENT_KEYS, 'capabilities'])
        && isDirectGeometryCapabilities(candidate.capabilities)
    }
    if (candidate.status !== 'failed'
      || !exactDataKeys(candidate, [...EVENT_KEYS, 'execution', 'error'])
      || !(candidate.execution === null || record(candidate.execution) !== null)
      || !isDirectGeometryError(candidate.error)) return false
    return failureExecutionIsConsistent(
      candidate.execution as GeometryExecutionDescriptor | null,
      candidate.error,
      request,
    )
  } catch {
    return false
  }
}

export function isDirectGeometryNodeTerminal(
  value: unknown,
  request: DirectGeometryRequest,
): value is DirectGeometryTerminal {
  return isWithinDirectIpcLimits(value) && isDirectGeometryTerminal(value, request)
}
