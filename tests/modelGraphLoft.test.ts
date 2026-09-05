import { expect, it } from 'vitest'
import { compileModelGraph, MODELGRAPH_LOFT_EXAMPLE } from '../src/services/modelGraph'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../src/services/geometryBuildEngine'

it('builds a closed offset taper in the production worker with either profile winding', async () => {
  const runtime = new DirectGeometrySupervisor()
  try {
    const geometry = new HeadlessGeometryService(defaultGeometryBuildEngine, runtime)
    for (const reverse of [false, true]) {
      const doc = JSON.parse(JSON.stringify(MODELGRAPH_LOFT_EXAMPLE))
      if (reverse) doc.nodes[0].profile.reverse()
      const result = await geometry.analyze(compileModelGraph(doc).source, 'full')
      expect(result.volume).toBeCloseTo(20/3*(400+100+200), 3)
      expect(result.topology.boundary).toBe(0)
      expect(result.topology.nonManifold).toBe(0)
      expect(result.bounds).toEqual({ min: [-10,-10,0], max: [10,10,20] })
    }
  } finally { await runtime.close() }
})
it('rejects invalid sections and nonconvex profiles before geometry execution', () => {
  const doc = () => JSON.parse(JSON.stringify(MODELGRAPH_LOFT_EXAMPLE))
  const descending = doc(); descending.nodes[0].sections[1].z = 0
  expect(() => compileModelGraph(descending)).toThrow('strictly increase')
  const collapsed = doc(); collapsed.nodes[0].sections[1].scale[0] = 0
  expect(() => compileModelGraph(collapsed)).toThrow('positive scales')
  const concave = doc(); concave.nodes[0].profile = [[0,0],[2,0],[1,1],[2,2],[0,2]]
  expect(() => compileModelGraph(concave)).toThrow('convex')
})
it('supports parameter driven sections and downstream boolean hollowing', async () => {
  const doc = JSON.parse(JSON.stringify(MODELGRAPH_LOFT_EXAMPLE))
  doc.parameters = [{id:'height',value:20}]
  doc.nodes[0].sections[1].z = {param:'height'}
  doc.nodes.push({id:'hole',op:'cylinder',radius:2,height:22}, {id:'cut',op:'difference',base:'adapter',subtract:['hole']})
  doc.root = 'cut'
  const result = await new HeadlessGeometryService().analyze(compileModelGraph(doc).source, 'full')
  expect(result.volume).toBeLessThan(4666.67)
  expect(result.volume).toBeGreaterThan(4300)
  expect(result.topology.nonManifold).toBe(0)
})
