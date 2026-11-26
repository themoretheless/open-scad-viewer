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
it('encloses six known moments of a rational ellipse about its center',async()=>{
  const r=await build('bound-wire-moments')
  expect(r.converged).toBe(true)
  for(const [i,x] of [2*Math.PI,0,0,2*Math.PI,0,Math.PI/2].entries()){
    const [lo,hi]=r.moments[i]
    expect(lo).toBeLessThanOrEqual(x)
    expect(hi).toBeGreaterThanOrEqual(x)
    expect(hi-lo).toBeLessThanOrEqual(0.02)
  }
})
it('returns outward bounds when the cell budget is exhausted',async()=>{
  const r=await build('bound-wire-moments','max_cells:8192','max_cells:4')
  expect(r).toMatchObject({cells:4,converged:false})
  expect(r.moments[0][0]).toBeLessThanOrEqual(2*Math.PI)
  expect(r.moments[0][1]).toBeGreaterThanOrEqual(2*Math.PI)
})
