import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import {
  type GeometryBuildPurpose,
  type GeometryEngineClass,
  type GeometryEngineManifest,
  type GeometryEngineRegistrySnapshot,
  type GeometryExecutionDescriptor,
  ACTIVE_GEOMETRY_MANIFEST_VERSIONS,
  freezeGeometryExecutionDescriptor,
  GEOMETRY_ENGINE_ROUTES,
  GEOMETRY_MANIFEST_ARCHIVE,
  geometryProviderAdmissionForManifest,
  GeometryLanguageContractError,
  planGeometrySourceExecution,
} from '../core/geometryExecution'
import { parseOpenSCAD, warmGeometryKernel } from './openscadParser'
import { buildBrepSemanticScene } from './brepSemanticScene'
import { lowerOpenSCADToSemanticProgram } from './semanticProgramLowerer'
import { sha256Hex } from '../core/sha256'

const EXECUTION_BY_ERROR = new WeakMap<object, GeometryExecutionDescriptor>()
const PROVIDER_READINESS_TIMEOUT_MS = 250

export { GeometryLanguageContractError } from '../core/geometryExecution'

export function attachGeometryExecutionToError(
  error: unknown,
  execution: GeometryExecutionDescriptor,
): void {
  if (error !== null && (typeof error === 'object' || typeof error === 'function')
    && !EXECUTION_BY_ERROR.has(error)) {
    EXECUTION_BY_ERROR.set(error, execution)
  }
}

export function geometryExecutionForError(error: unknown): GeometryExecutionDescriptor | undefined {
  return error !== null && (typeof error === 'object' || typeof error === 'function')
    ? EXECUTION_BY_ERROR.get(error)
    : undefined
}

const MESH_MANIFEST: GeometryEngineManifest = Object.freeze({
  ...GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS.mesh],
  availability: 'available',
  unavailableReason: null,
} satisfies GeometryEngineManifest)

const BREP_MANIFEST: GeometryEngineManifest = Object.freeze({
  ...GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS.brep],
  availability: 'available',
  unavailableReason: null,
} satisfies GeometryEngineManifest)

const MANIFESTS: Readonly<Record<GeometryEngineClass, GeometryEngineManifest>> = Object.freeze({
  mesh: MESH_MANIFEST,
  brep: BREP_MANIFEST,
})

export interface GeometryBuildRequest {
  quality: GeometryQuality
  purpose: GeometryBuildPurpose
}

export interface GeometryBuildControl {
  shouldAbort?: () => boolean
  onYield?: () => void
}

export interface GeometryBuildResult {
  result: GeometryEvaluationResult
  execution: GeometryExecutionDescriptor
}

/** Runtime provider for one qualified, source-routed geometry engine class. */
export interface GeometryBackendProvider {
  readonly engineClass: GeometryEngineClass
  readonly engineKey: string
  readonly kernelFingerprint: string
  readonly capabilityManifestVersion: string
  warm(): Promise<void>
  build(
    source: string,
    request: GeometryBuildRequest,
    control?: GeometryBuildControl,
  ): Promise<GeometryEvaluationResult>
}

class MeshBackendProvider implements GeometryBackendProvider {
  readonly engineClass = 'mesh' as const
  readonly engineKey = MESH_MANIFEST.engineKey
  readonly kernelFingerprint = MESH_MANIFEST.kernelFingerprint
  readonly capabilityManifestVersion = MESH_MANIFEST.capabilityManifestVersion

  async warm(): Promise<void> {
    await warmGeometryKernel()
  }

  build(
    source: string,
    request: GeometryBuildRequest,
    control: GeometryBuildControl = {},
  ): Promise<GeometryEvaluationResult> {
    return parseOpenSCAD(source, {
      quality: request.quality,
      shouldAbort: control.shouldAbort,
      onYield: control.onYield,
    })
  }
}

/** Closed-matrix B-rep provider: same WASM kernel, SemanticProgram-required path.
 * Manifold is a peer engine only — never a silent fallback from this provider (see
 * docs/design/manifold-keep-as-peer-adr.md). */
class BrepBackendProvider implements GeometryBackendProvider {
  readonly engineClass = 'brep' as const
  readonly engineKey = BREP_MANIFEST.engineKey
  readonly kernelFingerprint = BREP_MANIFEST.kernelFingerprint
  readonly capabilityManifestVersion = BREP_MANIFEST.capabilityManifestVersion

  async warm(): Promise<void> {
    await warmGeometryKernel()
  }

