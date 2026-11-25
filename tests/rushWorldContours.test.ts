import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/world-curvature-contour.r','utf8')
async function build(text:string){return buildTextNurbsScene(compileRushFrontend(text).document)}
it('encloses length, point distance, world bounds and actual rendered chords',async()=>{
  const scene=await build(source)
  const reports=scene.measurements!.map(m=>JSON.parse(m.report))
  const x=Math.sqrt(Math.cbrt(4)-1)/2
  function encloses(bounds:number[],number:number){
    expect(bounds[0]).toBeLessThanOrEqual(number)
    expect(bounds[1]).toBeGreaterThanOrEqual(number)
  }
  encloses(reports.find(r=>r.lengthBoundsMm).lengthBoundsMm,1)
  encloses(reports.find(r=>r.distanceBoundsMm).distanceBoundsMm,Math.hypot(x,x*x))
  const bounds=reports.find(r=>r.worldBoundsMm).worldBoundsMm
  encloses(bounds[0],x);encloses(bounds[1],0);encloses(bounds[1],1);encloses(bounds[2],x*x)
  const path=reports.find(r=>r.pointsMm)
  expect(path.pointsMm).toHaveLength(3)
  expect(path.worldErrorUpperMm).toHaveLength(2)
  expect(scene.meshes).toHaveLength(1)
  expect(Array.from(scene.meshes[0]!.vertices).filter((_,i)=>i%6===0).every(v=>Math.abs(v-x)<0.001)).toBe(true)
})
it('refuses to construct a downstream curve when the original chart has no contour',async()=>{
  await expect(build(source.replaceAll('gap_level_per_mm2:1/','gap_level_per_mm2:4/'))).rejects.toThrow()
})
