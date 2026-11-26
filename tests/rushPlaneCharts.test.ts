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
it('classifies active affine dimension and retains exact rational inverse coefficients',async()=>{
  expect((await build('active-surface-hull')).hull).toMatchObject({dimension:2})
  expect((await build('active-surface-hull','[2,2,3]','[2,2,4]')).hull.dimension).toBe(3)
  expect(await build('project-plane-chart')).toMatchObject({retainedAxes:[0,1],omittedAxis:2,offset:'2',slopes:['1/2','0'],firstPole:[0,0,0]})
  await expect(build('project-plane-chart','[2,2,3]','[2,2,4]')).rejects.toThrow(/plane/)
})
it('lifts edited coordinates and projects members in a common chart',async()=>{
  expect(await build('lift-plane-chart')).toMatchObject({firstPole:[4,0,4]})
  expect(await build('shared-plane-chart')).toMatchObject({firstPole:[4,0,0],slopes:['1/2','0']})
  await expect(build('shared-plane-chart','[6,2,5]','[6,2,6]')).rejects.toThrow(/common plane/)
})
