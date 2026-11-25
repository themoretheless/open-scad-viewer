import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=(kind:string)=>readFileSync('examples/rush/fit-'+kind+'-cloud.r','utf8')
async function fit(text:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  const r=JSON.parse(scene.measurements![0]!.report)
  expect(r.rmsMm).toBeLessThan(0.00001)
  expect(r.maxDeviationMm).toBeLessThan(0.00002)
  return r.parameters
}
it('fits the analytic radii, cone angle, plane and translated torus',async()=>{
  const s=await fit(source('sphere'))
  expect(s.radiusMm).toBeCloseTo(2,6)
  const c=await fit(source('cylinder'))
  expect(c.radiusMm).toBeCloseTo(3,6)
  expect(Math.abs(c.axis[2])).toBeCloseTo(1,6)
  const cone=await fit(source('cone'))
  expect(cone.halfAngleRad).toBeCloseTo(0.35,6)
  expect(cone.vertex[2]).toBeCloseTo(-1,5)
  const t=await fit(source('torus'))
  expect(t.majorRadiusMm).toBeCloseTo(4,5)
  expect(t.minorRadiusMm).toBeCloseTo(1,5)
  const plane=await fit(source('plane'))
  expect(Math.abs(plane.normal[2])).toBeCloseTo(1,8)
  expect(plane.center[2]).toBeCloseTo(5,8)
})
it('rejects degenerate samples and an invalid fitted plane display extent',async()=>{
  await expect(fit(source('sphere').replace(/\[[^\[\]]+mm\]/g,'[0mm,0mm,0mm]'))).rejects.toThrow()
  await expect(fit(source('plane').replace('width:4mm','width:0mm'))).rejects.toThrow(/positive/i)
})
