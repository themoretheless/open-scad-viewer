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

it('distinguishes a segment speed violation from a permitted join jump',async()=>{
  const r=await join('check-timed-chain')
  expect(r).toMatchObject({status:'Exceeds',segmentStatuses:['Within','Exceeds'],joinStatus:'Within'})
  expect(r.cells).toBeLessThanOrEqual(1000)
  expect((await join('check-timed-chain',s=>s.replace('durations:[1,1]','durations:[1,2]'))).status).toBe('Within')
})
it('classifies a rational right angle in a knot, a join and a closed chain',async()=>{
  const {knots}=await join('inspect-knot-turns')
  expect(knots[0]).toMatchObject({parameter:1,status:'Exceeds'})
  const r=await join('inspect-join-turn')
  expect(r.turnStatus).toBe('Exceeds')
  for(const c of [knots[0].cosine,r.cosine]){
    expect(c[0]).toBeLessThanOrEqual(0)
    expect(c[1]).toBeGreaterThanOrEqual(0)
    expect(c[1]-c[0]).toBeLessThan(1e-8)
  }
  const chain=await join('inspect-chain-turns')
  expect(chain.status).toBe('Exceeds')
  expect(chain.joins.map((j:{segments:number[]})=>j.segments)).toEqual([[0,1],[1,0]])
  expect(chain.joins[1].gapStatus).toBe('Exceeds')
})
