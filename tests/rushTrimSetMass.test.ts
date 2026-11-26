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
it('includes the island and shares the area budget across both components',async()=>{
  const r=await join('periodic-trim-set-area')
  expect(r.measure.area.withinTolerance).toBe(true)
  encloses(r.measure.area.boundsMm2,1.3125)
  expect(r.components).toHaveLength(2)
  encloses(r.components[0].boundsMm2,1.25)
  encloses(r.components[1].boundsMm2,0.0625)
  expect(r.measure.area.cells).toBe(r.components[0].cells+r.components[1].cells)
})
it('encloses the common centroid and withholds it at insufficient coverage',async()=>{
  const r=await join('periodic-trim-set-centroid')
  expect(r.components).toHaveLength(2)
  for(const [i,v] of [3/28,1/7,1].entries()){
    encloses(r.centroidBoundsMm[i],v)
    expect(r.centroidBoundsMm[i][1]-r.centroidBoundsMm[i][0]).toBeLessThan(0.6)
  }
  const limited=await join('periodic-trim-set-centroid',s=>s.replace('max_surface_cells:20000','max_surface_cells:2').replace('max_trim_cells:1000000','max_trim_cells:1'))
  expect(limited.centroidBoundsMm).toBeNull()
})
