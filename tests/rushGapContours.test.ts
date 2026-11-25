import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/curvature-threshold-contour.r','utf8')
async function reports(text:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return scene.measurements!.map(m=>JSON.parse(m.report))
}
it('encloses the analytic threshold crossing of a parabolic cylinder in its original chart',async()=>{
  const rows=await reports(source)
  const root=(Math.sqrt(Math.cbrt(4)-1)/2-0.25)/0.75
  const edge=rows.find(r=>r.roots)
  expect(edge.varyingDomain).toEqual([2,6])
  expect(edge.roots.brackets[0].variation).toBe(1)
  const sampled=rows.find(r=>r.samples)
  expect(sampled.graph).toMatchObject({status:'unique',increasing:false})
  expect(sampled.samples.map((s:{transverse:number})=>s.transverse)).toEqual([7,8,9])
  for(const [lo,hi] of [edge.roots.brackets[0].unitInterval,...sampled.samples.map((s:{unitRootInterval:number[]})=>s.unitRootInterval)]) {
    expect(lo).toBeLessThanOrEqual(root)
    expect(hi).toBeGreaterThanOrEqual(root)
  }
  const slope=rows.find(r=>r.slope).slope
  expect(slope[0]).toBeLessThanOrEqual(0)
  expect(slope[1]).toBeGreaterThanOrEqual(0)
})
it('carries absence of a contour through graph and sampling and rejects invalid spans',async()=>{
  const rows=await reports(source.replaceAll('gap_level_per_mm2:1/','gap_level_per_mm2:4/'))
  expect(rows.filter(r=>r.graph).every(r=>r.graph.status==='empty')).toBe(true)
  await expect(reports(source.replace('knot_span:[2,1]','knot_span:[0,1]'))).rejects.toThrow()
})
