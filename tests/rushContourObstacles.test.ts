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
it('encloses the nearest contour distance to four solid planar obstacles',async()=>{
  for(const [name,expected] of [['contour-box-distance',0.4],['contour-disk-distance',0.375],['contour-capsule-distance',0.375],['contour-rectangle-distance',0.25]] as const){
    const r=await join(name)
    encloses(r.distance,expected)
    expect(r.witnessContour).toBe(1)
    expect(r.workUsed).toBeLessThanOrEqual(10000)
  }
  await expect(join('contour-disk-distance',s=>s.replace('radius_mm:0.125','radius_mm:-1'))).rejects.toThrow()
})
it('tracks both closest contour indices and rejects an empty second set',async()=>{
  const r=await join('planar-contour-sets-distance')
  encloses(r.distance,1)
  expect(r.witnessContours).toEqual([0,0])
  expect(r.witnessParameters).toHaveLength(2)
  expect(r.converged).toBe(true)
  await expect(join('planar-contour-sets-distance',s=>s.replace('first_count:2','first_count:3'))).rejects.toThrow()
})
