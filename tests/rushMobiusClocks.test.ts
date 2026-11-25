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
it('preserves the Bezier domain and moves each original curve knot to the requested clock', async () => {
  expect((await build('mobius-bezier-curve')).knots).toEqual([2,2,5,5])
  for(const name of ['mobius-spline-curve','fit-mobius-curve']) {
    expect((await build(name)).knots).toEqual([0,0,1.5,3,3])
  }
  await expect(build('mobius-spline-curve','clock_factor:2','clock_factor:0')).rejects.toThrow()
})
it('applies independent U and V clocks and refuses invalid interior landmarks', async () => {
  expect((await build('mobius-bezier-surface')).uKnots).toEqual([0,0,1,1])
  for(const name of ['mobius-spline-surface','fit-mobius-surface']) {
    expect(await build(name)).toMatchObject({uKnots:[0,0,1.5,3,3],vKnots:[0,0,1.5,3,3]})
  }
  await expect(build('fit-mobius-surface','[2,1.5]','[2,3]')).rejects.toThrow()
})