  async build(
    source: string,
    request: GeometryBuildRequest,
    control: GeometryBuildControl = {},
  ): Promise<GeometryEvaluationResult> {
    const lowered = lowerOpenSCADToSemanticProgram(source, {
      quality: request.quality,
      shouldAbort: control.shouldAbort,
    })
    control.onYield?.()
    const scene = await buildBrepSemanticScene(
      lowered,
      { quality: request.quality, segments: request.quality === 'preview' ? 8 : 16 },
      { shouldAbort: control.shouldAbort },
    )
    control.onYield?.()
    return scene.result
  }
}

export class GeometryEngineUnavailableError extends Error {
  constructor(
    readonly execution: GeometryExecutionDescriptor,
    readonly reason: string,
    readonly availabilityCause: GeometryEngineAvailabilityCause = 'not-deployed',
  ) {
    super(`Geometry engine ${execution.engineClass} is unavailable: ${reason}`)
    this.name = 'GeometryEngineUnavailableError'
  }
}

export type GeometryEngineAvailabilityCause =
  | 'not-deployed'
  | 'provider-missing'
  | 'readiness-timeout'
  | 'readiness-failed'
  | 'revoked'
  | 'quarantined'

export class GeometryCapabilityUnavailableError extends Error {
  constructor(
    readonly execution: GeometryExecutionDescriptor,
    readonly missingCapabilities: string[],
  ) {
    super(`Geometry engine ${execution.engineClass} does not provide required capabilities: ${missingCapabilities.join(', ')}`)
    this.name = 'GeometryCapabilityUnavailableError'
  }
}

export class GeometryProviderContractError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'GeometryProviderContractError'
  }
}

export interface GeometryEngineRuntimePolicy {
  /** Immutable manifest digests denied by the embedding process at startup. */
  readonly revokedManifestDigests?: readonly string[]
  /** Optional live policy source; records are structurally attested by the host. */
  readonly revocations?: GeometryManifestRevocationRegistry
}

export interface GeometryManifestRevocationRecord {
  readonly schemaVersion: 1
  readonly epoch: number
  readonly manifestDigest: string
  readonly reason: string
  readonly effectiveAt: string
  readonly authority: string
  readonly attestationSha256: string
}

export interface GeometryManifestRevocationSnapshot {
  readonly epoch: number
  readonly revokedManifestDigests: readonly string[]
}

function revocationAttestationPreimage(
  record: Omit<GeometryManifestRevocationRecord, 'attestationSha256'>,
): string {
  return [
    'geometry-manifest-revocation-v1',
    record.schemaVersion,
    record.epoch,
    record.manifestDigest,
    record.reason,
    record.effectiveAt,
    record.authority,
  ].join('\0')
}

export function createGeometryManifestRevocationRecord(
  record: Omit<GeometryManifestRevocationRecord, 'schemaVersion' | 'attestationSha256'>,
): GeometryManifestRevocationRecord {
  const unsigned = { schemaVersion: 1 as const, ...record }
  return Object.freeze({
    ...unsigned,
    attestationSha256: sha256Hex(revocationAttestationPreimage(unsigned)),
  })
}

/**
 * Process-local monotonic policy. Remote records must be authenticated by the
 * embedding host before they are passed here; the attestation protects exact
 * record integrity after that trust boundary.
 */
export class GeometryManifestRevocationRegistry {
  private epoch = 0
  private readonly revoked = new Set<string>()
  private readonly records: GeometryManifestRevocationRecord[] = []

  constructor(records: readonly GeometryManifestRevocationRecord[] = []) {
    records.forEach(record => this.apply(record))
  }

  apply(record: GeometryManifestRevocationRecord): void {
    if (record.schemaVersion !== 1
      || !Number.isSafeInteger(record.epoch)
      || record.epoch !== this.epoch + 1) {
      throw new TypeError('Geometry revocation epochs must be consecutive positive integers')
    }
    if (!/^[a-f0-9]{64}$/.test(record.manifestDigest)
      || !record.reason.trim() || record.reason.length > 512
      || !record.authority.trim() || record.authority.length > 128
      || !Number.isFinite(Date.parse(record.effectiveAt))) {
      throw new TypeError('Geometry revocation record is malformed')
    }
    const expected = sha256Hex(revocationAttestationPreimage({
      schemaVersion: 1,
      epoch: record.epoch,
      manifestDigest: record.manifestDigest,
      reason: record.reason,
      effectiveAt: record.effectiveAt,
      authority: record.authority,
    }))
    if (record.attestationSha256 !== expected) {
      throw new TypeError('Geometry revocation record attestation does not match its fields')
    }
    this.epoch = record.epoch
    this.revoked.add(record.manifestDigest)
    this.records.push(Object.freeze({ ...record }))
  }

