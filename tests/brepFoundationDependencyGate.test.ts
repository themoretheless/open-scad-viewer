import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const root = resolve(import.meta.dirname, '..')
const readJson = (path: string): Record<string, unknown> => (
  JSON.parse(readFileSync(resolve(root, path), 'utf8')) as Record<string, unknown>
)

type Row = {
  id: string
  plan: string
  evidence: string
  maturity: string
  releaseState: 'shipped' | 'candidate'
  dependencies: string[]
}

const validateSchema = (
  value: unknown,
  schema: Record<string, unknown>,
  path = '$',
): string[] => {
  const errors: string[] = []
  if ('const' in schema && value !== schema.const) errors.push(`${path} must equal schema const`)
  if (Array.isArray(schema.enum) && !schema.enum.includes(value)) errors.push(`${path} is outside enum`)
  if (schema.type === 'object') {
    if (typeof value !== 'object' || value === null || Array.isArray(value)) return [`${path} must be object`]
    const object = value as Record<string, unknown>
    const properties = (schema.properties ?? {}) as Record<string, Record<string, unknown>>
    for (const required of (schema.required ?? []) as string[]) {
      if (!(required in object)) errors.push(`${path}.${required} is required`)
    }
    if (schema.additionalProperties === false) {
      for (const key of Object.keys(object)) {
        if (!(key in properties)) errors.push(`${path}.${key} is not allowed`)
      }
    }
    for (const [key, child] of Object.entries(properties)) {
      if (key in object) errors.push(...validateSchema(object[key], child, `${path}.${key}`))
    }
  } else if (schema.type === 'array') {
    if (!Array.isArray(value)) return [`${path} must be array`]
    if (typeof schema.minItems === 'number' && value.length < schema.minItems) {
      errors.push(`${path} has too few items`)
    }
    if (schema.uniqueItems === true && new Set(value.map(item => JSON.stringify(item))).size !== value.length) {
      errors.push(`${path} items must be unique`)
    }
    if (schema.items) {
      value.forEach((item, index) => {
        errors.push(...validateSchema(item, schema.items as Record<string, unknown>, `${path}[${index}]`))
      })
    }
  } else if (schema.type === 'string') {
    if (typeof value !== 'string') return [`${path} must be string`]
    if (typeof schema.minLength === 'number' && value.length < schema.minLength) {
      errors.push(`${path} is too short`)
    }
    if (typeof schema.pattern === 'string' && !(new RegExp(schema.pattern).test(value))) {
      errors.push(`${path} does not match pattern`)
    }
  } else if (schema.type === 'boolean' && typeof value !== 'boolean') {
    errors.push(`${path} must be boolean`)
  }
  return errors
}

const transitiveDependents = (rows: Row[], resetId: string): Set<string> => {
  const stale = new Set([resetId])
  let changed = true
  while (changed) {
    changed = false
    for (const row of rows) {
      if (!stale.has(row.id) && row.dependencies.some(dependency => stale.has(dependency))) {
        stale.add(row.id)
        changed = true
      }
    }
  }
  return stale
}

