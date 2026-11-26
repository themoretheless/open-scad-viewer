import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string,transform=(s:string)=>s) {
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(readFileSync('examples/rush/'+name+'.r','utf8'))).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}
it('preserves endpoints while removing redundant knots and degrees from individual and assembled profiles',async()=>{
  const single=await run('compress-profile-curve')
  expect(single.reducedDegrees).toBe(1)
  expect(single.completed).toBe(true)
  const batch=await run('compress-profile-segments')
  expect(batch.controlCounts).toEqual([2,2])
  expect(batch.end).toEqual([0,2,0])
  for(const name of ['compress-profile-chain','orient-profile-chain','restore-profile-partition','assemble-open-profile']){
    const r=await run(name)
    expect(r.degree).toBe(1)
    expect(r.controlCount).toBe(2)
    expect(r.start).toEqual([0,0,0])
    expect(r.end).toEqual([2,0,0])
    expect(r.domain).toEqual([0,2])
  }
  await expect(run('compress-profile-segments',s=>s.replace('max_work:1000000','max_work:1'))).rejects.toThrow(/budget/i)
},180000)
it('assembles closed wires in both chosen orientations, preserving height in multispan spatial edges',async()=>{
  for(const name of ['assemble-closed-profile','assemble-planar-profile','orient-planar-profile','assemble-projected-profile','assemble-oblique-profile','assemble-spatial-profile','assemble-multispan-spatial-profile']){
    const r=await run(name)
    expect(r.completed).toBe(true)
    expect(r.start).toEqual(r.end)
    expect(r.controlCount).toBeGreaterThanOrEqual(4)
  }
  await expect(run('assemble-multispan-spatial-profile',s=>s.replaceAll('[0, 2, 1]','[0, 2, 0]').replaceAll('[2, 0, 1]','[2, 0, 0]'))).rejects.toThrow()
},180000)
