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
it('certifies an exact shared edge within the requested spatial tolerance',async()=>{
  expect(await build('resolve-boundary-section')).toMatchObject({status:'transverse',proven:[['0','2']],possible:[['0','2']],refinement:{resolution:'WithinTolerance',passes:1,maxUnresolvedLengthSquared:'0'}})
})
it('rejects an invalid refinement pass limit',async()=>{
  await expect(build('resolve-boundary-section','max_passes:8','max_passes:55')).rejects.toThrow()
})
