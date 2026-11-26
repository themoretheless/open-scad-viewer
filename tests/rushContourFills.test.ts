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

it('certifies a single turn and distinguishes nonzero from even-odd fill for nested contours',async()=>{
  for(const name of ['orientation-winding','curve-winding-at-point','curve-winding-over-box']){
    const r=await join(name)
    expect(r.winding).toBe(1)
    expect(r.net[0]).toBeLessThanOrEqual(2*Math.PI)
    expect(r.net[1]).toBeGreaterThanOrEqual(2*Math.PI)
  }
  for(const name of ['contour-fill-at-point','contour-fill-over-box']){
    const r=await join(name)
    expect(r.winding).toBe(2)
    expect(r.filled).toBe(true)
    expect((await join(name,s=>s.replace('even_odd:false','even_odd:true'))).filled).toBe(false)
  }
})
it('signs nearest-contour distance by fill and retains the witness for the box center',async()=>{
  for(const name of ['signed-contour-distance','signed-contour-box-distance']){
    const r=await join(name)
    expect(r.signedDistance[0]).toBeLessThanOrEqual(-0.5)
    expect(r.signedDistance[1]).toBeGreaterThanOrEqual(-0.5)
    expect((r.center??r).witnessContour).toBe(1)
    const alt=await join(name,s=>s.replace('even_odd:false','even_odd:true'))
    expect(alt.signedDistance[0]).toBeGreaterThan(0)
  }
  await expect(join('curve-winding-at-point',s=>s.replace('closed:true','closed:false'))).rejects.toThrow()
})
