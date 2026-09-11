import { GEOMETRY_MANIFEST_ARCHIVE as CAD_MANIFESTS } from '../src/core/geometryExecution'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { describe, expect, it, vi } from 'vitest'
import {
  GEOMETRY_ENGINE_ROUTES,
  GEOMETRY_MANIFEST_ARCHIVE,
  GEOMETRY_PROVIDER_ADMISSION_REASON_CODES,
  GeometryLanguageContractError,
  geometryEngineClassForLanguageContract,
  parseGeometrySourceRoutingHeader,
  planGeometrySourceExecution,
} from '../src/core/geometryExecution'
import { sha256Hex } from '../src/core/sha256'
import {
  GeometryBuildEngine,
  geometryExecutionForError,
  type GeometryBackendProvider,
} from '../src/services/geometryBuildEngine'
import {
  expectedPublicErrorRetryable,
} from '../src/mcp/errorContract'
import { PUBLIC_ERROR_CODES, publicToolError } from '../src/mcp/publicError'
import { referenceGeometryRoute } from './support/referenceGeometryRouter'

type MatrixInput =
  | { kind: 'literal'; value: string }
  | { kind: 'repeatedSource'; value: string; count: number }
  | { kind: 'requiredCapabilityId'; length: number }
  | { kind: 'requiredCapabilityCount'; count: number }
  | { kind: 'utf16CodeUnits'; values: number[] }

type MatrixExpected = {
  kind: 'route'
  languageContract: 'legacy/current' | 'openscad-viewer/brep-1'
  engineClass: 'mesh' | 'brep'
  requiredCapabilities?: string[]
  requiredCapabilityCount?: number
} | {
  kind: 'error'
  message: string
  reportedContract: string | null
  line: number | null
}

interface RoutingMatrix {
  schemaVersion: number
  contractId: string
  status: string
  languageRoutes: typeof GEOMETRY_ENGINE_ROUTES
  automaticFallback: boolean
  sourceLimits: { characters: number }
  errorPrecedence: string[]
  providerAdmissionReasonCodes: string[]
  publicDiagnostics: Array<{
    code: typeof PUBLIC_ERROR_CODES[number]
    retryable?: boolean
    retryableByAvailabilityCause?: Record<string, boolean>
  }>
  cases: Array<{ id: string; input: MatrixInput; expected: MatrixExpected }>
  runtimeCases: Array<{
    id: string
    expectedError: string
    expectedMessage: string
    availabilityCause?: string
    missingCapabilities?: string[]
    publicCode: typeof PUBLIC_ERROR_CODES[number]
    retryable: boolean
    publicDetails?: Record<string, string | boolean | null>
    expectedExecution?: Record<string, unknown>
    warmCalls: number
    buildCalls: number
    manifoldCallsForBrep: number
  }>
  surfaceCases: Array<{
    id: string
    stage: string
    surface: string
    expectedOutcome: string
    evidenceFile: string
    evidenceTest: string
  }>
}

const matrixUrl = new URL('../docs/qualification/geometry-routing-contract-v1.json', import.meta.url)
const matrixBytes = readFileSync(matrixUrl)
const matrix = JSON.parse(matrixBytes.toString('utf8')) as RoutingMatrix
const MATRIX_SHA256 = 'ee7eaf15cc77b25f503b2f8dc518738512bbf57258fea8253c0814e19b7820ca'
const request = { quality: 'full', purpose: 'analysis' } as const

function materialize(input: MatrixInput): string {
  if (input.kind === 'literal') return input.value
  if (input.kind === 'repeatedSource') return input.value.repeat(input.count)
  if (input.kind === 'utf16CodeUnits') return String.fromCharCode(...input.values)
  if (input.kind === 'requiredCapabilityId') {
    return `// @requires a${'x'.repeat(input.length - 1)}\ncube(1);`
  }
  const capabilities = Array.from(
    { length: input.count },
    (_, index) => `cap${index.toString().padStart(2, '0')}`,
  )
  return `// @requires ${capabilities.join(' ')}\ncube(1);`
}

