import { RUNTIME_GEOMETRY_EVIDENCE_V65 } from './runtimeGeometryEvidenceV65'
import { RUNTIME_GEOMETRY_EVIDENCE_V64 } from './runtimeGeometryEvidenceV64'
import { RUNTIME_GEOMETRY_EVIDENCE_V63 } from './runtimeGeometryEvidenceV63'
import { RUNTIME_GEOMETRY_EVIDENCE_V62 } from './runtimeGeometryEvidenceV62'
import { RUNTIME_GEOMETRY_EVIDENCE_V61 } from './runtimeGeometryEvidenceV61'
import { RUNTIME_GEOMETRY_EVIDENCE_V60 } from './runtimeGeometryEvidenceV60'
import { RUNTIME_GEOMETRY_EVIDENCE_V59 } from './runtimeGeometryEvidenceV59'
import { RUNTIME_GEOMETRY_EVIDENCE_V58 } from './runtimeGeometryEvidenceV58'
import { RUNTIME_GEOMETRY_EVIDENCE_V57 } from './runtimeGeometryEvidenceV57'
import { RUNTIME_GEOMETRY_EVIDENCE_V56 } from './runtimeGeometryEvidenceV56'
import { RUNTIME_GEOMETRY_EVIDENCE_V55 } from './runtimeGeometryEvidenceV55'
import { RUNTIME_GEOMETRY_EVIDENCE_V54 } from './runtimeGeometryEvidenceV54'
import { RUNTIME_GEOMETRY_EVIDENCE_V53 } from './runtimeGeometryEvidenceV53'
import { RUNTIME_GEOMETRY_EVIDENCE_V52 } from './runtimeGeometryEvidenceV52'
import { RUNTIME_GEOMETRY_EVIDENCE_V51 } from './runtimeGeometryEvidenceV51'
import { RUNTIME_GEOMETRY_EVIDENCE_V50 } from './runtimeGeometryEvidenceV50'
import { RUNTIME_GEOMETRY_EVIDENCE_V49 } from './runtimeGeometryEvidenceV49'
import { RUNTIME_GEOMETRY_EVIDENCE_V48 } from './runtimeGeometryEvidenceV48'
import { RUNTIME_GEOMETRY_EVIDENCE_V47 } from './runtimeGeometryEvidenceV47'
import { RUNTIME_GEOMETRY_EVIDENCE_V46 } from './runtimeGeometryEvidenceV46'
import { RUNTIME_GEOMETRY_EVIDENCE_V45 } from './runtimeGeometryEvidenceV45'
import { RUNTIME_GEOMETRY_EVIDENCE_V44 } from './runtimeGeometryEvidenceV44'
import { RUNTIME_GEOMETRY_EVIDENCE_V43 } from './runtimeGeometryEvidenceV43'
import { RUNTIME_GEOMETRY_EVIDENCE_V42 } from './runtimeGeometryEvidenceV42'
import { RUNTIME_GEOMETRY_EVIDENCE_V41 } from './runtimeGeometryEvidenceV41'
import { RUNTIME_GEOMETRY_EVIDENCE_V40 } from './runtimeGeometryEvidenceV40'
import { RUNTIME_GEOMETRY_EVIDENCE_V39 } from './runtimeGeometryEvidenceV39'
import { RUNTIME_GEOMETRY_EVIDENCE_V38 } from './runtimeGeometryEvidenceV38'
import { RUNTIME_GEOMETRY_EVIDENCE_V37 } from './runtimeGeometryEvidenceV37'
import { RUNTIME_GEOMETRY_EVIDENCE_V36 } from './runtimeGeometryEvidenceV36'
import { RUNTIME_GEOMETRY_EVIDENCE_V35 } from './runtimeGeometryEvidenceV35'
import { RUNTIME_GEOMETRY_EVIDENCE_V34 } from './runtimeGeometryEvidenceV34'
import { RUNTIME_GEOMETRY_EVIDENCE_V33 } from './runtimeGeometryEvidenceV33'
import { RUNTIME_GEOMETRY_EVIDENCE_V32 } from './runtimeGeometryEvidenceV32'
import { RUNTIME_GEOMETRY_EVIDENCE_V31 } from './runtimeGeometryEvidenceV31'
import { RUNTIME_GEOMETRY_EVIDENCE_V30 } from './runtimeGeometryEvidenceV30'
import { RUNTIME_GEOMETRY_EVIDENCE_V29 } from './runtimeGeometryEvidenceV29'
import { RUNTIME_GEOMETRY_EVIDENCE_V28 } from './runtimeGeometryEvidenceV28'
import { RUNTIME_GEOMETRY_EVIDENCE_V27 } from './runtimeGeometryEvidenceV27'
import { RUNTIME_GEOMETRY_EVIDENCE_V26 } from './runtimeGeometryEvidenceV26'
import { RUNTIME_GEOMETRY_EVIDENCE_V25 } from './runtimeGeometryEvidenceV25'
import { RUNTIME_GEOMETRY_EVIDENCE_V24 } from './runtimeGeometryEvidenceV24'
import { RUNTIME_GEOMETRY_EVIDENCE_V23 } from './runtimeGeometryEvidenceV23'
import { RUNTIME_GEOMETRY_EVIDENCE_V22 } from './runtimeGeometryEvidenceV22'
import { RUNTIME_GEOMETRY_EVIDENCE_V21 } from './runtimeGeometryEvidenceV21'
import { RUNTIME_GEOMETRY_EVIDENCE_V20 } from './runtimeGeometryEvidenceV20'
import { RUNTIME_GEOMETRY_EVIDENCE_V19 } from './runtimeGeometryEvidenceV19'
import { RUNTIME_GEOMETRY_EVIDENCE_V18 } from './runtimeGeometryEvidenceV18'
import { RUNTIME_GEOMETRY_EVIDENCE_V17 } from './runtimeGeometryEvidenceV17'
import { RUNTIME_GEOMETRY_EVIDENCE_V16 } from './runtimeGeometryEvidenceV16'
import { RUNTIME_GEOMETRY_EVIDENCE_V15 } from './runtimeGeometryEvidenceV15'
import { RUNTIME_GEOMETRY_EVIDENCE_V14 } from './runtimeGeometryEvidenceV14'
import { RUNTIME_GEOMETRY_EVIDENCE_V13 } from './runtimeGeometryEvidenceV13'
import { RUNTIME_GEOMETRY_EVIDENCE_V5 } from './runtimeGeometryEvidenceV5'
import { RUNTIME_GEOMETRY_EVIDENCE_V6 } from './runtimeGeometryEvidenceV6'
import { RUNTIME_GEOMETRY_EVIDENCE_V7 } from './runtimeGeometryEvidenceV7'
import { RUNTIME_GEOMETRY_EVIDENCE_V8 } from './runtimeGeometryEvidenceV8'
import { RUNTIME_GEOMETRY_EVIDENCE_V9 } from './runtimeGeometryEvidenceV9'
import { RUNTIME_GEOMETRY_EVIDENCE_V10 } from './runtimeGeometryEvidenceV10'
import { RUNTIME_GEOMETRY_EVIDENCE_V11 } from './runtimeGeometryEvidenceV11'
import { RUNTIME_GEOMETRY_EVIDENCE_V12 } from './runtimeGeometryEvidenceV12'
import { CURRENT_RUNTIME_GEOMETRY_EVIDENCE } from './currentRuntimeGeometryEvidence'
import { RUNTIME_GEOMETRY_EVIDENCE_V3 } from './runtimeGeometryEvidenceV3'
import { RUNTIME_GEOMETRY_EVIDENCE_V4 } from './runtimeGeometryEvidenceV4'
import { OWN_RUST_CAD_EVIDENCE } from './ownRustCadEvidence'
import type { GeometryQuality } from './build'
import { type GeometryLanguageContract } from './geometryRouting'
import { deepFreeze } from './objects'
import {
  computeGeometryManifestDigest,
  qualifiedRuntimeManifestVersion,
} from './geometryExecution/geometryEngineClassForLanguageContract'
export {
  geometryEngineClassForLanguageContract,
  canonicalJson,
  toWireManifest,
  computeGeometryManifestDigest,
  geometryProviderAdmissionForManifest,
  planGeometrySourceExecution,
  freezeGeometryExecutionDescriptor,
} from './geometryExecution/geometryEngineClassForLanguageContract'

