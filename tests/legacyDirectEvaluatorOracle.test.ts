import { readFileSync } from 'node:fs'
import { beforeAll, describe, expect, it } from 'vitest'
import { parseOpenSCAD } from '../src/services/openscadParser'
import {
  REFERENCE_LEGACY_DIRECT_COMPARISON_CONTRACT,
  ReferenceLegacyOracleError,
  referenceCaptureLegacyOutcome,
  referenceCompareLegacyOutcomes,
  referenceCompareLegacyPreviewFull,
  referenceLegacyMeshBytes,
  referenceLegacySceneBytes,
  referenceLegacySha256,
  referenceSnapshotLegacySuccess,
  type ReferenceLegacyFieldClass,
  type ReferenceLegacyOutcome,
} from './support/referenceLegacyDirectEvaluatorOracle'

const SUCCESS_FIXTURES = Object.freeze([
  {
    id: 'colored-transform',
    source: 'color("#336699cc") translate([1,2,3]) cube([2,3,4]);',
    quality: 'full' as const,
    meshLength: 2_718,
    meshHash: 'f30cbfb0f82d8fe340e963907edd3cd875c7cf5374195838d0191e9a7e023a6c',
    sceneLength: 2_950,
    sceneHash: '173da4b643da29f5bec5dc7d7636801e7f25729888fc0274f9c80a571453953f',
  },
  {
    id: 'boolean-difference',
    source: 'difference(){ cube([4,4,4], center=true); sphere(r=1,$fn=16); }',
    quality: 'full' as const,
    meshLength: 8_679,
    meshHash: 'a2203bc67e6d228b3f2df34eedfbb1cbe1567f2c8e471fcf53107fda181dc080',
    sceneLength: 8_913,
    sceneHash: 'f2772a523f2ee05fe15778bc2454ee89e7a80ab79d7b97fca263a092909ad8bc',
  },
  {
    id: 'repeated-loop-identities',
    source: 'module peg(x=0){translate([x,0,0]) sphere(r=1,$fn=12);} for(i=[0,2,0]) peg(i);',
    quality: 'full' as const,
    meshLength: 15_444,
    meshHash: '1268389d42122d7bd1c5be3afaec1d6f20b2feab53d0cda6c5c4419cff607924',
    sceneLength: 13_925,
    sceneHash: '54b63b5e67e5b84e4a6b8030c950bd5a332ee05c7115fbcc912fda81c6fe93c2',
  },
  {
    id: 'reduced-preview',
    source: 'sphere(r=5,$fn=96);',
    quality: 'preview' as const,
    meshLength: 54_658,
    meshHash: '5485602e24e6eee188bcbfab134b7357c3245ac57bd2623d8ae6089bec7cab21',
    sceneLength: 54_894,
    sceneHash: 'dc5a0f0e6fadcb2d1029f3b9088e9abdca9c85d73121d548298bce27e055a61b',
  },
  {
    id: 'reduced-preview-full-companion',
    source: 'sphere(r=5,$fn=96);',
    quality: 'full' as const,
    meshLength: 214_402,
    meshHash: 'a332772cf4859292ba05c1e7b1cfd441ecea83740d367883491b1fe173f0145a',
    sceneLength: 214_638,
    sceneHash: '802cdd2462b15d30c98b2556f9ad9797324416ece9d0e5f74f4f5c1952ab352d',
  },
])

const outcomes = new Map<string, ReferenceLegacyOutcome>()

function success(id: string): Extract<ReferenceLegacyOutcome, { tag: 'success' }> {
  const outcome = outcomes.get(id)
  if (outcome?.tag !== 'success') throw new Error(`Missing successful fixture ${id}`)
  return outcome
}

function clone<T>(value: T): T {
  return structuredClone(value)
}

function expectMutationClass(
  baseline: ReferenceLegacyOutcome,
  mutate: (candidate: ReferenceLegacyOutcome) => void,
  fieldClass: ReferenceLegacyFieldClass,
): void {
  const candidate = clone(baseline)
  mutate(candidate)
  expect(referenceCompareLegacyOutcomes(baseline, candidate))
    .toEqual(expect.arrayContaining([expect.objectContaining({ fieldClass })]))
}

beforeAll(async () => {
  for (const fixture of SUCCESS_FIXTURES) {
    outcomes.set(fixture.id, await referenceCaptureLegacyOutcome(() => parseOpenSCAD(
      fixture.source,
      { quality: fixture.quality },
    )))
  }
})

