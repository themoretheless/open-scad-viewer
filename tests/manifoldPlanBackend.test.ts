import { readFileSync, readdirSync, statSync } from 'node:fs'
import { relative, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import type { SemanticNode, SemanticValueType } from '../src/core/semanticProgram'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'
import { executeSemanticProgram } from '../src/services/semanticProgramExecutor'
import {
  MANIFOLD_PLAN_CARRIERS,
  ManifoldPlanBackend,
  type ManifoldPlanPayloadFamily,
} from '../src/services/manifoldPlanBackend'
import type {
  ManifoldKernelHandle,
  ManifoldKernelOps,
  ManifoldKernelSolidAnalysis,
} from '../src/services/manifoldKernelOps'
import {
  evaluateOpenSCADViaInjectedManifoldPlan,
  evaluateOpenSCADViaManifoldPlanForQualification,
  ManifoldPlanQualificationLane,
} from '../src/services/manifoldPlanEvaluator'
import {
  referenceCaptureLegacyOutcome,
  referenceLegacyMeshBytes,
  referenceLegacySceneBytes,
  referenceLegacySha256,
  type ReferenceLegacyOutcome,
} from './support/referenceLegacyDirectEvaluatorOracle'
import {
  referenceFrozenLegacySha256,
  referenceFrozenMetricEqual,
  referenceValidateFrozenLegacyManifest,
  type ReferenceFrozenLegacyExpected,
  type ReferenceFrozenLegacyManifest,
} from './support/referenceFrozenLegacyOutcome'

const FROZEN_MANIFEST_SHA256 = 'a6b4d6025aa59081501d4c54bce921857512cd492714efda560a31b946184897'
const frozenManifestText = readFileSync(
  new URL('./fixtures/manifold-plan-oracle-v1.json', import.meta.url),
  'utf8',
)
const frozenManifestInput: unknown = JSON.parse(frozenManifestText)
const FROZEN_MANIFEST = referenceValidateFrozenLegacyManifest(frozenManifestInput)

function compareFrozenLegacyOutcome(
  expected: ReferenceFrozenLegacyExpected,
  actual: ReferenceLegacyOutcome,
  manifest: ReferenceFrozenLegacyManifest = FROZEN_MANIFEST,
): string[] {
  const differences: string[] = []
  const compare = (path: string, left: unknown, right: unknown): void => {
    if (!Object.is(left, right)) differences.push(path)
  }
  compare('$.tag', expected.tag, actual.tag)
  if (expected.tag === 'success') {
    if (actual.tag !== 'success') return differences
    const meshBytes = referenceLegacyMeshBytes(actual)
    const sceneBytes = referenceLegacySceneBytes(actual)
    compare('$.success.lme1ByteLength', expected.lme1ByteLength, meshBytes.length)
    compare('$.success.lme1Sha256', expected.lme1Sha256, referenceLegacySha256(meshBytes))
    compare('$.success.lse1ByteLength', expected.lse1ByteLength, sceneBytes.length)
    compare('$.success.lse1Sha256', expected.lse1Sha256, referenceLegacySha256(sceneBytes))
    compare('$.success.warnings', JSON.stringify(expected.warnings), JSON.stringify(actual.success.warnings))
    compare('$.success.quality', expected.quality, actual.success.quality)
    compare('$.success.reduced', expected.reduced, actual.success.reduced)
    compare('$.success.meshCount', expected.meshCount, actual.success.meshes.length)
    compare('$.success.sceneAssetCount', expected.sceneAssetCount, actual.success.scene.assets.length)
    compare('$.success.sceneEntityCount', expected.sceneEntityCount, actual.success.scene.entities.length)
    if (!referenceFrozenMetricEqual(
      expected.volume,
      actual.success.volume,
      manifest.comparison.metrics.volume,
    )) differences.push('$.success.volume')
    if (!referenceFrozenMetricEqual(
      expected.surfaceArea,
      actual.success.surfaceArea,
      manifest.comparison.metrics.surfaceArea,
    )) differences.push('$.success.surfaceArea')
    return differences
  }
  if (actual.tag === 'success') return differences
  compare('$.error.name', expected.name, actual.error.name)
  compare('$.error.message', expected.message, actual.error.message)
  compare('$.error.code', expected.code, actual.error.code)
  compare('$.error.line', expected.line, actual.error.line)
  compare('$.error.column', expected.column, actual.error.column)
  compare('$.error.start', expected.start, actual.error.start)
  compare('$.error.end', expected.end, actual.error.end)
  return differences
}

describe('SemanticProgram to Manifold qualification adapter', () => {
  it('authenticates the hand-frozen qualification manifest and corpus', () => {
    expect(referenceFrozenLegacySha256(frozenManifestText)).toBe(FROZEN_MANIFEST_SHA256)
    expect(FROZEN_MANIFEST.corpusSha256)
      .toBe('c58ff48a7f174cb547b4f8aad8ada72a30816d5009a87d046d4f78d634db7482')
    expect(FROZEN_MANIFEST.cases).toHaveLength(65)
    expect(FROZEN_MANIFEST.cases.filter(item => item.expected.tag === 'success')).toHaveLength(43)
    expect(FROZEN_MANIFEST.cases.filter(item => item.expected.tag === 'error')).toHaveLength(22)

    const supportSource = readFileSync(
      new URL('./support/referenceFrozenLegacyOutcome.ts', import.meta.url),
      'utf8',
    )
    expect(supportSource.match(/^\s*import\s+.*$/gm) ?? [])
      .toEqual(["import { createHash } from 'node:crypto'"])
    expect(supportSource).not.toMatch(/(?:from\s*|import\s*\()['"][^'"]*src\//)
  })

  it.each(FROZEN_MANIFEST.cases)(
    'matches the hand-frozen legacy outcome: $name',
    async fixture => {
      const candidate = await referenceCaptureLegacyOutcome(() => (
        evaluateOpenSCADViaManifoldPlanForQualification(fixture.source, {
          quality: fixture.quality,
        })
      ))
      expect(compareFrozenLegacyOutcome(fixture.expected, candidate)).toEqual([])
    },
  )

  it('detects manifest and observable mutations independently of the candidate', async () => {
    const mutatedManifest = JSON.parse(frozenManifestText) as {
      cases: Array<{ expected: { tag: string; lme1Sha256?: string } }>
    }
    const mutableSuccess = mutatedManifest.cases.find(item => item.expected.tag === 'success')
    if (mutableSuccess?.expected.lme1Sha256 === undefined) {
      throw new Error('Frozen manifest has no mutable success case')
    }
    mutableSuccess.expected.lme1Sha256 = '0'.repeat(64)
    expect(() => referenceValidateFrozenLegacyManifest(mutatedManifest))
      .toThrow('$.corpusSha256: corpus digest mismatch')

    const successFixture = FROZEN_MANIFEST.cases[0]
    if (successFixture?.expected.tag !== 'success') {
      throw new Error('First frozen fixture must be a success case')
    }
    const success = await referenceCaptureLegacyOutcome(() => (
      evaluateOpenSCADViaManifoldPlanForQualification(successFixture.source, {
        quality: successFixture.quality,
      })
    ))
    expect(compareFrozenLegacyOutcome(successFixture.expected, success)).toEqual([])
    expect(compareFrozenLegacyOutcome({
      ...successFixture.expected,
      lme1Sha256: '0'.repeat(64),
    }, success)).toContain('$.success.lme1Sha256')
    expect(compareFrozenLegacyOutcome({
      ...successFixture.expected,
      lse1Sha256: '0'.repeat(64),
    }, success)).toContain('$.success.lse1Sha256')
    expect(compareFrozenLegacyOutcome({
      ...successFixture.expected,
      warnings: [...successFixture.expected.warnings, 'mutated warning'],
    }, success)).toContain('$.success.warnings')
    expect(compareFrozenLegacyOutcome({
      ...successFixture.expected,
      volume: successFixture.expected.volume + 1,
    }, success)).toContain('$.success.volume')

    const errorFixture = FROZEN_MANIFEST.cases.find(item => item.expected.tag === 'error')
    if (errorFixture?.expected.tag !== 'error' || errorFixture.expected.end === null) {
      throw new Error('Frozen manifest has no spanned error case')
    }
    const error = await referenceCaptureLegacyOutcome(() => (
      evaluateOpenSCADViaManifoldPlanForQualification(errorFixture.source, {
        quality: errorFixture.quality,
      })
    ))
    expect(compareFrozenLegacyOutcome(errorFixture.expected, error)).toEqual([])
    expect(compareFrozenLegacyOutcome({
      ...errorFixture.expected,
      end: errorFixture.expected.end + 1,
    }, error)).toContain('$.error.end')
  })

  it('does not impose a smaller discarded-effect cap than the retained node budget', async () => {
    const source = `${Array.from(
      { length: 1_001 },
      () => 'difference(){if(false)cube(1);cube(1);}',
    ).join('')}cube(2);`
    expect(source).toHaveLength(39_047)
    expect(referenceFrozenLegacySha256(source))
      .toBe('11e7285cec7d48fd3ea9227b2ace16ac802b25400fa0d290aba8371ae2d6e602')
    const expected: ReferenceFrozenLegacyExpected = {
      tag: 'success',
      lme1Sha256: 'f1344e36d1a2bb026cd534b4076d5c8ae36dcc10a651d08d8dad810b664308bf',
      lme1ByteLength: 2_390,
      lse1Sha256: '9890cce1c9fcbb9d5afcef445ce697dc7aeba7e5a01c10010c59fb3142e8ebd1',
      lse1ByteLength: 2_622,
      warnings: [],
      volume: 8,
      surfaceArea: 24,
      quality: 'full',
      reduced: false,
      meshCount: 1,
      sceneAssetCount: 1,
      sceneEntityCount: 1,
    }
    const candidate = await referenceCaptureLegacyOutcome(() => (
      evaluateOpenSCADViaManifoldPlanForQualification(source)
    ))
    expect(compareFrozenLegacyOutcome(expected, candidate)).toEqual([])
  }, 15_000)

  it('matches the frozen source-length failure before opening a backend session', async () => {
    const source = ' '.repeat(250_001)
    const message = `Line 1, column 1: Source exceeds 250,000 characters\n${source}\n^`
    expect(message).toHaveLength(250_055)
    expect(referenceFrozenLegacySha256(message))
      .toBe('480083bf026186d0d8fbef7c5ec470c859e3c055792fab3917218c845710238c')
    const expected: ReferenceFrozenLegacyExpected = {
      tag: 'error',
      name: 'OpenSCADParseError',
      message,
      code: null,
      line: 1,
      column: 1,
      start: 0,
      end: 1,
    }
    const kernel = fakeKernel()
    const candidate = await referenceCaptureLegacyOutcome(() => (
      evaluateOpenSCADViaInjectedManifoldPlan(source, new ManifoldPlanBackend(kernel.ops))
    ))
    expect(compareFrozenLegacyOutcome(expected, candidate)).toEqual([])
    expect(kernel.live.size).toBe(0)
  })

  it('matches frozen cooperative cancellation as a distinct terminal outcome', async () => {
    const source = Array.from({ length: 120 }, (_, index) => `cube([1,1,${index + 1}]);`).join('\n')
    let calls = 0
    const candidate = await referenceCaptureLegacyOutcome(() => (
      evaluateOpenSCADViaManifoldPlanForQualification(source, {
        shouldAbort: () => calls++ > 0,
        yieldControl: async () => {},
      })
    ))
    const expected: ReferenceFrozenLegacyExpected = {
      tag: 'cancelled',
      name: 'AbortedError',
      message: 'Evaluation aborted: superseded by a newer request',
      code: null,
      line: null,
      column: null,
      start: null,
      end: null,
    }
    expect(compareFrozenLegacyOutcome(expected, candidate)).toEqual([])
  })

  it('keeps the carrier family closed to Region/d2 and SolidSet/d3 mesh values', () => {
    expect(MANIFOLD_PLAN_CARRIERS).toEqual(['solid-set/d3/mesh', 'region/d2/mesh'])
    expect(Object.isFrozen(MANIFOLD_PLAN_CARRIERS)).toBe(true)
  })

  it('confines manifold-3d imports to the narrow KernelOps adapter', () => {
    const root = new URL('../src/', import.meta.url)
    const rootPath = fileURLToPath(root)
    const files: string[] = []
    const visit = (directory: URL) => {
      for (const name of readdirSync(directory)) {
        const child = new URL(name, directory.href.endsWith('/') ? directory : new URL(`${directory.href}/`))
        if (statSync(child).isDirectory()) visit(new URL(`${child.href}/`))
        else if (/\.(?:ts|vue)$/.test(name)) files.push(fileURLToPath(child))
      }
    }
    for (const directory of ['core/', 'components/']) visit(new URL(directory, root))
    files.push(fileURLToPath(new URL('../src/services/openscadCompiler.ts', import.meta.url)))
    expect(files.map(file => relative(rootPath, file).split(sep).join('/')).sort()).toEqual([
      'components/CommandPalette.vue',
      'components/CustomizerPanel.vue',
      'components/ExampleGallery.vue',
      'components/InspectPanel.vue',
      'components/KeyboardShortcuts.vue',
      'components/SceneOutliner.vue',
      'components/ViewCube.vue',
      'components/cadPanels.types.ts',
      'core/build.ts',
      'core/geometryExecution.ts',
      'core/geometryRouting.ts',
      'core/languageContract.ts',
      'core/mesh.ts',
      'core/qualityTargets.ts',
      'core/scene.ts',
      'core/semanticProgram.ts',
      'core/sha256.ts',
      'services/openscadCompiler.ts',
    ])
    const importsManifold = /(?:from\s*|import\s*\()['"]manifold-3d\//
    for (const file of files) expect(readFileSync(file, 'utf8')).not.toMatch(importsManifold)
    expect(readFileSync(new URL('../src/services/manifoldPlanBackend.ts', import.meta.url), 'utf8'))
      .not.toContain('manifold-3d')
    const forbiddenPlanImports = /from\s*['"]\.\/(?:openscadCompiler|openscadParser|geometryWorkerProtocol)['"]/
    expect(readFileSync(new URL('../src/services/manifoldPlanBackend.ts', import.meta.url), 'utf8'))
      .not.toMatch(forbiddenPlanImports)
    expect(readFileSync(new URL('../src/services/legacyV5Assembler.ts', import.meta.url), 'utf8'))
      .not.toMatch(forbiddenPlanImports)
    expect(readFileSync(new URL('../src/services/manifoldKernelOps.ts', import.meta.url), 'utf8'))
      .toContain("from 'manifold-3d/manifold'")
  })
})

interface FakeHandle extends ManifoldKernelHandle {
  readonly serial: number
  readonly original: number | null
  readonly empty: boolean
}

interface FakeKernelOptions {
  failAsOriginal?: boolean
  failOriginalIdAt?: 1 | 2
  failTransform?: boolean
  failPolyhedron?: boolean
  failDelete?: boolean
  emptyPolygon?: boolean
}

function fakeKernel(options: FakeKernelOptions = {}): {
  readonly ops: ManifoldKernelOps
  readonly live: Set<FakeHandle>
  readonly deleted: FakeHandle[]
} {
  const live = new Set<FakeHandle>()
  const deleted: FakeHandle[] = []
  let serial = 0
  let original = 100
  const originalIdCalls = new Map<number, number>()
  const create = (dimension: 2 | 3, value: { original?: boolean; empty?: boolean } = {}) => {
    const handle = {
      dimension,
      serial: serial++,
      original: value.original ? original++ : null,
      empty: value.empty ?? false,
    } as FakeHandle
    live.add(handle)
    return handle
  }
  const result = (dimension: 2 | 3) => create(dimension)
  const emptyAnalysis: ManifoldKernelSolidAnalysis = {
    volume: 0,
    surfaceArea: 0,
    mesh: {
      numProp: 6,
      numTri: 0,
      numVert: 0,
      vertProperties: new Float32Array(),
      triVerts: new Uint32Array(),
      mergeFromVert: new Uint32Array(),
      mergeToVert: new Uint32Array(),
      runIndex: new Uint32Array(),
      runOriginalID: new Uint32Array(),
      runFlags: new Uint8Array(),
      faceID: new Uint32Array(),
    },
  }
  const ops: ManifoldKernelOps = {
    implementationKey: 'manifold-wasm-plan-v1',
    empty2: () => create(2, { empty: true }),
    empty3: () => create(3, { empty: true }),
    box: () => create(3, { original: true }),
    sphere: () => create(3, { original: true }),
    cylinder: () => create(3, { original: true }),
    polyhedron: () => {
      if (options.failPolyhedron) throw new Error('fake polyhedron failure')
      return create(3, { original: true })
    },
    rectangle: () => result(2),
    circle: () => result(2),
    polygon: () => create(2, { empty: options.emptyPolygon }),
    transform2: () => {
      if (options.failTransform) throw new Error('fake transform failure')
      return result(2)
    },
    transform3: () => {
      if (options.failTransform) throw new Error('fake transform failure')
      return result(3)
    },
    boolean2: () => result(2),
    boolean3: () => result(3),
    hull2: () => result(2),
    hull3: () => result(3),
    linearExtrude: () => result(3),
    rotateExtrude: () => result(3),
    projection: () => result(2),
    offset: () => result(2),
    mirrorZ: () => result(3),
    translateZ: () => result(3),
    isEmpty: handle => (handle as FakeHandle).empty,
    originalId: handle => {
      const owned = handle as FakeHandle
      const call = (originalIdCalls.get(owned.serial) ?? 0) + 1
      originalIdCalls.set(owned.serial, call)
      if (options.failOriginalIdAt === call) throw new Error(`fake originalId failure ${call}`)
      return owned.original
    },
    asOriginal: handle => {
      if (options.failAsOriginal) throw new Error('fake asOriginal failure')
      return create(handle.dimension, { original: true })
    },
    analyzeSolid: () => emptyAnalysis,
    delete(handle) {
      const owned = handle as FakeHandle
      if (options.failDelete) throw new Error(`fake delete failure ${owned.serial}`)
      if (!live.delete(owned)) throw new Error(`double delete ${owned.serial}`)
      deleted.push(owned)
    },
  }
  return { ops, live, deleted }
}

describe('Manifold plan ownership lifecycle', () => {
  it('returns typed refusals for language, carrier, evidence, and context mismatches', async () => {
    const kernel = fakeKernel()
    const backend = new ManifoldPlanBackend(kernel.ops)
    const signal = new AbortController().signal
    expect(() => backend.begin({
      programHash: 'program',
      languageContract: 'openscad-viewer/brep-1',
      limits: { maxNodes: 1 },
      signal,
    })).toThrowError(expect.objectContaining({ code: 'E_MANIFOLD_PLAN_UNSUPPORTED' }))

    const session = backend.begin({
      programHash: 'program',
      languageContract: 'legacy/current',
      limits: { maxNodes: 1 },
      signal,
    })
    const analytic: SemanticNode = {
      id: 0,
      kind: 'sphere-analytic',
      valueType: {
        geometryKind: 'solid',
        space: 'd3',
        representation: 'analytic-brep',
        evidence: { tag: 'representation-preserving' },
      },
      radius: 1,
    }
    const context = {
      nodeIndex: 0,
      carrierKey: 'solid/d3/analytic-brep' as const,
      inputNodeIndices: [],
      producer: null,
      programHash: 'program',
      languageContract: 'legacy/current' as const,
      signal,
    }
    expect(() => session.evaluate(analytic, [], context))
      .toThrowError(expect.objectContaining({ code: 'E_MANIFOLD_PLAN_UNSUPPORTED' }))
    expect(() => session.evaluate(analytic, [], {
      ...context,
      carrierKey: 'solid-set/d3/mesh',
    })).toThrowError(expect.objectContaining({ code: 'E_MANIFOLD_PLAN_CONTRACT' }))

    const certifiedType: SemanticValueType = {
      geometryKind: 'solid-set',
      space: 'd3',
      representation: 'mesh',
      evidence: {
        tag: 'certified-approximation',
        certificateProfile: 'test-profile',
        certificatePolicyHash: 'test-policy',
      },
    }
    expect(() => session.evaluate({
      id: 0,
      kind: 'box',
      valueType: certifiedType,
      size: [1, 1, 1],
      center: false,
    }, [], {
      ...context,
      carrierKey: 'solid-set/d3/mesh',
    })).toThrowError(expect.objectContaining({ code: 'E_MANIFOLD_PLAN_UNSUPPORTED' }))

    const transform: SemanticNode = {
      id: 0,
      kind: 'transform',
      valueType: {
        geometryKind: 'solid-set',
        space: 'd3',
        representation: 'mesh',
        evidence: { tag: 'representation-preserving' },
      },
      input: 0,
      matrix: [
        1, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1,
      ],
    }
    const transformContext = {
      ...context,
      carrierKey: 'solid-set/d3/mesh' as const,
    }
    expect(() => session.evaluate(transform, [], transformContext))
      .toThrowError(expect.objectContaining({ code: 'E_MANIFOLD_PLAN_CONTRACT' }))
    expect(() => session.evaluate(transform, [], {
      ...transformContext,
      producer: { occurrenceIndex: 0, operationIndex: 0, operationName: 'color' },
    })).toThrowError(expect.objectContaining({ code: 'E_MANIFOLD_PLAN_CONTRACT' }))
    await session.close({ tag: 'failure', code: 'E_SEMANTIC_BACKEND_FAILURE', node: 0 })
    expect(kernel.live.size).toBe(0)
  })

  it('releases the committed lease when an injected clock throws after execution', async () => {
    const kernel = fakeKernel()
    const backend = new ManifoldPlanBackend(kernel.ops)
    let samples = 0
    await expect(evaluateOpenSCADViaInjectedManifoldPlan('cube(1);', backend, {
      now: () => {
        samples++
        if (samples === 4) throw new Error('clock failed after commit')
        return samples
      },
    })).rejects.toThrow('clock failed after commit')
    expect(kernel.live.size).toBe(0)

    await expect(evaluateOpenSCADViaInjectedManifoldPlan('cube(1);', backend, {
      yieldControl: async () => {},
    })).resolves.toMatchObject({ meshes: expect.any(Array) })
    expect(kernel.live.size).toBe(0)
    expect(new Set(kernel.deleted).size).toBe(kernel.deleted.length)
  })

  it('releases every kernel-prefix lease before surfacing a captured language terminal', async () => {
    const kernel = fakeKernel()
    const backend = new ManifoldPlanBackend(kernel.ops)
    await expect(evaluateOpenSCADViaInjectedManifoldPlan(
      'cube(1); assert(false, "after kernel");',
      backend,
    )).rejects.toThrow(/Assertion 'false' failed: after kernel/)
    expect(kernel.live.size).toBe(0)
    expect(new Set(kernel.deleted).size).toBe(kernel.deleted.length)
  })

  it('rejects duplicate commit leases, cleans the session, and reopens its lane', async () => {
    const kernel = fakeKernel()
    const backend = new ManifoldPlanBackend(kernel.ops)
    const signal = new AbortController().signal
    const begin = () => backend.begin({
      programHash: 'program',
      languageContract: 'legacy/current',
      limits: { maxNodes: 1 },
      signal,
    })
    const session = begin()
    const node: SemanticNode = {
      id: 0,
      kind: 'box',
      valueType: {
        geometryKind: 'solid-set',
        space: 'd3',
        representation: 'mesh',
        evidence: { tag: 'representation-preserving' },
      },
      size: [1, 1, 1],
      center: false,
    }
    const evaluated = await session.evaluate(node, [], {
      nodeIndex: 0,
      carrierKey: 'solid-set/d3/mesh',
      inputNodeIndices: [],
      producer: null,
      programHash: 'program',
      languageContract: 'legacy/current',
      signal,
    })
    if (evaluated.tag === 'empty') throw new Error('fake box unexpectedly returned empty')
    await expect(session.close({
      tag: 'commit',
      retained: [evaluated.lease, evaluated.lease],
    })).rejects.toMatchObject({ code: 'E_MANIFOLD_PLAN_CONTRACT' })
    expect(kernel.live.size).toBe(0)

    const recovery = begin()
    await recovery.close({ tag: 'failure', code: 'E_SEMANTIC_BACKEND_FAILURE', node: null })
  })


  it('holds the serialized lane through root disposal and deletes every handle once', async () => {
    const kernel = fakeKernel()
    const backend = new ManifoldPlanBackend(kernel.ops)
    const artifact = lowerOpenSCADToSemanticProgram('translate([1,2,3]) cube(1);')
    const result = await executeSemanticProgram(artifact, backend)
    expect(kernel.live.size).toBe(1)
    expect(kernel.deleted).toHaveLength(1)
    await expect(executeSemanticProgram(artifact, backend)).rejects.toMatchObject({
      code: 'E_SEMANTIC_BACKEND_BEGIN',
      backendCause: { code: 'E_MANIFOLD_PLAN_BUSY' },
    })
    const firstDispose = result.dispose()
    expect(result.dispose()).toBe(firstDispose)
    await firstDispose
    expect(kernel.live.size).toBe(0)
    expect(new Set(kernel.deleted).size).toBe(kernel.deleted.length)

    const second = await executeSemanticProgram(artifact, backend)
    await second.dispose()
    expect(kernel.live.size).toBe(0)
  })

  it('quarantines the backend lane when committed-result deletion has indeterminate ownership', async () => {
    const kernel = fakeKernel({ failDelete: true })
    const backend = new ManifoldPlanBackend(kernel.ops)
    const artifact = lowerOpenSCADToSemanticProgram('cube(1);')
    const result = await executeSemanticProgram(artifact, backend)
    expect(kernel.live.size).toBe(1)
    const firstDispose = result.dispose()
    expect(result.dispose()).toBe(firstDispose)
    await expect(firstDispose).rejects.toMatchObject({
      message: 'Manifold result cleanup failed',
    })
    expect(kernel.live.size).toBe(1)
    await expect(executeSemanticProgram(artifact, backend)).rejects.toMatchObject({
      code: 'E_SEMANTIC_BACKEND_BEGIN',
      backendCause: {
        code: 'E_MANIFOLD_PLAN_LIFECYCLE',
        message: 'ManifoldPlanBackend is quarantined after an indeterminate cleanup failure',
      },
    })
    expect(kernel.live.size).toBe(1)
  })

  it('evicts a quarantined backend generation and reloads without probing the poisoned object', async () => {
    const poisoned = fakeKernel({ failDelete: true })
    const healthy = fakeKernel()
    const loaded: ManifoldPlanBackend[] = []
    const lane = new ManifoldPlanQualificationLane(async () => {
      const backend = new ManifoldPlanBackend(loaded.length === 0 ? poisoned.ops : healthy.ops)
      loaded.push(backend)
      return backend
    })

    await expect(lane.evaluate('cube(1);')).rejects.toThrow('Manifold result cleanup failed')
    expect(loaded).toHaveLength(1)
    expect(loaded[0].lifecycleState).toBe('quarantined')
    expect(lane.moduleGeneration).toBe(1)
    expect(poisoned.live.size).toBe(1)

    await expect(lane.evaluate('cube(2);')).resolves.toMatchObject({ meshes: expect.any(Array) })
    expect(loaded).toHaveLength(2)
    expect(loaded[1]).not.toBe(loaded[0])
    expect(loaded[1].lifecycleState).toBe('available')
    expect(lane.moduleGeneration).toBe(2)
    expect(healthy.live.size).toBe(0)

    await expect(lane.evaluate('cube(3);')).resolves.toMatchObject({ meshes: expect.any(Array) })
    expect(loaded).toHaveLength(2)
    expect(poisoned.live.size).toBe(1)
    expect(healthy.live.size).toBe(0)
  })

  it('keeps qualification-lane module acquisition behind complete source admission and lowering', async () => {
    const kernel = fakeKernel()
    let loads = 0
    const lane = new ManifoldPlanQualificationLane(async () => {
      loads++
      return new ManifoldPlanBackend(kernel.ops)
    })

    await expect(lane.evaluate(' '.repeat(250_001))).rejects.toThrow(/Source exceeds/)
    await expect(lane.evaluate('cube(')).rejects.toThrow()
    expect(loads).toBe(0)
    expect(lane.moduleGeneration).toBe(0)
    expect(kernel.live.size).toBe(0)

    await expect(lane.evaluate('assert(false, "terminal");')).rejects.toThrow(/terminal/)
    expect(loads).toBe(1)
    expect(lane.moduleGeneration).toBe(1)
    expect(kernel.live.size).toBe(0)
  })

  it('rolls back all allocated inputs when a downstream kernel operation fails', async () => {
    const kernel = fakeKernel({ failTransform: true })
    const backend = new ManifoldPlanBackend(kernel.ops)
    await expect(executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('translate([1,0,0]) cube(1);'),
      backend,
    )).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE' })
    expect(kernel.live.size).toBe(0)
    expect(kernel.deleted).toHaveLength(1)
  })

  it('preserves both a mapped legacy kernel error and session cleanup failure', async () => {
    const kernel = fakeKernel({ failPolyhedron: true, failDelete: true })
    const backend = new ManifoldPlanBackend(kernel.ops)
    const source = 'cube(1); polyhedron(points=[[0,0,0],[1,0,0],[0,1,0],[0,0,1]],faces=[[0,2,1],[0,1,3],[0,3,2],[1,2,3]]);'
    let failure: unknown
    try {
      await evaluateOpenSCADViaInjectedManifoldPlan(source, backend)
    } catch (error) {
      failure = error
    }
    expect(failure).toBeInstanceOf(AggregateError)
    const errors = (failure as AggregateError).errors
    expect(errors[0]).toBeInstanceOf(Error)
    expect((errors[0] as Error).name).toBe('OpenSCADParseError')
    expect((errors[0] as Error).message).toContain('Invalid manifold polyhedron: fake polyhedron failure')
    expect(errors[1]).toBeInstanceOf(AggregateError)
    expect((errors[1] as AggregateError).errors).toEqual([
      expect.objectContaining({ message: 'fake delete failure 0' }),
    ])
  })

  it.each([
    ['first originalId sample', { failOriginalIdAt: 1 as const }, 'cube(1);'],
    ['second originalId sample', { failOriginalIdAt: 2 as const }, 'cube(1);'],
    ['asOriginal construction', { failAsOriginal: true }, 'hull(){cube(1);translate([2,0,0])cube(1);}'],
  ])('consumes every promotion handle when %s throws', async (_name, options, source) => {
    const fault: FakeKernelOptions = { ...options }
    const kernel = fakeKernel(fault)
    const backend = new ManifoldPlanBackend(kernel.ops)
    await expect(executeSemanticProgram(
      lowerOpenSCADToSemanticProgram(source),
      backend,
    )).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE' })
    expect(kernel.live.size).toBe(0)
    expect(new Set(kernel.deleted).size).toBe(kernel.deleted.length)

    fault.failAsOriginal = false
    fault.failOriginalIdAt = undefined
    const recovered = await executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('cube(1);'),
      backend,
    )
    await recovered.dispose()
    expect(kernel.live.size).toBe(0)
  })

  it('retains a legacy materialized-empty handle until result disposal', async () => {
    const kernel = fakeKernel({ emptyPolygon: true })
    const result = await executeSemanticProgram(
      lowerOpenSCADToSemanticProgram('polygon([[0,0],[1,0],[0,1]]);'),
      new ManifoldPlanBackend(kernel.ops),
    )
    expect(result.outputs[0].value.tag).toBe('materialized-empty')
    expect(kernel.live.size).toBe(1)
    expect(kernel.deleted).toHaveLength(0)
    await result.dispose()
    expect(kernel.live.size).toBe(0)
    expect(kernel.deleted).toHaveLength(1)
  })
})

type _PayloadFamilyIsClosed = keyof ManifoldPlanPayloadFamily
