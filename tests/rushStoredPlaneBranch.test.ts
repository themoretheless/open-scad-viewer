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
it('extrudes the spatial section and certifies the stored curve against the plane',async()=>{
 const r=await build('store-plane-branch')
 expect(r).toMatchObject({xyzErrorUpper:'0',planeResidualUpper:'0',spatial:{degree:1,controlPoints:[[0.5,0,0],[0.5,1,0]]}})
})
it('retains synchronized absolute UV and spatial control points',async()=>{
 const r=await build('pair-plane-branch')
 expect(r.gapTolerance).toBe('0')
 expect(r.pcurve.controlPoints).toEqual([[0.5,0],[0.5,1]])
 expect(r.spatial.knots).toEqual(r.pcurve.knots)
})
