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
function value(q:string){const [n,d='1']=q.split('/');return Number(n)/Number(d)}
it('clips the common line into two material intervals around the hole',async()=>{
  const r=await build('clip-plane-section')
  expect(r.status).toBe('transverse')
  expect(r.sharedInterior).toHaveLength(2)
  expect(r.possible).toHaveLength(2)
  for(const [i,y] of [0.25,1.75].entries()){
    const [a,b]=r.sharedInterior[i].map(value)
    expect(a).toBeLessThan(y);expect(b).toBeGreaterThan(y)
    expect(a>1 || b<1).toBe(true)
  }
})
it('retains exact world endpoints and both matching Cartesian chart segments',async()=>{
  const r=await build('build-plane-section-segments')
  expect(r.segments).toHaveLength(2)
  for(const s of r.segments){
    for(const i of [0,1]){
      expect(s.xyz[i]).toEqual(['1',s.parameter[i],'0'])
      expect(s.pcurves[0][i]).toEqual(['1',s.parameter[i]])
      expect(s.pcurves[1][i]).toEqual([s.parameter[i],'0'])
    }
  }
})
