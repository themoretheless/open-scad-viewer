import {readFileSync} from 'node:fs'
import {it,expect} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,alter=(x:string)=>x){
  const source=alter(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return {scene,report:JSON.parse(scene.measurements![0]!.report)}
}
it('publishes shell orientation and admission separately from the capped candidate',async()=>{
  const {report}=await build('inspect-inferred-solid')
  expect(report).toMatchObject({qualified:true,boundaryProven:true,joinsExact:true,orientations:[{shell:0,outward:true}]})
  const reversed=await build('inspect-inferred-solid',x=>x.replace('[[0,1,3],[1,2,3],[2,0,3]]','[[1,0,3],[2,1,3],[0,2,3]]'))
  expect(reversed.report.qualified).toBe(false)
})
it('keeps the expanded oblique cap on its plane and qualifies a closed material body',async()=>{
  const {scene,report}=await build('expanded-section-sheets')
  expect(report).toMatchObject({edgeCount:4,supportErrorUpper:['0'],shellsClosed:[false]})
  expect(scene.surfaceArea).toBeCloseTo(Math.sqrt(44),5)
  const body=await build('build-expanded-solid')
  expect(body.report).toMatchObject({bodyCount:1,boundaryProven:true,supportErrorUpper:['0']})
  expect(body.scene.volume).toBeCloseTo(1/6,6)
})
it('retains compact exact supports for submillimeter and near-limit oblique rims',async()=>{
  const small=await build('submillimeter-section-sheet')
  const [[u0,u1],[v0,v1]]=small.report.supportDomains[0]
  expect(u1-u0).toBeGreaterThanOrEqual(3/1024)
  expect(u1-u0).toBeLessThanOrEqual(9/1024)
  expect(v1-v0).toBeGreaterThanOrEqual(4/1024)
  expect(v1-v0).toBeLessThanOrEqual(12/1024)
  const large=await build('coordinate-limit-section-sheet')
  expect(large.report.supportDomains[0][1][1]).toBeGreaterThanOrEqual(999999.75)
  expect(large.report.supportDomains[0][1][1]).toBeLessThanOrEqual(1000000)
  for(const r of [small,large]) expect(r.report.supportErrorUpper).toEqual(['0'])
})
