import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import {
  BREP_CAPABILITY_MATRIX,
  assertBrepCapabilityAllowsTopology,
} from '../src/services/geometry/brepCapability'

type JsonObject = Record<string, unknown>
type IndexRow = {
  id: string
  plan: string
  evidence: string
  maturity: string
  releaseState: 'shipped' | 'candidate'
  dependencies: string[]
}

const root = resolve(import.meta.dirname, '..')
const readJson = (path: string): JsonObject => (
  JSON.parse(readFileSync(resolve(root, path), 'utf8')) as JsonObject
)
const v3FrozenPaths = [
  'docs/qualification/plans/brep-capability-qualification-plan-v3.schema.json',
  'docs/qualification/brep-capability-evidence-v3.schema.json',
  'docs/qualification/plans/g8-full-matrix-index-v3.json',
  'docs/qualification/brep-full-closed-matrix-v3.json',
  'docs/qualification/brep-capability-registry-release-full-v3.json',
  'docs/qualification/plans/authorized-heal-gap-le1-2.json',
  'docs/qualification/authorized-heal-gap-le1-2-evidence-v3.json',
  'docs/qualification/plans/nurbs-boolean-bezier-le3-3.json',
  'docs/qualification/nurbs-boolean-bezier-le3-3-evidence-v3.json',
  'docs/qualification/plans/nurbs-boolean-bezier-le3-4.json',
  'docs/qualification/nurbs-boolean-bezier-le3-4-evidence-v3.json',
  'docs/qualification/plans/nurbs-boolean-bezier-le3-5.json',
  'docs/qualification/nurbs-boolean-bezier-le3-5-evidence-v3.json',
  'docs/qualification/plans/step-interchange-3.json',
  'docs/qualification/step-interchange-3-evidence-v3.json',
] as const
const frozenV3Digest = (): string => {
  const hash = createHash('sha256')
  for (const path of [...v3FrozenPaths].sort()) {
    hash.update(path).update('\0').update(readFileSync(resolve(root, path))).update('\n')
  }
  return hash.digest('hex')
}

