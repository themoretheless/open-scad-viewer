import { describe, expect, it } from 'vitest'
import { solveRushGraphSketch, type SketchPoint, type SketchConstraint } from '../src/services/rushGraphSketch'
import { compileRushGraph, setRushGraphParameters, RUSH_GRAPH_SKETCH_EXAMPLE, RushGraphError } from '../src/services/rushGraph'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../src/services/geometryBuildEngine'
const points: SketchPoint[] = [{ id: 'a', position: [0.2, -0.1] }, { id: 'b', position: [19, 1] }, { id: 'c', position: [21, 9] }, { id: 'd', position: [-1, 11] }]
const constraints: SketchConstraint[] = [
  { id: 'origin', kind: 'fix', point: 'a', at: [0, 0] },
  { id: 'bottom', kind: 'horizontal', a: 'a', b: 'b' },
  { id: 'right', kind: 'vertical', a: 'b', b: 'c' },
  { id: 'top', kind: 'horizontal', a: 'c', b: 'd' },
  { id: 'left', kind: 'vertical', a: 'd', b: 'a' },
  { id: 'width', kind: 'distance', a: 'a', b: 'b', value: 20 },
  { id: 'height', kind: 'distance', a: 'b', b: 'c', value: 10 },
]
const document = (extra = {}) => ({ language: 'rush/ir-1', units: 'mm', parameters: [], nodes: [{ id: 'outline', op: 'sketch', points, constraints, boundary: ['a', 'b', 'c', 'd'], ...extra }, { id: 'part', op: 'extrude', input: 'outline', height: 3 }], root: 'part' })
describe('bounded sketch constraint solver', () => {
  it('solves perturbed coordinates deterministically without changing the input', () => {
    const before = JSON.stringify(points), result = solveRushGraphSketch(points, constraints)
    expect(result.status).toBe('solved')
    expect(result.degrees_of_freedom).toBe(0)
    expect(result.maximum_residual_mm).toBeLessThanOrEqual(1e-6)
    expect(result.points[2]!.position[0]).toBeCloseTo(20, 5)
    expect(result.points[2]!.position[1]).toBeCloseTo(10, 5)
    expect(solveRushGraphSketch(points, constraints)).toEqual(result)
    expect(JSON.stringify(points)).toBe(before)
  })
  it('builds the solved profile through the isolated production worker', async () => {
    const compiled = compileRushGraph(document())
    expect(compiled.sketch_solutions[0]!.status).toBe('solved')
    const runtime = new DirectGeometrySupervisor()
    try {
      const built = await new HeadlessGeometryService(defaultGeometryBuildEngine, runtime).analyze(compiled.source, 'full')
      expect(built.volume).toBeCloseTo(600, 3)
    } finally { await runtime.close() }
  })
  it('reports missing degrees of freedom and requires explicit acceptance', () => {
    const free = solveRushGraphSketch(points, [])
    expect(free.status).toBe('underconstrained')
    expect(free.degrees_of_freedom).toBe(8)
    expect(() => compileRushGraph(document({ constraints: [] }))).toThrow('fully constrained')
    expect(compileRushGraph(document({ constraints: [], allow_underconstrained: true })).sketch_solutions[0]!.status).toBe('underconstrained')
  })
  it('distinguishes inconsistent linear equations from nonlinear nonconvergence', () => {
    const conflict: SketchConstraint[] = [{ id: 'one', kind: 'fix', point: 'a', at: [0, 0] }, { id: 'two', kind: 'fix', point: 'a', at: [10, 0] }]
    const result = solveRushGraphSketch(points, conflict)
    expect(result.status).toBe('inconsistent')
    expect(result.constraints.filter(c => !c.satisfied).map(c => c.id)).toEqual(['one', 'two'])
    const nonlinear = solveRushGraphSketch(points.map(p => ({ ...p, position: [0, 0] })), [{ id: 'distance', kind: 'distance', a: 'a', b: 'b', value: 10 }])
    expect(nonlinear.status).toBe('not_converged')
    try { compileRushGraph(document({ constraints: conflict })); throw new Error('Expected failure') } catch (error) {
      expect(error).toBeInstanceOf(RushGraphError)
      expect((error as RushGraphError).details).toMatchObject({ status: 'inconsistent' })
    }
  })
  it('handles parallel, perpendicular, equal length and coincident constraints', () => {
    const exact: SketchPoint[] = [{ id: 'a', position: [0, 0] }, { id: 'b', position: [10, 0] }, { id: 'c', position: [10, 10] }, { id: 'd', position: [0, 10] }, { id: 'e', position: [0, 0] }]
    const relations: SketchConstraint[] = [
      { id: 'parallel', kind: 'parallel', a: 'a', b: 'b', c: 'd', d: 'c' },
      { id: 'perpendicular', kind: 'perpendicular', a: 'a', b: 'b', c: 'b', d: 'c' },
      { id: 'equal', kind: 'equal_length', a: 'a', b: 'b', c: 'b', d: 'c' },
      { id: 'coincident', kind: 'coincident', a: 'a', b: 'e' },
    ]
    const result = solveRushGraphSketch(exact, relations)
    expect(result.constraints.every(c => c.satisfied)).toBe(true)
    expect(result.status).toBe('underconstrained')
    const degenerate = solveRushGraphSketch(exact.map(p => ({ ...p, position: [0, 0] })), relations)
    expect(degenerate.status).toBe('not_converged')
    expect(degenerate.degenerate_constraints).toContain('parallel')
  })
  it('re-solves parameter changes atomically and preserves strict length units', () => {
    const before = compileRushGraph(RUSH_GRAPH_SKETCH_EXAMPLE)
    const after = setRushGraphParameters(before.document, before.document_sha256, [{ id: 'width', value: 30 }])
    expect(after.sketch_solutions[0]!.points.find(p => p.id === 'b')!.position[0]).toBeCloseTo(30, 5)
    expect(before.sketch_solutions[0]!.points.find(p => p.id === 'b')!.position[0]).toBeCloseTo(20, 5)
    const q = (value: number) => ({ op: 'quantity', value, unit: 'mm' })
    const typed = document({ points: points.map(p => ({ ...p, position: p.position.map(q) })), constraints: constraints.map(c => c.kind === 'fix' ? { ...c, at: c.at.map(q) } : c.kind === 'distance' ? { ...c, value: q(c.value) } : c) })
    expect(compileRushGraph({ ...typed, type_policy: 'strict', nodes: [typed.nodes[0], { id: 'part', op: 'extrude', input: 'outline', height: q(3) }] }).sketch_solutions[0]!.status).toBe('solved')
  })
  it('rejects invalid references, duplicate IDs and zero distances', () => {
    expect(() => solveRushGraphSketch(points, [{ id: 'bad', kind: 'fix', point: 'missing', at: [0, 0] }])).toThrow('Unknown sketch point')
    expect(() => solveRushGraphSketch([...points, points[0]!], [])).toThrow('unique')
    expect(() => solveRushGraphSketch(points, [{ id: 'bad', kind: 'distance', a: 'a', b: 'b', value: 0 }])).toThrow('positive')
    expect(() => compileRushGraph(document({ boundary: ['a', 'b', 'a'] }))).toThrow('Boundary')
  })
})

