import { GEOMETRY_WORKER_PAYLOAD_LIMITS, GEOMETRY_WORKER_PROTOCOL_VERSION } from '../geometryWorkerProtocol'
import type { GeometryBuildRequest, GeometryCancelRequest, GeometryWorkerEvent, GeometryWorkerRequest } from '../geometryWorkerProtocol'
import type { GeometryEvaluationResult } from '../../core/build'
import { MAX_GEOMETRY_SOURCE_CHARACTERS } from '../../core/geometryExecution'
import { MAX_NATIVE_GEOMETRY_CHARACTERS } from '../../core/nativeGeometry'
import { hasBoundedDataProperties, hasExactKeys, isBuildError, isCancelReason, isDenseExactArray, isExecutionDescriptor, isFloat32Payload, isMeshData, isNonNegativeFiniteNumber, isNonNegativeSafeInteger, isPhase, isPhaseTimings, isQuality, isRecord, isUint32Payload, optionalKey } from './isRecord'
import { isWellFormedUnicode } from '../../core/unicode'
import { sha256Hex } from '../../core/sha256'

/**
 * Reject oversized publications before the deep element/BVH graph scan. This
 * loop is O(mesh count), never O(vertex/triangle count), and allocates no
 * scratch buffers.
 */
export function hasSafeSuccessPayload(value: Record<string, unknown>): boolean {
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
      ], ['entityId', 'geometryAssetId', 'faceIdsAuthoritative', 'faceIdsInferred', 'nativeGeometry'])
      || !isFloat32Payload(candidate.vertices)
      || !isUint32Payload(candidate.indices)
      || !isUint32Payload(candidate.edgeIndices)
      || (candidate.faceIdsInferred !== undefined && typeof candidate.faceIdsInferred !== 'boolean')
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

