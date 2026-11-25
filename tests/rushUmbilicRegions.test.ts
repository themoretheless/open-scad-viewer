import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/umbilic-regions.r','utf8')
async function inspect(text:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return scene.measurements!.map(m=>JSON.parse(m.report))
}
it('classifies the entire plane as umbilic at original and refined resolution',async()=>{
  const rows=await inspect(source)
  expect(rows).toHaveLength(5)
  expect(rows.find(r=>r.cells?.[0].status).cells[0].status).toBe('EverywhereUmbilic')
  expect(rows.find(r=>r.regions?.[0].status).regions[0].unitDomain).toEqual([[0,1],[0,1]])
  expect(rows.find(r=>r.cells?.[0].comparison).cells[0].comparison).toBe('AtMost')
  expect(rows.find(r=>r.regions?.[0].comparison).regions[0].comparison).toBe('AtMost')
  const bounds=rows.find(r=>r.cells?.[0].boundsPerMm2).cells[0].boundsPerMm2
  expect(bounds[0]).toBeLessThanOrEqual(0)
  expect(bounds[1]).toBeLessThan(1e-12)
})
it('preserves ambiguity at finite depth and rejects exhausted proof work',async()=>{
  const saddle=source.replace('[1mm,1mm,0]]]','[1mm,1mm,1mm]]]')
  const r=await inspect(saddle)
  expect(r.find(row=>row.cells?.[0].status).cells[0].status).not.toBe('EverywhereUmbilic')
  await expect(inspect(source.replace('max_work:1000000','max_work:1'))).rejects.toThrow()
})
