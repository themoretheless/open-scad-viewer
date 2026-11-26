import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,before='',after=''){
  const text=readFileSync('examples/rush/'+name+'.r','utf8').replace(before,after)
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
function encloses(r:number[],x:number){expect(r[0]).toBeLessThanOrEqual(x);expect(r[1]).toBeGreaterThanOrEqual(x)}
it('adds the physical perimeter of every loop of an inclined face',async()=>{
  const r=await build('planar-face-perimeter')
  expect(r.converged).toBe(true)
  encloses(r.contoursMm[0],4+2*Math.sqrt(5))
  encloses(r.contoursMm[1],2+Math.sqrt(5))
  encloses(r.perimeterMm,6+3*Math.sqrt(5))
})
it('distinguishes a hole boundary from an orthogonal projection inside material',async()=>{
  const r=await build('planar-boundary-distance')
  encloses(r.distanceMm,Math.sqrt(1.25))
  expect(r.witness.contour).toBe(1)
  const hole=await build('planar-material-distance')
  expect(hole.classification.location).toBe('Outside')
  encloses(hole.distanceMm,Math.sqrt(1.25))
  const solid=await build('planar-material-distance','query_point:[1,1,1]','query_point:[0.25,1,1]')
  expect(solid.classification.location).toBe('Inside')
  encloses(solid.distanceMm,1)
})
