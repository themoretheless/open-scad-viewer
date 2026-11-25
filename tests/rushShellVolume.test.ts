import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/signed-box-volume.r','utf8')
async function measure(text:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return scene.measurements!.map(m=>JSON.parse(m.report)).find(r=>r.boundsMm3)
}
it('encloses signed box volume in cubic millimetres and reverses under global orientation reversal',async()=>{
  for(const [text,expected] of [[source,30],[source.replace('[0,2,4]','[1,3,5]'),-30]] as const) {
    const r=await measure(text)
    expect(r.faceCount).toBe(6)
    expect(r.withinTolerance).toBe(true)
    expect(r.boundsMm3[0]).toBeLessThanOrEqual(expected)
    expect(r.boundsMm3[1]).toBeGreaterThanOrEqual(expected)
    expect(r.errorUpperMm3).toBeLessThanOrEqual(1e-5)
  }
})
it('supports directed flux and rejects a nonexistent reversed face',async()=>{
  expect((await measure(source.replace('tolerance_mm3:','flux_axis:[0,0,2],tolerance_mm3:'))).signedVolumeMm3).toBeCloseTo(30,9)
  await expect(measure(source.replace('[0,2,4]','[0,2,6]'))).rejects.toThrow()
})
