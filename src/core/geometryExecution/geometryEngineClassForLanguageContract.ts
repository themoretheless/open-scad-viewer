import { ACTIVE_GEOMETRY_MANIFEST_VERSIONS, CURRENT_GEOMETRY_MANIFEST_VERSIONS, ENGINE_CLASS_BY_LANGUAGE_CONTRACT, GEOMETRY_MANIFEST_ARCHIVE } from '../geometryExecution'
import type { GeometryBuildPurpose, GeometryEngineClass, GeometryEngineStaticManifest, GeometryExecutionDescriptor, GeometryProviderAdmissionDecision } from '../geometryExecution'
import { parseGeometrySourceRoutingHeader } from '../geometryRouting'
import type { GeometryLanguageContract } from '../geometryRouting'
import type { GeometryQuality } from '../build'
import { deepFreeze } from '../objects'
import { isWellFormedUnicode } from '../unicode'
import { sha256Hex } from '../sha256'

export function geometryEngineClassForLanguageContract(
  languageContract: GeometryLanguageContract,
): GeometryEngineClass {
  return ENGINE_CLASS_BY_LANGUAGE_CONTRACT[languageContract]
}

/** RFC-8785-style ordering for the JSON-only manifest payload used here. */
export function canonicalJson(value: unknown): string {
  if (value === null || typeof value === 'boolean') return JSON.stringify(value)
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) throw new TypeError('Canonical JSON cannot encode non-finite numbers')
    return JSON.stringify(value)
  }
  if (typeof value === 'string') {
    if (!isWellFormedUnicode(value)) throw new TypeError('Canonical JSON requires well-formed Unicode')
    return JSON.stringify(value)
  }
  if (typeof value !== 'object') throw new TypeError('Canonical JSON supports JSON values only')
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`
  return `{${Object.entries(value as Record<string, unknown>)
    .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)
    .map(([key, item]) => `${JSON.stringify(key)}:${canonicalJson(item)}`)
    .join(',')}}`
}

export function assertExactObjectKeys(
  value: unknown,
  expected: readonly string[],
  label: string,
): asserts value is Record<string, unknown> {
  if (value === null || Array.isArray(value) || typeof value !== 'object') {
    throw new TypeError(`${label} must be an object`)
  }
  const actual = Object.keys(value).sort()
  const sortedExpected = [...expected].sort()
  if (actual.length !== sortedExpected.length
    || actual.some((key, index) => key !== sortedExpected[index])) {
    throw new TypeError(`${label} has unknown or missing fields`)
  }
}

/**
 * Snake-case wire mapping of every immutable static-manifest field except the
 * digest itself and its derived MCP resource URI. Snake-case names preserve
 * the already-published v1 digest bytes.
 */
export function toWireManifest(engine: GeometryEngineStaticManifest) {
  return {
    engine_class: engine.engineClass,
    display_name: engine.displayName,
    permanent: engine.permanent,
    maturity: engine.maturity,
    engine_key: engine.engineKey,
    kernel_fingerprint: engine.kernelFingerprint,
    semantic_program_version: engine.semanticProgramVersion,
    capability_manifest_version: engine.capabilityManifestVersion,
    language_contracts: engine.languageContracts,
    input_contract: engine.inputContract,
    capabilities: engine.capabilities,
    planned_capabilities: engine.plannedCapabilities,
    qualities: engine.qualities,
    representations: engine.representations,
    planned_representations: engine.plannedRepresentations,
    export_formats: engine.exportFormats,
    planned_export_formats: engine.plannedExportFormats,
    limits: engine.limits,
    isolation: engine.isolation,
    deployment: engine.deployment,
    qualification: {
      status: engine.qualification.status,
      record_id: engine.qualification.recordId,
      corpus_version: engine.qualification.corpusVersion,
      target: engine.qualification.target,
    },
    dependency: {
      package_name: engine.dependency.packageName,
      version: engine.dependency.version,
      license_expression: engine.dependency.licenseExpression,
      sbom_ref: engine.dependency.sbomRef,
      sbom_sha256: engine.dependency.sbomSha256,
      lockfile_sha256: engine.dependency.lockfileSha256,
    },
    rollback_compatibility: {
      disable_engine_capability: engine.rollbackCompatibility.disableEngineCapability,
      source_contract_preserved: engine.rollbackCompatibility.sourceContractPreserved,
      cross_engine_fallback: engine.rollbackCompatibility.crossEngineFallback,
      minimum_catalog_schema: engine.rollbackCompatibility.minimumCatalogSchema,
    },
    automatic_fallback: engine.automaticFallback,
  }
}

/**
 * Browser-safe digest of every immutable static-manifest field except the
 * digest itself and its derived MCP resource URI.
 */
export function computeGeometryManifestDigest(engine: GeometryEngineStaticManifest): string {
  assertExactObjectKeys(engine, [
    'engineClass', 'displayName', 'permanent', 'maturity', 'engineKey',
    'kernelFingerprint', 'semanticProgramVersion', 'capabilityManifestVersion',
    'languageContracts', 'inputContract', 'capabilities', 'plannedCapabilities',
    'qualities', 'representations', 'plannedRepresentations', 'exportFormats',
    'plannedExportFormats', 'limits', 'isolation', 'deployment', 'qualification',
    'dependency', 'rollbackCompatibility', 'manifestDigest', 'automaticFallback',
  ], 'Geometry manifest')
  assertExactObjectKeys(engine.qualification, [
    'status', 'recordId', 'corpusVersion', 'target',
  ], 'Geometry manifest qualification')
  assertExactObjectKeys(engine.dependency, [
    'packageName', 'version', 'licenseExpression', 'sbomRef', 'sbomSha256', 'lockfileSha256',
  ], 'Geometry manifest dependency evidence')
  assertExactObjectKeys(engine.rollbackCompatibility, [
    'disableEngineCapability', 'sourceContractPreserved', 'crossEngineFallback',
    'minimumCatalogSchema',
  ], 'Geometry manifest rollback compatibility')
  return sha256Hex(canonicalJson(toWireManifest(engine)))
}

export function geometryProviderAdmissionForManifest(
  manifest: GeometryEngineStaticManifest,
): GeometryProviderAdmissionDecision {
  let integrityMatches = false
  try {
    integrityMatches = computeGeometryManifestDigest(manifest) === manifest.manifestDigest
  } catch {
    integrityMatches = false
  }
  if (!integrityMatches) {
    return deepFreeze({
      allowed: false,
      mode: 'denied',
      reasonCode: 'manifest-integrity-failed',
      reason: 'Provider manifest digest does not attest its complete immutable payload.',
    })
  }

  if (manifest.qualification.status !== 'qualified'
    || manifest.qualification.recordId === null
    || !manifest.qualification.recordId.trim()
    || manifest.qualification.corpusVersion === null
    || !manifest.qualification.corpusVersion.trim()
    || !manifest.qualification.target.trim()) {
    return deepFreeze({
      allowed: false,
      mode: 'denied',
      reasonCode: 'qualification-incomplete',
      reason: 'Provider manifest has no complete qualified record.',
    })
  }
  if (manifest.maturity !== 'production'
    || manifest.deployment === 'not-deployed'
    || manifest.isolation === 'not-deployed'
    || !/^[a-f0-9]{64}$/.test(manifest.manifestDigest)
    || !/^sha256:[a-f0-9]{64}$/.test(manifest.kernelFingerprint)) {
    return deepFreeze({
      allowed: false,
      mode: 'denied',
      reasonCode: 'runtime-not-deployable',
      reason: 'Qualified provider manifest is not deployable or lacks an exact runtime artifact digest.',
    })
  }
  if (manifest.dependency.packageName === null
    || !manifest.dependency.packageName.trim()
    || manifest.dependency.version === null
    || !manifest.dependency.version.trim()
    || manifest.dependency.licenseExpression === null
    || !manifest.dependency.licenseExpression.trim()
    || manifest.dependency.sbomRef === null
    || !manifest.dependency.sbomRef.trim()
    || manifest.dependency.sbomSha256 === null
    || !/^[a-f0-9]{64}$/.test(manifest.dependency.sbomSha256)
    || manifest.dependency.lockfileSha256 === null
    || !/^[a-f0-9]{64}$/.test(manifest.dependency.lockfileSha256)) {
    return deepFreeze({
      allowed: false,
      mode: 'denied',
      reasonCode: 'dependency-attestation-incomplete',
      reason: 'Qualified provider manifest lacks complete dependency attestation.',
    })
  }
  return deepFreeze({
    allowed: true,
    mode: 'qualified',
    reasonCode: 'qualified',
    reason: 'Provider manifest has a complete deployable qualification record.',
  })
}

export function qualifiedRuntimeManifestVersion(
  engineClass: GeometryEngineClass,
): keyof typeof GEOMETRY_MANIFEST_ARCHIVE {
  const currentVersion = CURRENT_GEOMETRY_MANIFEST_VERSIONS[engineClass]
  const current = GEOMETRY_MANIFEST_ARCHIVE[currentVersion]
  if (geometryProviderAdmissionForManifest(current).allowed) return currentVersion

  const fallback = Object.entries(GEOMETRY_MANIFEST_ARCHIVE)
    .reverse()
    .find(([, manifest]) => manifest.engineClass === engineClass
      && geometryProviderAdmissionForManifest(manifest).allowed)
  if (!fallback) {
    throw new TypeError(`No qualified runtime manifest is available for ${engineClass}`)
  }
  return fallback[0] as keyof typeof GEOMETRY_MANIFEST_ARCHIVE
}

export function planGeometrySourceExecution(
  source: string,
  request: { quality: GeometryQuality; purpose: GeometryBuildPurpose },
): GeometryExecutionDescriptor {
  const header = parseGeometrySourceRoutingHeader(source)
  const engineClass = geometryEngineClassForLanguageContract(header.languageContract)
  const manifest = engineClass === 'mesh'
    ? GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS.mesh]
    : GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS.brep]
  return deepFreeze({
    languageContract: header.languageContract,
    requiredCapabilities: header.requiredCapabilities,
    engineClass,
    engineKey: manifest.engineKey,
    kernelFingerprint: manifest.kernelFingerprint,
    semanticProgramVersion: manifest.semanticProgramVersion,
    capabilityManifestVersion: manifest.capabilityManifestVersion,
    manifestDigest: manifest.manifestDigest,
    purpose: request.purpose,
    quality: request.quality,
    representation: engineClass === 'mesh' ? 'mesh' : 'brep',
    evidence: 'planned',
    effectiveLimits: { ...manifest.limits },
    automaticFallback: false,
  } satisfies GeometryExecutionDescriptor)
}

export function freezeGeometryExecutionDescriptor<T extends GeometryExecutionDescriptor>(
  descriptor: T,
): T {
  return deepFreeze(descriptor)
}
