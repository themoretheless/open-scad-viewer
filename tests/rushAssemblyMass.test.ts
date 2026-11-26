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

function encloses(bounds:number[],value:number){
  expect(bounds[0]).toBeLessThanOrEqual(value)
  expect(bounds[1]).toBeGreaterThanOrEqual(value)
}
it('combines independently measured rectangles about their common centroid',async()=>{
  const r=await join('combine-surface-measures')
  expect(r.pieceCount).toBe(2)
  encloses(r.areaBoundsMm2,3)
  encloses(r.centroidBoundsMm[0],5.5/3)
  encloses(r.centralInertiaBounds[1][1],2*4/12+1/12+2/3*2.5**2)
})
it('weights centroid and parallel-axis inertia with each material density',async()=>{
  const r=await join('combine-surface-densities')
  encloses(r.massBounds,4)
  encloses(r.centroidBoundsMm[0],2.25)
  encloses(r.centralInertiaBounds[1][1],2*4/12+2/12+2.5**2)
  await expect(join('combine-surface-densities',s=>s.replace('[[1,1],[2,2]]','[[1,1],[0,2]]'))).rejects.toThrow()
})
