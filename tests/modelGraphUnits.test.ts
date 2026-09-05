import { describe, expect, it } from 'vitest'
import { compileModelGraph, setModelGraphParameters, ModelGraphError, type Expression } from '../src/services/modelGraph'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../src/services/geometryBuildEngine'
const q = (value: number, unit: 'mm' | 'cm' | 'm' | 'in' | 'deg' | 'rad' = 'mm'): Expression => ({ op: 'quantity', value, unit })
const model = (width: Expression, extra = {}) => ({ language: 'modelgraph/1', units: 'mm', type_policy: 'strict', parameters: [], nodes: [{ id: 'part', op: 'box', size: [width, q(2), q(3)] }], root: 'part', ...extra })

describe('ModelGraph dimensions and declared constraints', () => {
  it('converts mixed length units and preserves physical geometry through the worker', async () => {
    const compiled = compileModelGraph(model({ op: 'add', args: [q(1, 'cm'), q(0.01, 'm')] }))
    expect(compiled.source).toContain('cube([20,2,3]')
    expect(compileModelGraph(model(q(1, 'in'))).source).toContain('cube([25.4,2,3]')
    const runtime = new DirectGeometrySupervisor()
    try {
      const result = await new HeadlessGeometryService(defaultGeometryBuildEngine, runtime).analyze(compiled.source, 'full')
      expect(result.volume).toBeCloseTo(120)
      expect(result.bounds).toEqual({ min: [0, 0, 0], max: [20, 2, 3] })
    } finally { await runtime.close() }
  })
  it('tracks products, ratios and square roots without losing dimensions', () => {
    expect(compileModelGraph(model({ op: 'sqrt', value: { op: 'multiply', args: [q(3), q(3)] } })).source).toContain('cube([3,2,3]')
    expect(compileModelGraph(model({ op: 'multiply', args: [q(4), { op: 'divide', args: [q(1, 'cm'), q(5)] }] })).source).toContain('cube([8,2,3]')
    expect(() => compileModelGraph(model({ op: 'multiply', args: [q(2), q(3)] }))).toThrow('Expected length^1')
    expect(() => compileModelGraph(model({ op: 'sqrt', value: q(4) }))).toThrow('exponents')
  })
  it('rejects incompatible arithmetic and typed fields even in compatibility mode', () => {
    expect(() => compileModelGraph(model({ op: 'add', args: [q(2), q(3, 'deg')] }))).toThrow('Incompatible dimensions')
    expect(() => compileModelGraph(model({ op: 'add', args: [q(2), 3] }))).toThrow('Incompatible dimensions')
    expect(() => compileModelGraph(model(2))).toThrow('Expected length^1')
    expect(compileModelGraph(model(2, { type_policy: 'legacy' })).source).toContain('cube([2,2,3]')
    expect(() => compileModelGraph(model(q(2, 'deg'), { type_policy: 'legacy' }))).toThrow('Expected length^1')
    expect(() => compileModelGraph({ ...model(q(2)), nodes: [{ id: 'solid', op: 'box', size: [q(2), q(2), q(2)] }, { id: 'part', op: 'scale', vector: [q(2), 1, 1], input: 'solid' }] })).toThrow('Expected length^0')
  })
  it('uses typed angles for rotation and trigonometry', () => {
    const doc = { ...model(q(2)), nodes: [{ id: 'solid', op: 'box', size: [q(2), q(2), q(2)] }, { id: 'part', op: 'rotate', vector: [0, 0, q(Math.PI / 2, 'rad')], input: 'solid' }] }
    expect(compileModelGraph(doc).source).toContain('rotate([0,0,90])')
    expect(compileModelGraph(model({ op: 'multiply', args: [q(3), { op: 'sin', value: q(90, 'deg') }] })).source).toContain('cube([3,2,3]')
    expect(() => compileModelGraph(model({ op: 'sin', value: q(2) }))).toThrow('expects an angle')
    expect(() => compileModelGraph(model({ op: 'sin', value: 90 }))).toThrow('expects an angle')
  })
  it('preserves dimensions through closures, ranges and scalar functions', () => {
    const width: Expression = { op: 'apply', function: { op: 'lambda', parameters: ['x'], body: { op: 'multiply', args: [{ local: 'x' }, 2] } }, args: [q(3)] }
    expect(compileModelGraph(model(width)).source).toContain('cube([6,2,3]')
    expect(compileModelGraph(model({ op: 'at', input: { op: 'range', start: q(1, 'cm'), step: q(5), count: 3 }, index: 2 })).source).toContain('cube([20,2,3]')
    expect(compileModelGraph(model({ op: 'call', function: 'size', args: {} }, { functions: [{ id: 'size', kind: 'scalar', parameters: [], body: q(3) }] })).source).toContain('cube([3,2,3]')
    expect(() => compileModelGraph(model({ op: 'if', condition: q(1), then: q(2), else: q(3) }))).toThrow('dimensionless')
  })
  it('preserves declared parameter units on atomic updates and enforces bounds', () => {
    const before = compileModelGraph(model({ param: 'width' }, { parameters: [{ id: 'width', value: 2, unit: 'cm', min: 1, max: 4, integer: true }] }))
    const after = setModelGraphParameters(before.document, before.document_sha256, [{ id: 'width', value: 3 }])
    expect(after.source).toContain('cube([30,2,3]')
    expect(before.document.parameters[0].value).toBe(2)
    expect(() => setModelGraphParameters(before.document, before.document_sha256, [{ id: 'width', value: 5 }])).toThrow('bounds')
    expect(() => setModelGraphParameters(before.document, before.document_sha256, [{ id: 'width', value: 2.5 }])).toThrow('integer')
  })
  it('returns named constraint measurements and all failed constraints', () => {
    const constraints = [{ id: 'minimumWall', left: { param: 'wall' }, relation: 'ge', right: q(2), message: 'Wall must be at least 2 mm.' }, { id: 'maximumWall', left: { param: 'wall' }, relation: 'le', right: q(5), message: 'Wall must be at most 5 mm.' }]
    const good = compileModelGraph(model({ param: 'wall' }, { parameters: [{ id: 'wall', value: 0.3, unit: 'cm' }], constraints }))
    expect(good.constraint_report.map(item => item.passed)).toEqual([true, true])
    expect(good.constraint_report[0]).toMatchObject({ actual: 3, expected: 2, dimension: [1, 0] })
    try { setModelGraphParameters(good.document, good.document_sha256, [{ id: 'wall', value: 0.1 }]); throw new Error('Expected a constraint failure') } catch (error) {
      expect(error).toBeInstanceOf(ModelGraphError)
      expect((error as ModelGraphError).code).toBe('constraint_failed')
      expect((error as ModelGraphError).details).toEqual(expect.arrayContaining([expect.objectContaining({ id: 'minimumWall', passed: false, actual: 1, expected: 2 })]))
    }
    expect(() => compileModelGraph(model(q(2), { constraints: [{ id: 'bad', left: q(2), relation: 'eq', right: q(2, 'deg'), message: 'bad' }] }))).toThrow('Incompatible dimensions')
  })
  it('uses explicit dimensional tolerances and rejects invalid domains', () => {
    const constraint = { id: 'fit', left: q(2.01), right: q(2), relation: 'eq', tolerance: q(0.02), message: 'Fit mismatch' }
    expect(compileModelGraph(model(q(2), { constraints: [constraint] })).constraint_report[0].passed).toBe(true)
    expect(() => compileModelGraph(model(q(2), { constraints: [{ ...constraint, tolerance: q(-1) }] }))).toThrow('nonnegative')
    expect(() => compileModelGraph(model(q(2), { constraints: [{ ...constraint, tolerance: 0.02 }] }))).toThrow('Incompatible dimensions')
    expect(() => compileModelGraph(model(q(1001, 'm')))).toThrow('finite')
    expect(() => compileModelGraph(model({ op: 'divide', args: [q(2), 0] }))).toThrow('finite')
  })
})
