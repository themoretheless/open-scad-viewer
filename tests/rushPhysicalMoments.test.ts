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

it('tightens a centroid enclosure with independently uncertain masses and rejects zero mass',async()=>{
  const r=await join('mass-weighted-centroid')
  expect(r.pieceCount).toBe(2)
  expect(r.centroidBounds[0][0]).toBeCloseTo(3,12)
  expect(r.centroidBounds[0][1]).toBeCloseTo(4.8,12)
  await expect(join('mass-weighted-centroid',s=>s.replace('[[1,2],[2,4]]','[[0,2],[2,4]]'))).rejects.toThrow()
})
it('enforces physical inertia inequalities, separates moments and certifies a rotated axis',async()=>{
  const r=await join('physical-principal-moments')
  expect(r.principalBounds[2][1]).toBeCloseTo(7,12)
  const gaps=await join('physical-moment-gaps')
  for(const pair of gaps.gapBounds){expect(pair[0]).toBeCloseTo(1,7);expect(pair[1]).toBeCloseTo(1,7)}
  const cert=await join('certify-principal-direction')
  expect(cert.sineUpper).toBeGreaterThanOrEqual(0.01/Math.sqrt(2*(1+0.99**2)))
  expect(cert.sineUpper).toBeLessThan(0.015)
})
