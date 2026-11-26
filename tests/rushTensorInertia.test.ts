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

it('encloses each ordered principal moment and rejects asymmetric tensors',async()=>{
  const r=await join('tensor-principal-bounds')
  for(const [i,pair] of [[0,9],[5,12],[11,21]].entries())
    for(const [j,v] of pair.entries())expect(r.principalBounds[i][j]).toBeCloseTo(v,10)
  await expect(join('tensor-principal-bounds',s=>s.replace('[-0.5, 1]','[-0.4, 1]'))).rejects.toThrow()
})
it('uses signed off-diagonal entries and the common centroid of a trimmed patch set',async()=>{
  for(const [sign,expected] of [[1,[4.5,7]],[-1,[3,5.5]]] as const){
    const r=await join('tensor-axis-bounds',s=>s.replace('[1,1,0]',JSON.stringify([sign,1,0])))
    for(const [i,v] of expected.entries())expect(r.axisBounds[i]).toBeCloseTo(v,10)
  }
  const area=1.3125,cx=3/28,cy=1/7
  const vx=1/16-13/768+49/12288-area*cx*cx,vy=1/16-area*cy*cy
  const exact=1.5**3/12-0.25/12+0.0625*0.25/12+(vx+vy)/2+area*cx*cy
  const {axisBounds}=await join('trim-set-axis-bounds')
  expect(axisBounds[0]).toBeLessThanOrEqual(exact)
  expect(axisBounds[1]).toBeGreaterThanOrEqual(exact)
})
