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
it('reverses the absolute curve clock, including its internal knot',async()=>{
  expect((await build('affine-clock-curve')).knots).toEqual([0,0,2,6,6])
  await expect(build('affine-clock-curve','[6,0]','[2,2]')).rejects.toThrow(/distinct/)
})
it('changes the two surface clocks independently',async()=>{
  expect(await build('affine-clock-surface')).toMatchObject({uKnots:[0,0,2,6,6],vKnots:[-3,-3,-1,3,3]})
})
