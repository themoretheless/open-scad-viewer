import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function join(name:string, transform=(s:string)=>s){
  const source=transform(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}

function encloses(bounds:number[], value:number){
  expect(bounds[0]).toBeLessThanOrEqual(value)
  expect(bounds[1]).toBeGreaterThanOrEqual(value)
  expect(bounds[1]!-bounds[0]!).toBeLessThan(0.02)
}
it('bounds contact length and duration for a straight path at constant speed',async()=>{
  for(const name of ['violation-path-length','violation-obstacle-lengths','violation-path-duration']){
    const r=await join(name)
    const duration=name==='violation-path-duration'
    encloses(duration?r.duration:r.length,duration?2:0.5)
    if(r.byObstacle){
      encloses(r.byObstacle[0],0)
      encloses(r.byObstacle[1],duration?2:0.5)
    }
    expect(r.workUsed).toBeLessThanOrEqual(1000)
  }
})
it('counts overlapping violations once and rejects a nonpositive clock',async()=>{
  const r=await join('violation-obstacle-lengths',s=>s.replace('[1,2,0,2,3,1]','[0.5,0,0,0.1]'))
  encloses(r.length,0.5)
  for(const bounds of r.byObstacle) encloses(bounds,0.5)
  await expect(join('violation-path-duration',s=>s.replace('duration_s:4','duration_s:0'))).rejects.toThrow()
})