  snapshot(): GeometryManifestRevocationSnapshot {
    return Object.freeze({
      epoch: this.epoch,
      revokedManifestDigests: Object.freeze([...this.revoked].sort()),
    })
  }

  history(): readonly GeometryManifestRevocationRecord[] {
    return Object.freeze([...this.records])
  }
}

function cloneManifest(manifest: GeometryEngineManifest): GeometryEngineManifest {
  return {
    ...manifest,
    languageContracts: [...manifest.languageContracts],
    capabilities: [...manifest.capabilities],
    plannedCapabilities: [...manifest.plannedCapabilities],
    qualities: [...manifest.qualities],
    representations: [...manifest.representations],
    plannedRepresentations: [...manifest.plannedRepresentations],
    exportFormats: [...manifest.exportFormats],
    plannedExportFormats: [...manifest.plannedExportFormats],
    limits: { ...manifest.limits },
    qualification: { ...manifest.qualification },
    dependency: { ...manifest.dependency },
    rollbackCompatibility: { ...manifest.rollbackCompatibility },
  }
}

/**
 * Source-routed geometry facade. Both engine classes are permanent registry
 * entries, while only qualified providers are executable. Resolution never
 * retries or falls back across engine classes.
 */
export class GeometryBuildEngine {
  private readonly providers: ReadonlyMap<GeometryEngineClass, GeometryBackendProvider>
  private readonly readiness = new Map<GeometryEngineClass, {
    state: 'unknown' | 'available' | 'unavailable' | 'quarantined'
    reason: string | null
    cause: GeometryEngineAvailabilityCause | null
  }>()
  private readonly readinessAttempts = new Map<GeometryEngineClass, Promise<boolean>>()
  private readonly revokedManifestDigests: ReadonlySet<string>
  private readonly revocations: GeometryManifestRevocationRegistry | null

  constructor(
    providers: readonly GeometryBackendProvider[] = [new MeshBackendProvider(), new BrepBackendProvider()],
    policy: GeometryEngineRuntimePolicy = {},
  ) {
    const revokedManifestDigests = new Set(policy.revokedManifestDigests ?? [])
    for (const digest of revokedManifestDigests) {
      if (!/^[a-f0-9]{64}$/.test(digest)) {
        throw new TypeError('Revoked geometry manifest digests must be lowercase SHA-256 values')
      }
    }
    this.revokedManifestDigests = revokedManifestDigests
    this.revocations = policy.revocations ?? null
    const entries = new Map<GeometryEngineClass, GeometryBackendProvider>()
    for (const provider of providers) {
      const manifest = MANIFESTS[provider.engineClass]
      const archivedManifest = GEOMETRY_MANIFEST_ARCHIVE[
        ACTIVE_GEOMETRY_MANIFEST_VERSIONS[provider.engineClass]
      ]
      const admission = geometryProviderAdmissionForManifest(archivedManifest)
      if (!admission.allowed || manifest.availability !== 'available') {
        throw new TypeError(
          `Cannot register an unqualified geometry provider for ${provider.engineClass}: ${admission.reason}`,
        )
      }
      if (provider.engineKey !== manifest.engineKey
        || provider.kernelFingerprint !== manifest.kernelFingerprint
        || provider.capabilityManifestVersion !== manifest.capabilityManifestVersion) {
        throw new TypeError(
          `Cannot register an unqualified geometry provider: identity does not match the qualified ${provider.engineClass} manifest`,
        )
      }
      if (entries.has(provider.engineClass)) {
        throw new TypeError(`Duplicate geometry provider for ${provider.engineClass}`)
      }
      entries.set(provider.engineClass, provider)
      this.readiness.set(provider.engineClass, { state: 'unknown', reason: null, cause: null })
    }
    this.providers = entries
  }

  private runtimeManifest(engineClass: GeometryEngineClass): GeometryEngineManifest {
    const manifest = cloneManifest(MANIFESTS[engineClass])
    if (this.isManifestRevoked(manifest.manifestDigest)) {
      return {
        ...manifest,
        availability: 'unavailable',
        unavailableReason: 'The selected immutable geometry manifest is revoked by runtime policy.',
      }
    }
    const readiness = this.readiness.get(engineClass)
    if (manifest.availability === 'available'
      && (!this.providers.has(engineClass) || readiness?.state !== 'available')) {
      return {
        ...manifest,
        availability: 'unavailable',
        unavailableReason: readiness?.reason
          ?? (this.providers.has(engineClass)
            ? 'The qualified provider has not passed its runtime readiness check.'
            : 'No qualified runtime provider is registered in this process.'),
      }
    }
    return manifest
  }

