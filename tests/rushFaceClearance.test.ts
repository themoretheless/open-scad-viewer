import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string,transform=(s:string)=>s) {
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(readFileSync('examples/rush/'+name+'.r','utf8'))).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}
it('measures the full curve and patch witnesses before trimming',async()=>{
  for(const name of ['curve-patch-clearance','patch-pair-clearance']){
    const r=await run(name)
    expect(r.decision).toBe('below_minimum')
    expect(r.bounds_mm[0]).toBeLessThanOrEqual(0.101)
    expect(r.bounds_mm[1]).toBeLessThan(0.12)
  }
},180000)
it('excludes a hole from face and swept-ball distance witnesses',async()=>{
  const pair=await run('trimmed-face-clearance')
  expect(pair.decision).toBe('meets_minimum')
  const r=await run('curve-trimmed-face-distance')
  expect(r.lower_bound_mm).toBeGreaterThan(0.35)
  expect(r.upper_bound_mm).toBeLessThan(0.45)
  const ball=await run('swept-ball-face-clearance')
  expect(ball.signed_margin_mm[0]).toBeCloseTo(r.lower_bound_mm-0.2,8)
  expect(ball.signed_margin_mm[1]).toBeCloseTo(r.upper_bound_mm-0.2,8)
},180000)
