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
it('returns one canonical nonbinary line clock in either input order',async()=>{
  const line={status:'line',clockAxis:2,origin:['-1/3','1/3','0'],direction:['2/3','1/3','1']}
  expect(await build('intersect-support-planes')).toMatchObject(line)
  expect(await build('intersect-support-planes','left.intersect_support_planes(target_geometry:right','right.intersect_support_planes(target_geometry:left')).toMatchObject(line)
})
it('distinguishes parallel planes from coincident planes',async()=>{
  const original='[[[0,0,1],[0,1,0]],[[1,0,3],[1,1,2]]]'
  expect((await build('intersect-support-planes',original,'[[[0,0,1],[0,1,2]],[[1,0,2],[1,1,3]]]')).status).toBe('parallel')
  expect((await build('intersect-support-planes',original,'[[[0,0,0],[0,1,1]],[[1,0,1],[1,1,2]]]')).status).toBe('coincident')
})
