import { describe, expect, it } from 'vitest'
import { compileModelGraph, setModelGraphParameters, MODELGRAPH_EXAMPLE } from '../src/services/modelGraph'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../src/services/geometryBuildEngine'
import { HeadlessGeometryService } from '../src/mcp/geometryService'

describe('ModelGraph/1 frontend', () => {
  it('transfers generated geometry through the production worker contract', async () => {
    const runtime = new DirectGeometrySupervisor()
    try {
      const service = new HeadlessGeometryService(defaultGeometryBuildEngine, runtime)
      const analysis = await service.analyze(compileModelGraph(MODELGRAPH_EXAMPLE).source, 'full')
      expect(analysis.volume).toBeGreaterThan(9100)
      expect(analysis.volume).toBeLessThan(9600)
    } finally { await runtime.close() }
  })

  it('builds real geometry and preserves its actual Manifold execution identity', async () => {
    const compiled = compileModelGraph(MODELGRAPH_EXAMPLE)
    const analysis = await new HeadlessGeometryService().analyze(compiled.source, 'full')
    expect(analysis.volume).toBeGreaterThan(9100)
    expect(analysis.volume).toBeLessThan(9600)
    expect(analysis.execution.engineClass).toBe('manifold')
    expect(analysis.bounds).toEqual({ min: [0, 0, 0], max: [40, 30, 8] })
  })

  it('guards atomic parameter edits by canonical document hash and dimensions', () => {
    const before = compileModelGraph(MODELGRAPH_EXAMPLE)
    const reordered = Object.fromEntries(Object.entries(MODELGRAPH_EXAMPLE).reverse())
    expect(compileModelGraph(reordered).document_sha256).toBe(before.document_sha256)
    const after = setModelGraphParameters(before.document, before.document_sha256, [{ id: 'width', value: 50 }])
    expect(after.document.parameters[0].value).toBe(50)
    expect(before.document.parameters[0].value).toBe(40)
    expect(() => setModelGraphParameters(after.document, before.document_sha256, [{ id: 'width', value: 60 }])).toThrow('Document changed')
    expect(() => setModelGraphParameters(before.document, before.document_sha256, [{ id: 'width', value: -1 }])).toThrow('positive')
  })

  it('rejects code injection, dangling references, duplicate IDs and cycles before geometry execution', () => {
    expect(() => compileModelGraph({ ...MODELGRAPH_EXAMPLE, source: 'cube(1);' })).toThrow()
    expect(() => compileModelGraph({ ...MODELGRAPH_EXAMPLE, root: 'missing' })).toThrow('Unknown node')
    expect(() => compileModelGraph({ ...MODELGRAPH_EXAMPLE, nodes: [{ id: 'x', op: 'translate', vector: [0, 0, 0], input: 'x' }], root: 'x' })).toThrow('Cycle')
    expect(() => compileModelGraph({ ...MODELGRAPH_EXAMPLE, nodes: [...MODELGRAPH_EXAMPLE.nodes, MODELGRAPH_EXAMPLE.nodes[0]] })).toThrow('unique')
    expect(() => compileModelGraph({ ...MODELGRAPH_EXAMPLE, nodes: [{ id: 'x);cube(1)', op: 'sphere', radius: 1 }], root: 'x' })).toThrow()
  })

  it('bounds shared-reference expansion rather than just counting unique nodes', () => {
    const nodes: unknown[] = [{ id: 'n0', op: 'sphere', radius: 1 }]
    for (let i = 1; i < 15; i++) nodes.push({ id: `n${i}`, op: 'union', inputs: [`n${i - 1}`, `n${i - 1}`] })
    expect(() => compileModelGraph({ ...MODELGRAPH_EXAMPLE, nodes, root: 'n14' })).toThrow('expanded')
  })
})