export {
  GEOMETRY_LANGUAGE_CONTRACTS,
  GeometryLanguageContractError,
  MAX_GEOMETRY_SOURCE_CHARACTERS,
  parseGeometrySourceRoutingHeader,
  type GeometryLanguageContract,
  type GeometrySourceRoutingHeader,
} from './geometryRouting'

export const GEOMETRY_ENGINE_CLASSES = Object.freeze(['mesh', 'brep'] as const)

export type GeometryEngineClass = (typeof GEOMETRY_ENGINE_CLASSES)[number]

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
  displayName: 'Own Rust CAD',
  permanent: true,
  maturity: 'production',
  engineKey: 'own-rust-cad-v2',
  kernelFingerprint: `sha256:${OWN_RUST_CAD_EVIDENCE.wasmSha256}`,
  semanticProgramVersion: 'legacy-direct-evaluator-v1',
  capabilityManifestVersion: 'own-rust-node-v2',
  languageContracts: ['legacy/current'],
  inputContract: 'legacy-source-direct',
  capabilities: ['analysis.metrics', 'csg.boolean', 'export.obj', 'export.stl', 'geometry.mesh'],
  plannedCapabilities: ['provenance.source-ranges'],
  qualities: ['preview', 'full'],
  representations: ['mesh'],
  plannedRepresentations: [],
  exportFormats: ['stl', 'obj'],
  plannedExportFormats: [],
  limits: { sourceCharacters: 250_000, triangles: 750_000 },
  isolation: 'in-process-serialized',
  deployment: 'node-mcp',
  qualification: { status: 'baseline-pending', recordId: null, corpusVersion: null, target: 'browser-worker/node-mcp' },
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
ownRustCadManifest.manifestDigest = computeGeometryManifestDigest(ownRustCadManifest)

