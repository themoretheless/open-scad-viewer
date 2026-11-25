import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function reports(name:string){
  const text=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return scene.measurements!.map(row=>JSON.parse(row.report))
}
it('proves zero planar torsion and encloses nonconstant cubic torsion',async()=>{
  expect((await reports('exact-planar-torsion'))[0].value).toEqual([0,0,0])
  const r=(await reports('bounded-cubic-torsion'))[0]
  expect(Math.abs(r.value[0]-1/(1+9/4+9/16))).toBeLessThanOrEqual(r.continuousErrorUpper[0]+1e-14)
})
it('constructs exact flat fields and enclosure-backed curvature fields on a saddle',async()=>{
  const exact=await reports('exact-plane-curvatures')
  expect(exact.map(r=>r.value[0])).toEqual([0,0,0])
  const bounded=await reports('bounded-saddle-curvatures')
  expect(bounded.map(r=>r.value[0]).sort((a,b)=>a-b)).toEqual([-1,0,4])
  for(const r of bounded) expect(r.continuousErrorUpper[0]).toBeLessThanOrEqual(1e-6)
})
