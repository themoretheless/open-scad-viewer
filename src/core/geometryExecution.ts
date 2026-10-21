import {RUNTIME_GEOMETRY_EVIDENCE_V5} from './runtimeGeometryEvidenceV5'
import {RUNTIME_GEOMETRY_EVIDENCE_V4} from './runtimeGeometryEvidenceV4'
import packagedGeometryIdentity from '../generated/geometry-kernels/identity'
import { CURRENT_RUNTIME_GEOMETRY_EVIDENCE } from './currentRuntimeGeometryEvidence'
import { RUNTIME_GEOMETRY_EVIDENCE_V3 } from './runtimeGeometryEvidenceV3'
import { OWN_RUST_CAD_EVIDENCE } from './ownRustCadEvidence'
import type { GeometryQuality } from './build'
import { sha256Hex } from './sha256'
import {
  parseGeometrySourceRoutingHeader,
  type GeometryLanguageContract,
} from './geometryRouting'
import { isWellFormedUnicode } from './unicode'
import { deepFreeze } from './objects'
import { computeGeometryManifestDigest, qualifiedRuntimeManifestVersion } from './geometryExecution/geometryEngineClassForLanguageContract'
export { geometryEngineClassForLanguageContract, canonicalJson, toWireManifest, computeGeometryManifestDigest, geometryProviderAdmissionForManifest, planGeometrySourceExecution, freezeGeometryExecutionDescriptor } from './geometryExecution/geometryEngineClassForLanguageContract'

export {
  GEOMETRY_LANGUAGE_CONTRACTS,
  GeometryLanguageContractError,
  MAX_GEOMETRY_SOURCE_CHARACTERS,
  parseGeometrySourceRoutingHeader,
  type GeometryLanguageContract,
  type GeometrySourceRoutingHeader,
} from './geometryRouting'

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

export const ENGINE_CLASS_BY_LANGUAGE_CONTRACT = Object.freeze({
  'legacy/current': 'mesh',
  'openscad-viewer/brep-1': 'brep',
} as const satisfies Readonly<Record<GeometryLanguageContract, GeometryEngineClass>>)

export type GeometryBuildPurpose = 'preview' | 'full' | 'analysis' | 'export'
export type GeometryRepresentation = 'mesh' | 'brep'
export type GeometryExecutionEvidence = 'planned' | 'runtime' | 'legacy-backfill'

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

// Private candidate binding. Publication requires the referenced complete
// qualification record; historical manifests remain immutable.
function bindCurrentRuntime(
  base: GeometryEngineStaticManifest, version: string, engineKey: string,
  evidence: {wasmSha256:string;noticesSha256:string;lockfileSha256:string;recordId:string;corpusVersion:string},
): GeometryEngineStaticManifest {
  const manifest: GeometryEngineStaticManifest = {
    ...base, engineKey, capabilityManifestVersion: version,
    kernelFingerprint: `sha256:${evidence.wasmSha256}`,
    qualification: {status: 'qualified', recordId: evidence.recordId,
      corpusVersion: evidence.corpusVersion, target: 'browser-worker/node-mcp'},
    dependency: {...base.dependency, sbomSha256: evidence.noticesSha256,
      lockfileSha256: evidence.lockfileSha256},
    manifestDigest: '',
  }
  manifest.manifestDigest = computeGeometryManifestDigest(manifest)
  return manifest
}

const ownRustCadRuntimeV3 = bindCurrentRuntime(ownRustCadManifestV1, 'own-rust-node-v3', 'own-rust-cad-v3', RUNTIME_GEOMETRY_EVIDENCE_V3)
const brepClosedRuntimeV3 = bindCurrentRuntime(brepClosedManifestV1, 'brep-closed-v3', 'rust-brep-closed-v3', RUNTIME_GEOMETRY_EVIDENCE_V3)

const ownRustCadRuntimeV4 = bindCurrentRuntime(ownRustCadManifestV1, 'own-rust-node-v4', 'own-rust-cad-v4', RUNTIME_GEOMETRY_EVIDENCE_V4)
const brepClosedRuntimeV4 = bindCurrentRuntime(brepClosedManifestV1, 'brep-closed-v4', 'rust-brep-closed-v4', RUNTIME_GEOMETRY_EVIDENCE_V4)

const ownRustCadRuntimeV5 = bindCurrentRuntime(ownRustCadManifestV1, 'own-rust-node-v5', 'own-rust-cad-v5', RUNTIME_GEOMETRY_EVIDENCE_V5)
const brepClosedRuntimeV5 = bindCurrentRuntime(brepClosedManifestV1, 'brep-closed-v5', 'rust-brep-closed-v5', RUNTIME_GEOMETRY_EVIDENCE_V5)

const ownRustCadRuntimeV6 = bindCurrentRuntime(ownRustCadManifestV1, 'own-rust-node-v6', 'own-rust-cad-v6', CURRENT_RUNTIME_GEOMETRY_EVIDENCE)
const brepClosedRuntimeV6 = bindCurrentRuntime(brepClosedManifestV1, 'brep-closed-v6', 'rust-brep-closed-v6', CURRENT_RUNTIME_GEOMETRY_EVIDENCE)

export const GEOMETRY_MANIFEST_ARCHIVE = deepFreeze({
  'own-rust-node-v6': ownRustCadRuntimeV6,
  'brep-closed-v6': brepClosedRuntimeV6,
  'own-rust-node-v5': ownRustCadRuntimeV5,
  'brep-closed-v5': brepClosedRuntimeV5,
  'own-rust-node-v4': ownRustCadRuntimeV4,
  'brep-closed-v4': brepClosedRuntimeV4,
  'own-rust-node-v3': ownRustCadRuntimeV3,
  'brep-closed-v3': brepClosedRuntimeV3,
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
  mesh: 'own-rust-node-v6',
  brep: 'brep-closed-v6',
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

/**
 * Runtime providers stay on the newest admitted manifest while a newer
 * catalog entry is still baseline-pending.
 */
export const ACTIVE_GEOMETRY_MANIFEST_VERSIONS = Object.freeze({
  mesh: qualifiedRuntimeManifestVersion('mesh'),
  brep: qualifiedRuntimeManifestVersion('brep'),
})

export interface GeometryEngineRegistrySnapshot {
  contractVersion: 1
  sourceDirectedRouting: true
  automaticFallback: false
  routes: readonly GeometryEngineRoute[]
  engines: GeometryEngineManifest[]
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
