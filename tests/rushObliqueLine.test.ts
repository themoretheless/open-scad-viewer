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
it('distinguishes an exact double contact from a transverse irrational root',async()=>{
  const a=await build('isolate-uv-line')
  expect(a.spans[0].contact).toMatchObject({roots:[{domain:[4,4],crosses:false}],atStart:false,atEnd:false})
  const b=await build('isolate-uv-line','[3,4,-5]','[1,-1,0]')
  const [root]=b.spans[0].contact.roots
  const t=2+4*(Math.sqrt(2)-1)
  expect(root.crosses).toBe(true)
  expect(root.domain[0]).toBeLessThan(t)
  expect(root.domain[1]).toBeGreaterThan(t)
  expect(root.domain[1]-root.domain[0]).toBeLessThanOrEqual(1e-7)
})
it('preserves endpoint contacts and refuses a zero line normal',async()=>{
  expect((await build('isolate-uv-line','[3,4,-5]','[1,1,-1]')).spans[0].contact).toMatchObject({roots:[],atStart:true,atEnd:true})
  await expect(build('isolate-uv-line','[3,4,-5]','[0,0,-1]')).rejects.toThrow(/nonzero/)
})