export function hasValidEventEnvelope(candidate: Record<string, unknown>): boolean {
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
      && (candidate.type === 'build' || candidate.type === 'cancel' || candidate.type === 'sweep-preview-ack')
      && isNonNegativeSafeInteger(candidate.documentRevision)
      && isNonNegativeSafeInteger(candidate.jobId)
    if (!envelopeIsValid) return false
    if (candidate.type === 'build') {
      const build = candidate as Partial<GeometryBuildRequest>
      return hasExactKeys(value, [
        'protocolVersion', 'type', 'documentRevision', 'jobId', 'source',
        'sourceSha256', 'quality',
      ], ['selectionSurfaces','acknowledgeSweepPreviews'])
        && (build.acknowledgeSweepPreviews===undefined||typeof build.acknowledgeSweepPreviews==='boolean')
        && (build.selectionSurfaces === undefined || typeof build.selectionSurfaces === 'boolean')
        && typeof build.source === 'string'
        && build.source.length <= MAX_GEOMETRY_SOURCE_CHARACTERS
        && isWellFormedUnicode(build.source)
        && typeof build.sourceSha256 === 'string'
        && /^[a-f0-9]{64}$/.test(build.sourceSha256)
        && sha256Hex(build.source) === build.sourceSha256
        && isQuality(build.quality)
    }
    if(candidate.type==='sweep-preview-ack')return hasExactKeys(value,['protocolVersion','type','documentRevision','jobId','quality','sourceSha256','nodeId','sections'])
      && isQuality(candidate.quality) && typeof candidate.sourceSha256==='string' && /^[a-f0-9]{64}$/.test(candidate.sourceSha256)
      && typeof candidate.nodeId==='string' && candidate.nodeId.length>0 && candidate.nodeId.length<=GEOMETRY_WORKER_PAYLOAD_LIMITS.identityCharacters
      && Number.isInteger(candidate.sections) && (candidate.sections as number)>=2 && (candidate.sections as number)<=1025
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
      case 'sweep-preview':
        return hasExactKeys(value,[
          'protocolVersion','documentRevision','jobId','quality','sourceSha256',
          'status','phase','nodeId','sections','accepted','sampledControlDeviation','budget','meshes',
        ],['phaseResolved','continuousBound','continuousErrorUpper','knownProfileErrorUpper','frameTransportCertified','certifiedErrorUpper','endpointContourErrorUpper','profileRegularityCertified','wallRegularityCertified']) && value.phase==='compiling'
          && typeof value.nodeId==='string' && value.nodeId.length>0
          && value.nodeId.length<=GEOMETRY_WORKER_PAYLOAD_LIMITS.identityCharacters
          && Number.isInteger(value.sections) && (value.sections as number)>=2 && (value.sections as number)<=1025
          && typeof value.accepted==='boolean'
          && isNonNegativeFiniteNumber(value.sampledControlDeviation)
          && isNonNegativeFiniteNumber(value.budget)
          && hasSafeSuccessPayload({...value,warnings:[]})
          && (value.continuousBound===undefined||typeof value.continuousBound==='boolean')
          && (value.continuousBound!==true||isNonNegativeFiniteNumber(value.continuousErrorUpper))
          && (value.phaseResolved===undefined||typeof value.phaseResolved==='boolean')
          && (value.frameTransportCertified===undefined||typeof value.frameTransportCertified==='boolean')
          && (value.profileRegularityCertified===undefined||typeof value.profileRegularityCertified==='boolean')
          && (value.wallRegularityCertified===undefined||value.wallRegularityCertified===null||typeof value.wallRegularityCertified==='boolean')
          && (value.certifiedErrorUpper===undefined||value.certifiedErrorUpper===null||isNonNegativeFiniteNumber(value.certifiedErrorUpper))
          && (value.endpointContourErrorUpper===undefined||value.endpointContourErrorUpper===null||(Array.isArray(value.endpointContourErrorUpper)&&value.endpointContourErrorUpper.length===2&&value.endpointContourErrorUpper.every(isNonNegativeFiniteNumber)))
          && (value.continuousErrorUpper===undefined||isNonNegativeFiniteNumber(value.continuousErrorUpper))
          && (value.knownProfileErrorUpper===undefined||isNonNegativeFiniteNumber(value.knownProfileErrorUpper))
          && (value.knownProfileErrorUpper===undefined||value.continuousErrorUpper===undefined||value.knownProfileErrorUpper===value.continuousErrorUpper)
          && (value.continuousBound===undefined?value.accepted===((value.knownProfileErrorUpper===undefined||(value.knownProfileErrorUpper as number)<=(value.budget as number))&&(value.sampledControlDeviation as number)<=(value.budget as number)&&(value.phaseResolved??true)&&(value.frameTransportCertified??true)&&(value.profileRegularityCertified??true)&&(value.wallRegularityCertified===undefined||value.wallRegularityCertified===true)&&(value.certifiedErrorUpper===undefined?(value.continuousErrorUpper===undefined||(value.continuousErrorUpper as number)<=(value.budget as number)):(value.certifiedErrorUpper!==null&&(value.certifiedErrorUpper as number)<=(value.budget as number)))):(!value.accepted||(value.continuousBound&&((value.knownProfileErrorUpper===undefined||(value.knownProfileErrorUpper as number)<=(value.budget as number))&&(value.sampledControlDeviation as number)<=(value.budget as number)&&(value.phaseResolved??true)&&(value.frameTransportCertified??true)&&(value.profileRegularityCertified??true)&&(value.wallRegularityCertified===undefined||value.wallRegularityCertified===true)&&(value.certifiedErrorUpper===undefined?(value.continuousErrorUpper===undefined||(value.continuousErrorUpper as number)<=(value.budget as number)):(value.certifiedErrorUpper!==null&&(value.certifiedErrorUpper as number)<=(value.budget as number)))))))
          && (value.meshes as unknown[]).every(mesh=>isMeshData(mesh)&&!('nativeGeometry' in mesh)
            && mesh.faceIdsAuthoritative===false && mesh.provenance.every(run=>run.source===null))
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
