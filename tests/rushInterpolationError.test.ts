import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/stored-grid-deviation.r','utf8')
async function inspect(text:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return scene.measurements!.map(m=>JSON.parse(m.report))
}
it('includes the measured off-plane corner in stored triangle and grid bounds',async()=>{
  const rows=await inspect(source)
  expect(rows).toHaveLength(3)
  expect(rows.find(r=>r.totalUpperMm===undefined).interpolationUpperMm).toBeLessThan(1e-12)
  for(const r of rows.filter(r=>r.totalUpperMm!==undefined)) {
    expect(r.totalUpperMm).toBeGreaterThanOrEqual(0.125)
    expect(r.totalUpperMm).toBeLessThan(0.125000001)
  }
  expect(rows.find(r=>r.worstCell)).toMatchObject({worstCell:[0,0],knotRectangles:1})
})
it('rejects unordered grid lines and evaluates bowed cells away from the four corners',async()=>{
  await expect(inspect(source.replace('u_lines:[0,1]','u_lines:[1,0]'))).rejects.toThrow()
  const warped=source.replace('[1mm,1mm,0]]]','[1mm,1mm,1mm]]]')
  const report=(await inspect(warped)).find(r=>r.totalUpperMm===undefined)
  expect(report.interpolationUpperMm).toBeGreaterThanOrEqual(0.25)
})
