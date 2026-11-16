import {
  MAX_STORED_ARTIFACT_BYTES,
  MAX_STORED_BUILD_WARNINGS,
  MAX_STORED_WARNING_LENGTH,
  isWellFormedUnicode,
  validateModelId,
  validateModelSource,
  type RecordBuildInput,
  type StoreArtifactInput,
} from '../modelStore'
import { validateBuildDiagnostic, validateDiagnosticExecutionContext } from './boundedLimit'
import { expectedArtifactMimeType, serializeExecutionDescriptor, sha256 } from './validateExecutionDescriptor'

export function validateRevision(revision: number): void {
  if (!Number.isSafeInteger(revision) || revision < 0) {
    throw new RangeError('revision must be a non-negative safe integer')
  }
}

export function validateRecordBuildInput(
  id: string,
  input: RecordBuildInput,
): { executionJson: string; errorJson: string | null } {
  validateModelId(id)
  validateModelSource(input.source)
  const hasModelId = input.modelId !== undefined
  const hasModelRevision = input.modelRevision !== undefined
  if (hasModelId !== hasModelRevision) {
    throw new TypeError('modelId and modelRevision must be provided together')
  }
  if (input.modelId !== undefined) validateModelId(input.modelId)
  if (input.modelRevision !== undefined) validateRevision(input.modelRevision)
  if (!/^[a-f0-9]{64}$/i.test(input.sourceSha256)) throw new TypeError('sourceSha256 must be a SHA-256 hex digest')
  if (sha256(input.source) !== input.sourceSha256.toLowerCase()) {
    throw new TypeError('sourceSha256 must match the exact build source')
  }
  if (input.quality !== 'preview' && input.quality !== 'full') throw new TypeError('Unsupported build quality')
  if (!['succeeded', 'failed', 'cancelled'].includes(input.status)) throw new TypeError('Unsupported build status')
  if (!Number.isFinite(input.durationMs) || input.durationMs < 0)
    throw new RangeError('durationMs must be non-negative')
  if (
    input.warnings.length > MAX_STORED_BUILD_WARNINGS ||
    input.warnings.some(warning => warning.length > MAX_STORED_WARNING_LENGTH || !isWellFormedUnicode(warning))
  ) {
    throw new RangeError(
      `Build warnings are limited to ${MAX_STORED_BUILD_WARNINGS} entries of ${MAX_STORED_WARNING_LENGTH} characters`,
    )
  }
  if (input.status === 'succeeded' && !input.metrics) throw new TypeError('Successful builds require metrics')
  if (input.status !== 'succeeded' && input.metrics) throw new TypeError('Only successful builds may contain metrics')
  if (input.status === 'succeeded' && input.error) throw new TypeError('Successful builds cannot contain an error')
  if (input.status !== 'succeeded' && !input.error) {
    throw new TypeError('Failed and cancelled builds require an attested diagnostic')
  }
  if (input.metrics) {
    for (const [name, value] of [
      ['meshCount', input.metrics.meshCount],
      ['triangleCount', input.metrics.triangleCount],
    ] as const) {
      if (!Number.isSafeInteger(value) || value < 0) throw new RangeError(`${name} must be a non-negative safe integer`)
    }
    if (
      !Number.isFinite(input.metrics.volume) ||
      input.metrics.volume < 0 ||
      !Number.isFinite(input.metrics.surfaceArea) ||
      input.metrics.surfaceArea < 0
    ) {
      throw new RangeError('Build volume and surface area must be finite and non-negative')
    }
  }
  const executionJson = serializeExecutionDescriptor(input.execution, input.quality, input.status)
  let errorJson: string | null = null
  if (input.error) {
    validateBuildDiagnostic(input.error, 'Build diagnostic')
    validateDiagnosticExecutionContext(input.error, input.execution, 'Build diagnostic')
    const snapshot = JSON.parse(JSON.stringify(input.error)) as unknown
    validateBuildDiagnostic(snapshot, 'Build diagnostic snapshot')
    validateDiagnosticExecutionContext(snapshot, input.execution, 'Build diagnostic snapshot')
    errorJson = JSON.stringify(snapshot)
  }
  return { executionJson, errorJson }
}

export function validateStoreArtifactInput(id: string, input: StoreArtifactInput): void {
  validateModelId(id)
  validateModelId(input.buildId)
  if (input.modelId !== undefined) validateModelId(input.modelId)
  if (
    !input.fileName ||
    input.fileName.length > 255 ||
    /[/\\\0]/.test(input.fileName) ||
    !isWellFormedUnicode(input.fileName)
  ) {
    throw new TypeError('fileName must be a plain file name of at most 255 characters')
  }
  if (input.format !== 'stl' && input.format !== 'obj' && input.format !== 'step') {
    throw new TypeError('Unsupported artifact format')
  }
  if (input.mimeType !== expectedArtifactMimeType(input.format))
    throw new TypeError('mimeType must match artifact format')
  if (!/^[a-f0-9]{64}$/i.test(input.sha256)) throw new TypeError('sha256 must be a SHA-256 hex digest')
  if (!(input.data instanceof Uint8Array)) throw new TypeError('Artifact data must be a Uint8Array')
  if (input.data.byteLength > MAX_STORED_ARTIFACT_BYTES) {
    throw new RangeError(`Artifact exceeds the ${MAX_STORED_ARTIFACT_BYTES.toLocaleString('en-US')}-byte storage limit`)
  }
  if (sha256(input.data).toLowerCase() !== input.sha256.toLowerCase()) {
    throw new TypeError('sha256 must match artifact data')
  }
}
