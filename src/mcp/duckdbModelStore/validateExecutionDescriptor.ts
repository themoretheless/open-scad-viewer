import { MAX_STORED_ARTIFACT_BYTES, MAX_STORED_BUILD_WARNINGS, MAX_STORED_EXECUTION_JSON_BYTES, MAX_STORED_WARNING_LENGTH, isWellFormedUnicode, validateModelId, validateModelName, validateModelSource } from '../modelStore'
import type { BuildMetrics, StoredArtifactSummary, StoredBuild, StoredModel, StoredModelRevision } from '../modelStore'
import { LEGACY_MANIFOLD_EXECUTION, geometryEngineClassForLanguageContract, parseGeometrySourceRoutingHeader } from '../../core/geometryExecution'
import type { GeometryExecutionDescriptor } from '../../core/geometryExecution'
import type { Row } from '../duckdbModelStore'
import { createHash } from 'node:crypto'
import { finiteNumber, isoTimestamp, legacyExecutionDescriptor, optionalString, parseJson, requiredString, safeInteger, storedBuildDiagnostic, validateBoundedJson, validateDiagnosticExecutionContext, validateManifestAttestation } from './boundedLimit'

export function validateExecutionDescriptor(
  value: unknown,
  expectedQuality?: 'preview' | 'full',
  expectedStatus?: 'succeeded' | 'failed' | 'cancelled',
): GeometryExecutionDescriptor {
  if (value === undefined) throw new TypeError('Build execution provenance is required')
  validateBoundedJson(value, 'Build execution provenance')
  const encoded = JSON.stringify(value)
  if (Buffer.byteLength(encoded, 'utf8') > MAX_STORED_EXECUTION_JSON_BYTES) {
    throw new RangeError(`Build execution provenance exceeds ${MAX_STORED_EXECUTION_JSON_BYTES.toLocaleString()} bytes`)
  }
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    throw new TypeError('Build execution provenance must be an object')
  }
  const descriptor = value as Record<string, unknown>
  const requiredStringFields = [
    'languageContract',
    'engineClass',
    'engineKey',
    'kernelFingerprint',
    'semanticProgramVersion',
    'capabilityManifestVersion',
    'manifestDigest',
    'purpose',
    'quality',
    'representation',
    'evidence',
  ] as const
  if (requiredStringFields.some(field => typeof descriptor[field] !== 'string'
    || !(descriptor[field] as string).length || (descriptor[field] as string).length > 256)) {
    throw new TypeError('Build execution provenance has an invalid required string field')
  }
  if (descriptor.engineClass !== 'mesh' && descriptor.engineClass !== 'brep') {
    throw new TypeError('Build execution provenance has an unsupported engine class')
  }
  if (descriptor.languageContract !== 'legacy/current'
    && descriptor.languageContract !== 'openscad-viewer/brep-1') {
    throw new TypeError('Build execution provenance has an unsupported language contract')
  }
  const routedEngine = geometryEngineClassForLanguageContract(descriptor.languageContract)
  if (descriptor.engineClass !== routedEngine) {
    throw new TypeError('Build execution provenance contradicts the language-contract engine route')
  }
  if (!['preview', 'full', 'analysis', 'export'].includes(descriptor.purpose as string)) {
    throw new TypeError('Build execution provenance has an unsupported purpose')
  }
  if (descriptor.quality !== 'preview' && descriptor.quality !== 'full') {
    throw new TypeError('Build execution provenance has an unsupported quality')
  }
  if (expectedQuality !== undefined && descriptor.quality !== expectedQuality) {
    throw new TypeError('Build execution provenance quality must match build quality')
  }
  if (!Array.isArray(descriptor.requiredCapabilities)
    || descriptor.requiredCapabilities.length > 32
    || descriptor.requiredCapabilities.some(capability => typeof capability !== 'string'
      || !/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(capability))
    || new Set(descriptor.requiredCapabilities).size !== descriptor.requiredCapabilities.length) {
    throw new TypeError('Build execution provenance has invalid required capabilities')
  }
  if (descriptor.representation !== 'mesh' && descriptor.representation !== 'brep') {
    throw new TypeError('Build execution provenance has an unsupported representation')
  }
  if (descriptor.engineClass === 'mesh' && descriptor.representation !== 'mesh') {
    throw new TypeError('Mesh execution provenance must use the mesh representation')
  }
  if (descriptor.evidence !== 'planned'
    && descriptor.evidence !== 'runtime'
    && descriptor.evidence !== 'legacy-backfill') {
    throw new TypeError('Build execution provenance has unsupported evidence')
  }
  if (descriptor.evidence === 'legacy-backfill' && (
    descriptor.languageContract !== LEGACY_MANIFOLD_EXECUTION.languageContract
    || descriptor.engineClass !== LEGACY_MANIFOLD_EXECUTION.engineClass
    || descriptor.engineKey !== LEGACY_MANIFOLD_EXECUTION.engineKey
    || descriptor.kernelFingerprint !== LEGACY_MANIFOLD_EXECUTION.kernelFingerprint
    || descriptor.semanticProgramVersion !== LEGACY_MANIFOLD_EXECUTION.semanticProgramVersion
    || descriptor.capabilityManifestVersion !== LEGACY_MANIFOLD_EXECUTION.capabilityManifestVersion
    || descriptor.manifestDigest !== LEGACY_MANIFOLD_EXECUTION.manifestDigest
    || descriptor.representation !== LEGACY_MANIFOLD_EXECUTION.representation
    || descriptor.purpose !== descriptor.quality
    || (descriptor.requiredCapabilities as unknown[]).length !== 0
    || Object.keys(descriptor.effectiveLimits as Record<string, number>).length !== 0
  )) {
    throw new TypeError('Legacy-backfill provenance is reserved for the exact historical Manifold descriptor')
  }
  if (expectedStatus === 'succeeded' && descriptor.evidence === 'planned') {
    throw new TypeError('A succeeded build cannot have planned-only execution evidence')
  }
  if (!descriptor.effectiveLimits || Array.isArray(descriptor.effectiveLimits)
    || typeof descriptor.effectiveLimits !== 'object'
    || Object.values(descriptor.effectiveLimits).some(limit => typeof limit !== 'number'
      || !Number.isFinite(limit) || limit < 0)) {
    throw new TypeError('Build execution provenance has invalid effective limits')
  }
  if (descriptor.automaticFallback !== false) {
    throw new TypeError('Build execution provenance must disable automatic fallback')
  }
  if (descriptor.evidence !== 'legacy-backfill') {
    validateManifestAttestation(descriptor)
  }
  return value as unknown as GeometryExecutionDescriptor
}

