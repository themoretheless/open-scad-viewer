import { OWN_RUST_CAD_EVIDENCE } from './ownRustCadEvidence'
import type { GeometryQuality } from './build'
import { sha256Hex } from './sha256'
import {
  parseGeometrySourceRoutingHeader,
  type GeometryLanguageContract,
} from './geometryRouting'

export {
  GEOMETRY_LANGUAGE_CONTRACTS,
  GeometryLanguageContractError,
  MAX_GEOMETRY_SOURCE_CHARACTERS,
  parseGeometrySourceRoutingHeader,
  type GeometryLanguageContract,
  type GeometrySourceRoutingHeader,
} from './geometryRouting'

function deepFreeze<T>(value: T): T {
  if (value !== null && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.values(value as Record<string, unknown>).forEach(item => deepFreeze(item))
    Object.freeze(value)
  }
  return value
}

export const GEOMETRY_ENGINE_CLASSES = Object.freeze(['mesh', 'brep'] as const)

export type GeometryEngineClass = typeof GEOMETRY_ENGINE_CLASSES[number]

export interface GeometryEngineRoute {
  readonly languageContract: GeometryLanguageContract
  readonly engineClass: GeometryEngineClass
  readonly fallback: 'never'
}

/**
 * The sole authoritative language-to-engine map. A caller may select a
 * language contract in source, but cannot select or retry another engine.
 */
export const GEOMETRY_ENGINE_ROUTES = deepFreeze([
  { languageContract: 'legacy/current', engineClass: 'mesh', fallback: 'never' },
  { languageContract: 'openscad-viewer/brep-1', engineClass: 'brep', fallback: 'never' },
] as const satisfies readonly GeometryEngineRoute[])

const ENGINE_CLASS_BY_LANGUAGE_CONTRACT = Object.freeze({
  'legacy/current': 'mesh',
  'openscad-viewer/brep-1': 'brep',
} as const satisfies Readonly<Record<GeometryLanguageContract, GeometryEngineClass>>)

export function geometryEngineClassForLanguageContract(
  languageContract: GeometryLanguageContract,
): GeometryEngineClass {
  return ENGINE_CLASS_BY_LANGUAGE_CONTRACT[languageContract]
}

export type GeometryBuildPurpose = 'preview' | 'full' | 'analysis' | 'export'
export type GeometryRepresentation = 'mesh' | 'brep'
export type GeometryExecutionEvidence = 'planned' | 'runtime' | 'legacy-backfill'

function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xD800 && unit <= 0xDBFF) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xDC00 && next <= 0xDFFF)) return false
    } else if (unit >= 0xDC00 && unit <= 0xDFFF) return false
  }
  return true
}

/**
 * Immutable provenance attached to every geometry result and persisted build.
 * A descriptor records the engine selected by the source language contract;
 * it is never a caller-controlled fallback decision.
 */
export interface GeometryExecutionDescriptor {
  readonly languageContract: GeometryLanguageContract
  readonly requiredCapabilities: readonly string[]
  readonly engineClass: GeometryEngineClass
  readonly engineKey: string
  readonly kernelFingerprint: string
  readonly semanticProgramVersion: string
  readonly capabilityManifestVersion: string
  readonly manifestDigest: string
  readonly purpose: GeometryBuildPurpose
  readonly quality: GeometryQuality
  readonly representation: GeometryRepresentation
  readonly evidence: GeometryExecutionEvidence
  readonly effectiveLimits: Readonly<Record<string, number>>
  readonly automaticFallback: false
}

export interface GeometryManifestQualification {
  status: 'qualified' | 'baseline-pending' | 'not-qualified'
  recordId: string | null
  corpusVersion: string | null
  target: string
}

export interface GeometryManifestDependencyEvidence {
  packageName: string | null
  version: string | null
  licenseExpression: string | null
  sbomRef: string | null
  sbomSha256: string | null
  lockfileSha256: string | null
}

export interface GeometryManifestRollbackCompatibility {
  disableEngineCapability: true
  sourceContractPreserved: true
  crossEngineFallback: false
  minimumCatalogSchema: number
}

