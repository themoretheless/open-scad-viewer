import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('builds the skew B-rep and measures its centroid and complete inertia tensor',async()=>{
  expect((await measure('build-profile-boolean-solid')).volumeMm3).toBe('12')
  expect(await measure('measure-profile-extrusion-inertia')).toMatchObject({
    volumeMm3:'12',centroidMm:['5/2','2','3/2'],inertiaMm5:[['13','0','-3'],['0','14','0'],['-3','0','9']],
  })
})
it('classifies skew material, clips a crossing and measures cap and wall area',async()=>{
  expect((await measure('locate-profile-extrusion-points')).locations).toEqual(['Interior','Boundary','Exterior'])
  expect((await measure('clip-profile-extrusion-path')).spans).toEqual([[['3/10','7/10']]])
  const area=await measure('bound-profile-extrusion-area')
  for(const [key,value] of Object.entries({capsMm2:8,wallsMm2:24,totalMm2:32})){
    const [lo,hi]=area[key].map((s:string)=>{const [n,d='1']=s.split('/');return Number(n)/Number(d)})
    expect(lo).toBeLessThanOrEqual(value);expect(hi).toBeGreaterThanOrEqual(value);expect(hi-lo).toBeLessThanOrEqual(0.000001)
  }
})