export function serializeExecutionDescriptor(
  value: GeometryExecutionDescriptor,
  quality: 'preview' | 'full',
  status: 'succeeded' | 'failed' | 'cancelled',
): string {
  const descriptor = validateExecutionDescriptor(value, quality, status)
  if (descriptor.evidence === 'legacy-backfill') {
    throw new TypeError('New builds cannot use legacy-backfill execution evidence')
  }
  return JSON.stringify(descriptor)
}

export function sha256(data: Uint8Array | string): string {
  return createHash('sha256').update(data).digest('hex')
}

export function expectedArtifactMimeType(format: 'stl' | 'obj'): string {
  return format === 'stl' ? 'model/stl' : 'model/obj'
}

export function modelFromRow(row: Row): StoredModel {
  const model = {
    id: requiredString(row, 'id'), name: requiredString(row, 'name'), source: requiredString(row, 'source'),
    revision: safeInteger(row, 'revision'),
    createdAt: isoTimestamp(row, 'created_at'),
    updatedAt: isoTimestamp(row, 'updated_at'),
  }
  validateModelId(model.id)
  validateModelName(model.name)
  validateModelSource(model.source)
  if (model.revision < 0) throw new TypeError('DuckDB returned an invalid revision')
  return model
}

export function modelRevisionFromRow(row: Row): StoredModelRevision {
  const revision = {
    id: requiredString(row, 'id'),
    name: requiredString(row, 'name'),
    source: requiredString(row, 'source'),
    revision: safeInteger(row, 'revision'),
    createdAt: isoTimestamp(row, 'created_at'),
  }
  validateModelId(revision.id)
  validateModelName(revision.name)
  validateModelSource(revision.source)
  if (revision.revision < 0) throw new TypeError('DuckDB returned an invalid revision')
  return revision
}

