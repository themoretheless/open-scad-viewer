import { expect, it } from 'vitest'
import { compileModelGraph } from '../src/services/modelGraph'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../src/services/geometryBuildEngine'
const document = (nodes: unknown[]) => ({ language:'modelgraph/1', units:'mm', parameters:[], segments:24, nodes, root:(nodes.at(-1) as {id:string}).id })
const box = {id:'b',op:'box',size:[10,10,10]}
it('executes new CAD operations in the production worker with closed output and measured volumes', async () => {
  const runtime = new DirectGeometrySupervisor()
  try {
    const geometry = new HeadlessGeometryService(defaultGeometryBuildEngine, runtime)
    const cases: [unknown[],number,number][] = [
      [[box,{id:'m',op:'mirror',input:'b',normal:[1,0,0]}],1000,0.001],
      [[box,{id:'p',op:'linear_pattern',input:'b',count:3,step:[20,0,0]}],3000,0.001],
      [[box,{id:'p',op:'circular_pattern',input:'b',count:4,angle_step:90}],4000,0.001],
      [[box,{id:'t',op:'translate',input:'b',vector:[20,0,0]},{id:'h',op:'hull',inputs:['b','t']}],3000,0.001],
      [[box,{id:'p',op:'projection',input:'b'},{id:'e',op:'extrude',input:'p',height:2}],200,0.001],
      [[box,{id:'s',op:'section',input:'b',height:5},{id:'e',op:'extrude',input:'s',height:2}],200,0.001],
      [[{id:'r',op:'rectangle',size:[10,10]},{id:'o',op:'offset',input:'r',distance:1},{id:'e',op:'extrude',input:'o',height:1}],140+2*Math.sqrt(2),0.001],
      [[{id:'r',op:'rectangle',size:[10,10]},{id:'e',op:'advanced_extrude',input:'r',height:10,twist:0,top_scale:[0.5,0.5],slices:8}],1750/3,0.01],
      [[{id:'c',op:'cone',radius_bottom:10,radius_top:0,height:10}],Math.PI*1000/3,15],
      [[{id:'t',op:'torus',major_radius:10,minor_radius:2}],80*Math.PI*Math.PI,20],
    ]
    for (const [nodes,volume,tolerance] of cases) {
      const result = await geometry.analyze(compileModelGraph(document(nodes)).source,'full').catch(error => { throw new Error(JSON.stringify(nodes), {cause:error}) })
      expect(Math.abs(result.volume-volume),JSON.stringify(nodes)).toBeLessThan(tolerance)
      expect(result.topology.boundary).toBe(0)
      expect(result.topology.nonManifold).toBe(0)
    }
  } finally { await runtime.close() }
}, 60000)
it('rejects wrong dimensions, invalid normals and unbounded pattern/extrusion parameters', () => {
  for (const nodes of [
    [box,{id:'m',op:'mirror',input:'b',normal:[0,0,0]}],
    [box,{id:'p',op:'linear_pattern',input:'b',count:257,step:[1,0,0]}],
    [box,{id:'o',op:'offset',input:'b',distance:1}],
    [{id:'t',op:'torus',major_radius:2,minor_radius:2}],
    [{id:'r',op:'rectangle',size:[10,10]},{id:'e',op:'advanced_extrude',input:'r',height:10,twist:0,top_scale:[0,1],slices:8}],
  ]) expect(() => compileModelGraph(document(nodes))).toThrow()
})

it('resizes from measured bounds and splits into independently closed documents', async () => {
  const {modifyModelGraph} = await import('../src/mcp/modelGraphModify')
  const runtime = new DirectGeometrySupervisor()
  try {
    const geometry = new HeadlessGeometryService(defaultGeometryBuildEngine,runtime)
    const original = document([box,{id:'t',op:'translate',input:'b',vector:[5,10,15]}])
    const resized = await modifyModelGraph(original,{operation:'resize',size:[20,30,40],anchor:'minimum'},geometry)
    expect(resized.results[0].analysis.bounds).toEqual({min:[5,10,15],max:[25,40,55]})
    expect(resized.results[0].analysis.volume).toBeCloseTo(24000,3)
    const split = await modifyModelGraph(original,{operation:'split',axis:'z',position:19},geometry)
    expect(split.results[0].analysis.volume).toBeCloseTo(400,6)
    expect(split.results[1].analysis.volume).toBeCloseTo(600,6)
    for (const result of split.results) {
      expect(result.analysis.topology.boundary).toBe(0)
      expect(result.analysis.topology.nonManifold).toBe(0)
      expect(compileModelGraph(result.document).document_sha256).toBe(result.document_sha256)
    }
    await expect(modifyModelGraph(original,{operation:'split',axis:'z',position:15},geometry)).rejects.toThrow('strictly inside')
  } finally {await runtime.close()}
},60000)
