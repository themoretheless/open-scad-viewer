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
it('brackets the last violation and maps classifications to each segment clock',async()=>{
  const last=await join('last-chain-violation')
  expect(last.kind).toBe('Confirmed')
  encloses(last.time,5)
  expect(last.latestSegment).toBe(1)
  expect(last.obstacle).toBe(1)
  const timeline=await join('chain-clearance-timeline')
  const cells=timeline.intervals
  expect(cells[0].segment).toBe(0)
  encloses(cells[0].time[0],0)
  encloses(cells.at(-1).time[1],6)
  expect(cells.some((c:{segment:number,status:string})=>c.segment===1 && c.status==='Exceeds')).toBe(true)
  for(let i=1;i<cells.length;i++) expect(cells[i].time[0]).toEqual(cells[i-1].time[1])
})
it('measures a continuous violation across the shared segment endpoint and detects a clear scene',async()=>{
  encloses((await join('longest-chain-violation')).duration,2)
  const seam=await join('longest-chain-violation',s=>s.replace('[1.5,0,0,0.1]','[1,0,0,0.1]'))
  encloses(seam.duration,1.5)
  const clear=await join('last-chain-violation',s=>s.replace('[1.5,0,0,0.1]','[1.5,5,0,0.1]'))
  expect(clear.kind).toBe('Clear')
})
