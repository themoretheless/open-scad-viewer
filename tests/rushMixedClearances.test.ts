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

it('retains the original obstacle index and encloses the minimum signed gap',async()=>{
  const result=await join('minimum-mixed-gap')
  expect(result.witnessObstacle).toBe(1)
  expect(result.gap[0]).toBeLessThanOrEqual(-0.15)
  expect(result.gap[1]).toBeGreaterThanOrEqual(-0.15)
  expect(result.witnessParameter).toBeCloseTo(0.5)
  await expect(join('minimum-mixed-gap',s=>s.replace('max_cells:1000','max_cells:1'))).rejects.toThrow()
})
it('classifies both obstacles in order and subtracts the required margin',async()=>{
  for(const [name,margin] of [['mixed-path-clearance',0],['mixed-margin-clearance',0.1]] as const){
    const report=await join(name)
    expect(report.cells[0].domain[0]).toBe(0)
    expect(report.cells.at(-1).domain[1]).toBe(1)
    const center=report.cells.find((c:{domain:number[]})=>c.domain[0]!<=0.5 && c.domain[1]!>=0.5)
    expect(center.gaps).toHaveLength(2)
    expect(center.gaps[0][0]).toBeGreaterThan(1)
    expect(center.gaps[1][0]).toBeLessThanOrEqual(-0.15-margin)
    expect(center.gaps[1][1]).toBeGreaterThanOrEqual(-0.15-margin)
  }
  await expect(join('mixed-margin-clearance',s=>s.replace('margin_mm:0.1','margin_mm:-1'))).rejects.toThrow()
})
