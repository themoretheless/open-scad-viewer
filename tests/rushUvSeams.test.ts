import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(kind:string,before='',after=''){
  const text=readFileSync('examples/rush/'+kind+'-uv-seam.r','utf8').replace(before,after)
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
it('distinguishes tangency from crossing and encloses both roots in absolute parameters',async()=>{
  expect((await build('classify')).spans[0].contact).toMatchObject({crossings:1,touches:1,atStart:false,atEnd:false})
  const roots=(await build('isolate')).spans[0].contact.roots
  expect(roots.map((r:{crosses:boolean})=>r.crosses)).toEqual([false,true])
  for(const [i,t] of [3,4].entries()){
    expect(roots[i].domain[0]).toBeLessThanOrEqual(t)
    expect(roots[i].domain[1]).toBeGreaterThanOrEqual(t)
    expect(roots[i].domain[1]-roots[i].domain[0]).toBeLessThanOrEqual(0.001)
  }
})
it('returns the selected exact seam piece and refuses budget exhaustion',async()=>{
  expect(await build('split')).toMatchObject({pieceCount:3,side:'above',domain:[4,6]})
  expect(await build('split','piece:2','piece:1')).toMatchObject({side:'below',domain:[3,4]})
  await expect(build('split','max_work:1000000','max_work:1')).rejects.toThrow()
})
