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
it('encloses physical signed area of a tilted patch minus its hole',async()=>{
  const r=await build('signed-planar-face-area')
  const expected=3*Math.sqrt(1.25)
  expect(r.converged).toBe(true)
  expect(r.signedAreaMm2[0]).toBeLessThanOrEqual(expected)
  expect(r.signedAreaMm2[1]).toBeGreaterThanOrEqual(expected)
  expect(r.signedAreaMm2[1]-r.signedAreaMm2[0]).toBeLessThanOrEqual(0.00001)
})
it('retains contour orientation and refuses an insufficient span budget',async()=>{
  const r=await build('signed-planar-face-area','[1,0],[1,1],[0,1]','[0,1],[1,1],[1,0]')
  const expected=-5*Math.sqrt(1.25)
  expect(r.signedAreaMm2[0]).toBeLessThanOrEqual(expected)
  expect(r.signedAreaMm2[1]).toBeGreaterThanOrEqual(expected)
  await expect(build('signed-planar-face-area','max_cells:1024','max_cells:1')).rejects.toThrow(/cell budget/)
})
