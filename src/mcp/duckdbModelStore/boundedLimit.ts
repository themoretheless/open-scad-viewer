import { MAX_STORED_DIAGNOSTIC_DETAIL_COUNT, MAX_STORED_DIAGNOSTIC_DETAIL_LENGTH, MAX_STORED_DIAGNOSTIC_JSON_BYTES, MAX_STORED_DIAGNOSTIC_MESSAGE_LENGTH, MAX_STORED_DIAGNOSTIC_NAME_LENGTH, isWellFormedUnicode } from '../modelStore'
import type { BuildDiagnostic, StoredBuildDiagnostic } from '../modelStore'
import { LEGACY_MANIFOLD_EXECUTION } from '../../core/geometryExecution'
import type { GeometryExecutionDescriptor } from '../../core/geometryExecution'
import { MAX_LIST_LIMIT } from '../duckdbModelStore'
import type { JsonValue, Row } from '../duckdbModelStore'
import { archivedGeometryManifest } from '../engineManifest'
import { expectedPublicErrorRetryable, isPublicErrorCode } from '../errorContract'

export function boundedLimit(limit = 100): number {
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > MAX_LIST_LIMIT) {
    throw new RangeError(`limit must be an integer between 1 and ${MAX_LIST_LIMIT}`)
  }
  return limit
}

export function requiredString(row: Row, key: string): string {
  const value = row[key]
  if (typeof value !== 'string') throw new TypeError(`DuckDB returned an invalid ${key}`)
  return value
}

export function optionalString(row: Row, key: string): string | null {
  const value = row[key]
  if (value === null) return null
  if (typeof value !== 'string') throw new TypeError(`DuckDB returned an invalid ${key}`)
  return value
}

export function finiteNumber(row: Row, key: string): number {
  const value = row[key]
  const number = typeof value === 'bigint' ? Number(value) : value
  if (typeof number !== 'number' || !Number.isFinite(number)) {
    throw new TypeError(`DuckDB returned an invalid ${key}`)
  }
  return number
}

export function safeInteger(row: Row, key: string): number {
  const value = finiteNumber(row, key)
  if (!Number.isSafeInteger(value)) throw new TypeError(`DuckDB returned an invalid ${key}`)
  return value
}

export function isoTimestamp(row: Row, key: string): string {
  const value = row[key]
  if (value instanceof Date && Number.isFinite(value.getTime())) return value.toISOString()
  if (typeof value === 'string') {
    const normalized = value.includes('T') ? value : `${value.replace(' ', 'T')}Z`
    const parsed = new Date(normalized)
    if (Number.isFinite(parsed.getTime())) return parsed.toISOString()
  }
  throw new TypeError(`DuckDB returned an invalid ${key}`)
}

export function parseJson<T>(row: Row, key: string, fallback: T): T {
  const value = row[key]
  if (value === null) return fallback
  if (typeof value !== 'string') throw new TypeError(`DuckDB returned invalid JSON in ${key}`)
  return JSON.parse(value) as T
}

export function validateDiagnosticFields(value: unknown, label: string): asserts value is Record<string, unknown> {
  if (!value || Array.isArray(value) || typeof value !== 'object'
    || Object.getPrototypeOf(value) !== Object.prototype) {
    throw new TypeError(`${label} is not an object`)
  }
  const diagnostic = value as Record<string, unknown>
  if (typeof diagnostic.name !== 'string' || !diagnostic.name
    || diagnostic.name.length > MAX_STORED_DIAGNOSTIC_NAME_LENGTH
    || typeof diagnostic.message !== 'string'
    || diagnostic.message.length > MAX_STORED_DIAGNOSTIC_MESSAGE_LENGTH
    || !isWellFormedUnicode(diagnostic.name)
    || !isWellFormedUnicode(diagnostic.message)) {
    throw new RangeError(`${label} exceeds the storage limits or contains invalid Unicode`)
  }
  if (diagnostic.code !== undefined
    && (typeof diagnostic.code !== 'string' || !/^[a-z][a-z0-9_]{0,63}$/.test(diagnostic.code))) {
    throw new TypeError(`${label} has an invalid stable code`)
  }
  if (diagnostic.retryable !== undefined && typeof diagnostic.retryable !== 'boolean') {
    throw new TypeError(`${label} has an invalid retry class`)
  }
  if (diagnostic.details !== undefined) {
    if (!diagnostic.details || Array.isArray(diagnostic.details)
      || typeof diagnostic.details !== 'object'
      || Object.getPrototypeOf(diagnostic.details) !== Object.prototype) {
      throw new TypeError(`${label} has invalid details`)
    }
    const entries = Object.entries(diagnostic.details as Record<string, unknown>)
    if (entries.length > MAX_STORED_DIAGNOSTIC_DETAIL_COUNT) {
      throw new RangeError(`${label} has too many details`)
    }
    for (const [key, detail] of entries) {
      if (!/^[a-z][a-z0-9_]{0,63}$/.test(key)
        || (typeof detail === 'string'
          && (detail.length > MAX_STORED_DIAGNOSTIC_DETAIL_LENGTH || !isWellFormedUnicode(detail)))
        || (typeof detail === 'number' && !Number.isFinite(detail))
        || (detail !== null && typeof detail !== 'string'
          && typeof detail !== 'number' && typeof detail !== 'boolean')) {
        throw new TypeError(`${label} has an invalid detail`)
      }
    }
  }
  for (const field of ['line', 'column'] as const) {
    const location = diagnostic[field]
    if (location !== undefined && (!Number.isSafeInteger(location) || (location as number) < 1)) {
      throw new RangeError(`${label} ${field} must be a positive safe integer`)
    }
  }
}

