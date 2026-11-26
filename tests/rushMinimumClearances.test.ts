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

it('bounds the signed minimum for a swept ball against four obstacle shapes',async()=>{
  for(const [name,expected] of [['minimum-sphere-gap',-0.15],['minimum-capsule-gap',-0.15],['minimum-box-gap',-0.05],['minimum-oriented-box-gap',-0.05]] as const){
    const r=await join(name)
    expect(r.gap[0]).toBeLessThanOrEqual(expected)
    expect(r.gap[1]).toBeGreaterThanOrEqual(expected)
    expect(r.gap[1]-r.gap[0]).toBeLessThanOrEqual(0.001)
    expect(r.workUsed).toBeLessThanOrEqual(1000)
    expect(r.witnessObstacle).toBe(0)
    expect(r.witnessParameter).toBeGreaterThanOrEqual(0.4)
    expect(r.witnessParameter).toBeLessThanOrEqual(0.6)
  }
})
it('selects the controlling obstacle and rejects invalid radii',async()=>{
  const r=await join('minimum-sphere-gap',s=>s.replace('[[0.5,0,0,0.1]]','[[0.5,2,0,0.1],[0.5,0,0,0.1]]'))
  expect(r.witnessObstacle).toBe(1)
  await expect(join('minimum-sphere-gap',s=>s.replace('radius_mm:0.05','radius_mm:-1'))).rejects.toThrow()
})
