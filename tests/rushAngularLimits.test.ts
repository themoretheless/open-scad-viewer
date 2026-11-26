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

it('classifies three angular derivatives of a changing direction in source parameter order',async()=>{
  for(const name of ['angular-speed-limit','angular-acceleration-limit','angular-jerk-limit']){
    const r=await join(name)
    expect(r.cells[0].domain[0]).toBe(0)
    expect(r.cells.at(-1).domain[1]).toBe(1)
    expect(r.cells.some((c:{status:string})=>c.status==='Exceeds')).toBe(true)
    expect(r.cells.some((c:{status:string})=>c.status==='Within')).toBe(true)
  }
  const r=await join('angular-motion-limits')
  expect(r.cells[0].statuses).toEqual(['Exceeds','Within','Within'])
})
it('encloses the required dilation and tests an elapsed deadline against all angular limits',async()=>{
  for(const name of ['angular-slowdown-bounds','angular-duration-bounds']){
    const r=await join(name)
    expect(r.slowdown[0]).toBeLessThanOrEqual(2)
    expect(r.slowdown[1]).toBeGreaterThanOrEqual(2)
    expect(r.slowdown[1]).toBeLessThan(2.1)
    if(r.duration) expect(r.duration[1]).toBeCloseTo(r.slowdown[1])
  }
  expect((await join('angular-motion-deadline')).status).toBe('Within')
  expect((await join('angular-motion-deadline',s=>s.replace('deadline:3','deadline:1'))).status).toBe('Exceeds')
})
