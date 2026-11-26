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
it('encloses the signed workspace clearance for a moving ball and a rounded vertex hull',async()=>{
  for(const [name,expected] of [['minimum-profiled-ball-workspace',-0.35],['minimum-profiled-hull-workspace',-0.4]] as const){
    const r=await join(name)
    encloses(r.gap,expected)
    expect(r.witnessParameter).toBe(1)
    expect(r.witnessPlane).toBe(0)
    expect(r.workUsed).toBeLessThanOrEqual(1000)
  }
})
it('classifies entry into the workspace wall and preserves the controlling plane index',async()=>{
  for(const name of ['profiled-ball-workspace','profiled-hull-workspace']){
    const r=await join(name)
    expect(r.cells[0].status).toBe('Within')
    expect(r.cells.at(-1).status).toBe('Exceeds')
    expect(r.cells.at(-1).planeResiduals).toHaveLength(1)
  }
  const r=await join('minimum-profiled-ball-workspace',s=>s.replace('[[1,0,0,0.75]]','[[0,1,0,2],[1,0,0,0.75]]'))
  expect(r.witnessPlane).toBe(1)
  await expect(join('profiled-hull-workspace',s=>s.replace('[0.5,0.2]','[0.5,-0.2]'))).rejects.toThrow()
})
