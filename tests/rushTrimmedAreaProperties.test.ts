import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source=readFileSync('examples/rush/trimmed-area-properties.r','utf8')
async function run(text:string){
  const result=await buildTextNurbsScene(compileRushFrontend(text).document)
  return result.measurements!.map(row=>JSON.parse(row.report))
}
function check(bounds:number[],x:number){
  expect(bounds[0]).toBeLessThanOrEqual(x)
  expect(bounds[1]).toBeGreaterThanOrEqual(x)
}
it('measures the outer contour, a seam-crossing window and the full patch set independently',async()=>{
  const [area,props,window,set]=await run(source)
  check(area.boundsMm2,1.25)
  ;[0.1,0.15,1].forEach((v,i)=>check(props.centroidBoundsMm[i],v))
  expect(props.centralInertiaBoundsMm4[0][1]).toEqual(props.centralInertiaBoundsMm4[1][0])
  expect(props.principalMomentBoundsMm4).toHaveLength(3)
  check(window.boundsMm2,0.1875)
  check(window.centroidBoundsMm[0],1/48)
  check(set.boundsMm2,1.3125)
  check(set.componentAreas[1].boundsMm2,0.0625)
  const moment=83/768+49/12288+1.3125*(1-2*3/28)
  check(set.axisMomentBoundsMm4,moment)
  check(set.referenceInertiaBoundsMm4[2][2],moment)
  check(set.axisRadiusOfGyrationBoundsMm,Math.sqrt(moment/1.3125))
  expect(set.principalMomentBoundsMm4).toHaveLength(3)
  expect(set.withinMomentTolerance).toBe(true)
  const tighter=await run(source.replace('moment_tolerance:10,axis_point','moment_tolerance:0.00000001,axis_point'))
  expect(tighter[3].withinMomentTolerance).toBe(false)
},120000)
it('distinguishes an empty clipped window from incomplete measurement',async()=>{
  const reports=await run(source.replace('[[3.75,4.25],[0.375,0.625]]','[[5,6],[0.25,0.5]]').replaceAll('max_surface_cells:20000','max_surface_cells:2'))
  expect(reports[2].boundsMm2).toEqual([0,0])
  expect(reports[3].boundsMm2).toBeNull()
},120000)
