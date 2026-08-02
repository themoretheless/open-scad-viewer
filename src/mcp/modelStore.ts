import {
  MAX_GEOMETRY_SOURCE_CHARACTERS,
  type GeometryExecutionDescriptor,
} from '../core/geometryExecution'
import type { PublicErrorCode } from './errorContract'

export const MAX_MODEL_SOURCE_LENGTH = MAX_GEOMETRY_SOURCE_CHARACTERS
export const MAX_MODEL_NAME_LENGTH = 255
export const MAX_MODEL_ID_LENGTH = 128
// Base64 plus the JSON-RPC envelope must remain below the MCP stdio transport's
// common 10 MiB frame limit when an artifact is read as a resource.
export const MAX_STORED_ARTIFACT_BYTES = 6 * 1024 * 1024
export const MAX_STORED_BUILD_WARNINGS = 32
export const MAX_STORED_WARNING_LENGTH = 512
export const MAX_STORED_DIAGNOSTIC_NAME_LENGTH = 128
export const MAX_STORED_DIAGNOSTIC_MESSAGE_LENGTH = 2_048
export const MAX_STORED_DIAGNOSTIC_DETAIL_COUNT = 16
export const MAX_STORED_DIAGNOSTIC_DETAIL_LENGTH = 4_096
export const MAX_STORED_DIAGNOSTIC_JSON_BYTES = 96 * 1024
export const MAX_STORED_EXECUTION_JSON_BYTES = 16 * 1024

export interface StoredModel {
  id: string
  name: string
  source: string
  revision: number
  createdAt: string
  updatedAt: string
}

export interface StoredModelSummary extends Omit<StoredModel, 'source'> {
  sourceLength: number
}

export interface StoredModelRevision {
  id: string
  name: string
  source: string
  revision: number
  createdAt: string
}

export interface StoredModelRevisionSummary extends Omit<StoredModelRevision, 'source'> {
  sourceLength: number
}

export interface SaveModelInput {
  id?: string
  name: string
  source: string
  /** Optimistic concurrency guard. A new model has revision 0. */
  expectedRevision?: number
}

export interface SaveModelSourceInput {
  id: string
  source: string
  /** Required optimistic guard for a source-only update that preserves the latest model name. */
  expectedRevision: number
}

export interface BuildDiagnostic {
  contractVersion: 1
  name: string
  message: string
  code: PublicErrorCode
  retryable: boolean
  details?: Record<string, string | number | boolean | null>
  line?: number
  column?: number
}

export interface LegacyBuildDiagnostic {
  contractVersion: 0
  evidence: 'legacy-unattested'
  name: string
  message: string
  code?: string
  retryable?: boolean
  details?: Record<string, string | number | boolean | null>
  line?: number
  column?: number
}

export type StoredBuildDiagnostic = BuildDiagnostic | LegacyBuildDiagnostic

export interface BuildMetrics {
  meshCount: number
  triangleCount: number
  volume: number
  surfaceArea: number
  reduced: boolean
}

export type BuildSourceAttestation =
  | 'model-revision'
  | 'inline-snapshot'
  | 'historical-unattested'

export interface RecordBuildInput {
  id?: string
  modelId?: string
  modelRevision?: number
  /** Exact source used; inline history persists it, model-backed history binds its revision. */
  source: string
  sourceSha256: string
  quality: 'preview' | 'full'
  durationMs: number
  warnings: string[]
  status: 'succeeded' | 'failed' | 'cancelled'
  metrics?: BuildMetrics
  error?: BuildDiagnostic
  /** Immutable engine-routing and capability provenance for this execution. */
  execution: GeometryExecutionDescriptor
}

export interface StoredBuild extends Omit<RecordBuildInput, 'id' | 'modelId' | 'modelRevision' | 'source' | 'metrics' | 'error' | 'execution'> {
  id: string
  modelId: string | null
  modelRevision: number | null
  /** Explicit evidence class; historical-unattested is never produced by a new write. */
  sourceAttestation: BuildSourceAttestation
  metrics: BuildMetrics | null
  error: StoredBuildDiagnostic | null
  execution: GeometryExecutionDescriptor
  createdAt: string
}