function productionOutcome(source: string): ReturnType<typeof referenceGeometryRoute> {
  try {
    const route = parseGeometrySourceRoutingHeader(source)
    return {
      kind: 'route',
      languageContract: route.languageContract,
      engineClass: geometryEngineClassForLanguageContract(route.languageContract),
      requiredCapabilities: route.requiredCapabilities,
    }
  } catch (error) {
    if (!(error instanceof GeometryLanguageContractError)) throw error
    return {
      kind: 'error',
      message: error.message,
      reportedContract: error.reportedContract,
      line: error.line,
    }
  }
}

function assertExpected(
  outcome: ReturnType<typeof referenceGeometryRoute>,
  expected: MatrixExpected,
): void {
  if (expected.kind === 'error') {
    expect(outcome).toEqual(expected)
    return
  }
  expect(outcome).toMatchObject({
    kind: 'route',
    languageContract: expected.languageContract,
    engineClass: expected.engineClass,
  })
  if (outcome.kind !== 'route') return
  if (expected.requiredCapabilities) {
    expect(outcome.requiredCapabilities).toEqual(expected.requiredCapabilities)
  }
  if (expected.requiredCapabilityCount !== undefined) {
    expect(outcome.requiredCapabilities).toHaveLength(expected.requiredCapabilityCount)
  }
}

function manifoldProvider(options: {
  warm?: ReturnType<typeof vi.fn>
  build?: ReturnType<typeof vi.fn>
} = {}) {
  const warm = options.warm ?? vi.fn().mockResolvedValue(undefined)
  const build = options.build ?? vi.fn()
  const provider: GeometryBackendProvider = {
    engineClass: 'mesh',
    engineKey: CAD_MANIFESTS['own-rust-node-v1'].engineKey,
    kernelFingerprint: CAD_MANIFESTS['own-rust-node-v1'].kernelFingerprint,
    capabilityManifestVersion: 'own-rust-node-v1',
    warm,
    build,
  }
  return { provider, warm, build }
}