export interface GeometryEngineStaticManifest {
  engineClass: GeometryEngineClass
  displayName: string
  permanent: true
  maturity: 'production' | 'contract'
  engineKey: string
  kernelFingerprint: string
  semanticProgramVersion: string
  capabilityManifestVersion: string
  languageContracts: readonly GeometryLanguageContract[]
  inputContract: 'legacy-source-direct' | 'semantic-program-required'
  capabilities: readonly string[]
  plannedCapabilities: readonly string[]
  qualities: readonly GeometryQuality[]
  representations: readonly GeometryRepresentation[]
  plannedRepresentations: readonly GeometryRepresentation[]
  exportFormats: readonly string[]
  plannedExportFormats: readonly string[]
  limits: Readonly<Record<string, number>>
  isolation: 'in-process-serialized' | 'not-deployed'
  deployment: 'node-mcp' | 'not-deployed'
  qualification: GeometryManifestQualification
  dependency: GeometryManifestDependencyEvidence
  rollbackCompatibility: GeometryManifestRollbackCompatibility
  manifestDigest: string
  automaticFallback: false
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

function assertExactObjectKeys(
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

export interface GeometryEngineManifest extends GeometryEngineStaticManifest {
  availability: 'available' | 'unavailable'
  unavailableReason: string | null
}

const ownRustCadManifest: GeometryEngineStaticManifest = {
  engineClass: 'mesh',
  displayName: 'Own Rust CAD', permanent: true, maturity: 'production',
  engineKey: 'own-rust-cad-v2', kernelFingerprint: `sha256:${OWN_RUST_CAD_EVIDENCE.wasmSha256}`,
  semanticProgramVersion: 'legacy-direct-evaluator-v1', capabilityManifestVersion: 'own-rust-node-v2',
  languageContracts: ['legacy/current'], inputContract: 'legacy-source-direct',
  capabilities: ['analysis.metrics', 'csg.boolean', 'export.obj', 'export.stl', 'geometry.mesh'],
  plannedCapabilities: ['provenance.source-ranges'], qualities: ['preview', 'full'],
  representations: ['mesh'], plannedRepresentations: [], exportFormats: ['stl', 'obj'], plannedExportFormats: [],
  limits: {sourceCharacters: 250_000, triangles: 750_000}, isolation: 'in-process-serialized', deployment: 'node-mcp',
  qualification: {status: 'baseline-pending', recordId: null, corpusVersion: null, target: 'browser-worker/node-mcp'},
  dependency: {packageName: 'workspace:geometry-bridge', version: '0.1.0', licenseExpression: 'MIT', sbomRef: 'THIRD_PARTY_NOTICES.md', sbomSha256: OWN_RUST_CAD_EVIDENCE.noticesSha256, lockfileSha256: OWN_RUST_CAD_EVIDENCE.lockfileSha256},
  rollbackCompatibility: {disableEngineCapability: true, sourceContractPreserved: true, crossEngineFallback: false, minimumCatalogSchema: 3},
  manifestDigest: '', automaticFallback: false,
}
ownRustCadManifest.manifestDigest = computeGeometryManifestDigest(ownRustCadManifest)

// Historical snapshots intentionally do not inherit the mutable current catalog.
const ownRustCadManifestV1: GeometryEngineStaticManifest = {
  engineClass: 'mesh',
  displayName: 'Own Rust CAD', permanent: true, maturity: 'production',
  engineKey: 'own-rust-cad-v1',
  kernelFingerprint: 'sha256:fde93f46f61330609eaab5c7470be0bb24a2ff14a64d0b83788c5c0051d82af6',
  semanticProgramVersion: 'legacy-direct-evaluator-v1',
  capabilityManifestVersion: 'own-rust-node-v1',
  languageContracts: ['legacy/current'], inputContract: 'legacy-source-direct',
  capabilities: ['analysis.metrics', 'csg.boolean', 'export.obj', 'export.stl', 'geometry.mesh'],
  plannedCapabilities: ['provenance.source-ranges'], qualities: ['preview', 'full'],
  representations: ['mesh'], plannedRepresentations: [], exportFormats: ['stl', 'obj'], plannedExportFormats: [],
  limits: {sourceCharacters: 250_000, triangles: 750_000}, isolation: 'in-process-serialized', deployment: 'node-mcp',
  qualification: {status: 'qualified', recordId: 'docs/qualification/own-rust-cad-v1.json', corpusVersion: 'own-rust-cad-v1', target: 'browser-worker/node-mcp'},
  dependency: {
    packageName: 'workspace:geometry-bridge', version: '0.1.0', licenseExpression: 'MIT', sbomRef: 'THIRD_PARTY_NOTICES.md',
    sbomSha256: 'ff988c02c87aac5ee582a393378bb59ff40a024850dd377abd0cde7fdad231e6',
    lockfileSha256: '70eb933b3607298caa00f7cdd96a953f313e6cc080a34f9b3c5c58694c765704',
  },
  rollbackCompatibility: {disableEngineCapability: true, sourceContractPreserved: true, crossEngineFallback: false, minimumCatalogSchema: 3},
  manifestDigest: '', automaticFallback: false,
}
ownRustCadManifestV1.manifestDigest = computeGeometryManifestDigest(ownRustCadManifestV1)

const brepClosedManifest: GeometryEngineStaticManifest = {
  engineClass: 'brep',
  displayName: 'Rust B-rep/NURBS kernel (closed analytic peer)',
  permanent: true,
  maturity: 'production',
  engineKey: 'rust-brep-closed-v2',
  kernelFingerprint: `sha256:${OWN_RUST_CAD_EVIDENCE.wasmSha256}`,
  semanticProgramVersion: 'semantic-program-contract-v1',
  capabilityManifestVersion: 'brep-closed-v2',
  languageContracts: ['openscad-viewer/brep-1'],
  inputContract: 'semantic-program-required',
  capabilities: [
    'analysis.metrics',
    'csg.boolean',
    'export.obj',
    'export.stl',
    'export.step',
    'geometry.brep',
    'nurbs.curves',
    'nurbs.surfaces',
    'topology.stable-ids',
  ],
  plannedCapabilities: [],
  qualities: ['preview', 'full'],
  representations: ['brep', 'mesh'],
  plannedRepresentations: [],
  exportFormats: ['stl', 'obj', 'step'],
  plannedExportFormats: [],
  limits: { sourceCharacters: 250_000, triangles: 750_000 },
  isolation: 'in-process-serialized',
  deployment: 'node-mcp',
  qualification: {
    status: 'baseline-pending',
    recordId: null,
    corpusVersion: null,
    target: 'browser-worker/node-mcp',
  },
  dependency: {
    packageName: 'workspace:geometry-bridge',
    version: '0.1.0',
    licenseExpression: 'MIT',
    sbomRef: 'THIRD_PARTY_NOTICES.md',
    sbomSha256: OWN_RUST_CAD_EVIDENCE.noticesSha256,
    lockfileSha256: OWN_RUST_CAD_EVIDENCE.lockfileSha256,
  },
  rollbackCompatibility: {
    disableEngineCapability: true,
    sourceContractPreserved: true,
    crossEngineFallback: false,
    minimumCatalogSchema: 3,
  },
  manifestDigest: '',
  automaticFallback: false,
}
brepClosedManifest.manifestDigest = computeGeometryManifestDigest(brepClosedManifest)

const brepClosedManifestV1: GeometryEngineStaticManifest = {
  engineClass: 'brep',
  displayName: 'Rust B-rep/NURBS kernel (closed analytic peer)',
  permanent: true,
  maturity: 'production',
  engineKey: 'rust-brep-closed-v1',
  kernelFingerprint: 'sha256:fde93f46f61330609eaab5c7470be0bb24a2ff14a64d0b83788c5c0051d82af6',
  semanticProgramVersion: 'semantic-program-contract-v1',
  capabilityManifestVersion: 'brep-closed-v1',
  languageContracts: ['openscad-viewer/brep-1'],
  inputContract: 'semantic-program-required',
  capabilities: [
    'analysis.metrics', 'csg.boolean', 'export.obj', 'export.stl', 'export.step',
    'geometry.brep', 'nurbs.curves', 'nurbs.surfaces', 'topology.stable-ids',
  ],
  plannedCapabilities: [],
  qualities: ['preview', 'full'],
  representations: ['brep', 'mesh'],
  plannedRepresentations: [],
  exportFormats: ['stl', 'obj', 'step'],
  plannedExportFormats: [],
  limits: { sourceCharacters: 250_000, triangles: 750_000 },
  isolation: 'in-process-serialized',
  deployment: 'node-mcp',
  qualification: {
    status: 'qualified',
    recordId: 'docs/qualification/brep-closed-matrix-v1.json',
    corpusVersion: 'brep-closed-matrix-v1',
    target: 'browser-worker/node-mcp',
  },
  dependency: {
    packageName: 'workspace:geometry-bridge', version: '0.1.0', licenseExpression: 'MIT', sbomRef: 'THIRD_PARTY_NOTICES.md',
    sbomSha256: 'ff988c02c87aac5ee582a393378bb59ff40a024850dd377abd0cde7fdad231e6',
    lockfileSha256: '70eb933b3607298caa00f7cdd96a953f313e6cc080a34f9b3c5c58694c765704',
  },
  rollbackCompatibility: {
    disableEngineCapability: true, sourceContractPreserved: true,
    crossEngineFallback: false, minimumCatalogSchema: 3,
  },
  manifestDigest: '',
  automaticFallback: false,
}
brepClosedManifestV1.manifestDigest = computeGeometryManifestDigest(brepClosedManifestV1)

export const GEOMETRY_MANIFEST_ARCHIVE = deepFreeze({
  'own-rust-node-v1': deepFreeze(ownRustCadManifestV1),
  'own-rust-node-v2': deepFreeze(ownRustCadManifest),
  'brep-closed-v1': deepFreeze(brepClosedManifestV1),
  'brep-closed-v2': deepFreeze(brepClosedManifest),
  'brep-contract-v1': deepFreeze({
    engineClass: 'brep',
    displayName: 'Rust B-rep/NURBS kernel',
    permanent: true,
    maturity: 'contract',
    engineKey: 'rust-brep-reserved-v1',
    kernelFingerprint: 'not-deployed',
    semanticProgramVersion: 'semantic-program-contract-v1',
    capabilityManifestVersion: 'brep-contract-v1',
    languageContracts: ['openscad-viewer/brep-1'],
    inputContract: 'semantic-program-required',
    capabilities: [],
    plannedCapabilities: [
      'analysis.metrics',
      'export.obj',
      'export.step',
      'export.stl',
      'geometry.brep',
      'nurbs.curves',
      'nurbs.surfaces',
      'topology.stable-ids',
    ],
    qualities: ['preview', 'full'],
    representations: [],
    plannedRepresentations: ['brep', 'mesh'],
    exportFormats: [],
    plannedExportFormats: ['stl', 'obj', 'step'],
    limits: { sourceCharacters: 250_000 },
    isolation: 'not-deployed',
    deployment: 'not-deployed',
    qualification: {
      status: 'not-qualified',
      recordId: null,
      corpusVersion: null,
      target: 'not-deployed',
    },
    dependency: {
      packageName: null,
      version: null,
      licenseExpression: null,
      sbomRef: null,
      sbomSha256: null,
      lockfileSha256: null,
    },
    rollbackCompatibility: {
      disableEngineCapability: true,
      sourceContractPreserved: true,
      crossEngineFallback: false,
      minimumCatalogSchema: 3,
    },
    manifestDigest: 'c4eefdbf1ff0add1e1705413d6ab766774487db0cb5a1d1434ab6c25f4ef1964',
    automaticFallback: false,
  } satisfies GeometryEngineStaticManifest),
} satisfies Record<string, GeometryEngineStaticManifest>)

export const CURRENT_GEOMETRY_MANIFEST_VERSIONS = Object.freeze({
  mesh: 'own-rust-node-v2',
  brep: 'brep-closed-v2',
} as const)

export type GeometryProviderAdmissionMode =
  | 'legacy-grandfathered'
  | 'qualified'
  | 'denied'

export const GEOMETRY_PROVIDER_ADMISSION_REASON_CODES = Object.freeze([
  'manifest-integrity-failed',
  'legacy-exact-exception',
  'qualification-incomplete',
  'runtime-not-deployable',
  'dependency-attestation-incomplete',
  'qualified',
] as const)

export type GeometryProviderAdmissionReasonCode =
  typeof GEOMETRY_PROVIDER_ADMISSION_REASON_CODES[number]

export interface GeometryProviderAdmissionDecision {
  readonly allowed: boolean
  readonly mode: GeometryProviderAdmissionMode
  readonly reasonCode: GeometryProviderAdmissionReasonCode
  readonly reason: string
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

function qualifiedRuntimeManifestVersion(
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

/**
 * Runtime providers stay on the newest admitted manifest while a newer
 * catalog entry is still baseline-pending.
 */
export const ACTIVE_GEOMETRY_MANIFEST_VERSIONS = Object.freeze({
  mesh: qualifiedRuntimeManifestVersion('mesh'),
  brep: qualifiedRuntimeManifestVersion('brep'),
})

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

export interface GeometryEngineRegistrySnapshot {
  contractVersion: 1
  sourceDirectedRouting: true
  automaticFallback: false
  routes: readonly GeometryEngineRoute[]
  engines: GeometryEngineManifest[]
}

export function freezeGeometryExecutionDescriptor<T extends GeometryExecutionDescriptor>(
  descriptor: T,
): T {
  return deepFreeze(descriptor)
}

export const LEGACY_MANIFOLD_EXECUTION: GeometryExecutionDescriptor = deepFreeze({
  languageContract: 'legacy/current',
  requiredCapabilities: [],
  engineClass: 'mesh',
  engineKey: 'manifold-wasm-v1',
  kernelFingerprint: 'manifold-wasm-v1',
  semanticProgramVersion: 'legacy-direct-evaluator-v1',
  capabilityManifestVersion: 'manifold-node-v1',
  manifestDigest: 'ae4ee188f2cf4898699318745e9eda48f96672e49b4b6d857f371ce7ed90013c',
  purpose: 'full',
  quality: 'full',
  representation: 'mesh',
  evidence: 'legacy-backfill',
  effectiveLimits: {},
  automaticFallback: false,
})