export function buildFromRow(row: Row): StoredBuild {
  const statusValue = requiredString(row, 'status')
  if (statusValue !== 'succeeded' && statusValue !== 'failed' && statusValue !== 'cancelled') {
    throw new TypeError('DuckDB returned an invalid build status')
  }
  const status = statusValue
  const qualityValue = requiredString(row, 'quality')
  if (qualityValue !== 'preview' && qualityValue !== 'full') throw new TypeError('DuckDB returned an invalid build quality')
  const warnings = parseJson<unknown>(row, 'warnings_json', [])
  if (!Array.isArray(warnings) || warnings.length > MAX_STORED_BUILD_WARNINGS
    || warnings.some(value => typeof value !== 'string'
      || value.length > MAX_STORED_WARNING_LENGTH || !isWellFormedUnicode(value))) {
    throw new TypeError('DuckDB returned invalid build warnings')
  }
  const sourceSha256 = requiredString(row, 'source_sha256')
  if (!/^[a-f0-9]{64}$/i.test(sourceSha256)) throw new TypeError('DuckDB returned an invalid source digest')
  const metrics: BuildMetrics | null = status === 'succeeded' ? {
    meshCount: safeInteger(row, 'mesh_count'),
    triangleCount: safeInteger(row, 'triangle_count'),
    volume: finiteNumber(row, 'volume'),
    surfaceArea: finiteNumber(row, 'surface_area'),
    reduced: row.reduced === true,
  } : null
  const execution = validateExecutionDescriptor(
    parseJson<unknown>(row, 'execution_json', legacyExecutionDescriptor(qualityValue)),
    qualityValue,
    status,
  )
  const rawError = parseJson<unknown>(row, 'error_json', null)
  const error = rawError === null
    ? null
    : storedBuildDiagnostic(rawError, 'DuckDB build diagnostic')
  if (error?.contractVersion === 1) {
    validateDiagnosticExecutionContext(error, execution, 'DuckDB build diagnostic')
  }
  if (status === 'succeeded' && error !== null) {
    throw new TypeError('DuckDB returned a successful build with a diagnostic')
  }
  if (status === 'failed' && error === null) {
    throw new TypeError('DuckDB returned a failed build without a diagnostic')
  }
  const modelId = optionalString(row, 'model_id')
  const modelRevision = row.model_revision === null ? null : safeInteger(row, 'model_revision')
  const inlineSource = optionalString(row, 'source_snapshot')
  const sourceAttestationValue = requiredString(row, 'source_attestation')
  if (sourceAttestationValue !== 'model-revision'
    && sourceAttestationValue !== 'inline-snapshot'
    && sourceAttestationValue !== 'historical-unattested') {
    throw new TypeError('DuckDB returned an invalid build source attestation class')
  }
  const sourceAttestation = sourceAttestationValue
  if ((modelId === null) !== (modelRevision === null)) {
    throw new TypeError('DuckDB returned an incomplete build model revision reference')
  }
  if (modelId !== null && inlineSource !== null) {
    throw new TypeError('DuckDB returned both model-backed and inline build source snapshots')
  }
  if ((sourceAttestation === 'model-revision' && (modelId === null || inlineSource !== null))
    || (sourceAttestation === 'inline-snapshot' && (modelId !== null || inlineSource === null))
    || (sourceAttestation === 'historical-unattested' && (modelId !== null || inlineSource !== null))) {
    throw new TypeError('DuckDB returned build source evidence that contradicts its attestation class')
  }
  let attestedSource: string | null = inlineSource
  if (modelId !== null && modelRevision !== null) {
    const revisionSource = optionalString(row, 'revision_source')
    if (revisionSource === null) {
      throw new TypeError('DuckDB returned a build whose immutable model revision is missing')
    }
    if (sha256(revisionSource) !== sourceSha256.toLowerCase()) {
      throw new TypeError('DuckDB returned a build digest that does not match its immutable model revision')
    }
    attestedSource = revisionSource
  }
  if (attestedSource !== null) {
    validateModelSource(attestedSource)
    if (sha256(attestedSource) !== sourceSha256.toLowerCase()) {
      throw new TypeError('DuckDB returned a build digest that does not match its immutable source snapshot')
    }
    if (execution.evidence !== 'legacy-backfill') {
      const route = parseGeometrySourceRoutingHeader(attestedSource)
      if (route.languageContract !== execution.languageContract
        || route.requiredCapabilities.length !== execution.requiredCapabilities.length
        || route.requiredCapabilities.some((capability, index) => (
          capability !== execution.requiredCapabilities[index]
        ))) {
        throw new TypeError('DuckDB returned build provenance that does not match its immutable source route')
      }
    }
  }
  return {
    id: requiredString(row, 'id'),
    modelId,
    modelRevision,
    sourceAttestation,
    sourceSha256,
    quality: qualityValue,
    durationMs: finiteNumber(row, 'duration_ms'),
    warnings: warnings as string[],
    status,
    metrics,
    error,
    execution,
    createdAt: isoTimestamp(row, 'created_at'),
  }
}

export function artifactSummaryFromRow(row: Row): StoredArtifactSummary {
  const formatValue = requiredString(row, 'format')
  if (formatValue !== 'stl' && formatValue !== 'obj') throw new TypeError('DuckDB returned an invalid artifact format')
  const summary: StoredArtifactSummary = {
    id: requiredString(row, 'id'),
    buildId: requiredString(row, 'build_id'),
    modelId: optionalString(row, 'model_id'),
    format: formatValue,
    fileName: requiredString(row, 'file_name'),
    mimeType: requiredString(row, 'mime_type'),
    sha256: requiredString(row, 'sha256'),
    byteLength: safeInteger(row, 'byte_length'),
    createdAt: isoTimestamp(row, 'created_at'),
  }
  validateModelId(summary.id)
  validateModelId(summary.buildId)
  if (summary.modelId !== null) validateModelId(summary.modelId)
  if (!summary.fileName || summary.fileName.length > 255 || /[/\\\0]/.test(summary.fileName)
    || !isWellFormedUnicode(summary.fileName)) throw new TypeError('DuckDB returned an invalid artifact file name')
  if (summary.mimeType !== expectedArtifactMimeType(summary.format)) throw new TypeError('DuckDB returned an invalid artifact MIME type')
  if (!/^[a-f0-9]{64}$/i.test(summary.sha256)) throw new TypeError('DuckDB returned an invalid artifact digest')
  if (summary.byteLength < 0 || summary.byteLength > MAX_STORED_ARTIFACT_BYTES) {
    throw new RangeError('DuckDB returned an oversized artifact')
  }
  return summary
}