describe('B-rep Foundation v2 dependency gate', () => {
  const index = readJson('docs/qualification/plans/g8-full-matrix-index-v2.json')
  const rows = index.capabilities as Row[]
  const byId = new Map(rows.map(row => [row.id, row]))

  it('has a closed acyclic dependency graph with exact plan agreement', () => {
    const visiting = new Set<string>()
    const visited = new Set<string>()
    const visit = (id: string): void => {
      expect(visiting.has(id), `dependency cycle at ${id}`).toBe(false)
      if (visited.has(id)) return
      visiting.add(id)
      const row = byId.get(id)
      expect(row, id).toBeDefined()
      for (const dependency of row?.dependencies ?? []) {
        expect(byId.has(dependency), `${id} -> ${dependency}`).toBe(true)
        visit(dependency)
      }
      visiting.delete(id)
      visited.add(id)
    }

    for (const row of rows) {
      visit(row.id)
      const plan = readJson(row.plan)
      const planDependencies = (plan.dependencies ?? []) as Array<{
        capability: string
        requiredMaturity: string
      }>
      if (plan.schemaVersion === 2) {
        expect(planDependencies.map(item => item.capability), row.id).toEqual(row.dependencies)
        expect(planDependencies.every(item => item.requiredMaturity === 'Qualified'), row.id).toBe(true)
      } else {
        expect(row.dependencies, row.id).toEqual([])
      }
    }
  })

  it('allows shipment only with a fully Qualified dependency closure', () => {
    const release = readJson('docs/qualification/brep-capability-registry-release-full-v2.json')
    const released = new Set(release.capabilities as string[])
    const assertQualifiedClosure = (id: string, seen = new Set<string>()): void => {
      if (seen.has(id)) return
      seen.add(id)
      const row = byId.get(id)
      expect(row, id).toBeDefined()
      expect(row?.maturity, id).toBe('Qualified')
      for (const dependency of row?.dependencies ?? []) assertQualifiedClosure(dependency, seen)
    }

    for (const row of rows) {
      expect(released.has(row.id), row.id).toBe(row.releaseState === 'shipped')
      if (released.has(row.id)) assertQualifiedClosure(row.id)
    }
  })

  it('transitively stales every dependent after false-Complete reset', () => {
    expect(transitiveDependents(rows, 'numeric-evidence-curved-brep/1')).toEqual(new Set([
      'numeric-evidence-curved-brep/1',
      'boundary-correspondence/1',
      'exact-sew/1',
      'global-solid-audit/1',
      'persistent-naming/1',
      'authorized-heal-gap-le1/1',
      'nurbs-boolean-bezier-le3/2',
      'analytic-multi-edge-fillet/1',
      'exact-parallel-frame-sweep/1',
      'certified-brep-tessellation/1',
      'certified-mass-properties/1',
      'step-interchange/2',
    ]))
    expect(transitiveDependents(rows, 'exact-sew/1')).toEqual(new Set([
      'exact-sew/1',
      'global-solid-audit/1',
      'persistent-naming/1',
      'authorized-heal-gap-le1/1',
      'nurbs-boolean-bezier-le3/2',
      'step-interchange/2',
      'analytic-multi-edge-fillet/1',
      'exact-parallel-frame-sweep/1',
      'certified-brep-tessellation/1',
      'certified-mass-properties/1',
    ]))
  })

  it('keeps all dependency-aware plans and evidence honest and schema-valid', () => {
    const planSchema = readJson('docs/qualification/plans/brep-capability-qualification-plan-v2.schema.json')
    const evidenceSchema = readJson('docs/qualification/brep-capability-evidence-v2.schema.json')
    const dependencyAware = rows.filter(row => readJson(row.plan).schemaVersion === 2)
    const candidates = rows.filter(row => row.releaseState === 'candidate')

    expect(candidates.map(row => row.id)).toEqual(['authorized-heal-gap-le1/1'])
    for (const row of dependencyAware) {
      expect(existsSync(resolve(root, row.plan)), row.plan).toBe(true)
      expect(existsSync(resolve(root, row.evidence)), row.evidence).toBe(true)
      const plan = readJson(row.plan)
      const evidence = readJson(row.evidence)
      expect(validateSchema(plan, planSchema), row.plan).toEqual([])
      expect(validateSchema(evidence, evidenceSchema), row.evidence).toEqual([])
      if (row.releaseState === 'shipped') {
        expect(plan.lifecycle).toEqual({ status: 'qualified', qualificationClaim: 'qualified' })
        expect((plan.evidence as { state: string }).state).toBe('qualified')
        expect(evidence).toMatchObject({
          capability: row.id, maturity: 'Qualified', state: 'qualified',
          attestation: { fabricatedRuns: false },
        })
        expect((evidence.runs as unknown[]).length).toBe(3)
        expect((evidence.oracles as unknown[]).length).toBeGreaterThan(0)
        expect(evidence.unresolvedRows).toEqual([])
      } else {
        expect(plan.lifecycle).toEqual({
          status: 'frozen-pending-execution',
          qualificationClaim: 'none',
        })
        expect((plan.evidence as { state: string }).state).toBe('not-executed')
        expect(evidence).toMatchObject({
          capability: row.id, maturity: 'Unavailable', state: 'not-executed',
          runs: [], attestation: { fabricatedRuns: false },
        })
        expect((evidence.unresolvedRows as unknown[]).length).toBeGreaterThan(0)
      }
    }
  })
})