// Historical snapshots intentionally do not inherit the mutable current catalog.
const ownRustCadManifestV1: GeometryEngineStaticManifest = {
  engineClass: 'mesh',
  displayName: 'Own Rust CAD',
  permanent: true,
  maturity: 'production',
  engineKey: 'own-rust-cad-v1',
  kernelFingerprint: 'sha256:fde93f46f61330609eaab5c7470be0bb24a2ff14a64d0b83788c5c0051d82af6',
  semanticProgramVersion: 'legacy-direct-evaluator-v1',
  capabilityManifestVersion: 'own-rust-node-v1',
  languageContracts: ['legacy/current'],
  inputContract: 'legacy-source-direct',
  capabilities: ['analysis.metrics', 'csg.boolean', 'export.obj', 'export.stl', 'geometry.mesh'],
  plannedCapabilities: ['provenance.source-ranges'],
  qualities: ['preview', 'full'],
  representations: ['mesh'],
  plannedRepresentations: [],
  exportFormats: ['stl', 'obj'],
  plannedExportFormats: [],
  limits: { sourceCharacters: 250_000, triangles: 750_000 },
  isolation: 'in-process-serialized',
  deployment: 'node-mcp',
  qualification: {
    status: 'qualified',
    recordId: 'docs/qualification/own-rust-cad-v1.json',
    corpusVersion: 'own-rust-cad-v1',
    target: 'browser-worker/node-mcp',
  },
  dependency: {
    packageName: 'workspace:geometry-bridge',
    version: '0.1.0',
    licenseExpression: 'MIT',
    sbomRef: 'THIRD_PARTY_NOTICES.md',
    sbomSha256: 'ff988c02c87aac5ee582a393378bb59ff40a024850dd377abd0cde7fdad231e6',
    lockfileSha256: '70eb933b3607298caa00f7cdd96a953f313e6cc080a34f9b3c5c58694c765704',
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
    status: 'qualified',
    recordId: 'docs/qualification/brep-closed-matrix-v1.json',
    corpusVersion: 'brep-closed-matrix-v1',
    target: 'browser-worker/node-mcp',
  },
  dependency: {
    packageName: 'workspace:geometry-bridge',
    version: '0.1.0',
    licenseExpression: 'MIT',
    sbomRef: 'THIRD_PARTY_NOTICES.md',
    sbomSha256: 'ff988c02c87aac5ee582a393378bb59ff40a024850dd377abd0cde7fdad231e6',
    lockfileSha256: '70eb933b3607298caa00f7cdd96a953f313e6cc080a34f9b3c5c58694c765704',
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
brepClosedManifestV1.manifestDigest = computeGeometryManifestDigest(brepClosedManifestV1)

// Private candidate binding. Publication requires the referenced complete
// qualification record; historical manifests remain immutable.
function bindCurrentRuntime(
  base: GeometryEngineStaticManifest,
  version: string,
  engineKey: string,
  evidence: {
    wasmSha256: string
    noticesSha256: string
    lockfileSha256: string
    recordId: string
    corpusVersion: string
  },
): GeometryEngineStaticManifest {
  const manifest: GeometryEngineStaticManifest = {
    ...base,
    engineKey,
    capabilityManifestVersion: version,
    kernelFingerprint: `sha256:${evidence.wasmSha256}`,
    qualification: {
      status: 'qualified',
      recordId: evidence.recordId,
      corpusVersion: evidence.corpusVersion,
      target: 'browser-worker/node-mcp',
    },
    dependency: { ...base.dependency, sbomSha256: evidence.noticesSha256, lockfileSha256: evidence.lockfileSha256 },
    manifestDigest: '',
  }
  manifest.manifestDigest = computeGeometryManifestDigest(manifest)
  return manifest
}

const ownRustCadRuntimeV3 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v3',
  'own-rust-cad-v3',
  RUNTIME_GEOMETRY_EVIDENCE_V3,
)
const brepClosedRuntimeV3 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v3',
  'rust-brep-closed-v3',
  RUNTIME_GEOMETRY_EVIDENCE_V3,
)

const ownRustCadRuntimeV4 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v4',
  'own-rust-cad-v4',
  RUNTIME_GEOMETRY_EVIDENCE_V4,
)
const brepClosedRuntimeV4 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v4',
  'rust-brep-closed-v4',
  RUNTIME_GEOMETRY_EVIDENCE_V4,
)

const ownRustCadRuntimeV5 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v5',
  'own-rust-cad-v5',
  RUNTIME_GEOMETRY_EVIDENCE_V5,
)
const brepClosedRuntimeV5 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v5',
  'rust-brep-closed-v5',
  RUNTIME_GEOMETRY_EVIDENCE_V5,
)

const ownRustCadRuntimeV6 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v6',
  'own-rust-cad-v6',
  RUNTIME_GEOMETRY_EVIDENCE_V6,
)
const brepClosedRuntimeV6 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v6',
  'rust-brep-closed-v6',
  RUNTIME_GEOMETRY_EVIDENCE_V6,
)

const ownRustCadRuntimeV7 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v7',
  'own-rust-cad-v7',
  RUNTIME_GEOMETRY_EVIDENCE_V7,
)
const brepClosedRuntimeV7 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v7',
  'rust-brep-closed-v7',
  RUNTIME_GEOMETRY_EVIDENCE_V7,
)

const ownRustCadRuntimeV8 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v8',
  'own-rust-cad-v8',
  RUNTIME_GEOMETRY_EVIDENCE_V8,
)
const brepClosedRuntimeV8 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v8',
  'rust-brep-closed-v8',
  RUNTIME_GEOMETRY_EVIDENCE_V8,
)

const ownRustCadRuntimeV9 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v9',
  'own-rust-cad-v9',
  RUNTIME_GEOMETRY_EVIDENCE_V9,
)
const brepClosedRuntimeV9 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v9',
  'rust-brep-closed-v9',
  RUNTIME_GEOMETRY_EVIDENCE_V9,
)