export function validateDiagnosticJsonSize(diagnostic: Record<string, unknown>, label: string): void {
  if (Buffer.byteLength(JSON.stringify(diagnostic), 'utf8') > MAX_STORED_DIAGNOSTIC_JSON_BYTES) {
    throw new RangeError(`${label} exceeds the bounded JSON payload limit`)
  }
}

export function rejectUnknownDiagnosticFields(
  diagnostic: Record<string, unknown>,
  allowed: readonly string[],
  label: string,
): void {
  const allowedFields = new Set(allowed)
  const unknown = Object.keys(diagnostic).find(key => !allowedFields.has(key))
  if (unknown !== undefined) throw new TypeError(`${label} has an unknown field ${unknown}`)
}

export function validateBuildDiagnostic(value: unknown, label: string): asserts value is BuildDiagnostic {
  validateDiagnosticFields(value, label)
  const diagnostic = value as Record<string, unknown>
  rejectUnknownDiagnosticFields(diagnostic, [
    'contractVersion', 'name', 'message', 'code', 'retryable', 'details', 'line', 'column',
  ], label)
  validateDiagnosticJsonSize(diagnostic, label)
  if (diagnostic.contractVersion !== 1) {
    throw new TypeError(`${label} must use diagnostic contract version 1`)
  }
  if (!isPublicErrorCode(diagnostic.code)) {
    throw new TypeError(`${label} has a code outside the frozen public taxonomy`)
  }
  if (typeof diagnostic.retryable !== 'boolean') {
    throw new TypeError(`${label} must include an attested retry class`)
  }
  const details = diagnostic.details as Record<string, unknown> | undefined
  const expectedRetryable = expectedPublicErrorRetryable(diagnostic.code, details)
  if (expectedRetryable === null || diagnostic.retryable !== expectedRetryable) {
    throw new TypeError(`${label} contradicts the frozen retry policy`)
  }
  if (diagnostic.code === 'engine_unavailable') {
    if (!details
      || typeof details.language_contract !== 'string'
      || (details.engine_class !== 'mesh' && details.engine_class !== 'brep')
      || typeof details.engine_key !== 'string'
      || !details.engine_key
      || details.automatic_fallback !== false) {
      throw new TypeError(`${label} lacks required engine-unavailable route details`)
    }
  } else if (diagnostic.code === 'capability_unavailable') {
    if (!details
      || typeof details.language_contract !== 'string'
      || (details.engine_class !== 'mesh' && details.engine_class !== 'brep')
      || typeof details.missing_capabilities !== 'string'
      || !details.missing_capabilities
      || details.automatic_fallback !== false) {
      throw new TypeError(`${label} lacks required capability-unavailable route details`)
    }
  } else if (diagnostic.code === 'language_contract_unsupported') {
    if (!details || !Object.prototype.hasOwnProperty.call(details, 'reported_contract')
      || (details.reported_contract !== null && typeof details.reported_contract !== 'string')) {
      throw new TypeError(`${label} lacks required language-contract details`)
    }
  }
}

export function storedBuildDiagnostic(value: unknown, label: string): StoredBuildDiagnostic {
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    throw new TypeError(`${label} is not an object`)
  }
  const version = (value as Record<string, unknown>).contractVersion
  if (version === 1) {
    validateBuildDiagnostic(value, label)
    return value
  }
  if (version !== undefined) throw new TypeError(`${label} has an unknown contract version`)
  validateDiagnosticFields(value, label)
  rejectUnknownDiagnosticFields(value as Record<string, unknown>, [
    'name', 'message', 'code', 'retryable', 'details', 'line', 'column',
  ], label)
  validateDiagnosticJsonSize(value as Record<string, unknown>, label)
  return {
    ...(value as Omit<StoredBuildDiagnostic, 'contractVersion' | 'evidence'>),
    contractVersion: 0,
    evidence: 'legacy-unattested',
  } as StoredBuildDiagnostic
}

