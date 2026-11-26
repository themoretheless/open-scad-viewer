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
it('uses the direction curve to certify rotating hull support and profiled rounding',async()=>{
  for(const [name,gap] of [['minimum-rotating-hull-gap',-0.1],['minimum-profiled-rotating-gap',-0.2]] as const){
    const r=await join(name)
    encloses(r.gap,gap)
    expect(r.witnessPlane).toBe(0)
    expect(r.workUsed).toBeLessThanOrEqual(10000)
  }
  for(const name of ['rotating-hull-workspace','profiled-rotating-workspace']){
    const r=await join(name)
    expect(r.cells[0].status).toBe('Within')
    expect(r.cells.at(-1).status).toBe('Exceeds')
  }
})
it('classifies obstacles across the rotation and rejects an out-of-plane direction',async()=>{
  for(const name of ['rotating-hull-obstacles','rounded-rotating-obstacles','profiled-rotating-obstacles']){
    const r=await join(name)
    expect(r.cells[0].status).toBe('Within')
    expect(r.cells.at(-1).status).toBe('Exceeds')
    expect(r.cells.at(-1).obstacles).toHaveLength(1)
  }
  await expect(join('rotating-hull-workspace',s=>s.replace('[0,1,0]','[0,1,1]'))).rejects.toThrow()
})
