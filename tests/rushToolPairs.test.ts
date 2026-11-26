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
it('retains the controlling tool indices and brackets first, last and continuous contact',async()=>{
  const gap=await join('minimum-tool-pair-gap')
  encloses(gap.gap,-0.25)
  encloses(gap.witnessTime,0.5)
  expect(gap.tools[0]).toBeLessThan(gap.tools[1])
  const r=await join('tool-pair-intervals')
  expect(new Set(r.cells.map((c:{tools:number[]})=>c.tools.join(','))).size).toBe(3)
  const half=0.25/Math.sqrt(2)
  for(const [name,t] of [['first-tool-pair-contact',0.5-half],['last-tool-pair-contact',0.5+half]] as const){
    const event=await join(name)
    expect(event.kind).toBe('Confirmed')
    encloses(event.time,t)
  }
  for(const name of ['tool-contact-duration','longest-tool-contact'])encloses((await join(name)).duration,2*half)
})
it('counts distinct simultaneous pairs, bounds threshold windows and reports the witness pairs',async()=>{
  expect((await join('peak-tool-pair-count')).count).toEqual([3,3])
  encloses((await join('simultaneous-tool-duration')).duration,0.25)
  const windows=await join('simultaneous-tool-windows')
  expect(windows.confirmed.some(([a,b]:number[])=>a!<=0.5 && b!>=0.5)).toBe(true)
  expect(windows.possible[0][0]).toBeLessThanOrEqual(0.375)
  expect(windows.possible.at(-1)[1]).toBeGreaterThanOrEqual(0.625)
  const peak=await join('peak-tool-contacts')
  expect(peak.count).toEqual([3,3])
  expect(peak.witness.tools).toHaveLength(3)
  const at=await join('tool-pairs-at-time')
  expect(at.confirmed).toEqual([[0,1],[0,2],[1,2]])
  await expect(join('minimum-tool-pair-gap',s=>s.replace('durations:[1,1,1]','durations:[1,1]'))).rejects.toThrow()
})
