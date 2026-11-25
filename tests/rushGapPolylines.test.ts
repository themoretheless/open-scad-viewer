import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/curvature-uv-polylines.r','utf8')
async function inspect(text:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return scene.measurements!.map(m=>JSON.parse(m.report))
}
it('certifies fixed and adaptive UV chords across the entire nonunit span',async()=>{
  const rows=await inspect(source)
  const lines=rows.filter(r=>r.segmentErrorUpperUv)
  expect(lines).toHaveLength(3)
  expect(lines.filter(r=>r.stop==='ToleranceMet')).toHaveLength(2)
  for(const line of lines){
    expect(line.samples[0].transverse).toBe(7)
    expect(line.samples.at(-1).transverse).toBe(9)
    expect(line.segmentErrorUpperUv.length).toBe(line.samples.length-1)
    expect(Math.max(...line.segmentErrorUpperUv)).toBeLessThanOrEqual(0.01)
  }
  const second=rows.find(r=>r.secondDerivative).secondDerivative
  expect(second[0]).toBeLessThanOrEqual(0)
  expect(second[1]).toBeGreaterThanOrEqual(0)
})
it('rejects a sample schedule that omits part of the source span',async()=>{
  await expect(inspect(source.replace('transverse:[7,8,9]','transverse:[7,8]'))).rejects.toThrow()
  await expect(inspect(source.replace('max_vertices:64','max_vertices:1'))).rejects.toThrow()
})