const ownRustCadRuntimeV10 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v10',
  'own-rust-cad-v10',
  RUNTIME_GEOMETRY_EVIDENCE_V10,
)
const brepClosedRuntimeV10 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v10',
  'rust-brep-closed-v10',
  RUNTIME_GEOMETRY_EVIDENCE_V10,
)

const ownRustCadRuntimeV11 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v11',
  'own-rust-cad-v11',
  RUNTIME_GEOMETRY_EVIDENCE_V11,
)
const brepClosedRuntimeV11 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v11',
  'rust-brep-closed-v11',
  RUNTIME_GEOMETRY_EVIDENCE_V11,
)

const ownRustCadRuntimeV12 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v12',
  'own-rust-cad-v12',
  RUNTIME_GEOMETRY_EVIDENCE_V12,
)
const brepClosedRuntimeV12 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v12',
  'rust-brep-closed-v12',
  RUNTIME_GEOMETRY_EVIDENCE_V12,
)

const ownRustCadRuntimeV13 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v13',
  'own-rust-cad-v13',
  RUNTIME_GEOMETRY_EVIDENCE_V13,
)
const brepClosedRuntimeV13 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v13',
  'rust-brep-closed-v13',
  RUNTIME_GEOMETRY_EVIDENCE_V13,
)

const ownRustCadRuntimeV14 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v14',
  'own-rust-cad-v14',
  RUNTIME_GEOMETRY_EVIDENCE_V14,
)
const brepClosedRuntimeV14 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v14',
  'rust-brep-closed-v14',
  RUNTIME_GEOMETRY_EVIDENCE_V14,
)

const ownRustCadRuntimeV15 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v15',
  'own-rust-cad-v15',
  RUNTIME_GEOMETRY_EVIDENCE_V15,
)
const brepClosedRuntimeV15 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v15',
  'rust-brep-closed-v15',
  RUNTIME_GEOMETRY_EVIDENCE_V15,
)

const ownRustCadRuntimeV16 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v16',
  'own-rust-cad-v16',
  RUNTIME_GEOMETRY_EVIDENCE_V16,
)
const brepClosedRuntimeV16 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v16',
  'rust-brep-closed-v16',
  RUNTIME_GEOMETRY_EVIDENCE_V16,
)

const ownRustCadRuntimeV17 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v17',
  'own-rust-cad-v17',
  RUNTIME_GEOMETRY_EVIDENCE_V17,
)
const brepClosedRuntimeV17 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v17',
  'rust-brep-closed-v17',
  RUNTIME_GEOMETRY_EVIDENCE_V17,
)

const ownRustCadRuntimeV18 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v18',
  'own-rust-cad-v18',
  RUNTIME_GEOMETRY_EVIDENCE_V18,
)
const brepClosedRuntimeV18 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v18',
  'rust-brep-closed-v18',
  RUNTIME_GEOMETRY_EVIDENCE_V18,
)

const ownRustCadRuntimeV19 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v19',
  'own-rust-cad-v19',
  RUNTIME_GEOMETRY_EVIDENCE_V19,
)
const brepClosedRuntimeV19 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v19',
  'rust-brep-closed-v19',
  RUNTIME_GEOMETRY_EVIDENCE_V19,
)

const ownRustCadRuntimeV20 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v20',
  'own-rust-cad-v20',
  RUNTIME_GEOMETRY_EVIDENCE_V20,
)
const brepClosedRuntimeV20 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v20',
  'rust-brep-closed-v20',
  RUNTIME_GEOMETRY_EVIDENCE_V20,
)

const ownRustCadRuntimeV21 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v21',
  'own-rust-cad-v21',
  RUNTIME_GEOMETRY_EVIDENCE_V21,
)
const brepClosedRuntimeV21 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v21',
  'rust-brep-closed-v21',
  RUNTIME_GEOMETRY_EVIDENCE_V21,
)

const ownRustCadRuntimeV22 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v22',
  'own-rust-cad-v22',
  RUNTIME_GEOMETRY_EVIDENCE_V22,
)
const brepClosedRuntimeV22 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v22',
  'rust-brep-closed-v22',
  RUNTIME_GEOMETRY_EVIDENCE_V22,
)

const ownRustCadRuntimeV23 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v23',
  'own-rust-cad-v23',
  RUNTIME_GEOMETRY_EVIDENCE_V23,
)
const brepClosedRuntimeV23 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v23',
  'rust-brep-closed-v23',
  RUNTIME_GEOMETRY_EVIDENCE_V23,
)

const ownRustCadRuntimeV24 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v24',
  'own-rust-cad-v24',
  RUNTIME_GEOMETRY_EVIDENCE_V24,
)
const brepClosedRuntimeV24 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v24',
  'rust-brep-closed-v24',
  RUNTIME_GEOMETRY_EVIDENCE_V24,
)

const ownRustCadRuntimeV25 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v25',
  'own-rust-cad-v25',
  RUNTIME_GEOMETRY_EVIDENCE_V25,
)
const brepClosedRuntimeV25 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v25',
  'rust-brep-closed-v25',
  RUNTIME_GEOMETRY_EVIDENCE_V25,
)

const ownRustCadRuntimeV26 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v26',
  'own-rust-cad-v26',
  RUNTIME_GEOMETRY_EVIDENCE_V26,
)
const brepClosedRuntimeV26 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v26',
  'rust-brep-closed-v26',
  RUNTIME_GEOMETRY_EVIDENCE_V26,
)

