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
it('integrates rational segments and subtracts an oppositely wound hole',async()=>{
  expect(await build('exact-wire-area')).toMatchObject({signedArea:'8'})
  expect(await build('exact-wire-mass')).toMatchObject({signedArea:'6',firstMoments:['12','6'],centroid:['2','1']})
  await expect(build('exact-wire-area','[0,2],[0,0]','[0,2],[1,0]')).rejects.toThrow(/UnprovedClosure/)
})
it('reports second moments about both origins as exact fractions',async()=>{
  expect(await build('exact-wire-inertia')).toMatchObject({aboutOrigin:['34','12','17/2'],aboutCentroid:['10','0','5/2']})
  await expect(build('exact-wire-inertia','1000000','1')).rejects.toThrow(/ResourceLimit/)
})
