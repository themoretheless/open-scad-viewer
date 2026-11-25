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
it('removes split seams in an ordered strip and a two-dimensional grid', async () => {
  expect((await build('exact-patch-strip')).removedKnots).toBe(1)
  expect((await build('exact-patch-grid')).alignmentInsertions).toBe(0)
  await expect(build('exact-patch-grid','[b,c,d]','[b,d,c]')).rejects.toThrow()
})
it('recovers the UV order of shuffled charts and rejects duplicate coverage', async () => {
  expect(await build('inspect-patch-grid')).toMatchObject({order:[1,3,2,0],uBreaks:[0,0.5,1],vBreaks:[0,0.25,1]})
  await expect(build('inspect-patch-grid','[a,c,b]','[a,c,d]')).rejects.toThrow()
})
