import { readFileSync } from 'node:fs'
import { expect,it } from 'vitest'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'
import { compileRushFrontend } from '../src/language/rushFrontend'

const source=readFileSync('examples/rush/trimmed-surface-mass.r','utf8')
async function report(program:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(program).document)
  expect(scene.measurements).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
function encloses(bounds:number[],exact:number){
  expect(bounds[0]).toBeLessThanOrEqual(exact)
  expect(bounds[1]).toBeGreaterThanOrEqual(exact)
  expect(bounds.every(Number.isFinite)).toBe(true)
}
it('measures two seam-crossing components with a hole and different densities',async()=>{
  const p=await report(source)
  const mass=2.875
  const x=0.34375/mass,y=0.375/mass
  const uncentered=2*83/768+6*49/12288
  encloses(p.massBoundsKg,mass)
  encloses(p.centerOfMassBoundsMm[0],x)
  encloses(p.centerOfMassBoundsMm[1],y)
  encloses(p.centralInertiaBoundsKgMm2[2][2],uncentered-mass*(x*x+y*y))
  expect(p.componentMasses.map((m:{outerContour:number})=>m.outerContour)).toEqual([0,2])
  encloses(p.componentMasses[0].boundsKg,2.5)
  encloses(p.componentMasses[1].boundsKg,0.375)
  encloses(p.componentMasses[0].massFractionBounds,20/23)
  encloses(p.componentMasses[1].massFractionBounds,3/23)
  encloses(p.centerOfMassPlaneDistanceBoundsMm,(x-y-1)/Math.sqrt(2))
  encloses(p.firstMassMomentAboutPlaneBoundsKgMm,mass*(x-y-1)/Math.sqrt(2))
  const moment=uncentered-2*0.34375+mass
  expect(p.withinMomentTolerance).toBe(true)
  expect(p.principalMomentBoundsKgMm2).toHaveLength(3)
  const vx=2*(1/16-13/768)+6*49/12288-mass*x*x
  const vy=2/16-mass*y*y
  const vz=2*(1.5**3/12-0.25/12)+6*0.0625*0.25/12
  const average=vz+(vx+vy)/2, split=Math.hypot((vx-vy)/2,mass*x*y)
  const moments=[vx+vy,average-split,average+split].sort((a,b)=>a-b)
  moments.forEach((moment,i)=>encloses(p.principalMomentBoundsKgMm2[i],moment))
  encloses(p.axisMomentBoundsKgMm2,moment)
  encloses(p.referenceInertiaBoundsKgMm2[2][2],moment)
  encloses(p.axisRadiusOfGyrationBoundsMm,Math.sqrt(moment/mass))
},120000)
it('keeps unavailable mass properties explicit with an exhausted surface budget',async()=>{
  const p=await report(source.replace('max_surface_cells:20000','max_surface_cells:2'))
  expect(p.referenceInertiaBoundsKgMm2).toBeNull()
},120000)
it('rejects duplicate densities and incomplete plane definitions',async()=>{
  await expect(report(source.replace('[2,6,6]','[0,6,6]'))).rejects.toThrow()
  await expect(report(source.replace(',plane_normal:[2,-2,0]',''))).rejects.toThrow()
},120000)

it('propagates interval densities in input-independent contour order',async()=>{
  const p=await report(source.replace('[2,6,6],[0,2,2]', '[0,2,4],[2,6,8]'))
  encloses(p.massBoundsKg,2*1.25+6/16)
  encloses(p.massBoundsKg,4*1.25+8/16)
  encloses(p.componentMasses[1].boundsKg,6/16)
  encloses(p.componentMasses[1].boundsKg,8/16)
},120000)