describe('frozen geometry routing contract v1', () => {
  it('pins the reviewed matrix and the sole immutable route table', () => {
    expect(createHash('sha256').update(matrixBytes).digest('hex')).toBe(MATRIX_SHA256)
    expect(matrix).toMatchObject({
      schemaVersion: 2,
      contractId: 'geometry-routing-contract-v1',
      status: 'frozen',
      automaticFallback: false,
    })
    expect(new Set(matrix.cases.map(testCase => testCase.id)).size).toBe(matrix.cases.length)
    expect(new Set(matrix.runtimeCases.map(testCase => testCase.id)).size).toBe(matrix.runtimeCases.length)
    expect(new Set(matrix.surfaceCases.map(testCase => testCase.id)).size).toBe(matrix.surfaceCases.length)
    expect(GEOMETRY_ENGINE_ROUTES).toEqual(matrix.languageRoutes)
    expect(Object.isFrozen(GEOMETRY_ENGINE_ROUTES)).toBe(true)
    expect(GEOMETRY_ENGINE_ROUTES.every(Object.isFrozen)).toBe(true)
    expect(Object.values(GEOMETRY_MANIFEST_ARCHIVE).every(manifest => (
      manifest.limits.sourceCharacters === matrix.sourceLimits.characters
    ))).toBe(true)
    expect(matrixBytes.toString('utf8')).not.toContain('\\ud800')
    expect(matrix.providerAdmissionReasonCodes).toEqual(GEOMETRY_PROVIDER_ADMISSION_REASON_CODES)
  })

  it('covers every frozen precedence stage with a live test anchor', () => {
    expect(matrix.errorPrecedence).toEqual([
      'source-envelope-and-digest',
      'leading-routing-header',
      'cancellation-staleness-and-queue',
      'static-capability-admission',
      'manifest-revocation',
      'deployment-and-provider-presence',
      'provider-readiness',
      'selected-provider-execution',
      'serialization-export-and-persistence',
    ])
    expect(new Set(matrix.surfaceCases.map(testCase => testCase.stage)))
      .toEqual(new Set(matrix.errorPrecedence))
    for (const evidence of matrix.surfaceCases) {
      const source = readFileSync(new URL(`../${evidence.evidenceFile}`, import.meta.url), 'utf8')
      expect(source, evidence.id).toContain(evidence.evidenceTest)
    }
  })

  it('pins the complete public diagnostic taxonomy and conditional retry classes', () => {
    expect(matrix.publicDiagnostics.map(diagnostic => diagnostic.code)).toEqual(PUBLIC_ERROR_CODES)
    for (const diagnostic of matrix.publicDiagnostics) {
      if (diagnostic.retryable !== undefined) {
        expect(expectedPublicErrorRetryable(diagnostic.code), diagnostic.code)
          .toBe(diagnostic.retryable)
      }
      for (const [availabilityCause, retryable] of Object.entries(
        diagnostic.retryableByAvailabilityCause ?? {},
      )) {
        expect(expectedPublicErrorRetryable(diagnostic.code, {
          availability_cause: availabilityCause,
        }), `${diagnostic.code}:${availabilityCause}`).toBe(retryable)
      }
    }
  })

  it.each(matrix.cases)('$id matches the frozen matrix and independent router', testCase => {
    const source = materialize(testCase.input)
    const oracle = referenceGeometryRoute(source)
    const production = productionOutcome(source)
    assertExpected(oracle, testCase.expected)
    assertExpected(production, testCase.expected)
    expect(production).toEqual(oracle)
  })

  it('deep-freezes every planned execution descriptor', () => {
    const descriptor = planGeometrySourceExecution(
      '// @requires geometry.mesh\ncube(1);',
      request,
    )
    expect(Object.isFrozen(descriptor)).toBe(true)
    expect(Object.isFrozen(descriptor.requiredCapabilities)).toBe(true)
    expect(Object.isFrozen(descriptor.effectiveLimits)).toBe(true)
    expect(() => {
      ;(descriptor.requiredCapabilities as string[]).push('forged')
    }).toThrow(TypeError)
  })

  it('matches browser SHA-256 to the independent Node implementation', () => {
    for (const source of ['', 'cube(1);', 'тор \ud83d\udef8', 'x'.repeat(250_000)]) {
      expect(sha256Hex(source)).toBe(createHash('sha256').update(source).digest('hex'))
    }
  })

  it('matches WebCrypto for Unicode source bytes', async () => {
    const source = '// π\ntranslate([1,2,3]) sphere(4); 🚀'
    const digest = await globalThis.crypto.subtle.digest('SHA-256', new TextEncoder().encode(source))
    expect(sha256Hex(source)).toBe(Buffer.from(digest).toString('hex'))
  })

  it.each(matrix.runtimeCases)('$id obeys the frozen refusal/no-fallback policy', async runtimeCase => {
    let error: unknown
    let warmCalls = 0
    let buildCalls = 0
    let manifoldCallsForBrep = 0

    if (runtimeCase.id === 'missing-manifold-provider') {
      try {
        await new GeometryBuildEngine([]).buildSource('cube(1);', request)
      } catch (caught) {
        error = caught
      }
    } else if (runtimeCase.id === 'undeployed-brep') {
      const backend = manifoldProvider()
      try {
        await new GeometryBuildEngine([backend.provider]).buildSource(
          '// @language openscad-viewer/brep-1\ncube(1);',
          request,
        )
      } catch (caught) {
        error = caught
      }
      warmCalls = backend.warm.mock.calls.length
      buildCalls = backend.build.mock.calls.length
      manifoldCallsForBrep = warmCalls + buildCalls
    } else if (runtimeCase.id === 'unsupported-capability-before-readiness') {
      const backend = manifoldProvider({
        warm: vi.fn().mockRejectedValue(new Error('must not warm')),
      })
      try {
        await new GeometryBuildEngine([backend.provider]).buildSource(
          '// @requires nurbs.surfaces\ncube(1);',
          request,
        )
      } catch (caught) {
        error = caught
      }
      warmCalls = backend.warm.mock.calls.length
      buildCalls = backend.build.mock.calls.length
    } else if (runtimeCase.id === 'malformed-header-before-revocation') {
      const backend = manifoldProvider()
      try {
        await new GeometryBuildEngine([backend.provider], {
          revokedManifestDigests: [GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].manifestDigest],
        }).buildSource('// @engine brep\ncube(1);', request)
      } catch (caught) {
        error = caught
      }
      warmCalls = backend.warm.mock.calls.length
      buildCalls = backend.build.mock.calls.length
    } else if (runtimeCase.id === 'capability-before-revocation') {
      const backend = manifoldProvider()
      try {
        await new GeometryBuildEngine([backend.provider], {
          revokedManifestDigests: [GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].manifestDigest],
        }).buildSource('// @requires nurbs.surfaces\ncube(1);', request)
      } catch (caught) {
        error = caught
      }
      warmCalls = backend.warm.mock.calls.length
      buildCalls = backend.build.mock.calls.length
    } else if (runtimeCase.id === 'revocation-before-provider-missing') {
      try {
        await new GeometryBuildEngine([], {
          revokedManifestDigests: [GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].manifestDigest],
        }).buildSource('cube(1);', request)
      } catch (caught) {
        error = caught
      }
    } else if (runtimeCase.id === 'revoked-manifold-manifest') {
      const backend = manifoldProvider()
      try {
        await new GeometryBuildEngine([backend.provider], {
          revokedManifestDigests: [GEOMETRY_MANIFEST_ARCHIVE['own-rust-node-v1'].manifestDigest],
        }).buildSource('cube(1);', request)
      } catch (caught) {
        error = caught
      }
      warmCalls = backend.warm.mock.calls.length
      buildCalls = backend.build.mock.calls.length
    } else if (runtimeCase.id === 'revoked-brep-never-runs-manifold') {
      const backend = manifoldProvider()
      try {
        await new GeometryBuildEngine([backend.provider], {
          revokedManifestDigests: [GEOMETRY_MANIFEST_ARCHIVE['brep-contract-v1'].manifestDigest],
        }).buildSource('// @language openscad-viewer/brep-1\ncube(1);', request)
      } catch (caught) {
        error = caught
      }
      warmCalls = backend.warm.mock.calls.length
      buildCalls = backend.build.mock.calls.length
      manifoldCallsForBrep = warmCalls + buildCalls
    } else if (runtimeCase.id === 'readiness-failure-is-not-retried') {
      const backend = manifoldProvider({
        warm: vi.fn().mockRejectedValue(new Error('readiness failed')),
      })
      const engine = new GeometryBuildEngine([backend.provider])
      for (let attempt = 0; attempt < 2; attempt++) {
        try {
          await engine.buildSource('cube(1);', request)
        } catch (caught) {
          error = caught
        }
      }
      warmCalls = backend.warm.mock.calls.length
      buildCalls = backend.build.mock.calls.length
    } else if (runtimeCase.id === 'provider-failure-has-no-retry') {
      const backend = manifoldProvider({
        build: vi.fn().mockRejectedValue(new RangeError('provider failure')),
      })
      try {
        await new GeometryBuildEngine([backend.provider]).buildSource('cube(1);', request)
      } catch (caught) {
        error = caught
      }
      warmCalls = backend.warm.mock.calls.length
      buildCalls = backend.build.mock.calls.length
    } else {
      throw new Error(`Unimplemented runtime matrix case ${runtimeCase.id}`)
    }

    expect(error).toBeInstanceOf(Error)
    expect((error as Error).name).toBe(runtimeCase.expectedError)
    expect((error as Error).message).toBe(runtimeCase.expectedMessage)
    if (runtimeCase.availabilityCause !== undefined) {
      expect(error).toMatchObject({ availabilityCause: runtimeCase.availabilityCause })
    }
    expect(warmCalls).toBe(runtimeCase.warmCalls)
    expect(buildCalls).toBe(runtimeCase.buildCalls)
    expect(manifoldCallsForBrep).toBe(runtimeCase.manifoldCallsForBrep)
    if (runtimeCase.missingCapabilities !== undefined) {
      expect(error).toMatchObject({ missingCapabilities: runtimeCase.missingCapabilities })
    }
    const execution = geometryExecutionForError(error)
      ?? (error as { execution?: unknown }).execution
    if (runtimeCase.expectedExecution !== undefined) {
      expect(execution).toMatchObject(runtimeCase.expectedExecution)
    } else {
      expect(execution).toBeUndefined()
    }
    const log = vi.spyOn(console, 'error').mockImplementation(() => undefined)
    const classified = publicToolError(error).error
    log.mockRestore()
    expect(classified.code).toBe(runtimeCase.publicCode)
    expect(classified.retryable).toBe(runtimeCase.retryable)
    if (runtimeCase.publicDetails !== undefined) {
      expect(classified.details).toMatchObject(runtimeCase.publicDetails)
    }
  })
})
