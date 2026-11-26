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
it('finds a reversed shared spatial trace with unequal clock domains',async()=>{
  expect(await build('compare-seam-orientations')).toMatchObject({forward:{status:'different'},reversed:{status:'equal'}})
})
it('retains forward agreement and reports both budget refusals independently',async()=>{
  expect(await build('compare-seam-orientations','[[0,1],[0,0]]','[[0,0],[0,1]]')).toMatchObject({forward:{status:'equal'},reversed:{status:'different'}})
  const limited = await build('compare-seam-orientations','1000000','1')
  expect(limited.forward.status).toBe('clockRefused')
  expect(limited.reversed.status).toBe('clockRefused')
  expect(limited.workUsed).toBeLessThanOrEqual(1)
})
