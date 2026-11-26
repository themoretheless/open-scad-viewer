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
it('orders both roots with shared boundary incidence and a separating witness',async()=>{
  const r=await build('partition-face-events')
  expect(r.status).toBe('transverse')
  expect(r.events.map((e:any)=>[e.clock.upperRoot,e.onBoundary])).toEqual([[false,[true,true]],[true,[true,true]]])
  expect(r.samples).toHaveLength(1)
})
it('encloses both irrational points on the physical intersection line',async()=>{
  const r=await build('enclose-face-events')
  expect(r.events).toHaveLength(2)
  for(const e of r.events){
    expect(e.enclosure.xyz).toEqual([e.enclosure.parameter,['0','0'],['0','0']])
    const [n,d]=e.enclosure.diameterSquared.split('/').map(BigInt)
    expect(n>0n && 1000000n*n<=d).toBe(true)
  }
})
