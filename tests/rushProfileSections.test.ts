import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('returns oriented polygons in the requested inclined or longitudinal plane',async()=>{
  for(const [name,parallel] of [['section-profile-extrusion',false],['section-profile-extrusion-parallel',true]] as const){
    const {cellsMm}=await measure(name)
    let signed=0
    for(const cell of cellsMm){
      const points=cell.map((p:string[])=>p.map(s=>{const [n,d='1']=s.split('/');return Number(n)/Number(d)}))
      for(let i=0;i<points.length;i++){
        const [x,y,z]=points[i],b=points[(i+1)%points.length]
        expect(parallel?y:y+z).toBe(parallel?2:3)
        signed+=parallel?z*b[0]-x*b[2]:x*b[1]-y*b[0]
      }
    }
    expect(signed).toBe(parallel?12:8)
  }
})
it('conserves exact volume for partial cuts and planes outside the solid',async()=>{
  expect(await measure('split-profile-extrusion-volume')).toMatchObject({belowMm3:'3/4',aboveMm3:'45/4',totalMm3:'12'})
  expect(await measure('split-profile-extrusion-volume',s=>s.replace('cut_offset:"3/2"','cut_offset:5'))).toMatchObject({belowMm3:'12',aboveMm3:'0',totalMm3:'12'})
})
