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
it('samples a nonbinary transverse coordinate without rounding',async()=>{
  const r=await build('sample-plane-branch')
  expect(r.samples[1]).toEqual({transverse:'1/3',root:['1/2','1/2']})
  await expect(build('sample-plane-branch','[1,3]','[3,1]')).rejects.toThrow()
})
it('certifies UV and spatial chords against the original rational surface',async()=>{
  const uv=[['1/2','0'],['1/2','1']]
  expect(await build('plane-branch-uv-polyline')).toMatchObject({uv,normalizedErrorUpper:'0'})
  expect(await build('plane-branch-xyz-polyline')).toMatchObject({uv,xyz:[['1/2','0','0'],['1/2','1','0']],xyzErrorUpper:'0'})
})