  private async ensureProviderReady(
    engineClass: GeometryEngineClass,
  ): Promise<GeometryBackendProvider | null> {
    if (this.isManifestRevoked(MANIFESTS[engineClass].manifestDigest)) {
      this.readiness.set(engineClass, {
        state: 'unavailable',
        reason: 'The selected immutable geometry manifest is revoked by runtime policy.',
        cause: 'revoked',
      })
      return null
    }
    const provider = this.providers.get(engineClass)
    if (!provider) return null
    const readiness = this.readiness.get(engineClass)
    if (readiness?.state === 'available') return provider
    if (readiness?.state === 'quarantined') return null
    const pending = this.readinessAttempts.get(engineClass)
    if (readiness?.state === 'unavailable') return null

    let attempt = pending
    if (!attempt) {
      attempt = Promise.resolve()
        .then(() => provider.warm())
        .then(() => {
          if (this.readiness.get(engineClass)?.state !== 'quarantined') {
            this.readiness.set(engineClass, { state: 'available', reason: null, cause: null })
          }
          return true
        }, () => {
          if (this.readiness.get(engineClass)?.state !== 'quarantined') {
            this.readiness.set(engineClass, {
              state: 'unavailable',
              reason: 'The qualified provider failed its runtime readiness check.',
              cause: 'readiness-failed',
            })
          }
          return false
        })
        .finally(() => {
          if (this.readinessAttempts.get(engineClass) === attempt) {
            this.readinessAttempts.delete(engineClass)
          }
        })
      this.readinessAttempts.set(engineClass, attempt)
    }

    let timeoutId: ReturnType<typeof setTimeout> | undefined
    const outcome = await Promise.race([
      attempt.then(available => available ? 'available' as const : 'unavailable' as const),
      new Promise<'timeout'>(resolve => {
        timeoutId = setTimeout(() => resolve('timeout'), PROVIDER_READINESS_TIMEOUT_MS)
      }),
    ])
    if (timeoutId !== undefined) clearTimeout(timeoutId)
    if (outcome === 'available' && this.readiness.get(engineClass)?.state === 'available') {
      return provider
    }
    if (outcome === 'timeout') {
      this.readiness.set(engineClass, {
        state: 'unavailable',
        reason: `The qualified provider readiness check exceeded ${PROVIDER_READINESS_TIMEOUT_MS} ms.`,
        cause: 'readiness-timeout',
      })
    }
    return null
  }

  private quarantineProvider(engineClass: GeometryEngineClass, reason: string): void {
    this.readiness.set(engineClass, { state: 'quarantined', reason, cause: 'quarantined' })
  }

  private isManifestRevoked(manifestDigest: string): boolean {
    return this.revokedManifestDigests.has(manifestDigest)
      || (this.revocations?.snapshot().revokedManifestDigests.includes(manifestDigest) ?? false)
  }

  private policyEpoch(): number {
    return this.revocations?.snapshot().epoch ?? 0
  }

  async capabilities(): Promise<GeometryEngineRegistrySnapshot> {
    await Promise.all([...this.providers.keys()].map(engineClass => (
      this.ensureProviderReady(engineClass)
    )))
    return {
      contractVersion: 1,
      sourceDirectedRouting: true,
      automaticFallback: false,
      routes: GEOMETRY_ENGINE_ROUTES.map(route => ({ ...route })),
      engines: [this.runtimeManifest('mesh'), this.runtimeManifest('brep')],
    }
  }

  async manifest(engineClass: GeometryEngineClass): Promise<GeometryEngineManifest> {
    await this.ensureProviderReady(engineClass)
    return this.runtimeManifest(engineClass)
  }

  planSource(source: string, request: GeometryBuildRequest): GeometryExecutionDescriptor {
    return planGeometrySourceExecution(source, request)
  }