const ownRustCadRuntimeV27 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v27',
  'own-rust-cad-v27',
  RUNTIME_GEOMETRY_EVIDENCE_V27,
)
const brepClosedRuntimeV27 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v27',
  'rust-brep-closed-v27',
  RUNTIME_GEOMETRY_EVIDENCE_V27,
)

const ownRustCadRuntimeV28 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v28',
  'own-rust-cad-v28',
  RUNTIME_GEOMETRY_EVIDENCE_V28,
)
const brepClosedRuntimeV28 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v28',
  'rust-brep-closed-v28',
  RUNTIME_GEOMETRY_EVIDENCE_V28,
)

const ownRustCadRuntimeV29 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v29',
  'own-rust-cad-v29',
  RUNTIME_GEOMETRY_EVIDENCE_V29,
)
const brepClosedRuntimeV29 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v29',
  'rust-brep-closed-v29',
  RUNTIME_GEOMETRY_EVIDENCE_V29,
)

const ownRustCadRuntimeV30 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v30',
  'own-rust-cad-v30',
  RUNTIME_GEOMETRY_EVIDENCE_V30,
)
const brepClosedRuntimeV30 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v30',
  'rust-brep-closed-v30',
  RUNTIME_GEOMETRY_EVIDENCE_V30,
)

const ownRustCadRuntimeV31 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v31',
  'own-rust-cad-v31',
  RUNTIME_GEOMETRY_EVIDENCE_V31,
)
const brepClosedRuntimeV31 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v31',
  'rust-brep-closed-v31',
  RUNTIME_GEOMETRY_EVIDENCE_V31,
)

const ownRustCadRuntimeV32 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v32',
  'own-rust-cad-v32',
  RUNTIME_GEOMETRY_EVIDENCE_V32,
)
const brepClosedRuntimeV32 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v32',
  'rust-brep-closed-v32',
  RUNTIME_GEOMETRY_EVIDENCE_V32,
)

const ownRustCadRuntimeV33 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v33',
  'own-rust-cad-v33',
  RUNTIME_GEOMETRY_EVIDENCE_V33,
)
const brepClosedRuntimeV33 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v33',
  'rust-brep-closed-v33',
  RUNTIME_GEOMETRY_EVIDENCE_V33,
)

const ownRustCadRuntimeV34 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v34',
  'own-rust-cad-v34',
  RUNTIME_GEOMETRY_EVIDENCE_V34,
)
const brepClosedRuntimeV34 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v34',
  'rust-brep-closed-v34',
  RUNTIME_GEOMETRY_EVIDENCE_V34,
)

const ownRustCadRuntimeV35 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v35',
  'own-rust-cad-v35',
  RUNTIME_GEOMETRY_EVIDENCE_V35,
)
const brepClosedRuntimeV35 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v35',
  'rust-brep-closed-v35',
  RUNTIME_GEOMETRY_EVIDENCE_V35,
)

const ownRustCadRuntimeV36 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v36',
  'own-rust-cad-v36',
  RUNTIME_GEOMETRY_EVIDENCE_V36,
)
const brepClosedRuntimeV36 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v36',
  'rust-brep-closed-v36',
  RUNTIME_GEOMETRY_EVIDENCE_V36,
)

const ownRustCadRuntimeV37 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v37',
  'own-rust-cad-v37',
  RUNTIME_GEOMETRY_EVIDENCE_V37,
)
const brepClosedRuntimeV37 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v37',
  'rust-brep-closed-v37',
  RUNTIME_GEOMETRY_EVIDENCE_V37,
)

const ownRustCadRuntimeV38 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v38',
  'own-rust-cad-v38',
  RUNTIME_GEOMETRY_EVIDENCE_V38,
)
const brepClosedRuntimeV38 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v38',
  'rust-brep-closed-v38',
  RUNTIME_GEOMETRY_EVIDENCE_V38,
)

const ownRustCadRuntimeV39 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v39',
  'own-rust-cad-v39',
  RUNTIME_GEOMETRY_EVIDENCE_V39,
)
const brepClosedRuntimeV39 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v39',
  'rust-brep-closed-v39',
  RUNTIME_GEOMETRY_EVIDENCE_V39,
)

const ownRustCadRuntimeV40 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v40',
  'own-rust-cad-v40',
  RUNTIME_GEOMETRY_EVIDENCE_V40,
)
const brepClosedRuntimeV40 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v40',
  'rust-brep-closed-v40',
  RUNTIME_GEOMETRY_EVIDENCE_V40,
)

const ownRustCadRuntimeV41 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v41',
  'own-rust-cad-v41',
  RUNTIME_GEOMETRY_EVIDENCE_V41,
)
const brepClosedRuntimeV41 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v41',
  'rust-brep-closed-v41',
  RUNTIME_GEOMETRY_EVIDENCE_V41,
)

const ownRustCadRuntimeV42 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v42',
  'own-rust-cad-v42',
  RUNTIME_GEOMETRY_EVIDENCE_V42,
)
const brepClosedRuntimeV42 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v42',
  'rust-brep-closed-v42',
  RUNTIME_GEOMETRY_EVIDENCE_V42,
)

const ownRustCadRuntimeV43 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v43',
  'own-rust-cad-v43',
  RUNTIME_GEOMETRY_EVIDENCE_V43,
)
const brepClosedRuntimeV43 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v43',
  'rust-brep-closed-v43',
  RUNTIME_GEOMETRY_EVIDENCE_V43,
)

