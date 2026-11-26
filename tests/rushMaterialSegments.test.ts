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
it('builds both exact segments and retains the trim hole',async()=>{
  const r=await build('build-material-segments')
  expect(r.edges.map((e:any)=>e.parameter)).toEqual([['0','1/2'],['3/2','2']])
  expect(r.edges[1].xyz).toEqual([['1','3/2','0'],['1','2','0']])
  expect(r.edges[1].endpointErrorSquared).toEqual(['0','0'])
})
it('uses the selected material segment as a construction curve',async()=>{
  const r=await build('material-segment-curve')
  expect(r).toMatchObject({selectedEdge:1,storageErrorSquared:{spatial:'0',correspondence:['0','0']}})
  expect(r.storedPcurves).toHaveLength(2)
  await expect(build('material-segment-curve','edge_index:1','edge_index:2')).rejects.toThrow(/Unknown material segment/)
})