export type ArtifactFormat = 'stl' | 'obj'

export interface StoreArtifactInput {
  id?: string
  buildId: string
  modelId?: string
  format: ArtifactFormat
  fileName: string
  mimeType: string
  sha256: string
  data: Uint8Array
}

export interface StoredArtifactSummary extends Omit<StoreArtifactInput, 'id' | 'data' | 'modelId'> {
  id: string
  modelId: string | null
  byteLength: number
  createdAt: string
}

export interface StoredArtifact extends StoredArtifactSummary {
  data: Uint8Array
}

export interface CatalogStats {
  modelCount: number
  revisionCount: number
  buildCount: number
  artifactCount: number
  storedSourceBytes: number
  storedArtifactBytes: number
  buildsByStatus: {
    succeeded: number
    failed: number
    cancelled: number
  }
  limits: {
    models: number
    revisions: number
    revisionsPerModel: number
    sourceBytes: number
    builds: number
    artifacts: number
    artifactBytes: number
  }
}

export interface ModelStore {
  saveModel(input: SaveModelInput): Promise<StoredModel>
  saveModelSource(input: SaveModelSourceInput): Promise<StoredModel>
  getModel(id: string): Promise<StoredModel | null>
  getModelRevision(id: string, revision: number): Promise<StoredModelRevision | null>
  listModels(limit?: number): Promise<StoredModelSummary[]>
  listModelRevisions(id: string, limit?: number): Promise<StoredModelRevisionSummary[]>
  recordBuild(input: RecordBuildInput): Promise<StoredBuild>
  getBuild(id: string): Promise<StoredBuild | null>
  listBuilds(options?: { modelId?: string; limit?: number }): Promise<StoredBuild[]>
  storeArtifact(input: StoreArtifactInput): Promise<StoredArtifactSummary>
  getArtifact(id: string): Promise<StoredArtifact | null>
  listArtifacts(limit?: number): Promise<StoredArtifactSummary[]>
  getCatalogStats(): Promise<CatalogStats>
  close(): Promise<void>
}

export class ModelRevisionConflictError extends Error {
  constructor(
    readonly modelId: string,
    readonly expectedRevision: number,
    readonly actualRevision: number | null,
  ) {
    super(actualRevision === null
      ? `Model ${modelId} does not exist at expected revision ${expectedRevision}`
      : `Model ${modelId} is at revision ${actualRevision}, expected ${expectedRevision}`)
    this.name = 'ModelRevisionConflictError'
  }
}

export class CatalogQuotaError extends RangeError {
  constructor(
    message: string,
    readonly quota: string,
    readonly limit: number,
  ) {
    super(message)
    this.name = 'CatalogQuotaError'
  }
}

export function validateModelId(id: string): void {
  if (!/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(id)) {
    throw new TypeError('Model id must start with an alphanumeric character and contain only letters, digits, ., _, :, or -')
  }
}

export function validateModelName(name: string): void {
  if (!name.trim() || name.length > MAX_MODEL_NAME_LENGTH || !isWellFormedUnicode(name)) {
    throw new TypeError(`Model name must contain 1-${MAX_MODEL_NAME_LENGTH} characters`)
  }
}

export function validateModelSource(source: string): void {
  if (!isWellFormedUnicode(source)) {
    throw new TypeError('OpenSCAD source must contain well-formed Unicode')
  }
  if (source.length > MAX_MODEL_SOURCE_LENGTH) {
    throw new RangeError(`OpenSCAD source exceeds ${MAX_MODEL_SOURCE_LENGTH.toLocaleString()} characters`)
  }
}

export function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xD800 && unit <= 0xDBFF) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xDC00 && next <= 0xDFFF)) return false
    } else if (unit >= 0xDC00 && unit <= 0xDFFF) return false
  }
  return true
}