const ownRustCadRuntimeV44 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v44',
  'own-rust-cad-v44',
  RUNTIME_GEOMETRY_EVIDENCE_V44,
)
const brepClosedRuntimeV44 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v44',
  'rust-brep-closed-v44',
  RUNTIME_GEOMETRY_EVIDENCE_V44,
)

const ownRustCadRuntimeV45 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v45',
  'own-rust-cad-v45',
  RUNTIME_GEOMETRY_EVIDENCE_V45,
)
const brepClosedRuntimeV45 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v45',
  'rust-brep-closed-v45',
  RUNTIME_GEOMETRY_EVIDENCE_V45,
)

const ownRustCadRuntimeV46 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v46',
  'own-rust-cad-v46',
  RUNTIME_GEOMETRY_EVIDENCE_V46,
)
const brepClosedRuntimeV46 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v46',
  'rust-brep-closed-v46',
  RUNTIME_GEOMETRY_EVIDENCE_V46,
)

const ownRustCadRuntimeV47 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v47',
  'own-rust-cad-v47',
  RUNTIME_GEOMETRY_EVIDENCE_V47,
)
const brepClosedRuntimeV47 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v47',
  'rust-brep-closed-v47',
  RUNTIME_GEOMETRY_EVIDENCE_V47,
)

const ownRustCadRuntimeV48 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v48',
  'own-rust-cad-v48',
  RUNTIME_GEOMETRY_EVIDENCE_V48,
)
const brepClosedRuntimeV48 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v48',
  'rust-brep-closed-v48',
  RUNTIME_GEOMETRY_EVIDENCE_V48,
)

const ownRustCadRuntimeV49 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v49',
  'own-rust-cad-v49',
  RUNTIME_GEOMETRY_EVIDENCE_V49,
)
const brepClosedRuntimeV49 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v49',
  'rust-brep-closed-v49',
  RUNTIME_GEOMETRY_EVIDENCE_V49,
)

const ownRustCadRuntimeV50 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v50',
  'own-rust-cad-v50',
  RUNTIME_GEOMETRY_EVIDENCE_V50,
)
const brepClosedRuntimeV50 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v50',
  'rust-brep-closed-v50',
  RUNTIME_GEOMETRY_EVIDENCE_V50,
)

const ownRustCadRuntimeV51 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v51',
  'own-rust-cad-v51',
  RUNTIME_GEOMETRY_EVIDENCE_V51,
)
const brepClosedRuntimeV51 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v51',
  'rust-brep-closed-v51',
  RUNTIME_GEOMETRY_EVIDENCE_V51,
)

const ownRustCadRuntimeV52 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v52',
  'own-rust-cad-v52',
  RUNTIME_GEOMETRY_EVIDENCE_V52,
)
const brepClosedRuntimeV52 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v52',
  'rust-brep-closed-v52',
  RUNTIME_GEOMETRY_EVIDENCE_V52,
)

const ownRustCadRuntimeV53 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v53',
  'own-rust-cad-v53',
  RUNTIME_GEOMETRY_EVIDENCE_V53,
)
const brepClosedRuntimeV53 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v53',
  'rust-brep-closed-v53',
  RUNTIME_GEOMETRY_EVIDENCE_V53,
)

const ownRustCadRuntimeV54 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v54',
  'own-rust-cad-v54',
  RUNTIME_GEOMETRY_EVIDENCE_V54,
)
const brepClosedRuntimeV54 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v54',
  'rust-brep-closed-v54',
  RUNTIME_GEOMETRY_EVIDENCE_V54,
)

const ownRustCadRuntimeV55 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v55',
  'own-rust-cad-v55',
  RUNTIME_GEOMETRY_EVIDENCE_V55,
)
const brepClosedRuntimeV55 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v55',
  'rust-brep-closed-v55',
  RUNTIME_GEOMETRY_EVIDENCE_V55,
)

const ownRustCadRuntimeV56 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v56',
  'own-rust-cad-v56',
  RUNTIME_GEOMETRY_EVIDENCE_V56,
)
const brepClosedRuntimeV56 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v56',
  'rust-brep-closed-v56',
  RUNTIME_GEOMETRY_EVIDENCE_V56,
)

const ownRustCadRuntimeV57 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v57',
  'own-rust-cad-v57',
  RUNTIME_GEOMETRY_EVIDENCE_V57,
)
const brepClosedRuntimeV57 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v57',
  'rust-brep-closed-v57',
  RUNTIME_GEOMETRY_EVIDENCE_V57,
)

const ownRustCadRuntimeV58 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v58',
  'own-rust-cad-v58',
  RUNTIME_GEOMETRY_EVIDENCE_V58,
)
const brepClosedRuntimeV58 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v58',
  'rust-brep-closed-v58',
  RUNTIME_GEOMETRY_EVIDENCE_V58,
)

const ownRustCadRuntimeV59 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v59',
  'own-rust-cad-v59',
  RUNTIME_GEOMETRY_EVIDENCE_V59,
)
const brepClosedRuntimeV59 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v59',
  'rust-brep-closed-v59',
  RUNTIME_GEOMETRY_EVIDENCE_V59,
)

const ownRustCadRuntimeV60 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v60',
  'own-rust-cad-v60',
  RUNTIME_GEOMETRY_EVIDENCE_V60,
)
const brepClosedRuntimeV60 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v60',
  'rust-brep-closed-v60',
  RUNTIME_GEOMETRY_EVIDENCE_V60,
)

