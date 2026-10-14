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
import { isWellFormedUnicode } from '../core/unicode'
export { isGeometryEvaluationResultPayload, isGeometryWorkerRequest, isGeometryWorkerEvent } from './geometryWorkerProtocol/isGeometryEvaluationResultPayload'

/**
 * Increment this when the worker wire format changes incompatibly. Keeping the
 * version in every message makes an old, cached worker fail visibly instead of
 * accidentally publishing data into a newer application state.
 */
export const GEOMETRY_WORKER_PROTOCOL_VERSION = 12 as const

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
  acknowledgeSweepPreviews?: boolean
  selectionSurfaces?: boolean
  type: 'build'
  source: string
}

export interface GeometryCancelRequest extends Omit<GeometryJobEnvelope, 'quality' | 'sourceSha256'> {
  type: 'cancel'
  reason: GeometryCancelReason
}

export interface GeometrySweepPreviewAcknowledgement extends GeometryJobEnvelope {
 type:'sweep-preview-ack'
 nodeId:string
 sections:number
}
export type GeometryWorkerRequest = GeometryBuildRequest | GeometryCancelRequest | GeometrySweepPreviewAcknowledgement

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

/** Display-only per-node sweep geometry, never an authoritative publication. */
export interface GeometrySweepPreview extends GeometryJobEnvelope {
 status:'sweep-preview'
 phase:'compiling'
 nodeId:string
 sections:number
 accepted:boolean
 continuousErrorUpper?:number
 /** Maximum for the certified subset of profiles, not the full boundary. */
 knownProfileErrorUpper?:number
 profileRegularityCertified?:boolean
 wallRegularityCertified?:boolean|null
 certifiedErrorUpper?:number|null
 endpointContourErrorUpper?:[number,number]|null
 frameTransportCertified?:boolean
 phaseResolved?:boolean
 sampledControlDeviation:number
 budget:number
 meshes:MeshData[]
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
  | GeometrySweepPreview
  | GeometryBuildTerminal

export const QUALITIES = new Set<GeometryQuality>(['preview', 'full'])
export const PHASES = new Set<GeometryBuildPhase>(['queued', 'initializing', 'compiling', 'serializing', 'complete'])
export const CANCEL_REASONS = new Set<GeometryCancelReason>(['user', 'superseded', 'disposed', 'worker-restart'])
export const MESH_VERTEX_STRIDE = 6
export const BVH_LEAF_BIT = 0x80000000
export const BVH_LEAF_COUNT_MASK = 0x7fffffff
export const MAX_BVH_LEAF_SIZE = 64
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