it('preserves named reports against the independent TypeScript solver', async () => {
  const {solveRushGraphSketch: reference} = await import('../benchmarks/rush/rushGraphSketch-reference')
  const cases: SketchConstraint[][] = [constraints, [], [{id:'far',kind:'distance',a:'a',b:'a',value:2e6}], [{id:'one',kind:'fix',point:'a',at:[0,0]},{id:'two',kind:'fix',point:'a',at:[10,0]}]]
  for (const cs of cases) {
    const before = JSON.stringify({points, cs})
    const actual = solveRushGraphSketch(points, cs), expected = reference(points, cs)
    expect(actual.status).toBe(expected.status)
    expect(actual.degrees_of_freedom).toBe(expected.degrees_of_freedom)
    expect(actual.redundant_equations).toBe(expected.redundant_equations)
    expect(actual.degenerate_constraints).toEqual(expected.degenerate_constraints)
    expect(actual.constraints.map(c => [c.id,c.satisfied])).toEqual(expected.constraints.map(c => [c.id,c.satisfied]))
    actual.points.forEach((p,i) => {expect(p.id).toBe(expected.points[i]!.id); p.position.forEach((v,j)=>expect(v).toBeCloseTo(expected.points[i]!.position[j]!,5))})
    expect(JSON.stringify({points,cs})).toBe(before)
  }
})
