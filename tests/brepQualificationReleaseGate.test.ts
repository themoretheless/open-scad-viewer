import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { BREP_CAPABILITY_MATRIX } from '../src/services/geometry/brepCapability'

const root = resolve(import.meta.dirname, '..')
const readJson = (path: string): Record<string, unknown> => (
  JSON.parse(readFileSync(resolve(root, path), 'utf8')) as Record<string, unknown>
)

describe('B-rep G8 release gate', () => {
  it('keeps the frozen v1 release internally closed without reclassifying it', () => {
    const matrix = readJson('docs/qualification/brep-full-closed-matrix-v1.json')
    const release = readJson('docs/qualification/brep-capability-registry-release-full-v1.json')
    const index = readJson('docs/qualification/plans/g8-full-matrix-index-v1.json')
    const indexed = index.capabilities as Array<{
      id: string
      plan: string
      evidence: string
      maturityTarget: string
    }>
    const released = release.capabilities as string[]

    expect(released.length).toBeGreaterThan(0)
    expect(indexed.length).toBeGreaterThan(0)
    expect(index.unresolvedInShippedMatrix).toEqual([])
    expect(release.unresolvedInShippedMatrix).toEqual([])
    expect(matrix.unresolvedInShippedMatrix).toEqual([])

    for (const row of indexed) {
      expect(existsSync(resolve(root, row.evidence)), row.evidence).toBe(true)
    }
  })

  it('keeps every plan document schema-addressable', () => {
    const plansDir = resolve(root, 'docs/qualification/plans')
    const excluded = new Set([
      'brep-capability-qualification-plan.schema.json',
      'brep-capability-qualification-plan-v2.schema.json',
      'brep-capability-qualification-plan-v3.schema.json',
      'brep-capability-qualification-plan-v4.schema.json',
      'brep-capability-qualification-plan-v5.schema.json',
      'brep-capability-qualification-plan-v6.schema.json',
      'brep-capability-qualification-plan-v7.schema.json',
      'brep-capability-qualification-plan-v8.schema.json',
      'brep-capability-qualification-plan-v9.schema.json',
      'brep-capability-qualification-plan-v10.schema.json',
      'brep-capability-qualification-plan-v11.schema.json',
      'brep-capability-qualification-plan-v12.schema.json',
      'g8-full-matrix-index-v1.json',
      'g8-full-matrix-index-v2.json',
      'g8-full-matrix-index-v3.json',
      'g8-full-matrix-index-v4.json',
      'g8-full-matrix-index-v5.json',
      'g8-full-matrix-index-v6.json',
      'g8-full-matrix-index-v7.json',
      'g8-full-matrix-index-v8.json',
      'g8-full-matrix-index-v9.json',
      'g8-full-matrix-index-v10.json',
      'g8-full-matrix-index-v11.json',
      'g8-full-matrix-index-v12.json',
      'analytic-boolean-1-g8.json',
      'analytic-chamfer-1-g8.json',
      'iges-interchange-1-g8.json',
      'nurbs-boolean-transverse-bicubic-1-g8.json',
    ])
    for (const name of readdirSync(plansDir).filter(name => name.endsWith('.json'))) {
      if (excluded.has(name)) continue
      const plan = readJson(`docs/qualification/plans/${name}`)
      const expectedSchema = plan.schemaVersion === 12
        ? './brep-capability-qualification-plan-v12.schema.json'
        : plan.schemaVersion === 11
        ? './brep-capability-qualification-plan-v11.schema.json'
        : plan.schemaVersion === 10
        ? './brep-capability-qualification-plan-v10.schema.json'
        : plan.schemaVersion === 9
        ? './brep-capability-qualification-plan-v9.schema.json'
        : plan.schemaVersion === 8
        ? './brep-capability-qualification-plan-v8.schema.json'
        : plan.schemaVersion === 7
        ? './brep-capability-qualification-plan-v7.schema.json'
        : plan.schemaVersion === 6
        ? './brep-capability-qualification-plan-v6.schema.json'
        : plan.schemaVersion === 5
        ? './brep-capability-qualification-plan-v5.schema.json'
        : plan.schemaVersion === 4
        ? './brep-capability-qualification-plan-v4.schema.json'
        : plan.schemaVersion === 3
          ? './brep-capability-qualification-plan-v3.schema.json'
        : plan.schemaVersion === 2
          ? './brep-capability-qualification-plan-v2.schema.json'
          : './brep-capability-qualification-plan.schema.json'
      expect(plan.$schema, name).toBe(expectedSchema)
      expect(plan.schema, name).toBe('open-scad-viewer/brep-capability-qualification-plan')
      expect(typeof plan.planId, name).toBe('string')
      expect(typeof plan.capability, name).toBe('string')
    }
  })

  it('ships only the qualified subset from the dependency-aware v2 index', () => {
    const matrix = readJson('docs/qualification/brep-full-closed-matrix-v2.json')
    const release = readJson('docs/qualification/brep-capability-registry-release-full-v2.json')
    const index = readJson('docs/qualification/plans/g8-full-matrix-index-v2.json')
    const indexed = index.capabilities as Array<{
      id: string
      maturity: string
      releaseState: 'shipped' | 'candidate'
      dependencies: string[]
    }>
    const shipped = indexed.filter(row => row.releaseState === 'shipped')
    const candidates = indexed.filter(row => row.releaseState === 'candidate')

    expect(new Set(release.capabilities as string[])).toEqual(new Set(shipped.map(row => row.id)))
    expect(new Set(matrix.admittedOps as string[])).toEqual(new Set(shipped.map(row => row.id)))
    expect(shipped.every(row => row.maturity === 'Qualified')).toBe(true)
    expect(candidates.every(row => row.maturity !== 'Qualified')).toBe(true)
    expect(new Set(release.excludedPendingQualification as string[]))
      .toEqual(new Set(candidates.map(row => row.id)))
    expect(release.unresolvedInShippedMatrix).toEqual([])
    expect(matrix.unresolvedInShippedMatrix).toEqual([])
    expect((release.explicitRefuse as string[])
      .every(id => (matrix.explicitRefuse as string[]).includes(id))).toBe(true)
    for (const row of shipped) {
      const runtime = BREP_CAPABILITY_MATRIX.find(capability => capability.id === row.id)
      expect(runtime?.maturity, row.id).toBe('Qualified')
    }
  })

  it('ships only qualified V3 successors without changing frozen V2 artifacts', () => {
    const v2Release = readJson('docs/qualification/brep-capability-registry-release-full-v2.json')
    const matrix = readJson('docs/qualification/brep-full-closed-matrix-v3.json')
    const release = readJson('docs/qualification/brep-capability-registry-release-full-v3.json')
    const index = readJson('docs/qualification/plans/g8-full-matrix-index-v3.json')
    const successors = [
      'authorized-heal-gap-le1/2',
      'nurbs-boolean-bezier-le3/3',
      'nurbs-boolean-bezier-le3/4',
      'nurbs-boolean-bezier-le3/5',
      'step-interchange/3',
    ]
    const qualifiedSuccessors = new Set([
      'authorized-heal-gap-le1/2',
      'nurbs-boolean-bezier-le3/3',
      'nurbs-boolean-bezier-le3/4',
      'nurbs-boolean-bezier-le3/5',
      'step-interchange/3',
    ])
    const indexed = index.capabilities as Array<{
      id: string
      plan: string
      evidence: string
      maturity: string
      releaseState: 'shipped' | 'candidate'
      dependencies: string[]
    }>
    const byId = new Map(indexed.map(row => [row.id, row]))
    const released = new Set(release.capabilities as string[])
    const assertQualifiedClosure = (id: string, seen = new Set<string>()): void => {
      if (seen.has(id)) return
      seen.add(id)
      const row = byId.get(id)
      expect(row, id).toBeDefined()
      expect(row?.maturity, id).toBe('Qualified')
      for (const dependency of row?.dependencies ?? []) {
        expect(byId.has(dependency), `${id} -> ${dependency}`).toBe(true)
        assertQualifiedClosure(dependency, seen)
      }
    }

    expect(release.successorOf).toBe(
      'docs/qualification/brep-capability-registry-release-full-v2.json',
    )
    expect(release.capabilities).toEqual([
      ...(v2Release.capabilities as string[]),
      ...successors.filter(id => qualifiedSuccessors.has(id)),
    ])
    expect(matrix.admittedOps).toEqual(release.capabilities)
    expect(release.unresolvedInShippedMatrix).toEqual([])
    expect(matrix.unresolvedInShippedMatrix).toEqual([])
    for (const row of indexed) {
      for (const dependency of row.dependencies) {
        expect(byId.has(dependency), `${row.id} -> ${dependency}`).toBe(true)
      }
      expect(released.has(row.id), row.id).toBe(row.releaseState === 'shipped')
      if (released.has(row.id)) assertQualifiedClosure(row.id)
    }

    for (const id of successors) {
      const row = byId.get(id)
      const qualified = qualifiedSuccessors.has(id)
      expect(row, id).toMatchObject({
        maturity: qualified ? 'Qualified' : 'Unavailable',
        releaseState: qualified ? 'shipped' : 'candidate',
      })
      expect((release.excludedPendingQualification as string[]).includes(id)).toBe(!qualified)
      expect((matrix.pendingQualification as string[]).includes(id)).toBe(!qualified)
      expect(BREP_CAPABILITY_MATRIX.find(capability => capability.id === id), id)
        .toMatchObject({
          maturity: qualified ? 'Qualified' : 'Unavailable',
          permitsTopologyChange: id.startsWith('nurbs-boolean-bezier-le3/'),
        })

      const plan = readJson(row?.plan ?? '')
      const evidence = readJson(row?.evidence ?? '')
      expect(plan).toMatchObject({
        schemaVersion: 3,
        lifecycle: {
          status: 'qualified',
          qualificationClaim: id === 'step-interchange/3' ? 'qualified' : 'finite-cell',
        },
        evidence: { state: 'qualified' },
      })
      expect(evidence).toMatchObject(qualified ? {
        schemaVersion: 3,
        capability: id,
        maturity: 'Qualified',
        state: 'qualified',
        unresolvedRows: [],
        attestation: { fabricatedRuns: false },
      } : {
        schemaVersion: 3,
        capability: id,
        maturity: 'Unavailable',
        state: 'not-executed',
        runs: [],
        oracles: [],
        attestation: { fabricatedRuns: false },
      })
      if (!qualified) expect((evidence.unresolvedRows as unknown[]).length).toBeGreaterThan(0)
    }

    expect((readJson(byId.get('authorized-heal-gap-le1/2')?.plan ?? '')
      .bindings as { implementation: string[] }).implementation)
      .toContain('crates/brep-core/src/transactions.rs')
    expect((readJson(byId.get('nurbs-boolean-bezier-le3/3')?.plan ?? '')
      .bindings as { implementation: string[] }).implementation)
      .toEqual(expect.arrayContaining([
        'crates/brep-core/src/profile_imprint.rs',
        'crates/brep-core/src/operations.rs',
      ]))
  })
})
