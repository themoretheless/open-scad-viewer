import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const program=readFileSync('examples/rush/trimmed-surface-integrals.r','utf8')
async function reports(source:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return scene.measurements!.map(row=>JSON.parse(row.report))
}
function contains(bounds:number[],value:number){
  expect(bounds[0]).toBeLessThanOrEqual(value)
  expect(bounds[1]).toBeGreaterThanOrEqual(value)
}
it('bounds the clipped area, world centroid and area inertia of a folded sheet with a hole',async()=>{
  const [window,centroid,inertia]=await reports(program)
  contains(window.measure.area.boundsMm2,0.1875)
  expect(window.measure.area.boundsMm2[1]-window.measure.area.boundsMm2[0]).toBeLessThan(0.025)
  ;[0.1,0.15,1].forEach((x,i)=>contains(centroid.centroidBoundsMm[i],x))
  const tensor=inertia.inertiaBoundsMm4
  contains(tensor[2][2],83/768)
  contains(tensor[0][2],-0.125)
  contains(tensor[1][2],-0.1875)
  expect(tensor[2][0]).toEqual(tensor[0][2])
},60000)
it('withholds a centroid for an unresolved region and clips disjoint windows to zero',async()=>{
  const [window,centroid]=await reports(program.replace('[[3.75,4.25],[0.375,0.625]]','[[5,6],[0.25,0.5]]').replaceAll('max_surface_cells:10000,max_trim_cells:1000000','max_surface_cells:2,max_trim_cells:1'))
  expect(window.measure.area.boundsMm2).toEqual([0,0])
  expect(centroid.centroidBoundsMm).toBeNull()
})

it('resolves authored NURBS trim references on an open surface',async()=>{
  const text=`// @rush/1
line = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0,0,0],[1mm,0,0]],weights:[1,1])
plane = line.surface_extrude(vector:[0,1mm,0])
border = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[0,0],[1,0],[1,1],[0,1],[0,0]],weights:[1,1,1,1,1])
result = plane.trimmed_surface_area_in_window(loops:[[border]],window:[[0.25,0.75],[0.25,0.75]],tolerance_uv:0.000001,tolerance_mm2:0.0001,max_surface_cells:1000,max_trim_cells:10000)
show result.tessellate(4,4)`
  const [result]=await reports(text)
  contains(result.measure.area.boundsMm2,0.25)
  expect(result.measure.area.withinTolerance).toBe(true)
})
