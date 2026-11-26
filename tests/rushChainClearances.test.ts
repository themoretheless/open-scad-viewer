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
it('converts the second curve window to elapsed chain time',async()=>{
  const r=await join('chain-violation-duration')
  encloses(r.total.duration,2)
  encloses(r.segments[0].duration,0)
  encloses(r.segments[1].byObstacle[1],2)
  const first=await join('first-chain-violation')
  expect(first.kind).toBe('Confirmed')
  encloses(first.time,3)
  expect(first.witnessSegment).toBe(1)
  expect(first.obstacle).toBe(1)
})
it('retains segment and obstacle witnesses and rejects missing segment clocks',async()=>{
  const r=await join('minimum-chain-gap')
  encloses(r.gap,-0.15)
  encloses(r.witnessTime,4)
  expect(r.witnessSegment).toBe(1)
  expect(r.witnessObstacle).toBe(1)
  expect(r.withinTolerance).toBe(true)
  await expect(join('minimum-chain-gap',s=>s.replace('durations:[2,4]','durations:[2]'))).rejects.toThrow()
})