describe('append-only V4 general NURBS Boolean candidate', () => {
  it('keeps the published V3 successor bundle byte-immutable', () => {
    expect(frozenV3Digest())
      .toBe('e86cb0af4eda28c37ec4eca7eaa658d6723d261d20b0e58916b960a93f97fc91')
  })

  it('adds exactly one unavailable successor without changing V3 shipped claims', () => {
    const v3Index = readJson('docs/qualification/plans/g8-full-matrix-index-v3.json')
    const v3Matrix = readJson('docs/qualification/brep-full-closed-matrix-v3.json')
    const v3Release = readJson('docs/qualification/brep-capability-registry-release-full-v3.json')
    const index = readJson('docs/qualification/plans/g8-full-matrix-index-v4.json')
    const matrix = readJson('docs/qualification/brep-full-closed-matrix-v4.json')
    const release = readJson('docs/qualification/brep-capability-registry-release-full-v4.json')
    const v3Rows = v3Index.capabilities as IndexRow[]
    const rows = index.capabilities as IndexRow[]
    const candidate = rows.at(-1)

    expect(index.successorOf).toBe('docs/qualification/plans/g8-full-matrix-index-v3.json')
    expect(matrix.successorOf).toBe('docs/qualification/brep-full-closed-matrix-v3.json')
    expect(release.successorOf)
      .toBe('docs/qualification/brep-capability-registry-release-full-v3.json')
    expect(rows.slice(0, -1)).toEqual(v3Rows)
    expect(candidate).toEqual({
      id: 'nurbs-boolean-bezier-le3/6',
      plan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-6.json',
      evidence: 'docs/qualification/nurbs-boolean-bezier-le3-6-evidence-v4.json',
      maturity: 'Unavailable',
      releaseState: 'candidate',
      dependencies: [
        'numeric-evidence-curved-brep/1',
        'boundary-correspondence/1',
        'exact-sew/1',
        'global-solid-audit/1',
        'persistent-naming/1',
        'nurbs-boolean-bezier-le3/5',
      ],
    })
    expect(release.capabilities).toEqual(v3Release.capabilities)
    expect(matrix.admittedOps).toEqual(v3Matrix.admittedOps)
    expect(release.unresolvedInShippedMatrix).toEqual([])
    expect(matrix.unresolvedInShippedMatrix).toEqual([])
    expect(release.excludedPendingQualification).toEqual([
      ...(v3Release.excludedPendingQualification as string[]),
      'nurbs-boolean-bezier-le3/6',
    ])
    expect(matrix.pendingQualification).toEqual([
      ...(v3Matrix.pendingQualification as string[]),
      'nurbs-boolean-bezier-le3/6',
    ])
  })

  it('closes candidate dependencies only through Qualified shipped V3 rows', () => {
    const index = readJson('docs/qualification/plans/g8-full-matrix-index-v4.json')
    const release = readJson('docs/qualification/brep-capability-registry-release-full-v4.json')
    const rows = index.capabilities as IndexRow[]
    const byId = new Map(rows.map(row => [row.id, row]))
    const shipped = new Set(release.capabilities as string[])
    const candidate = byId.get('nurbs-boolean-bezier-le3/6')

    expect(candidate?.releaseState).toBe('candidate')
    for (const dependency of candidate?.dependencies ?? []) {
      expect(byId.get(dependency), dependency).toMatchObject({
        maturity: 'Qualified',
        releaseState: 'shipped',
      })
      expect(shipped.has(dependency), dependency).toBe(true)
    }
    expect(candidate?.dependencies).toContain('nurbs-boolean-bezier-le3/5')
  })

  it('records the exact multi-span multi-branch contract with no fabricated evidence', () => {
    const plan = readJson('docs/qualification/plans/nurbs-boolean-bezier-le3-6.json')
    const evidence = readJson('docs/qualification/nurbs-boolean-bezier-le3-6-evidence-v4.json')
    const scope = plan.scope as { included: string[]; excluded: string[] }
    const rows = plan.matrix as Array<{ id: string; expected: string }>

    expect(plan).toMatchObject({
      $schema: './brep-capability-qualification-plan-v4.schema.json',
      schemaVersion: 4,
      capability: 'nurbs-boolean-bezier-le3/6',
      successorOf: 'nurbs-boolean-bezier-le3/5',
      lifecycle: { status: 'frozen-pending-execution', qualificationClaim: 'none' },
      evidence: { state: 'not-executed' },
    })
    expect(scope.included.join('\n')).toMatch(/non-periodic positive-weight/)
    expect(scope.included.join('\n')).toMatch(/Exact knot-insertion decomposition/)
    expect(scope.included.join('\n')).toMatch(/half-open \[k_i,k_i\+1\) knot-cell ownership/)
    expect(scope.included.join('\n')).toMatch(/UV DCEL/)
    expect(scope.included.join('\n')).toMatch(/closed-solid union, intersection/)
    for (const refusal of [
      'Tangency',
      'coincidence',
      'Singular poles',
      'Periodic',
      'Degree above 3',
    ]) expect(scope.excluded.join('\n')).toContain(refusal)
    expect(rows.map(row => row.id)).toEqual([
      'NB6-EXACT-PER-SPAN-BEZIER-DECOMPOSITION',
      'NB6-MULTISPAN-MULTIBRANCH-TRANSVERSE-WALK',
      'NB6-HALF-OPEN-KNOT-OWNERSHIP',
      'NB6-BRANCH-JOIN-CHAINS-AND-CYCLES',
      'NB6-OPERAND-UV-DCEL',
      'NB6-CLOSED-SOLID-BOOLEAN-AUTHORSHIP',
      'NB6-RESOURCE-AND-CERTIFICATE-MUTATIONS',
      'NB6-TANGENCY-COINCIDENCE-SINGULAR-PERIODIC-HIGH-DEGREE',
      'NB6-NATIVE-BRIDGE-WASM-PRODUCT',
    ])
    expect(evidence).toMatchObject({
      $schema: './brep-capability-evidence-v4.schema.json',
      schemaVersion: 4,
      capability: 'nurbs-boolean-bezier-le3/6',
      maturity: 'Unavailable',
      state: 'not-executed',
      runs: [],
      oracles: [],
      attestation: { fabricatedRuns: false },
    })
    expect(evidence.unresolvedRows).toEqual(plan.unresolvedRows)
    expect((evidence.unresolvedRows as unknown[])).toHaveLength(rows.length)
  })

  it('keeps runtime topology use unavailable and schemas closed', () => {
    expect(BREP_CAPABILITY_MATRIX.find(row => row.id === 'nurbs-boolean-bezier-le3/6'))
      .toMatchObject({
        maturity: 'Unavailable',
        permitsTopologyChange: true,
        qualificationPlan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-6.json',
      })
    expect(() => assertBrepCapabilityAllowsTopology('nurbs-boolean-bezier-le3/6'))
      .toThrow('is Unavailable; refuse topology change')

    const planSchema = readJson(
      'docs/qualification/plans/brep-capability-qualification-plan-v4.schema.json',
    )
    const evidenceSchema = readJson(
      'docs/qualification/brep-capability-evidence-v4.schema.json',
    )
    expect(planSchema).toMatchObject({
      $schema: 'https://json-schema.org/draft/2020-12/schema',
      type: 'object',
      additionalProperties: false,
      properties: { schemaVersion: { const: 4 } },
    })
    expect(evidenceSchema).toMatchObject({
      $schema: 'https://json-schema.org/draft/2020-12/schema',
      type: 'object',
      additionalProperties: false,
      properties: { schemaVersion: { const: 4 } },
    })
  })
})
