import type { GeometryQuality } from '../core/build'
import {
  ACTIVE_GEOMETRY_MANIFEST_VERSIONS,
  canonicalJson,
  GEOMETRY_ENGINE_ROUTES,
  GEOMETRY_MANIFEST_ARCHIVE,
  MAX_GEOMETRY_SOURCE_CHARACTERS,
  planGeometrySourceExecution,
  type GeometryBuildPurpose,
  type GeometryEngineRegistrySnapshot,
  type GeometryExecutionDescriptor,
} from '../core/geometryExecution'
import type { LanguageDiagnosticCode } from '../core/languageContract'
import { sha256Hex } from '../core/sha256'
import {
  attachGeometryExecutionToError,
  GeometryCapabilityUnavailableError,
  GeometryEngineUnavailableError,
  geometryExecutionForError,
  GeometryLanguageContractError,
  GeometryProviderContractError,
  type GeometryBuildResult,
  type GeometryEngineAvailabilityCause,
} from '../services/geometryBuildEngine'
import {
  GEOMETRY_WORKER_PAYLOAD_LIMITS,
  isGeometryEvaluationResultPayload,
} from '../services/geometryWorkerProtocol'
import { AbortedError, OpenSCADParseError } from '../services/openscadErrors'
import { isWellFormedUnicode as wellFormed } from '../core/unicode'
import { truncateWellFormed } from '../core/unicode'
import { record, ownData, exactDataKeys, positiveSafeInteger } from '../core/plainData'
export { isDirectGeometryBuildRequest, isDirectGeometryCapabilitiesRequest, isDirectGeometryRequest, isDirectGeometryCancel, isDirectGeometryStarted, isDirectGeometryCapabilities, isDirectGeometryTerminal, isDirectGeometryNodeTerminal } from './directGeometryProtocol/isDirectGeometryBuildRequest'
export { serializeDirectGeometryError, directGeometryExecutionForError, reconstructDirectGeometryRemoteError } from './directGeometryProtocol/serializeDirectGeometryError'

export const DIRECT_GEOMETRY_PROTOCOL_VERSION = 1 as const

const currentMeshManifest = GEOMETRY_MANIFEST_ARCHIVE[
  ACTIVE_GEOMETRY_MANIFEST_VERSIONS.mesh
]

/**
 * The identity is the current immutable package snapshot of the legacy direct
 * evaluator. Host isolation is an independent boundary fact and does not
 * mutate that manifest; archived snapshots remain catalog-readable only.
 */
export const DIRECT_GEOMETRY_IDENTITY = Object.freeze({
  boundaryVersion: 'mcp-direct-geometry-v1' as const,
  executionPath: 'legacy-direct-production' as const,
  engineClass: currentMeshManifest.engineClass,
  engineKey: currentMeshManifest.engineKey,
  kernelFingerprint: currentMeshManifest.kernelFingerprint,
  semanticProgramVersion: currentMeshManifest.semanticProgramVersion,
  capabilityManifestVersion: currentMeshManifest.capabilityManifestVersion,
  manifestDigest: currentMeshManifest.manifestDigest,
  inputContract: currentMeshManifest.inputContract,
  manifestIsolation: currentMeshManifest.isolation,
  hostIsolation: 'disposable-node-worker-per-job' as const,
  automaticFallback: false as const,
})

export type DirectGeometryIdentity = typeof DIRECT_GEOMETRY_IDENTITY
export type DirectGeometryOperation = 'build' | 'capabilities'
export type DirectGeometryCancelReason = 'cancelled' | 'deadline' | 'closed'

interface DirectGeometryBaseRequest {
  readonly protocolVersion: typeof DIRECT_GEOMETRY_PROTOCOL_VERSION
  readonly workerEpoch: number
  readonly jobId: number
  readonly identity: DirectGeometryIdentity
}

export interface DirectGeometryBuildRequest extends DirectGeometryBaseRequest {
  readonly type: 'build'
  readonly source: string
  readonly sourceSha256: string
  readonly quality: GeometryQuality
  readonly purpose: GeometryBuildPurpose
}

export interface DirectGeometryCapabilitiesRequest extends DirectGeometryBaseRequest {
  readonly type: 'capabilities'
  readonly sourceSha256: null
  readonly quality: null
  readonly purpose: null
}

export type DirectGeometryRequest = DirectGeometryBuildRequest | DirectGeometryCapabilitiesRequest

export interface DirectGeometryCancel extends DirectGeometryBaseRequest {
  readonly type: 'cancel'
  readonly sourceSha256: string | null
  readonly quality: GeometryQuality | null
  readonly purpose: GeometryBuildPurpose | null
  readonly reason: DirectGeometryCancelReason
}

