import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string, before='', after='') {
  const text = readFileSync('examples/rush/'+name+'.r','utf8').replace(before,after)
  const scene = await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
it('applies one curve clock per span and moves absolute seam coordinates', async()=>{
  expect((await build('piecewise-clock-curve')).knots).toEqual([0,0,2,3,3])
  expect((await build('relayout-clock-curve')).knots).toEqual([-2,-2,1,5,5])
  await expect(build('piecewise-clock-curve','[2,0.5]','[2]')).rejects.toThrow()
})
it('relocates both tensor seams and refuses a reversed span',async()=>{
  expect(await build('piecewise-clock-surface')).toMatchObject({uKnots:[0,0,2,3,3],vKnots:[0,0,1,3,3]})
  expect(await build('relayout-clock-surface')).toMatchObject({uKnots:[-2,-2,1,5,5],vKnots:[4,4,5,8,8]})
  await expect(build('relayout-clock-surface','[4,5,8]','[4,3,8]')).rejects.toThrow()
})
