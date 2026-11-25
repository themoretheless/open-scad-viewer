import {readFileSync} from 'node:fs'
import {it,expect} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const example=(name:string)=>readFileSync('examples/rush/'+name+'.r','utf8')
async function inspect(code:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(code).document)
  expect(scene.meshes).toHaveLength(1)
  return new Map(scene.measurements!.map(r=>[r.operation,JSON.parse(r.report)]))
}
it('returns sphere highlight and reflection contours and three ordered isophote levels',async()=>{
  const text=example('surface-optical-analysis')
  const m=await inspect(text)
  const highlights=m.get('surface_highlight_lines'), reflections=m.get('surface_reflection_lines')
  expect(highlights.sampled).toBe(true)
  expect(highlights.lights[0].polylinesUv.length).toBeGreaterThan(0)
  expect(reflections.lines.polylinesUv).toEqual(highlights.lights[0].polylinesUv)
  const iso=m.get('surface_isophote_analysis')
  expect(iso.levels).toHaveLength(3)
  for(const l of iso.levels) expect(l.polylinesUv.length).toBeGreaterThan(0)
  expect(iso.meanSpacing).toBeGreaterThan(0)
  await expect(inspect(text.replace('lights:[[0,0,1]]','lights:[[0,0,0]]'))).rejects.toThrow()
})
it('localizes a plate perturbation and returns zero residual for the flat control',async()=>{
  const text=example('surface-dent-map')
  const dent=(await inspect(text)).get('surface_dent_detection')
  expect(dent.maxResidualPerMm2).toBeGreaterThan(0.02)
  expect(dent.zones.length).toBeGreaterThan(0)
  const flat=(await inspect(text.replace('[2,2,0.4]','[2,2,0]'))).get('surface_dent_detection')
  expect(flat).toMatchObject({maxResidualPerMm2:0,zones:[]})
})
