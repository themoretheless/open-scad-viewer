import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string,transform=(s:string)=>s) {
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(readFileSync('examples/rush/'+name+'.r','utf8'))).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}
function contains(b:number[],x:number){
  expect(b[0]).toBeLessThanOrEqual(x)
  expect(b[1]).toBeGreaterThanOrEqual(x)
}
it('bounds the web and largest bore at an eccentric gear location',async()=>{
  const r=await run('eccentric-bore-stock')
  contains(r.rootWebMm,12.4)
  contains(r.diameterLimitMm,34.05)
},180000)
it('distinguishes three ring ligaments and impossible mounting circles',async()=>{
  const r=await run('ring-bolt-stock')
  contains(r.rootWebMm,1.4)
  contains(r.outerWebMm,1)
  contains(r.adjacentWebMm,31)
  const w=await run('ring-bolt-window')
  expect(w.status).toBe('feasible')
  expect(w.boltRadiusMm[0]).toBeCloseTo(34.6,9)
  expect(w.boltRadiusMm[1]).toBeCloseTo(42,9)
  const rejected=await run('ring-bolt-window',s=>s.replace('45mm','38mm').replace('hole_count:6','hole_count:60'))
  expect(rejected.status).toBe('infeasible')
},180000)
