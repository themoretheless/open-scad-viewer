import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string,offset=2){
  const source=readFileSync('examples/rush/'+name+'.r','utf8').replace('plane_offset_mm:2','plane_offset_mm:'+offset)
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('joins four face crossings into one attributed material loop and rejects tangency',async()=>{
  const {faces}=await measure('section-polyhedral-faces')
  expect(faces).toHaveLength(4)
  for(const f of faces) expect(f.segmentsMm).toHaveLength(1)
  const {loops}=await measure('assemble-polyhedral-section-loops')
  expect(loops).toHaveLength(1)
  expect(new Set(loops[0].faces).size).toBe(4)
  expect(loops[0].pointsMm.map((p:string[])=>p.join(',')).sort()).toEqual(['2,2,3','2,2,7','2,5,3','2,5,7'])
  await expect(measure('section-polyhedral-faces',1)).rejects.toThrow()
})
it('measures the retained section and distinguishes an empty slice',async()=>{
  expect(await measure('measure-polyhedral-section-area')).toMatchObject({areaSquaredMm4:'144',depths:[0]})
  expect(await measure('measure-polyhedral-section-centroid')).toMatchObject({centroidMm:['2','7/2','5']})
  expect(await measure('bound-polyhedral-section-perimeter')).toMatchObject({perimeterBoundsMm:['14','14']})
  expect(await measure('measure-polyhedral-section-centroid',9)).toMatchObject({centroidMm:null,loops:[]})
})
