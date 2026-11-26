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
it('projects members of a multi-face boundary in the same Cartesian plane',async()=>{
  expect(await build('project-planar-wire')).toMatchObject({firstPole:[4,0],retainedAxes:[0,1],loopPieces:[4],domain:[3,4]})
  expect(await build('project-planar-wire','patch_index:1','patch_index:0')).toMatchObject({firstPole:[0,0]})
  await expect(build('project-planar-wire','contour_counts:[1,1]','contour_counts:[2,1]')).rejects.toThrow(/contour count/)
})
it('lifts edited planar contours to the source plane and rejects broken closure',async()=>{
  expect(await build('lift-planar-wire')).toMatchObject({firstPole:[4,0,4],domain:[3,7],loopPieces:[1]})
  await expect(build('lift-planar-wire','[4,2],[4,0]','[4,2],[5,0]')).rejects.toThrow(/OpenLoop/)
})
