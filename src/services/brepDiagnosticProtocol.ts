/** Independent diagnostic transport. This is never a GeometryExecutionDescriptor. */
import {MAX_GEOMETRY_SOURCE_CHARACTERS, parseGeometrySourceRoutingHeader} from '../core/geometryExecution'
import {MAX_NATIVE_GEOMETRY_CHARACTERS} from '../core/nativeGeometry'
import {sha256Hex} from '../core/sha256'
import {geometryAssetId} from '../core/scene'
import {semanticResultItems} from '../core/semanticProgram'
import {normalizeSemanticProgram} from './semanticProgramValidator'
import {attestSemanticProgram} from './semanticProgramCodec'
import packedKernel from '../generated/geometry-kernels/bytes'
import {isGeometryEvaluationResultPayload} from './geometryWorkerProtocol'
import type {BrepSemanticDisplayPolicy, BrepSemanticSceneBuild} from './brepSemanticScene'

export const BREP_DIAGNOSTIC_LIMITS = Object.freeze({sourceCharacters: MAX_GEOMETRY_SOURCE_CHARACTERS,
  outputs: 256, triangles: 20_000, nativeCharacters: MAX_NATIVE_GEOMETRY_CHARACTERS,
  transportBytes: 16 * 1024 * 1024, startupMs: 10_000, deadlineMs: 30_000, joinMs: 1_000})
export const BREP_DIAGNOSTIC_IDENTITY = Object.freeze({protocol: 'brep-semantic-diagnostic-v1' as const,
  diagnosticOnly: true as const, qualification: 'not-qualified' as const, automaticFallback: false as const,
  // Hash the exact generated base64 text consumed by geometry/kernel.ts, not a frozen manifest.
  kernelPackedBase64Sha256: sha256Hex(packedKernel)})
export interface BrepDiagnosticRequest {
  readonly type: 'evaluate'
  readonly workerEpoch: number
  readonly jobId: number
  readonly source: string
  readonly sourceSha256: string
  readonly policy: BrepSemanticDisplayPolicy
  readonly identity: typeof BREP_DIAGNOSTIC_IDENTITY
}
export interface BrepDiagnosticEnvelope {
  readonly workerEpoch: number
  readonly jobId: number
  readonly sourceSha256: string
  readonly displayPolicyHash: string
  readonly identity: typeof BREP_DIAGNOSTIC_IDENTITY
}
export interface BrepDiagnosticErrorRecord {readonly name: string; readonly code: string | null; readonly message: string}
export type BrepDiagnosticMessage =
  | (BrepDiagnosticEnvelope & {readonly status: 'started'})
  | (BrepDiagnosticEnvelope & {readonly status: 'failed'; readonly error: BrepDiagnosticErrorRecord})
  | (BrepDiagnosticEnvelope & {readonly status: 'succeeded'; readonly scene: BrepSemanticSceneBuild; readonly programJson: string})
/** Correlated runtime data from the owned worker, with cross-object integrity checks.
 * This is not independent verification of source lowering, geometric correctness or qualification.
 */
export type BrepDiagnosticReceipt = Extract<BrepDiagnosticMessage, {status: 'succeeded'}>