const ownRustCadRuntimeV61 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v61',
  'own-rust-cad-v61',
  RUNTIME_GEOMETRY_EVIDENCE_V61,
)
const brepClosedRuntimeV61 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v61',
  'rust-brep-closed-v61',
  RUNTIME_GEOMETRY_EVIDENCE_V61,
)

const ownRustCadRuntimeV62 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v62',
  'own-rust-cad-v62',
  RUNTIME_GEOMETRY_EVIDENCE_V62,
)
const brepClosedRuntimeV62 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v62',
  'rust-brep-closed-v62',
  RUNTIME_GEOMETRY_EVIDENCE_V62,
)

const ownRustCadRuntimeV63 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v63',
  'own-rust-cad-v63',
  RUNTIME_GEOMETRY_EVIDENCE_V63,
)
const brepClosedRuntimeV63 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v63',
  'rust-brep-closed-v63',
  RUNTIME_GEOMETRY_EVIDENCE_V63,
)

const ownRustCadRuntimeV64 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v64',
  'own-rust-cad-v64',
  RUNTIME_GEOMETRY_EVIDENCE_V64,
)
const brepClosedRuntimeV64 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v64',
  'rust-brep-closed-v64',
  RUNTIME_GEOMETRY_EVIDENCE_V64,
)

const ownRustCadRuntimeV65 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v65',
  'own-rust-cad-v65',
  RUNTIME_GEOMETRY_EVIDENCE_V65,
)
const brepClosedRuntimeV65 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v65',
  'rust-brep-closed-v65',
  RUNTIME_GEOMETRY_EVIDENCE_V65,
)

const ownRustCadRuntimeV66 = bindCurrentRuntime(
  ownRustCadManifestV1,
  'own-rust-node-v66',
  'own-rust-cad-v66',
  CURRENT_RUNTIME_GEOMETRY_EVIDENCE,
)
const brepClosedRuntimeV66 = bindCurrentRuntime(
  brepClosedManifestV1,
  'brep-closed-v66',
  'rust-brep-closed-v66',
  CURRENT_RUNTIME_GEOMETRY_EVIDENCE,
)

