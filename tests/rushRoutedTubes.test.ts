import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string,transform=(s:string)=>s) {
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(readFileSync('examples/rush/'+name+'.r','utf8'))).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}
it('includes tube end caps and independent centerline witnesses',async()=>{
  const r=await run('routed-tube-pair')
  expect(r.signed_margin_mm[0]).toBeCloseTo(1.5,5)
  expect(r.signed_margin_mm[1]).toBeCloseTo(1.5,5)
  expect(r.points[1][0]).toBe(4)
  expect(Math.abs(r.points[1][1]-3)).toBeLessThan(0.000001)
  expect(r.points[1][2]).toBe(0)
},180000)
it('identifies the penetrating pair under a shared work budget',async()=>{
  const r=await run('routed-tube-bundle')
  expect(r.witness_pair).toEqual([1,2])
  expect(r.signed_margin_mm[0]).toBeCloseTo(-0.75,5)
  expect(r.signed_margin_mm[1]).toBeCloseTo(-0.75,5)
  await expect(run('routed-tube-bundle',s=>s.replace('max_cells:20000','max_cells:2'))).rejects.toThrow(/budget/i)
},180000)