  async buildSource(
    source: string,
    request: GeometryBuildRequest,
    control: GeometryBuildControl = {},
  ): Promise<GeometryBuildResult> {
    const execution = this.planSource(source, request)
    const manifest = MANIFESTS[execution.engineClass]
    const missingCapabilities = execution.requiredCapabilities.filter(capability => (
      !manifest.capabilities.includes(capability)
    ))
    if (missingCapabilities.length) {
      throw new GeometryCapabilityUnavailableError(execution, missingCapabilities)
    }
    if (this.isManifestRevoked(manifest.manifestDigest)) {
      throw new GeometryEngineUnavailableError(
        execution,
        'The selected immutable geometry manifest is revoked by runtime policy.',
        'revoked',
      )
    }
    const registeredProvider = this.providers.get(execution.engineClass)
    if (manifest.availability !== 'available' || !registeredProvider) {
      throw new GeometryEngineUnavailableError(
        execution,
        manifest.unavailableReason ?? 'No qualified runtime provider is registered.',
        manifest.availability !== 'available' ? 'not-deployed' : 'provider-missing',
      )
    }
    const provider = await this.ensureProviderReady(execution.engineClass)
    if (!provider) {
      throw new GeometryEngineUnavailableError(
        execution,
        this.runtimeManifest(execution.engineClass).unavailableReason
          ?? 'The qualified provider failed its runtime readiness check.',
        this.readiness.get(execution.engineClass)?.cause ?? 'readiness-failed',
      )
    }
    const policyEpoch = this.policyEpoch()
    if (this.isManifestRevoked(manifest.manifestDigest)) {
      throw new GeometryEngineUnavailableError(
        execution,
        'The selected immutable geometry manifest is revoked by runtime policy.',
        'revoked',
      )
    }
    const runtimeExecution = freezeGeometryExecutionDescriptor({
      ...execution,
      evidence: 'runtime',
    } satisfies GeometryExecutionDescriptor)
    let result: GeometryEvaluationResult
    try {
      result = await provider.build(source, request, control)
      if (result.quality !== request.quality) {
        this.quarantineProvider(
          execution.engineClass,
          'The qualified provider was quarantined after violating its result contract.',
        )
        throw new GeometryProviderContractError(
          `Geometry provider returned ${result.quality} quality for a ${request.quality} request`,
        )
      }
    } catch (error) {
      if (error instanceof GeometryEngineUnavailableError
        || error instanceof GeometryCapabilityUnavailableError
        || error instanceof GeometryLanguageContractError) {
        this.quarantineProvider(
          execution.engineClass,
          'The qualified provider was quarantined after throwing a facade-owned routing error.',
        )
        const contractError = new GeometryProviderContractError(
          'Geometry provider threw a facade-owned routing error',
        )
        attachGeometryExecutionToError(contractError, runtimeExecution)
        throw contractError
      }
      if (error === null || (typeof error !== 'object' && typeof error !== 'function')) {
        this.quarantineProvider(
          execution.engineClass,
          'The qualified provider was quarantined after rejecting with a non-object error.',
        )
        const contractError = new GeometryProviderContractError(
          'Geometry provider rejected with a non-object error',
        )
        attachGeometryExecutionToError(contractError, runtimeExecution)
        throw contractError
      }
      if (geometryExecutionForError(error) !== undefined) {
        this.quarantineProvider(
          execution.engineClass,
          'The qualified provider was quarantined after reusing an error across executions.',
        )
        const contractError = new GeometryProviderContractError(
          'Geometry provider reused an error object across executions',
        )
        attachGeometryExecutionToError(contractError, runtimeExecution)
        throw contractError
      }
      attachGeometryExecutionToError(error, runtimeExecution)
      throw error
    }
    const publicationReadiness = this.readiness.get(execution.engineClass)
    if (this.policyEpoch() !== policyEpoch
      || this.isManifestRevoked(manifest.manifestDigest)
      || publicationReadiness?.state !== 'available') {
      throw new GeometryEngineUnavailableError(
        runtimeExecution,
        publicationReadiness?.state === 'quarantined'
          ? 'The selected geometry provider was quarantined before result publication.'
          : 'Runtime policy changed before geometry result publication.',
        publicationReadiness?.state === 'quarantined' ? 'quarantined' : 'revoked',
      )
    }
    return { result, execution: runtimeExecution }
  }
}

/**
 * Evaluates source with exact-solid recording, for the Solid workspace.
 *
 * The Solid worker records and builds the graph within its own realm.
 * Callers must not reach the parser
 * directly: keeping every parser consumer behind this facade is what the architecture
 * test in tests/coreMesh.test.ts pins.
 */
export function evaluateExactSolids(source: string): Promise<GeometryEvaluationResult> {
  return parseOpenSCAD(source, { recordExactSolids: true })
}

export const defaultGeometryBuildEngine = new GeometryBuildEngine()
