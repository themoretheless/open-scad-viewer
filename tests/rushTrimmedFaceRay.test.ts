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
it('reports an exact forward hit and separates the hole and boundary cases',async()=>{
  expect(await build('cast-trimmed-face-ray')).toMatchObject({status:'hit',parameter:'1/3',point:['1/4','1','0']})
  expect(await build('cast-trimmed-face-ray','[0.25,1,1]','[1,1,1]')).toMatchObject({status:'miss',parameter:'1/3'})
  expect((await build('cast-trimmed-face-ray','[0.25,1,1]','[0.5,1,1]')).status).toBe('boundaryUnresolved')
})
it('distinguishes a backward intersection, a parallel line and a coplanar line',async()=>{
  expect(await build('cast-trimmed-face-ray','[0,0,-3]','[0,0,3]')).toMatchObject({status:'behind',parameter:'-1/3'})
  expect((await build('cast-trimmed-face-ray','[0,0,-3]','[1,0,0]')).status).toBe('parallel')
  expect((await build('cast-trimmed-face-ray','[0.25,1,1],ray_direction:[0,0,-3]','[0.25,1,0],ray_direction:[1,0,0]')).status).toBe('coplanar')
})
