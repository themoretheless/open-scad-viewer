import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,before='',after=''){
 const text=readFileSync('examples/rush/'+name+'.r','utf8').replace(before,after)
 const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
 expect(scene.meshes).toHaveLength(1)
 return {volume:scene.volume,report:JSON.parse(scene.measurements![0]!.report)}
}
it('sews a new face to the three existing free edges in traversal order',async()=>{
 for(const n of ['attach-section-rims','attach-bounded-rims']){
  const r=await build(n)
  expect(r.volume).toBeCloseTo(1/6,5)
  expect(r.report).toMatchObject({capFaces:[3],closed:true,sourceEdges:6,resultEdges:6,supportErrorUpper:['0']})
 }
 await expect(build('attach-section-rims','[2,0]','[0,0]')).rejects.toThrow()
})
it('discovers one free cycle and shares the exact work budget with attachment',async()=>{
 for(const n of ['discover-section-rims','discover-bounded-rims']){
  const r=await build(n)
  expect(r.volume).toBeCloseTo(1/6,5)
  expect(r.report.rimAddresses[0]).toHaveLength(3)
  expect(r.report.discoveryWork).toBeGreaterThan(0)
  expect(r.report.workUsed).toBeGreaterThan(r.report.discoveryWork)
 }
})