function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null)
}
function keys(value: unknown, expected: readonly string[]): value is Record<string, unknown> {
  if (!record(value)) return false
  const own = Reflect.ownKeys(value)
  return own.length === expected.length && expected.every(key => {
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    return descriptor !== undefined && 'value' in descriptor && descriptor.enumerable
  })
}
const hash = (v: unknown): v is string => typeof v === 'string' && /^[a-f0-9]{64}$/.test(v)
const positiveInteger = (v: unknown): v is number => Number.isSafeInteger(v) && (v as number) > 0
function identity(value: unknown): boolean {
  return keys(value, Object.keys(BREP_DIAGNOSTIC_IDENTITY))
    && Object.entries(BREP_DIAGNOSTIC_IDENTITY).every(([key, expected]) => value[key] === expected)
}
export function diagnosticDisplayPolicyHash(policy: BrepSemanticDisplayPolicy): string {
  return sha256Hex('brep-display-policy-v1\n' + JSON.stringify({version: 1, sampling: 'uniform-patch', quality: policy.quality, segments: policy.segments}))
}
export function isBrepDiagnosticRequest(value: unknown): value is BrepDiagnosticRequest {
  try {
    if (!keys(value, ['type','workerEpoch','jobId','source','sourceSha256','policy','identity'])
      || value.type !== 'evaluate' || !positiveInteger(value.workerEpoch) || !positiveInteger(value.jobId)
      || typeof value.source !== 'string' || value.source.length > BREP_DIAGNOSTIC_LIMITS.sourceCharacters
      || !hash(value.sourceSha256) || value.sourceSha256 !== sha256Hex(value.source)
      || !identity(value.identity) || !keys(value.policy, ['quality','segments'])
      || (value.policy.quality !== 'preview' && value.policy.quality !== 'full')
      || !positiveInteger(value.policy.segments) || value.policy.segments > 32) return false
    const route = parseGeometrySourceRoutingHeader(value.source)
    return route.languageContract === 'openscad-viewer/brep-1' && route.requiredCapabilities.length === 0
  } catch { return false }
}
export function diagnosticEnvelope(request: BrepDiagnosticRequest): BrepDiagnosticEnvelope {
  return {workerEpoch: request.workerEpoch, jobId: request.jobId, sourceSha256: request.sourceSha256,
    displayPolicyHash: diagnosticDisplayPolicyHash(request.policy), identity: BREP_DIAGNOSTIC_IDENTITY}
}

/** Conservative retained transport accounting, before expensive semantic/mesh validation.
 * Reject getters, shared memory, cycles, excessive depth and object fanout.
 */
