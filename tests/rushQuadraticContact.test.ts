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
it('identifies both exact roots across opposite contour parameterizations',async()=>{
  const report=await build('match-quadratic-contacts')
  expect(report.status).toBe('transverse')
  expect(report.points).toEqual([false,true].map(upperRoot=>({kind:'quadratic',linear:'-2/3',constant:'1/18',upperRoot})))
})
it('distinguishes nearby contacts after perturbing just one contour',async()=>{
  const report=await build('match-quadratic-contacts','[[1,0.9375],[0.5,0.1875],[0,0.5625]]','[[1,0.9375],[0.5,0.187744140625],[0,0.5625]]')
  expect(report).toMatchObject({status:'transverse',points:[]})
})
