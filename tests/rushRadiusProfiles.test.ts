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
it('bounds clearance under a linear or piecewise linear radius and identifies the controlling obstacle',async()=>{
  for(const name of ['minimum-variable-sphere-gap','minimum-variable-scene-gap','minimum-profiled-scene-gap']){
    const r=await join(name)
    encloses(r.gap,name.includes('profiled')?-0.3:-0.2)
    expect(r.witnessParameter).toBeCloseTo(0.5)
    expect(r.witnessObstacle).toBe(name.includes('sphere')?0:1)
    expect(r.workUsed).toBeLessThanOrEqual(1000)
  }
})
it('classifies the profile break in source coordinates and rejects unordered or negative radii',async()=>{
  for(const name of ['variable-sphere-clearance','variable-scene-clearance','profiled-scene-clearance']){
    const r=await join(name)
    const cell=r.cells.find((c:{domain:number[]})=>c.domain[0]!<=0.5 && c.domain[1]!>=0.5)
    expect(cell.status).toBe('Exceeds')
    expect(cell.gaps).toHaveLength(name.includes('sphere')?1:2)
    expect(cell.gaps.at(-1)[0]).toBeLessThanOrEqual(name.includes('profiled')?-0.3:-0.2)
  }
  await expect(join('profiled-scene-clearance',s=>s.replace('[0.5,0.2]','[0,0.2]'))).rejects.toThrow()
  await expect(join('variable-scene-clearance',s=>s.replace('0.05,0.15','-0.05,0.15'))).rejects.toThrow()
})