describe('independent pinned direct-evaluator differential oracle', () => {
  it('has no dependency on production source or the new adapter', () => {
    const supportSource = readFileSync(
      new URL('./support/referenceLegacyDirectEvaluatorOracle.ts', import.meta.url),
      'utf8',
    )
    const imports = supportSource.match(/^\s*import\s+.*$/gm) ?? []
    expect(imports).toEqual(["import { createHash } from 'node:crypto'"])
    expect(supportSource).not.toMatch(/(?:from\s*|import\s*\()['"][^'"]*src\//)

    const testSource = readFileSync(new URL('./legacyDirectEvaluatorOracle.test.ts', import.meta.url), 'utf8')
    const productionImports = testSource.match(/^\s*import\s+.*['"]\.\.\/src\/.*$/gm) ?? []
    expect(productionImports).toEqual([
      "import { parseOpenSCAD } from '../src/services/openscadParser'",
    ])
  })

  it('freezes exact, tolerant, validate-only, and unavailable field classes before adapter results', () => {
    expect(REFERENCE_LEGACY_DIRECT_COMPARISON_CONTRACT).toMatchObject({
      manifest: 'legacy-direct-evaluator-v1',
      comparator: 'legacy-direct-differential-v1',
      tolerant: {
        volume: { absolute: 1e-9, relative: 1e-9 },
        surfaceArea: { absolute: 1e-9, relative: 1e-9 },
      },
    })
    expect(REFERENCE_LEGACY_DIRECT_COMPARISON_CONTRACT.validateOnly).toHaveLength(2)
    expect(REFERENCE_LEGACY_DIRECT_COMPARISON_CONTRACT.unavailableFromDirectEvaluator)
      .toEqual(expect.arrayContaining([
        'canonical protocol-v5 scene/wire bytes',
        'multiple diagnostic ordering or warnings retained on failure',
      ]))
    expect(REFERENCE_LEGACY_DIRECT_COMPARISON_CONTRACT.blockingIntegrationRequirements)
      .toContain('candidate path must expose a complete ParseResult-compatible terminal outcome, not only opaque backend payloads')
    expect(REFERENCE_LEGACY_DIRECT_COMPARISON_CONTRACT.workerBoundaryPolicy).toEqual({
      identityCharacters: 256,
      oversizedDirectSuccess: 'worker-level-unavailable',
      requiredDisposition: 'bounded-error-before-v5-success-publication',
    })
  })

  it('classifies a direct 257-character identity as a bounded worker-v5 publication blocker', async () => {
    const source = 'assert(true) if(true) let(x=2) cube(x);'
    const raw = await parseOpenSCAD(source)
    expect(raw.meshes).toHaveLength(1)
    expect(raw.meshes[0].entityId).toHaveLength(257)

    await expect(referenceCaptureLegacyOutcome(() => Promise.resolve(raw)))
      .rejects.toMatchObject({
        name: 'ReferenceLegacyOracleError',
        path: '$.meshes[0].entityId',
      })
    await expect(referenceCaptureLegacyOutcome(() => Promise.resolve(raw)))
      .rejects.toThrow('expected well-formed text of at most 256 UTF-16 code units')
  })

  it.each(SUCCESS_FIXTURES)('pins independent LME1/LSE1 bytes for $id', fixture => {
    const outcome = success(fixture.id)
    const meshBytes = referenceLegacyMeshBytes(outcome)
    const sceneBytes = referenceLegacySceneBytes(outcome)
    expect(new TextDecoder().decode(meshBytes.subarray(0, 4))).toBe('LME1')
    expect(new TextDecoder().decode(sceneBytes.subarray(0, 4))).toBe('LSE1')
    expect(meshBytes).toHaveLength(fixture.meshLength)
    expect(referenceLegacySha256(meshBytes)).toBe(fixture.meshHash)
    expect(sceneBytes).toHaveLength(fixture.sceneLength)
    expect(referenceLegacySha256(sceneBytes)).toBe(fixture.sceneHash)
  })

  it('pins colors, stable evaluated identities, source spans, and independent scene deduplication', () => {
    const colored = success('colored-transform').success
    expect(colored.meshes[0].color).toEqual([0.2, 0.4, 0.6, 0.8])
    expect(colored.meshes[0].provenance).toEqual([
      expect.objectContaining({
        triangleStart: 0,
        triangleEnd: 12,
        source: expect.objectContaining({
          id: 38,
          start: 38,
          end: 52,
          label: 'cube()',
          originalId: 0,
        }),
      }),
    ])
    expect(colored.meshes[0].provenance[0].source?.operationId).toBe(
      'op:root/call%3Acolor%230/children/call%3Atranslate%230/children/call%3Acube%230',
    )
    expect(colored.meshes[0].provenance[0].source?.instanceId)
      .toBe(colored.meshes[0].entityId)

    const repeated = success('repeated-loop-identities').success
    expect(repeated.meshes).toHaveLength(3)
    expect(new Set(repeated.meshes.map(mesh => mesh.entityId)).size).toBe(3)
    expect(repeated.meshes[0].geometryAssetId).toBe(repeated.meshes[2].geometryAssetId)
    expect(repeated.scene.assets).toHaveLength(2)
    expect(repeated.scene.entities).toHaveLength(3)
  })

  it('pins aggregate volume and surface-area metrics under the frozen tolerance policy', () => {
    const colored = success('colored-transform').success
    expect(colored.volume).toBe(24)
    expect(colored.surfaceArea).toBe(52)

    const difference = success('boolean-difference').success
    expect(difference.volume).toBeCloseTo(60.16968259019412, 10)
    expect(difference.surfaceArea).toBeCloseTo(107.97276157095422, 10)
  })

  it('pins ordered success warnings including a geometry-free 2D publication', async () => {
    const twoDimensional = await referenceCaptureLegacyOutcome(() => parseOpenSCAD('square([2,3]);'))
    expect(twoDimensional).toMatchObject({
      tag: 'success',
      success: {
        warnings: [
          '1 top-level 2D object(s) are not displayed; wrap them in linear_extrude() or rotate_extrude()',
        ],
        volume: 0,
        surfaceArea: 0,
        meshes: [],
        scene: { assets: [], entities: [] },
      },
    })
    expect(success('reduced-preview').success.warnings).toEqual([
      '$fn=96 was clamped to 48 for preview rendering',
    ])
  })

  it.each([
    {
      id: 'unsupported operation',
      source: 'cube(1);\ntext("nope");',
      line: 2,
      column: 1,
      start: 9,
      end: 10,
      detail: 'Unsupported geometry operation text()',
    },
    {
      id: 'assertion short-circuit',
      source: 'cube(1);\nassert(false, "oracle failed") sphere(1);',
      line: 2,
      column: 1,
      start: 9,
      end: 10,
      detail: "Assertion 'false' failed: oracle failed",
    },
    {
      id: 'negative cube dimension',
      source: 'cube([-1,2,3]);',
      line: 1,
      column: 1,
      start: 0,
      end: 1,
      detail: 'Cube dimensions must be positive',
    },
  ])('pins first-error identity and UTF-16 span: $id', async fixture => {
    const outcome = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(fixture.source))
    expect(outcome).toMatchObject({
      tag: 'error',
      error: {
        name: 'OpenSCADParseError',
        code: null,
        line: fixture.line,
        column: fixture.column,
        start: fixture.start,
        end: fixture.end,
      },
    })
    if (outcome.tag !== 'error') throw new Error('Expected an error outcome')
    expect(outcome.error.message).toContain(fixture.detail)
  })

  it('pins a bounded negative before host-stack overflow', async () => {
    const source = `value = ${'-'.repeat(300)}1; cube(value);`
    const outcome = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(source))
    expect(outcome).toMatchObject({
      tag: 'error',
      error: {
        name: 'OpenSCADParseError',
        line: 1,
        column: 264,
        start: 263,
        end: 264,
      },
    })
    if (outcome.tag !== 'error') throw new Error('Expected an error outcome')
    expect(outcome.error.message).toContain('Expression exceeds 256 nested levels')
  })

  it('preserves exact long source-boundary errors without truncating direct parity', async () => {
    const source = 'x'.repeat(250_001)
    const baseline = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(source))
    const repeated = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(source))
    expect(baseline).toMatchObject({
      tag: 'error',
      error: {
        name: 'OpenSCADParseError',
        line: 1,
        column: 1,
        start: 0,
        end: 1,
      },
    })
    if (baseline.tag !== 'error') throw new Error('Expected a long source-boundary error')
    expect(baseline.error.message.length).toBeGreaterThan(250_000)
    expect(referenceCompareLegacyOutcomes(baseline, repeated)).toEqual([])

    const retargeted = clone(repeated)
    if (retargeted.tag !== 'error') throw new Error('Expected a repeated source-boundary error')
    ;(retargeted.error as { message: string }).message = `${retargeted.error.message} forged`
    expect(referenceCompareLegacyOutcomes(baseline, retargeted))
      .toEqual(expect.arrayContaining([expect.objectContaining({ fieldClass: 'error' })]))
  })

  it('compares full-equivalent and reduced preview/full pairs under separate frozen rules', async () => {
    const source = 'sphere(r=5,$fn=16);'
    const preview = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(source, { quality: 'preview' }))
    const full = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(source, { quality: 'full' }))
    expect(preview).toMatchObject({ tag: 'success', success: { quality: 'preview', reduced: false } })
    expect(full).toMatchObject({ tag: 'success', success: { quality: 'full', reduced: false } })
    expect(referenceCompareLegacyPreviewFull(preview, full)).toEqual([])

    const reducedPreview = success('reduced-preview')
    const reducedFull = success('reduced-preview-full-companion')
    expect(reducedPreview.success.reduced).toBe(true)
    expect(reducedFull.success.reduced).toBe(false)
    expect(referenceCompareLegacyPreviewFull(reducedPreview, reducedFull)).toEqual([])

    const forgedIdentity = clone(reducedFull)
    ;(forgedIdentity.success.meshes[0] as { entityId: string }).entityId = 'entity:forged'
    expect(referenceCompareLegacyPreviewFull(reducedPreview, forgedIdentity)).not.toEqual([])
  })

  it('normalizes only process-local original IDs across independent direct evaluations', async () => {
    const source = 'sphere(r=2,$fn=12);'
    const first = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(source))
    const second = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(source))
    expect(referenceCompareLegacyOutcomes(first, second)).toEqual([])
  })

  it('preserves the first-seen original-ID equality partition and rejects producer collapse', async () => {
    const baseline = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(
      'cube(1); sphere(r=1,$fn=12);',
    ))
    if (baseline.tag !== 'success') throw new Error('Expected a successful outcome')
    const originalIds = baseline.success.meshes.map(mesh => (
      mesh.provenance.find(run => run.source !== null)?.source?.originalId
    ))
    expect(originalIds).toEqual([0, 1])

    const forged = clone(baseline)
    if (forged.tag !== 'success') throw new Error('Expected a successful clone')
    const secondSource = forged.success.meshes[1].provenance.find(run => run.source !== null)?.source
    if (!secondSource) throw new Error('Expected a second producer source')
    ;(secondSource as { originalId: number }).originalId = 0
    expect(referenceCompareLegacyOutcomes(baseline, forged))
      .toEqual(expect.arrayContaining([
        expect.objectContaining({
          fieldClass: 'provenance',
          path: expect.stringContaining('.originalId'),
        }),
      ]))
  })

  it('pins cooperative cancellation as a distinct terminal outcome', async () => {
    const source = Array.from({ length: 120 }, (_, index) => `cube([1,1,${index + 1}]);`).join('\n')
    let calls = 0
    const cancelled = await referenceCaptureLegacyOutcome(() => parseOpenSCAD(source, {
      shouldAbort: () => calls++ > 0,
      yieldControl: async () => {},
    }))
    expect(cancelled).toEqual({
      tag: 'cancelled',
      error: {
        name: 'AbortedError',
        message: 'Evaluation aborted: superseded by a newer request',
        code: null,
        line: null,
        column: null,
        start: null,
        end: null,
      },
    })
    expect(calls).toBeGreaterThan(1)

    const forged = clone(cancelled) as ReferenceLegacyOutcome
    ;(forged as { tag: string }).tag = 'error'
    expect(referenceCompareLegacyOutcomes(cancelled, forged))
      .toEqual([expect.objectContaining({ fieldClass: 'outcome', path: '$.tag' })])
  })

  it.each([
    {
      name: 'warning order/text',
      fieldClass: 'warnings' as const,
      mutate: (candidate: ReferenceLegacyOutcome) => {
        if (candidate.tag !== 'success') return
        ;(candidate.success.warnings as string[]).push('forged warning')
      },
    },
    {
      name: 'quality',
      fieldClass: 'quality' as const,
      mutate: (candidate: ReferenceLegacyOutcome) => {
        if (candidate.tag !== 'success') return
        ;(candidate.success as { quality: string }).quality = 'preview'
      },
    },
    {
      name: 'reduction bit',
      fieldClass: 'quality' as const,
      mutate: (candidate: ReferenceLegacyOutcome) => {
        if (candidate.tag !== 'success') return
        ;(candidate.success as { reduced: boolean }).reduced = true
      },
    },
    {
      name: 'mesh bytes',
      fieldClass: 'mesh-bytes' as const,
      mutate: (candidate: ReferenceLegacyOutcome) => {
        if (candidate.tag !== 'success') return
        candidate.success.meshes[0].vertices.bytes[0] ^= 1
      },
    },
    {
      name: 'scene-only bytes',
      fieldClass: 'scene-bytes' as const,
      mutate: (candidate: ReferenceLegacyOutcome) => {
        if (candidate.tag !== 'success') return
        candidate.success.scene.assets[0].vertices.bytes[0] ^= 1
      },
    },
    {
      name: 'color',
      fieldClass: 'color' as const,
      mutate: (candidate: ReferenceLegacyOutcome) => {
        if (candidate.tag !== 'success') return
        ;(candidate.success.meshes[0].color as number[])[0] = 0.75
      },
    },
    {
      name: 'entity identity',
      fieldClass: 'identity' as const,
      mutate: (candidate: ReferenceLegacyOutcome) => {
        if (candidate.tag !== 'success') return
        ;(candidate.success.meshes[0] as { entityId: string }).entityId = 'entity:forged'
      },
    },
    {
      name: 'provenance span',
      fieldClass: 'provenance' as const,
      mutate: (candidate: ReferenceLegacyOutcome) => {
        if (candidate.tag !== 'success') return
        const source = candidate.success.meshes[0].provenance[0].source
        if (source) (source as { start: number }).start++
      },
    },
    {
      name: 'volume outside tolerance',
      fieldClass: 'metrics' as const,
      mutate: (candidate: ReferenceLegacyOutcome) => {
        if (candidate.tag !== 'success') return
        ;(candidate.success as { volume: number }).volume += 1e-4
      },
    },
  ])('detects comparator mutation: $name', ({ fieldClass, mutate }) => {
    expectMutationClass(success('colored-transform'), mutate, fieldClass)
  })

  it('uses the predeclared metric tolerance without hiding material drift', () => {
    const baseline = success('colored-transform')
    const within = clone(baseline)
    ;(within.success as { volume: number }).volume += 1e-10
    expect(referenceCompareLegacyOutcomes(baseline, within)).toEqual([])

    const outside = clone(baseline)
    ;(outside.success as { surfaceArea: number }).surfaceArea += 1e-4
    expect(referenceCompareLegacyOutcomes(baseline, outside))
      .toEqual([expect.objectContaining({ fieldClass: 'metrics', path: '$.success.surfaceArea' })])
  })

  it('detects error message/span mutations and rejects invalid timing/mesh schemas', async () => {
    const baseline = await referenceCaptureLegacyOutcome(() => parseOpenSCAD('cube([-1,2,3]);'))
    if (baseline.tag !== 'error') throw new Error('Expected an error outcome')
    const forged = clone(baseline)
    ;(forged.error as { message: string }).message = 'forged'
    ;(forged.error as { start: number }).start = 1
    expect(referenceCompareLegacyOutcomes(baseline, forged))
      .toEqual(expect.arrayContaining([
        expect.objectContaining({ fieldClass: 'error', path: '$.error.message' }),
        expect.objectContaining({ fieldClass: 'error', path: '$.error.start' }),
      ]))

    const raw = await parseOpenSCAD('cube(1);')
    expect(() => referenceSnapshotLegacySuccess({
      ...raw,
      timings: { ...raw.timings, analyzeMs: -1 },
    })).toThrow(ReferenceLegacyOracleError)
    const missingIdentity = { ...raw, meshes: raw.meshes.map(mesh => ({ ...mesh })) }
    delete (missingIdentity.meshes[0] as { geometryAssetId?: string }).geometryAssetId
    expect(() => referenceSnapshotLegacySuccess(missingIdentity))
      .toThrow(ReferenceLegacyOracleError)
  })
})
