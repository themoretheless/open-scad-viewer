import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function join(name:string, transform=(s:string)=>s){
  const source=transform(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}

function encloses(bounds:number[],value:number) {
  expect(bounds[0]).toBeLessThanOrEqual(value)
  expect(bounds[1]).toBeGreaterThanOrEqual(value)
  expect(bounds[1]!-bounds[0]!).toBeLessThan(0.02)
}
it('bounds simultaneous clearance on the scheduled overlap and includes stationary endpoints on request',async()=>{
  for(const name of ['minimum-scheduled-gap','minimum-held-gap']){
    const r=await join(name)
    encloses(r.gap,-0.15)
    encloses(r.witnessTime,2)
    expect(r.workUsed).toBeLessThanOrEqual(1000)
  }
  for(const name of ['scheduled-contact-intervals','held-contact-intervals']){
    const r=await join(name)
    const holds=name.startsWith('held')
    encloses(r.cells[0].time[0],holds?0:1)
    encloses(r.cells.at(-1).time[1],holds?4:3)
    expect(r.cells.some((c:{status:string})=>c.status==='Exceeds')).toBe(true)
  }
})
it('distinguishes disjoint moving schedules from contact while either tool waits',async()=>{
  const offset=(s:string)=>s.replace('start_times:[0,1]','start_times:[0,5]')
  for(const name of ['minimum-scheduled-gap','scheduled-contact-intervals']){
    expect((await join(name,offset)).kind).toBe('NoOverlap')
  }
  encloses((await join('minimum-held-gap',offset)).gap,-0.15)
  await expect(join('held-contact-intervals',s=>s.replace('durations:[4,2]','durations:[4,0]'))).rejects.toThrow()
})
