import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string,alter=(s:string)=>s){
  const source=alter(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.volume).toBeCloseTo(24,6)
  return JSON.parse(scene.measurements![0]!.report)
}
it('projects onto a direction and a translated scaled frame and compares two bodies',async()=>{
  expect((await measure('project-polyhedral-body')).dotBounds).toEqual(['1','3'])
  expect((await measure('measure-polyhedral-frame')).frameBounds).toEqual([['0','1'],['0','3'],['0','1']])
  expect(await measure('measure-polyhedral-axis-gap')).toMatchObject({signedGap:'2',planeDistanceSquaredMm2:'4'})
  await expect(measure('measure-polyhedral-frame',s=>s.replace('[2,0,0]','[0,1,0]'))).rejects.toThrow()
})
it('classifies a section plane and attributes point and coplanar contacts to material edges',async()=>{
  expect(await measure('classify-polyhedral-plane')).toMatchObject({relation:'Crosses',residualBounds:['-1','1'],distanceSquaredMm2:'0'})
  const {hits}=await measure('intersect-polyhedral-edges')
  expect(hits).toHaveLength(4)
  for(const hit of hits){expect(hit.pointMm[0]).toBe('2');expect(hit.fraction).toBe('1/2')}
  const touching=await measure('intersect-polyhedral-edges',s=>s.replace('plane_offset_mm:2','plane_offset_mm:1'))
  expect(touching.hits.filter((hit:{kind:string})=>hit.kind==='coplanar')).toHaveLength(4)
})