export const GEOMETRY_MANIFEST_ARCHIVE = deepFreeze({
  'own-rust-node-v66': ownRustCadRuntimeV66,
  'brep-closed-v66': brepClosedRuntimeV66,
  'own-rust-node-v65': ownRustCadRuntimeV65,
  'brep-closed-v65': brepClosedRuntimeV65,
  'own-rust-node-v64': ownRustCadRuntimeV64,
  'brep-closed-v64': brepClosedRuntimeV64,
  'own-rust-node-v63': ownRustCadRuntimeV63,
  'brep-closed-v63': brepClosedRuntimeV63,
  'own-rust-node-v62': ownRustCadRuntimeV62,
  'brep-closed-v62': brepClosedRuntimeV62,
  'own-rust-node-v61': ownRustCadRuntimeV61,
  'brep-closed-v61': brepClosedRuntimeV61,
  'own-rust-node-v60': ownRustCadRuntimeV60,
  'brep-closed-v60': brepClosedRuntimeV60,
  'own-rust-node-v59': ownRustCadRuntimeV59,
  'brep-closed-v59': brepClosedRuntimeV59,
  'own-rust-node-v58': ownRustCadRuntimeV58,
  'brep-closed-v58': brepClosedRuntimeV58,
  'own-rust-node-v57': ownRustCadRuntimeV57,
  'brep-closed-v57': brepClosedRuntimeV57,
  'own-rust-node-v56': ownRustCadRuntimeV56,
  'brep-closed-v56': brepClosedRuntimeV56,
  'own-rust-node-v55': ownRustCadRuntimeV55,
  'brep-closed-v55': brepClosedRuntimeV55,
  'own-rust-node-v54': ownRustCadRuntimeV54,
  'brep-closed-v54': brepClosedRuntimeV54,
  'own-rust-node-v53': ownRustCadRuntimeV53,
  'brep-closed-v53': brepClosedRuntimeV53,
  'own-rust-node-v52': ownRustCadRuntimeV52,
  'brep-closed-v52': brepClosedRuntimeV52,
  'own-rust-node-v51': ownRustCadRuntimeV51,
  'brep-closed-v51': brepClosedRuntimeV51,
  'own-rust-node-v50': ownRustCadRuntimeV50,
  'brep-closed-v50': brepClosedRuntimeV50,
  'own-rust-node-v49': ownRustCadRuntimeV49,
  'brep-closed-v49': brepClosedRuntimeV49,
  'own-rust-node-v48': ownRustCadRuntimeV48,
  'brep-closed-v48': brepClosedRuntimeV48,
  'own-rust-node-v47': ownRustCadRuntimeV47,
  'brep-closed-v47': brepClosedRuntimeV47,
  'own-rust-node-v46': ownRustCadRuntimeV46,
  'brep-closed-v46': brepClosedRuntimeV46,
  'own-rust-node-v45': ownRustCadRuntimeV45,
  'brep-closed-v45': brepClosedRuntimeV45,
  'own-rust-node-v44': ownRustCadRuntimeV44,
  'brep-closed-v44': brepClosedRuntimeV44,
  'own-rust-node-v43': ownRustCadRuntimeV43,
  'brep-closed-v43': brepClosedRuntimeV43,
  'own-rust-node-v42': ownRustCadRuntimeV42,
  'brep-closed-v42': brepClosedRuntimeV42,
  'own-rust-node-v41': ownRustCadRuntimeV41,
  'brep-closed-v41': brepClosedRuntimeV41,
  'own-rust-node-v40': ownRustCadRuntimeV40,
  'brep-closed-v40': brepClosedRuntimeV40,
  'own-rust-node-v39': ownRustCadRuntimeV39,
  'brep-closed-v39': brepClosedRuntimeV39,
  'own-rust-node-v38': ownRustCadRuntimeV38,
  'brep-closed-v38': brepClosedRuntimeV38,
  'own-rust-node-v37': ownRustCadRuntimeV37,
  'brep-closed-v37': brepClosedRuntimeV37,
  'own-rust-node-v36': ownRustCadRuntimeV36,
  'brep-closed-v36': brepClosedRuntimeV36,
  'own-rust-node-v35': ownRustCadRuntimeV35,
  'brep-closed-v35': brepClosedRuntimeV35,
  'own-rust-node-v34': ownRustCadRuntimeV34,
  'brep-closed-v34': brepClosedRuntimeV34,
  'own-rust-node-v33': ownRustCadRuntimeV33,
  'brep-closed-v33': brepClosedRuntimeV33,
  'own-rust-node-v32': ownRustCadRuntimeV32,
  'brep-closed-v32': brepClosedRuntimeV32,
  'own-rust-node-v31': ownRustCadRuntimeV31,
  'brep-closed-v31': brepClosedRuntimeV31,
  'own-rust-node-v30': ownRustCadRuntimeV30,
  'brep-closed-v30': brepClosedRuntimeV30,
  'own-rust-node-v29': ownRustCadRuntimeV29,
  'brep-closed-v29': brepClosedRuntimeV29,
  'own-rust-node-v28': ownRustCadRuntimeV28,
  'brep-closed-v28': brepClosedRuntimeV28,
  'own-rust-node-v27': ownRustCadRuntimeV27,
  'brep-closed-v27': brepClosedRuntimeV27,
  'own-rust-node-v26': ownRustCadRuntimeV26,
  'brep-closed-v26': brepClosedRuntimeV26,
  'own-rust-node-v25': ownRustCadRuntimeV25,
  'brep-closed-v25': brepClosedRuntimeV25,
  'own-rust-node-v24': ownRustCadRuntimeV24,
  'brep-closed-v24': brepClosedRuntimeV24,
  'own-rust-node-v23': ownRustCadRuntimeV23,
  'brep-closed-v23': brepClosedRuntimeV23,
  'own-rust-node-v22': ownRustCadRuntimeV22,
  'brep-closed-v22': brepClosedRuntimeV22,
  'own-rust-node-v21': ownRustCadRuntimeV21,
  'brep-closed-v21': brepClosedRuntimeV21,
  'own-rust-node-v20': ownRustCadRuntimeV20,
  'brep-closed-v20': brepClosedRuntimeV20,
  'own-rust-node-v19': ownRustCadRuntimeV19,
  'brep-closed-v19': brepClosedRuntimeV19,
  'own-rust-node-v18': ownRustCadRuntimeV18,
  'brep-closed-v18': brepClosedRuntimeV18,
  'own-rust-node-v17': ownRustCadRuntimeV17,
  'brep-closed-v17': brepClosedRuntimeV17,
  'own-rust-node-v16': ownRustCadRuntimeV16,
  'brep-closed-v16': brepClosedRuntimeV16,
  'own-rust-node-v15': ownRustCadRuntimeV15,
  'brep-closed-v15': brepClosedRuntimeV15,
  'own-rust-node-v14': ownRustCadRuntimeV14,
  'brep-closed-v14': brepClosedRuntimeV14,
  'own-rust-node-v13': ownRustCadRuntimeV13,
  'brep-closed-v13': brepClosedRuntimeV13,
  'own-rust-node-v12': ownRustCadRuntimeV12,
  'brep-closed-v12': brepClosedRuntimeV12,
  'own-rust-node-v11': ownRustCadRuntimeV11,
  'brep-closed-v11': brepClosedRuntimeV11,
  'own-rust-node-v10': ownRustCadRuntimeV10,
  'brep-closed-v10': brepClosedRuntimeV10,
  'own-rust-node-v9': ownRustCadRuntimeV9,
  'brep-closed-v9': brepClosedRuntimeV9,
  'own-rust-node-v8': ownRustCadRuntimeV8,
  'brep-closed-v8': brepClosedRuntimeV8,
  'own-rust-node-v7': ownRustCadRuntimeV7,
  'brep-closed-v7': brepClosedRuntimeV7,
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
  mesh: 'own-rust-node-v66',
  brep: 'brep-closed-v66',
} as const)

export type GeometryProviderAdmissionMode = 'legacy-grandfathered' | 'qualified' | 'denied'

export const GEOMETRY_PROVIDER_ADMISSION_REASON_CODES = Object.freeze([
  'manifest-integrity-failed',
  'legacy-exact-exception',
  'qualification-incomplete',
  'runtime-not-deployable',
  'dependency-attestation-incomplete',
  'qualified',
] as const)

export type GeometryProviderAdmissionReasonCode = (typeof GEOMETRY_PROVIDER_ADMISSION_REASON_CODES)[number]

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
  // Importing a diagnostic API must not instantiate a production provider.
  // Every actual route still performs the same artifact-bound admission.
  get mesh() {
    return qualifiedRuntimeManifestVersion('mesh')
  },
  get brep() {
    return qualifiedRuntimeManifestVersion('brep')
  },
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
