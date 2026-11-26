import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,before='',after=''){
  const text=readFileSync('examples/rush/'+name+'.r','utf8').replace(before,after)
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
it('stores matching spatial and chart edges with independently certified error',async()=>{
  const zero={spatial:'0',liftedPcurves:['0','0'],correspondence:['0','0']}
  for(const name of ['store-section-edge','optimize-section-edge']){
    const r=await build(name)
    expect(r).toMatchObject({selectedEdge:0,edgeCount:2,squaredError:zero})
    expect(r.pcurves.map((c:{degree:number})=>c.degree)).toEqual([1,1])
  }
})
it('stores both material components atomically and preserves the gap',async()=>{
  const r=await build('store-section-edges','edge_index:0','edge_index:1')
  expect(r.selectedEdge).toBe(1)
  expect(r.edges).toHaveLength(2)
  expect(r.edges[0].spatial.controlPoints[1][1]).toBeLessThan(r.edges[1].spatial.controlPoints[0][1])
  await expect(build('store-section-edges','edge_index:0','edge_index:2')).rejects.toThrow(/Unknown section edge/)
})
