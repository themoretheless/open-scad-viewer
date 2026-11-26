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
it('returns common closed edge intervals and isolates zero-width contacts',async()=>{
  expect(await build('shared-boundary-spans')).toMatchObject({status:'transverse',proven:[['0','2']]})
  expect(await build('shared-boundary-contacts')).toMatchObject({status:'transverse',proven:[['0','0'],['2','2']]})
})
it('distinguishes proven boundary coverage from its enclosing intervals',async()=>{
  expect((await build('refine-boundary-section')).proven).toEqual([['0','2']])
  expect(await build('bound-boundary-section')).toMatchObject({proven:[['0','2']],possible:[['0','2']]})
  await expect(build('bound-boundary-section','parameter_width:0.00001','parameter_width:0')).rejects.toThrow()
})
