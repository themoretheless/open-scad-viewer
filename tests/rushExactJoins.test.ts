import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function inspect(name: string, invalid = false) {
  let text = readFileSync('examples/rush/'+name+'.r','utf8')
  if(invalid) text=text.replace('target_geometry:right','target_geometry:left')
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
it('merges rational curve parts preserving the original nonunit domain',async()=>{
  expect(await inspect('exact-curve-join')).toMatchObject({seam:1,domain:[0,3]})
  await expect(inspect('exact-curve-join',true)).rejects.toThrow()
})
it('merges a tensor patch across the authored V seam',async()=>{
  expect(await inspect('exact-panel-join')).toMatchObject({direction:'v',seam:0.25})
  await expect(inspect('exact-panel-join',true)).rejects.toThrow()
})
