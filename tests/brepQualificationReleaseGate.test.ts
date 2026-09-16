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
      'g8-full-matrix-index-v1.json',
      'g8-full-matrix-index-v2.json',
      'analytic-boolean-1-g8.json',
      'analytic-chamfer-1-g8.json',
      'iges-interchange-1-g8.json',
      'nurbs-boolean-transverse-bicubic-1-g8.json',
    ])
    for (const name of readdirSync(plansDir).filter(name => name.endsWith('.json'))) {
      if (excluded.has(name)) continue
      const plan = readJson(`docs/qualification/plans/${name}`)
      const expectedSchema = plan.schemaVersion === 2
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
})