export function validateDiagnosticExecutionContext(
  diagnostic: BuildDiagnostic,
  execution: GeometryExecutionDescriptor,
  label: string,
): void {
  const details = diagnostic.details
  if (diagnostic.code === 'engine_unavailable') {
    if (details?.language_contract !== execution.languageContract
      || details.engine_class !== execution.engineClass
      || details.engine_key !== execution.engineKey
      || details.automatic_fallback !== execution.automaticFallback) {
      throw new TypeError(`${label} route details contradict execution provenance`)
    }
    return
  }
  if (diagnostic.code === 'capability_unavailable') {
    const manifest = archivedGeometryManifest(execution.capabilityManifestVersion)
    const missingCapabilities = execution.requiredCapabilities.filter(capability => (
      !manifest?.capabilities.includes(capability)
    ))
    if (details?.language_contract !== execution.languageContract
      || details.engine_class !== execution.engineClass
      || details.missing_capabilities !== missingCapabilities.join(',')
      || details.automatic_fallback !== execution.automaticFallback) {
      throw new TypeError(`${label} capability details contradict execution provenance`)
    }
    return
  }
  if (diagnostic.code === 'language_contract_unsupported') {
    throw new TypeError(`${label} cannot attach a pre-routing language refusal to execution provenance`)
  }
}

export function validateBoundedJson(value: unknown, label: string, depth = 0): asserts value is JsonValue {
  if (depth > 8) throw new RangeError(`${label} exceeds the maximum nesting depth`)
  if (value === null || typeof value === 'boolean') return
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) throw new TypeError(`${label} contains a non-finite number`)
    return
  }
  if (typeof value === 'string') {
    if (!isWellFormedUnicode(value)) throw new TypeError(`${label} contains invalid Unicode`)
    return
  }
  if (Array.isArray(value)) {
    if (value.length > 256) throw new RangeError(`${label} contains an oversized array`)
    value.forEach(item => validateBoundedJson(item, label, depth + 1))
    return
  }
  if (typeof value !== 'object' || Object.getPrototypeOf(value) !== Object.prototype) {
    throw new TypeError(`${label} must contain only JSON values`)
  }
  const entries = Object.entries(value as Record<string, unknown>)
  if (entries.length > 128) throw new RangeError(`${label} contains too many fields`)
  for (const [key, item] of entries) {
    if (!key || key.length > 128 || !isWellFormedUnicode(key)) {
      throw new TypeError(`${label} contains an invalid field name`)
    }
    validateBoundedJson(item, label, depth + 1)
  }
}

export function legacyExecutionDescriptor(quality: 'preview' | 'full'): GeometryExecutionDescriptor {
  return {
    ...LEGACY_MANIFOLD_EXECUTION,
    purpose: quality,
    quality,
  }
}

export function validateManifestAttestation(descriptor: Record<string, unknown>): void {
  const manifestVersion = descriptor.capabilityManifestVersion as string
  const manifest = archivedGeometryManifest(manifestVersion)
  if (!manifest) throw new TypeError('Build execution provenance references an unknown capability manifest')
  if (descriptor.engineClass !== manifest.engineClass
    || descriptor.engineKey !== manifest.engineKey
    || descriptor.kernelFingerprint !== manifest.kernelFingerprint
    || descriptor.semanticProgramVersion !== manifest.semanticProgramVersion
    || descriptor.manifestDigest !== manifest.manifestDigest) {
    throw new TypeError('Build execution provenance does not match its immutable capability manifest')
  }
  if (!(manifest.languageContracts as readonly unknown[]).includes(descriptor.languageContract)) {
    throw new TypeError('Build execution provenance language contract is absent from its capability manifest')
  }
  const representation = descriptor.representation as string
  const executableRepresentations = manifest.representations as readonly string[]
  const qualifiedCapabilities = manifest.capabilities as readonly string[]
  const manifestLimits = manifest.limits as Readonly<Record<string, number>>
  const declaredRepresentations = [
    ...manifest.representations,
    ...manifest.plannedRepresentations,
  ] as readonly string[]
  if (!declaredRepresentations.includes(representation)) {
    throw new TypeError('Build execution provenance representation is absent from its capability manifest')
  }
  if (descriptor.evidence === 'runtime') {
    if (manifest.deployment !== 'node-mcp'
      || !executableRepresentations.includes(representation)) {
      throw new TypeError('Build execution provenance claims runtime evidence for an undeployed representation')
    }
    if ((descriptor.requiredCapabilities as string[]).some(capability => (
      !qualifiedCapabilities.includes(capability)
    ))) {
      throw new TypeError('Runtime execution provenance requires an unqualified capability')
    }
  }
  const effectiveLimits = descriptor.effectiveLimits as Record<string, number>
  const limitKeys = Object.keys(effectiveLimits).sort()
  const manifestLimitKeys = Object.keys(manifest.limits).sort()
  if (descriptor.evidence !== 'legacy-backfill' && (
    limitKeys.length !== manifestLimitKeys.length
    || limitKeys.some((key, index) => key !== manifestLimitKeys[index])
    || limitKeys.some(key => effectiveLimits[key] !== manifestLimits[key])
  )) {
    throw new TypeError('Build execution provenance limits are not attested by its capability manifest')
  }
}
