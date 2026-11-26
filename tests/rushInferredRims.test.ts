import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function build(name:string,work=1000000){
  const source=readFileSync('examples/rush/'+name+'.r','utf8').replace('max_work:1000000','max_work:'+work)
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return {volume:scene.volume,report:JSON.parse(scene.measurements![0]!.report)}
}
it('identifies an exact source plane and keeps each of its three coedge addresses',async()=>{
  const result=await build('infer-rim-planes')
  expect(result.volume).toBe(0)
  expect(result.report.planes).toHaveLength(1)
  expect(result.report.planes[0]).toMatchObject({normal:['0','0','1'],offset:'0'})
  expect(result.report.planes[0].rings[0]).toHaveLength(3)
  await expect(build('infer-rim-planes',1)).rejects.toThrow()
})
it('closes the free rim without a plane hint and certifies the enclosed material',async()=>{
  for(const name of ['close-inferred-rims','close-bounded-inferred-rims','build-inferred-solid']){
    const {volume,report}=await build(name)
    expect(volume).toBeCloseTo(1/6,6)
    expect(report).toMatchObject({capFaces:[3],supportErrorUpper:['0']})
    if(name==='build-inferred-solid'){
      expect(report.boundaryProven).toBe(true)
      expect(report.volumeMm3[0]).toBeLessThanOrEqual(1/6)
      expect(report.volumeMm3[1]).toBeGreaterThan(1/6)
    } else expect(report.closed).toBe(true)
  }
})
