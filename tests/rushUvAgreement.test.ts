import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function compare(name:string,from='',to=''){
  const text=readFileSync('examples/rush/'+name+'.r','utf8').replace(from,to)
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report).outcome
}
it('proves equality in the original parameter and locates a spatial edge disagreement',async()=>{
  expect(await compare('verify-uv-edge')).toEqual({status:'equal'})
  expect(await compare('verify-uv-edge','control_points:[[1,0,0],[1,1,0]]','control_points:[[1,0,0],[1,1,1]]')).toMatchObject({status:'different',coordinate:2,parameter:[2,6]})
  expect(await compare('compare-uv-traces')).toEqual({status:'equal'})
})
it('distinguishes positional continuity from tangent-plane agreement',async()=>{
  const flat='[[2,0,0],[2,1,0]]', tilted='[[2,0,1],[2,1,1]]'
  expect(await compare('compare-uv-tangents')).toEqual({status:'equal'})
  expect(await compare('compare-uv-traces',flat,tilted)).toEqual({status:'equal'})
  expect(await compare('compare-uv-tangents',flat,tilted)).toMatchObject({status:'tangentPlaneMismatch',parameter:[2,6]})
})
