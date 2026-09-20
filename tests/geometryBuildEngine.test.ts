import { GEOMETRY_MANIFEST_ARCHIVE as CAD_MANIFESTS } from '../src/core/geometryExecution'
import { describe, expect, it, vi } from 'vitest'
import {
  createGeometryManifestRevocationRecord,
  geometryExecutionForError,
  GeometryBuildEngine,
  GeometryCapabilityUnavailableError,
  GeometryEngineUnavailableError,
  GeometryLanguageContractError,
  GeometryProviderContractError,
  GeometryManifestRevocationRegistry,
  type GeometryBackendProvider,
} from '../src/services/geometryBuildEngine'
import {
  computeGeometryManifestDigest,
  GEOMETRY_MANIFEST_ARCHIVE,
  geometryProviderAdmissionForManifest,
  type GeometryEngineStaticManifest,
} from '../src/core/geometryExecution'

const request = { quality: 'full', purpose: 'analysis' } as const

describe('GeometryBuildEngine', () => {
  it('publishes both initialized permanent source-routed engines with fallback disabled', async () => {
    const engine = new GeometryBuildEngine()
    // Cold compilation has a separate deadline; readiness still has its 250 ms bound.
    await engine.initializeSource('cube(1);', request)
    await engine.initializeSource('// @language openscad-viewer/brep-1\ncube(1);', request)
    const registry = await engine.capabilities()

    expect(registry).toMatchObject({
      contractVersion: 1,
      sourceDirectedRouting: true,
      automaticFallback: false,
      routes: [
        { languageContract: 'legacy/current', engineClass: 'mesh', fallback: 'never' },
        { languageContract: 'openscad-viewer/brep-1', engineClass: 'brep', fallback: 'never' },
      ],
    })
    expect(registry.engines.map(engine => ({
      engineClass: engine.engineClass,
      permanent: engine.permanent,
      availability: engine.availability,
      isolation: engine.isolation,
    }))).toEqual([
      {
        engineClass: 'mesh',
        permanent: true,
        availability: 'available',
        isolation: 'in-process-serialized',
      },
      {
        engineClass: 'brep',
        permanent: true,
        availability: 'available',
        isolation: 'in-process-serialized',
      },
    ])
  })

  it('reports process-local provider availability and rejects an unqualified B-rep provider', async () => {
    expect((await new GeometryBuildEngine([]).capabilities()).engines[0]).toMatchObject({
      engineClass: 'mesh',
      availability: 'unavailable',
      unavailableReason: expect.stringContaining('No qualified runtime provider'),
    })
    expect(() => new GeometryBuildEngine([{
      engineClass: 'brep',
      engineKey: 'rust-brep-reserved-v1',
      kernelFingerprint: 'not-deployed',
      capabilityManifestVersion: 'brep-contract-v1',
      warm: vi.fn().mockResolvedValue(undefined),
      build: vi.fn(),
    } as unknown as GeometryBackendProvider])).toThrow(/unqualified geometry provider/)
    expect(() => new GeometryBuildEngine([{
      engineClass: 'mesh',
      engineKey: 'unexpected-build',
      kernelFingerprint: 'unexpected-build',
      capabilityManifestVersion: 'own-rust-node-v1',
      warm: vi.fn().mockResolvedValue(undefined),
      build: vi.fn(),
    }])).toThrow(/identity does not match/)
  })

  it('admits only a complete qualified manifest', () => {
    expect(geometryProviderAdmissionForManifest(
      GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'],
    )).toMatchObject({ allowed: true, mode: 'qualified' })
    expect(geometryProviderAdmissionForManifest(
      GEOMETRY_MANIFEST_ARCHIVE['brep-contract-v1'],
    )).toMatchObject({
      allowed: false,
      mode: 'denied',
      reasonCode: 'qualification-incomplete',
    })
    expect(geometryProviderAdmissionForManifest({
      ...GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'],
      manifestDigest: '0'.repeat(64),
    })).toMatchObject({
      allowed: false,
      mode: 'denied',
      reasonCode: 'manifest-integrity-failed',
    })

    expect(geometryProviderAdmissionForManifest({
      ...GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'],
      displayName: 'tampered while retaining the admitted digest',
    })).toMatchObject({ allowed: false, reasonCode: 'manifest-integrity-failed' })
    expect(geometryProviderAdmissionForManifest({
      ...GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'],
      unexpectedEvidence: true,
    } as unknown as GeometryEngineStaticManifest)).toMatchObject({
      allowed: false,
      reasonCode: 'manifest-integrity-failed',
    })

    const unsignedQualified = {
      ...GEOMETRY_MANIFEST_ARCHIVE['brep-contract-v1'],
      maturity: 'production',
      kernelFingerprint: `sha256:${'a'.repeat(64)}`,
      isolation: 'in-process-serialized',
      deployment: 'node-mcp',
      qualification: {
        status: 'qualified',
        recordId: 'qualification:brep-v1',
        corpusVersion: 'brep-corpus-v1',
        target: 'browser-worker/node-mcp',
      },
      dependency: {
        packageName: 'rust-brep-kernel',
        version: '1.0.0',
        licenseExpression: 'Apache-2.0 OR MIT',
        sbomRef: 'sbom:brep-v1',
        sbomSha256: 'b'.repeat(64),
        lockfileSha256: 'c'.repeat(64),
      },
      manifestDigest: '0'.repeat(64),
    } satisfies GeometryEngineStaticManifest
    const qualified = {
      ...unsignedQualified,
      manifestDigest: computeGeometryManifestDigest(unsignedQualified),
    } satisfies GeometryEngineStaticManifest
    expect(geometryProviderAdmissionForManifest(qualified)).toMatchObject({
      allowed: true,
      mode: 'qualified',
      reasonCode: 'qualified',
    })

    const unsignedNotDeployable = {
      ...qualified,
      deployment: 'not-deployed',
      manifestDigest: '0'.repeat(64),
    } satisfies GeometryEngineStaticManifest
    expect(geometryProviderAdmissionForManifest({
      ...unsignedNotDeployable,
      manifestDigest: computeGeometryManifestDigest(unsignedNotDeployable),
    })).toMatchObject({ allowed: false, reasonCode: 'runtime-not-deployable' })

    const unsignedDependencyIncomplete = {
      ...qualified,
      dependency: { ...qualified.dependency, sbomSha256: null },
      manifestDigest: '0'.repeat(64),
    } satisfies GeometryEngineStaticManifest
    expect(geometryProviderAdmissionForManifest({
      ...unsignedDependencyIncomplete,
      manifestDigest: computeGeometryManifestDigest(unsignedDependencyIncomplete),
    })).toMatchObject({
      allowed: false,
      reasonCode: 'dependency-attestation-incomplete',
    })
  })

  it('recomputes every archived manifest digest in browser-safe core code', () => {
    for (const manifest of Object.values(GEOMETRY_MANIFEST_ARCHIVE)) {
      expect(computeGeometryManifestDigest(manifest)).toBe(manifest.manifestDigest)
    }
  })

  it('defaults legacy source to the mesh engine class and records the source-selected route', () => {
    const execution = new GeometryBuildEngine().planSource('cube(1);', request)

    expect(execution).toMatchObject({
      languageContract: 'legacy/current',
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      representation: 'mesh',
      purpose: 'analysis',
      evidence: 'planned',
      automaticFallback: false,
    })
  })

  it('advertises a provider as unavailable when its readiness check fails', async () => {
    const build = vi.fn()
    const warm = vi.fn().mockRejectedValue(new Error('private readiness detail'))
    const engine = new GeometryBuildEngine([{
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
      capabilityManifestVersion: 'own-rust-node-v1',
      warm,
      build,
    }])

    expect((await engine.capabilities()).engines[0]).toMatchObject({
      engineClass: 'mesh',
      availability: 'unavailable',
      unavailableReason: 'The qualified provider failed its runtime readiness check.',
    })
    await expect(engine.buildSource('cube(1);', request)).rejects.toBeInstanceOf(
      GeometryEngineUnavailableError,
    )
    await engine.capabilities()
    expect(warm).toHaveBeenCalledTimes(1)
    expect(build).not.toHaveBeenCalled()
  })

  it('bounds a hung readiness check and reuses its single in-flight attempt', async () => {
    const warm = vi.fn(() => new Promise<void>(() => undefined))
    const engine = new GeometryBuildEngine([{
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
      capabilityManifestVersion: 'own-rust-node-v1',
      warm,
      build: vi.fn(),
    }])

    const startedAt = performance.now()
    expect((await engine.capabilities()).engines[0]).toMatchObject({
      availability: 'unavailable',
      unavailableReason: expect.stringContaining('exceeded 250 ms'),
    })
    expect(performance.now() - startedAt).toBeLessThan(1_000)
    const secondStartedAt = performance.now()
    await engine.capabilities()
    expect(performance.now() - secondStartedAt).toBeLessThan(100)
    expect(warm).toHaveBeenCalledTimes(1)
  })

  it('recovers after late readiness without starting another warmup', async () => {
    vi.useFakeTimers()
    try {
      let ready!: () => void
      const warm = vi.fn(() => new Promise<void>(resolve => { ready = resolve }))
      const engine = new GeometryBuildEngine([{
        engineClass: 'mesh',
        engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
        kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
        capabilityManifestVersion: 'own-rust-node-v1',
        warm,
        build: vi.fn(),
      }])
      const first = engine.capabilities()
      await vi.advanceTimersByTimeAsync(251)
      expect((await first).engines[0]).toMatchObject({
        availability: 'unavailable', unavailableReason: expect.stringContaining('exceeded 250 ms'),
      })
      expect((await engine.capabilities()).engines[0].availability).toBe('unavailable')
      expect(warm).toHaveBeenCalledTimes(1)
      ready()
      await vi.advanceTimersByTimeAsync(0)
      expect((await engine.capabilities()).engines[0]).toMatchObject({
        availability: 'available', unavailableReason: null,
      })
      expect(warm).toHaveBeenCalledTimes(1)
      expect(vi.getTimerCount()).toBe(0)
    } finally { vi.useRealTimers() }
  })

  it('never warms the mesh kernel when a B-rep source selects an unavailable runtime', async () => {
    const build = vi.fn()
    const provider: GeometryBackendProvider = {
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
      capabilityManifestVersion: 'own-rust-node-v1',
      warm: vi.fn().mockResolvedValue(undefined),
      build,
    }
    const engine = new GeometryBuildEngine([provider])

    await expect(engine.buildSource(
      '// @language openscad-viewer/brep-1\ncube(1);',
      request,
    )).rejects.toMatchObject({
      name: 'GeometryEngineUnavailableError',
      execution: {
        languageContract: 'openscad-viewer/brep-1',
        engineClass: 'brep',
        automaticFallback: false,
      },
    } satisfies Partial<GeometryEngineUnavailableError>)
    expect(build).not.toHaveBeenCalled()
  })

  it('rejects unsupported requirements before invoking the selected provider', async () => {
    const build = vi.fn()
    const engine = new GeometryBuildEngine([{
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
      capabilityManifestVersion: 'own-rust-node-v1',
      warm: vi.fn().mockResolvedValue(undefined),
      build,
    }])

    await expect(engine.buildSource(
      '// @requires nurbs.surfaces\ncube(1);',
      request,
    )).rejects.toBeInstanceOf(GeometryCapabilityUnavailableError)
    expect(build).not.toHaveBeenCalled()
  })

  it('attaches runtime execution provenance to provider failures without changing their type', async () => {
    const failure = new RangeError('provider failure')
    const engine = new GeometryBuildEngine([{
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
      capabilityManifestVersion: 'own-rust-node-v1',
      warm: vi.fn().mockResolvedValue(undefined),
      build: vi.fn().mockRejectedValue(failure),
    }])

    await expect(engine.buildSource('cube(1);', request)).rejects.toBe(failure)
    expect(geometryExecutionForError(failure)).toMatchObject({
      engineClass: 'mesh',
      purpose: 'analysis',
      evidence: 'runtime',
      automaticFallback: false,
    })
  })

  it('quarantines providers that reuse errors or reject with primitives', async () => {
    const reused = new Error('reused')
    const reusedEngine = new GeometryBuildEngine([{
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
      capabilityManifestVersion: 'own-rust-node-v1',
      warm: vi.fn().mockResolvedValue(undefined),
      build: vi.fn().mockRejectedValue(reused),
    }])
    await expect(reusedEngine.buildSource('cube(1);', request)).rejects.toBe(reused)
    await expect(reusedEngine.buildSource('cube(1);', request))
      .rejects.toThrow(/reused an error object/)
    expect((await reusedEngine.capabilities()).engines[0].availability).toBe('unavailable')

    const primitiveEngine = new GeometryBuildEngine([{
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
      capabilityManifestVersion: 'own-rust-node-v1',
      warm: vi.fn().mockResolvedValue(undefined),
      build: vi.fn().mockRejectedValue('primitive failure'),
    }])
    await expect(primitiveEngine.buildSource('cube(1);', request))
      .rejects.toThrow(/non-object error/)
    expect((await primitiveEngine.capabilities()).engines[0].availability).toBe('unavailable')
  })

  it('rejects a provider result whose quality contradicts the execution descriptor', async () => {
    const engine = new GeometryBuildEngine([{
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
      capabilityManifestVersion: 'own-rust-node-v1',
      warm: vi.fn().mockResolvedValue(undefined),
      build: vi.fn().mockResolvedValue({
        meshes: [],
        warnings: [],
        volume: 0,
        surfaceArea: 0,
        quality: 'preview',
        reduced: false,
        timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
      }),
    }])

    let caught: unknown
    try {
      await engine.buildSource('cube(1);', request)
    } catch (error) {
      caught = error
    }
    expect(caught).toBeInstanceOf(GeometryProviderContractError)
    expect(geometryExecutionForError(caught)).toMatchObject({ evidence: 'runtime', quality: 'full' })
    expect((await engine.capabilities()).engines[0]).toMatchObject({
      availability: 'unavailable',
      unavailableReason: expect.stringContaining('quarantined'),
    })
    await expect(engine.buildSource('cube(1);', request)).rejects.toBeInstanceOf(
      GeometryEngineUnavailableError,
    )
  })

  it('blocks a late success when a monotonic revocation lands during execution', async () => {
    let finish!: (result: {
      meshes: []
      warnings: []
      volume: number
      surfaceArea: number
      quality: 'full'
      reduced: false
      timings: { parseMs: number; bindMs: number; initializeMs: number; evaluateMs: number; analyzeMs: number }
    }) => void
    const build = vi.fn(() => new Promise<Parameters<typeof finish>[0]>(resolve => {
      finish = resolve
    }))
    const revocations = new GeometryManifestRevocationRegistry()
    const engine = new GeometryBuildEngine([{
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
      capabilityManifestVersion: 'own-rust-node-v1',
      warm: vi.fn().mockResolvedValue(undefined),
      build,
    }], { revocations })

    const pending = engine.buildSource('cube(1);', request)
    await vi.waitFor(() => expect(build).toHaveBeenCalledTimes(1))
    revocations.apply(createGeometryManifestRevocationRecord({
      epoch: 1,
      manifestDigest: GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].manifestDigest,
      reason: 'qualification withdrawn',
      effectiveAt: '2026-08-01T00:00:00.000Z',
      authority: 'test-policy',
    }))
    finish({
      meshes: [],
      warnings: [],
      volume: 1,
      surfaceArea: 6,
      quality: 'full',
      reduced: false,
      timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
    })

    await expect(pending).rejects.toMatchObject({
      name: 'GeometryEngineUnavailableError',
      availabilityCause: 'revoked',
      execution: { evidence: 'runtime', engineClass: 'mesh' },
    })
    expect(build).toHaveBeenCalledTimes(1)
    await expect(engine.buildSource('cube(1);', request)).rejects.toMatchObject({
      availabilityCause: 'revoked',
    })
    expect(build).toHaveBeenCalledTimes(1)
  })

  it('blocks a concurrent late success after the shared provider is quarantined', async () => {
    type Result = {
      meshes: []
      warnings: []
      volume: number
      surfaceArea: number
      quality: 'preview' | 'full'
      reduced: false
      timings: { parseMs: number; bindMs: number; initializeMs: number; evaluateMs: number; analyzeMs: number }
    }
    const finishers: Array<(result: Result) => void> = []
    const build = vi.fn(() => new Promise<Result>(resolve => finishers.push(resolve)))
    const engine = new GeometryBuildEngine([{
      engineClass: 'mesh',
      engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
      kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
      capabilityManifestVersion: 'own-rust-node-v1',
      warm: vi.fn().mockResolvedValue(undefined),
      build,
    }])
    const first = engine.buildSource('cube(1);', request)
    const second = engine.buildSource('sphere(1);', request)
    await vi.waitFor(() => expect(finishers).toHaveLength(2))
    const base = {
      meshes: [] as [], warnings: [] as [], volume: 1, surfaceArea: 6, reduced: false as const,
      timings: { parseMs: 0, bindMs: 0, initializeMs: 0, evaluateMs: 0, analyzeMs: 0 },
    }
    finishers[0]({ ...base, quality: 'preview' })
    await expect(first).rejects.toBeInstanceOf(GeometryProviderContractError)
    finishers[1]({ ...base, quality: 'full' })
    await expect(second).rejects.toMatchObject({
      name: 'GeometryEngineUnavailableError',
      availabilityCause: 'quarantined',
      execution: { evidence: 'runtime' },
    })
    expect(build).toHaveBeenCalledTimes(2)
  })

  it('rejects non-monotonic or tampered revocation records', () => {
    const registry = new GeometryManifestRevocationRegistry()
    const record = createGeometryManifestRevocationRecord({
      epoch: 1,
      manifestDigest: GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].manifestDigest,
      reason: 'test',
      effectiveAt: '2026-08-01T00:00:00.000Z',
      authority: 'test-policy',
    })
    expect(() => registry.apply({ ...record, reason: 'tampered' })).toThrow(/attestation/)
    registry.apply(record)
    expect(() => registry.apply(record)).toThrow(/epochs/)
    expect(registry.snapshot()).toEqual({
      epoch: 1,
      revokedManifestDigests: [record.manifestDigest],
    })
  })

  it('rejects direct engine overrides, unknown contracts, duplicate and misplaced directives', () => {
    const engine = new GeometryBuildEngine()
    for (const source of [
      '// @engine brep\ncube(1);',
      '// @engine: brep\ncube(1);',
      '// @engine=brep\ncube(1);',
      '// @language\ncube(1);',
      '// @language openscad-viewer/brep-1 trailing\ncube(1);',
      '// @requires\ncube(1);',
      '// @language future/unknown\ncube(1);',
      '// @language legacy/current\n// @language legacy/current\ncube(1);',
      'cube(1);\n// @language openscad-viewer/brep-1',
      'cube(1); // @language openscad-viewer/brep-1',
      'cube(1); // @requires geometry.brep',
    ]) {
      expect(() => engine.planSource(source, request)).toThrow(GeometryLanguageContractError)
    }
  })

  it('ignores directive-shaped text inside block comments', () => {
    const execution = new GeometryBuildEngine().planSource(`/*
// @language openscad-viewer/brep-1
*/
cube(1);`, request)

    expect(execution).toMatchObject({
      languageContract: 'legacy/current',
      engineClass: 'mesh',
    })
  })

  it('does not interpret directive-shaped text inside multiline strings', () => {
    const execution = new GeometryBuildEngine().planSource(`note = "first
// @language openscad-viewer/brep-1
last";
cube(1);`, request)

    expect(execution).toMatchObject({
      languageContract: 'legacy/current',
      engineClass: 'mesh',
    })
  })
})