interface DirectGeometryEventEnvelope extends DirectGeometryBaseRequest {
  readonly sourceSha256: string | null
  readonly quality: GeometryQuality | null
  readonly purpose: GeometryBuildPurpose | null
  readonly kind: DirectGeometryOperation
}

export interface DirectGeometryStarted extends DirectGeometryEventEnvelope {
  readonly status: 'started'
}

export type DirectGeometryErrorCategory =
  | 'openscad-parse'
  | 'aborted'
  | 'language-contract'
  | 'engine-unavailable'
  | 'capability-unavailable'
  | 'provider-contract'
  | 'internal'

/** Fixed-shape error. Its message is always one line and never a source excerpt. */
export interface DirectGeometryError {
  readonly category: DirectGeometryErrorCategory
  readonly name: string
  readonly message: string
  readonly code: string | null
  readonly line: number | null
  readonly column: number | null
  readonly start: number | null
  readonly end: number | null
  readonly reportedContract: string | null
  readonly availabilityCause: GeometryEngineAvailabilityCause | null
  readonly missingCapabilities: readonly string[]
}

export interface DirectGeometryBuildSuccess extends DirectGeometryEventEnvelope {
  readonly status: 'succeeded'
  readonly kind: 'build'
  readonly built: GeometryBuildResult
}

export interface DirectGeometryCapabilitiesSuccess extends DirectGeometryEventEnvelope {
  readonly status: 'succeeded'
  readonly kind: 'capabilities'
  readonly capabilities: GeometryEngineRegistrySnapshot
}

export interface DirectGeometryFailure extends DirectGeometryEventEnvelope {
  readonly status: 'failed'
  readonly execution: GeometryExecutionDescriptor | null
  readonly error: DirectGeometryError
}

export type DirectGeometryTerminal =
  | DirectGeometryBuildSuccess
  | DirectGeometryCapabilitiesSuccess
  | DirectGeometryFailure

export const BUILD_REQUEST_KEYS = Object.freeze([
  'protocolVersion', 'type', 'workerEpoch', 'jobId', 'source', 'sourceSha256',
  'quality', 'purpose', 'identity',
])
export const CAPABILITIES_REQUEST_KEYS = Object.freeze([
  'protocolVersion', 'type', 'workerEpoch', 'jobId', 'sourceSha256', 'quality',
  'purpose', 'identity',
])
export const CANCEL_KEYS = Object.freeze([
  'protocolVersion', 'type', 'workerEpoch', 'jobId', 'sourceSha256', 'quality',
  'purpose', 'reason', 'identity',
])
export const EVENT_KEYS = Object.freeze([
  'protocolVersion', 'workerEpoch', 'jobId', 'sourceSha256', 'quality', 'purpose',
  'identity', 'status', 'kind',
])
export const ERROR_KEYS = Object.freeze([
  'category', 'name', 'message', 'code', 'line', 'column', 'start', 'end',
  'reportedContract', 'availabilityCause', 'missingCapabilities',
])
export const EXECUTION_KEYS = Object.freeze([
  'languageContract', 'requiredCapabilities', 'engineClass', 'engineKey',
  'kernelFingerprint', 'semanticProgramVersion', 'capabilityManifestVersion',
  'manifestDigest', 'purpose', 'quality', 'representation', 'evidence',
  'effectiveLimits', 'automaticFallback',
])
export const IDENTITY_KEYS = Object.freeze(Object.keys(DIRECT_GEOMETRY_IDENTITY))
export const PURPOSES = new Set<GeometryBuildPurpose>(['preview', 'full', 'analysis', 'export'])
export const AVAILABILITY_CAUSES = new Set<GeometryEngineAvailabilityCause>([
  'not-deployed', 'provider-missing', 'readiness-timeout', 'readiness-failed',
  'revoked', 'quarantined',
])
export const LANGUAGE_DIAGNOSTIC_CODES = new Set<LanguageDiagnosticCode>([
  'E_FEATURE_INCLUDE', 'E_FEATURE_USE', 'E_FEATURE_USER_FUNCTION',
  'E_FEATURE_VIEWPORT_MODIFIER',
])
export const MAX_ERROR_MESSAGE_CHARACTERS = 4_096
export const MAX_CAPABILITIES_TEXT_CHARACTERS = 64 * 1024
export const MAX_CAPABILITIES_DATA_NODES = 2_048

export class DirectGeometryRemoteError extends Error {
  constructor(readonly remote: DirectGeometryError) {
    super(remote.message)
    this.name = remote.name
  }
}
