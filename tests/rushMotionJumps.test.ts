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

it('scales cubic jerk by duration and retains the proved zero lower derivatives',async()=>{
  for(const [name,v,status] of [['inspect-knot-jumps',6,'Exceeds'],['inspect-timed-knot-jumps',0.75,'Within']] as const){
    const {knots}=await join(name)
    expect(knots).toHaveLength(1)
    expect(knots[0]).toMatchObject({parameter:1,statuses:['Within','Within',status]})
    expect(knots[0].jumps.slice(0,2)).toEqual([[0,0],[0,0]])
    expect(knots[0].jumps[2][0]).toBeLessThanOrEqual(v)
    expect(knots[0].jumps[2][1]).toBeGreaterThanOrEqual(v)
  }
})
it('checks both independent segment clocks and includes the closing join',async()=>{
  expect((await join('inspect-motion-join')).statuses[0]).toBe('Exceeds')
  expect((await join('inspect-motion-join',s=>s.replace('durations:[1,1]','durations:[1,2]'))).statuses[0]).toBe('Within')
  const r=await join('inspect-motion-chain')
  expect(r.status).toBe('Exceeds')
  expect(r.joins.map((j:{segments:number[]})=>j.segments)).toEqual([[0,1],[1,0]])
  expect(r.joins[1].positionStatus).toBe('Exceeds')
})
