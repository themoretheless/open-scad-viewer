import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source=readFileSync('examples/rush/trim-contour-measurements.r','utf8')
async function run(text:string){
  const result=await buildTextNurbsScene(compileRushFrontend(text).document)
  return result.measurements!.map(row=>JSON.parse(row.report))
}
function contains(bounds:number[],x:number){
  expect(bounds[0]).toBeLessThanOrEqual(x)
  expect(bounds[1]).toBeGreaterThanOrEqual(x)
}
it('retains seam joins and encloses stations, equal divisions and signed projections',async()=>{
  const [audit,locate,box,division,area,projection]=await run(source)
  expect(audit.joins).toHaveLength(6)
  expect(audit.joins[1]).toEqual({segment:0,kind:'corner',parameter:{numerator:'1',denominator:'2'}})
  contains(locate.parameterBounds,0.5)
  expect(box.parameterBounds).toEqual(locate.parameterBounds)
  ;[0,0,0.25].forEach((x,i)=>contains(box.pointBoundsMm[i],x))
  expect(division.stations).toHaveLength(6)
  expect(division.stations[0].parameterBounds).toEqual([0,0])
  expect(division.stations[5].parameterBounds).toEqual([4,4])
  expect(division.stations.every((s:{status:string})=>s.status==='tolerance')).toBe(true)
  ;[-0.75,-0.5,0].forEach((x,i)=>contains(area.boundsMm2[i],x))
  contains(projection.boundsMm2,-1.25/Math.SQRT2)
})
it('reports exhausted division work and an out-of-range target distance',async()=>{
  const p=await run(source.replace('max_cells:2048','max_cells:6').replaceAll('distance_mm:0.5mm','distance_mm:100mm'))
  expect(p[2].status).toBe('outside')
  expect(p[2].pointBoundsMm).toBeNull()
  expect(p[3].stations[1].status).toBe('workLimit')
})
