import { describe, expect, it } from 'vitest'
import { compileModelGraph } from '../src/services/modelGraph'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../src/services/geometryBuildEngine'
const doc = (nodes: unknown[], root = 'part', extra = {}) => ({ language: 'modelgraph/1', units: 'mm', parameters: [], nodes, root, ...extra })
async function build(document: unknown) {
  const runtime = new DirectGeometrySupervisor()
  try { return await new HeadlessGeometryService(defaultGeometryBuildEngine, runtime).analyze(compileModelGraph(document).source, 'full') }
  finally { await runtime.close() }
}
describe('ModelGraph profiles and solid features', () => {
  it('extrudes a profile with a subtracted hole in the production worker', async () => {
    const result = await build(doc([
      { id: 'outer', op: 'rectangle', size: [20, 10], center: true },
      { id: 'hole', op: 'circle', radius: 2 },
      { id: 'profile', op: 'difference', base: 'outer', subtract: ['hole'] },
      { id: 'part', op: 'extrude', input: 'profile', height: 5, center: true },
    ]))
    expect(result.volume).toBeGreaterThan(936)
    expect(result.volume).toBeLessThan(938)
    expect(result.bounds).toEqual({ min: [-10, -5, -2.5], max: [10, 5, 2.5] })
  })
  it('builds a solid from a polygon, including reversed winding', async () => {
    for (const points of [[[0, 0], [4, 0], [0, 3]], [[0, 3], [4, 0], [0, 0]]]) {
      const result = await build(doc([{ id: 'profile', op: 'polygon', points }, { id: 'part', op: 'extrude', input: 'profile', height: 2 }]))
      expect(result.volume).toBeCloseTo(12)
    }
  })
  it('revolves a radial profile into a hollow solid and supports partial angles', async () => {
    for (const angle of [360, 180]) {
      const result = await build(doc([
        { id: 'rectangle', op: 'rectangle', size: [2, 3] },
        { id: 'radial', op: 'translate', vector: [2, 0, 0], input: 'rectangle' },
        { id: 'part', op: 'revolve', angle, input: 'radial' },
      ]))
      expect(result.volume).toBeGreaterThan(Math.PI * 36 * angle / 360 * 0.98)
      expect(result.volume).toBeLessThan(Math.PI * 36 * angle / 360 * 1.01)
    }
  })
  it('composes profile functions with strict units', async () => {
    const q = (value: number) => ({ op: 'quantity', value, unit: 'mm' })
    const result = await build(doc([
      { id: 'profile', op: 'call', function: 'profile', args: { width: q(5) } },
      { id: 'part', op: 'extrude', height: q(2), input: 'profile' },
    ], 'part', { type_policy: 'strict', functions: [{ id: 'profile', kind: 'geometry', parameters: ['width'], nodes: [{ id: 'rect', op: 'rectangle', size: [{ local: 'width' }, q(3)] }], root: 'rect' }] }))
    expect(result.volume).toBeCloseTo(30)
  })
  it('rejects profile/solid mixing, unclosed-area profiles and nonplanar transforms', () => {
    expect(() => compileModelGraph(doc([{ id: 'part', op: 'circle', radius: 2 }]))).toThrow('Expected solid')
    expect(() => compileModelGraph(doc([{ id: 'body', op: 'box', size: [2, 2, 2] }, { id: 'part', op: 'extrude', height: 2, input: 'body' }]))).toThrow('Expected profile')
    for (const points of [[[0, 0], [1, 1], [0, 1], [1, 0]], [[0, 0], [1, 0], [2, 0]], [[0, 0], [2, 0], [0, 2], [0, 0]]]) {
      expect(() => compileModelGraph(doc([{ id: 'p', op: 'polygon', points }, { id: 'part', op: 'extrude', input: 'p', height: 1 }]))).toThrow()
    }
    expect(() => compileModelGraph(doc([{ id: 'p', op: 'circle', radius: 1 }, { id: 'moved', op: 'translate', vector: [0, 0, 1], input: 'p' }, { id: 'part', op: 'extrude', height: 2, input: 'moved' }]))).toThrow('Profiles must remain in XY')
    expect(() => compileModelGraph(doc([{ id: 'p', op: 'circle', radius: 1 }, { id: 'part', op: 'revolve', angle: 361, input: 'p' }]))).toThrow('360')
  })
})
