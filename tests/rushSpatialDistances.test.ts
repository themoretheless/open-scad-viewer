import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string,transform=(s:string)=>s) {
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(readFileSync('examples/rush/'+name+'.r','utf8'))).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}
it('bounds distances between closed contours and identifies the closest pair',async()=>{
  const sets=await run('spatial-contour-sets-distance')
  expect(sets.bounds_mm[0]).toBeCloseTo(2,6)
  expect(sets.bounds_mm[1]).toBeCloseTo(2,6)
  const pair=await run('closest-spatial-contours')
  expect(pair.contours).toEqual([1,2])
  expect(pair.bounds_mm[1]).toBeCloseTo(1,6)
  const inspection=await run('inspect-spatial-clearances')
  expect(inspection.checks.map((row:{decision:string})=>row.decision)).toEqual(['meets_minimum','meets_minimum','below_minimum'])
  expect((await run('prove-spatial-clearance-along')).proved).toBe(true)
  expect((await run('prove-spatial-clearance-along',s=>s.replace('minimum_mm:2','minimum_mm:2.01'))).proved).toBe(false)
},180000)
it('certifies exact thresholds, strict violations and non-axis-aligned supports',async()=>{
  expect((await run('exact-diagonal-clearance')).checks[0].bounds_mm[0]).toBe(5)
  const strict=await run('exact-diagonal-clearance',s=>s.replace('minimum_mm:5','minimum_mm:5.000000000000001'))
  expect(strict.checks[0].decision).toBe('below_minimum')
  expect((await run('oblique-contour-support')).proved).toBe(true)
  expect((await run('automatic-contour-support')).checks[0].decision).toBe('meets_minimum')
},180000)