function boundedPayload(value: unknown): boolean {
  const pending = [{value, depth: 0}], seen = new Set<object>(), buffers = new Set<ArrayBuffer>()
  let bytes = 0, work = 0
  while (pending.length) {
    const item = pending.pop()!, v = item.value
    if (++work > 200_000 || item.depth > 32) return false
    if (typeof v === 'string') {
      if (v.length > BREP_DIAGNOSTIC_LIMITS.transportBytes - bytes) return false
      bytes += new TextEncoder().encode(v).byteLength + 8
    }
    else if (typeof v === 'number') { if (!Number.isFinite(v)) return false; bytes += 8 }
    else if (typeof v === 'boolean' || v === null || v === undefined) bytes += 8
    else if (typeof v !== 'object') return false
    else {
      if (seen.has(v)) return false
      seen.add(v)
      if (ArrayBuffer.isView(v)) {
        if (!(v.buffer instanceof ArrayBuffer)) return false
        // A short view still retains/transfers its entire backing store.
        if (!buffers.has(v.buffer)) {buffers.add(v.buffer); bytes += v.buffer.byteLength}
      } else {
        if (!Array.isArray(v) && !record(v)) return false
        const own = Reflect.ownKeys(v)
        if (own.length > 200_000 - work - pending.length) return false
        for (const key of own) {
          if (typeof key !== 'string') return false
          const descriptor = Object.getOwnPropertyDescriptor(v, key)!
          if (!('value' in descriptor)) return false
          if (Array.isArray(v) && key === 'length') continue
          bytes += key.length * 3 + 16
          pending.push({value: descriptor.value, depth: item.depth + 1})
        }
      }
    }
    if (bytes > BREP_DIAGNOSTIC_LIMITS.transportBytes) return false
  }
  return true
}
function sceneMatches(value: unknown, programJson: unknown, request: BrepDiagnosticRequest): value is BrepSemanticSceneBuild {
  if (typeof programJson !== 'string' || programJson.length > BREP_DIAGNOSTIC_LIMITS.nativeCharacters) return false
  const program = normalizeSemanticProgram(JSON.parse(programJson), request.source)
  const attestation = attestSemanticProgram(program)
  const expectedOutputs = semanticResultItems(program.core.result)
  const provenance = new Map(program.provenance.map(p => [p.operation, p]))
  if (!keys(value, ['result','attestation','displayPolicyHash','outputs','metrics','deviationStatus'])
    || value.metrics !== 'display_mesh_estimates' || value.deviationStatus !== 'not_certified'
    || value.displayPolicyHash !== diagnosticDisplayPolicyHash(request.policy)
    || !keys(value.attestation, ['semanticProgramVersion','binaryFormat','sourceHash','programHash','tessellationPolicyHash','coreBytesSha256','envelopeBytesSha256','canonicalBytes'])
    || value.attestation.semanticProgramVersion !== 'semantic-program-contract-v1'
    || value.attestation.binaryFormat !== 'semantic-program-binary-v1'
    || value.attestation.sourceHash !== request.sourceSha256
    || !['programHash','tessellationPolicyHash','coreBytesSha256','envelopeBytesSha256'].every(k => hash((value.attestation as Record<string, unknown>)[k]))
    || !positiveInteger(value.attestation.canonicalBytes) || value.attestation.canonicalBytes > 64 * 1024 * 1024
    || !Array.isArray(value.outputs) || value.outputs.length > BREP_DIAGNOSTIC_LIMITS.outputs
    || value.outputs.length !== expectedOutputs.length
    || Object.entries(attestation).some(([key, expected]) => (value.attestation as Record<string, unknown>)[key] !== expected)
    || !isGeometryEvaluationResultPayload(value.result) || value.result.quality !== request.policy.quality
    || value.result.meshes.length > BREP_DIAGNOSTIC_LIMITS.outputs || value.result.reduced !== false) return false
  let triangles = 0, nativeCharacters = 0
  const meshes = new Map<string, typeof value.result.meshes[number]>()
  for (const mesh of value.result.meshes) {
    if (typeof mesh.entityId !== 'string') return false
    meshes.set(mesh.entityId, mesh)
  }
  if (meshes.size !== value.result.meshes.length) return false
  const entities = new Set<string>()
  for (let outputIndex = 0; outputIndex < value.outputs.length; outputIndex++) {
    const output = value.outputs[outputIndex]
    const expectedOutput = expectedOutputs[outputIndex]
    const occurrence = program.core.occurrences[expectedOutput.identityOccurrence]
    if (!keys(output, ['occurrenceId','operationId','entityId','nodeIndex','empty','snapshotRevision','geometryAssetId','topologyFaceIds'])
      || typeof output.entityId !== 'string' || !/^entity:v2:[a-f0-9]{64}$/.test(output.entityId)
      || typeof output.occurrenceId !== 'string' || !/^occv1:[a-f0-9]{64}$/.test(output.occurrenceId)
      || typeof output.operationId !== 'string' || !/^opv1:[a-f0-9]{64}$/.test(output.operationId)
      || !Number.isSafeInteger(output.nodeIndex) || (output.nodeIndex as number) < 0
      || !Array.isArray(output.topologyFaceIds) || output.topologyFaceIds.length > 4096
      || output.topologyFaceIds.some(id => typeof id !== 'string' || id.length > 128)
      || entities.has(output.entityId) || output.entityId !== occurrence.sceneEntityId
      || output.occurrenceId !== occurrence.occurrenceId || output.operationId !== program.core.operations[occurrence.operation].operationId
      || output.nodeIndex !== expectedOutput.node) return false
    entities.add(output.entityId)
    const mesh = meshes.get(output.entityId)
    if (output.empty === true) {
      if (mesh || output.snapshotRevision !== null || output.geometryAssetId !== null || output.topologyFaceIds.length) return false
    } else {
      if (output.empty !== false || !mesh || mesh.nativeGeometry?.kind !== 'brep'
        || mesh.nativeGeometry.nodeId !== output.entityId || mesh.nativeGeometry.revision !== output.snapshotRevision
        || mesh.geometryAssetId !== output.geometryAssetId || mesh.geometryAssetId !== geometryAssetId(mesh.vertices, mesh.indices)
        || !mesh.faceIdsAuthoritative || mesh.transform.some((entry, index) => entry !== (index % 5 === 0 ? 1 : 0))
        || mesh.topology.boundary !== 0 || mesh.topology.nonManifold !== 0 || mesh.topology.degenerate !== 0
        || mesh.nativeGeometry.documentJson !== programJson
        || mesh.color.some((v, index) => v !== expectedOutput.color[index])) return false
      const producer = program.core.occurrences[expectedOutput.producerOccurrence]
      const source = provenance.get(producer.operation)
      const run = mesh.provenance[0]
      if (mesh.provenance.length !== 1 || !keys(run, ['triangleStart','triangleEnd','backside','source'])
        || run.triangleStart !== 0 || run.triangleEnd !== mesh.indices.length / 3 || run.backside !== false) return false
      if (source) {
        if (!keys(run.source, ['id','originalId','instanceId','start','end','label'])
          || run.source.id !== producer.operation || run.source.originalId !== producer.operation
          || run.source.instanceId !== output.entityId || run.source.start !== source.span.start
          || run.source.end !== source.span.end || run.source.label !== source.label) return false
      } else if (run.source !== null) return false
      // Snapshot hashes are checked by the existing mesh validator; additionally
      // bind the advertised face list and triangle indices to that exact snapshot.
      const snapshot = JSON.parse(mesh.nativeGeometry.geometryJson) as {geometry?: {faces?: unknown[]; topologyIds?: {faces?: unknown[]}}}
      const faceList = snapshot.geometry?.topologyIds?.faces
      const reportedFaces = output.topologyFaceIds
      if (!Array.isArray(snapshot.geometry?.faces) || !Array.isArray(faceList)
        || faceList.length !== snapshot.geometry.faces.length || faceList.length !== output.topologyFaceIds.length
        || faceList.some((id, index) => typeof id !== 'string' || id !== reportedFaces[index])
        || !mesh.faceIds || mesh.faceIds.some(face => face >= faceList.length)) return false
      triangles += mesh.indices.length / 3
      nativeCharacters += mesh.nativeGeometry.geometryJson.length + mesh.nativeGeometry.documentJson.length
      meshes.delete(output.entityId)
    }
  }
  return meshes.size === 0 && (triangles > 0 || value.result.volume === 0 && value.result.surfaceArea === 0) && triangles <= BREP_DIAGNOSTIC_LIMITS.triangles && nativeCharacters <= BREP_DIAGNOSTIC_LIMITS.nativeCharacters
}
export function isBrepDiagnosticMessage(value: unknown, request: BrepDiagnosticRequest): value is BrepDiagnosticMessage {
  try {
    if (!boundedPayload(value) || !record(value)) return false
    const extra = value.status === 'started' ? [] : value.status === 'failed' ? ['error'] : value.status === 'succeeded' ? ['scene','programJson'] : null
    if (extra === null || !keys(value, ['workerEpoch','jobId','sourceSha256','displayPolicyHash','identity','status',...extra])
      || value.workerEpoch !== request.workerEpoch || value.jobId !== request.jobId || value.sourceSha256 !== request.sourceSha256
      || value.displayPolicyHash !== diagnosticDisplayPolicyHash(request.policy) || !identity(value.identity)) return false
    if (value.status === 'started') return true
    if (value.status === 'failed') return keys(value.error, ['name','code','message'])
      && typeof value.error.name === 'string' && value.error.name.length > 0 && value.error.name.length <= 128
      && typeof value.error.message === 'string' && value.error.message.length <= 4096
      && (value.error.code === null || typeof value.error.code === 'string' && value.error.code.length <= 128)
    return sceneMatches(value.scene, value.programJson, request)
  } catch { return false }
}
export function diagnosticError(error: unknown): BrepDiagnosticErrorRecord {
  const value = error instanceof Error ? error : new Error('Diagnostic execution failed')
  const code = 'code' in value && typeof value.code === 'string' ? value.code.slice(0,128) : null
  return {name: value.name.slice(0,128) || 'Error', message: value.message.slice(0,4096), code}
}
