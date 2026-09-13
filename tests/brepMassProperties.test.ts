import {expect,it} from 'vitest'
import {analyzeNurbsBrep,createBrepBox,createBrepCylinder,createBrepFrustum,createBrepSphere,createBrepTorus,createBrepTube,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {transformSelection} from '../src/services/directSolidTools'

it('integrates rational surfaces and trims independently of the display mesh',()=>{
 for(const [body,volume,area] of [
  [createBrepCylinder(3,8),72*Math.PI,66*Math.PI],
  [createBrepTube(3,2,8),40*Math.PI,90*Math.PI],
  [createBrepSphere(3),36*Math.PI,36*Math.PI],
  [createBrepTorus(8,2),64*Math.PI*Math.PI,64*Math.PI*Math.PI],
  [createBrepFrustum(3,0,4),12*Math.PI,24*Math.PI],
 ] as const){
  const before=JSON.stringify(body),analysis=analyzeNurbsBrep(body)
  expect(analysis.status).toBe('converged_estimate');expect(analysis.solidGeometryStatus).toBe('not_certified')
  expect(analysis.signedVolumeMm3).toBeCloseTo(volume,6);expect(analysis.surfaceAreaMm2).toBeCloseTo(area,6)
  expect(analysis.volumeErrorEstimateMm3).toBeLessThan(volume*1e-6)
  for(const n of [2,8]){const display=tessellateNurbsBrep(body,n);expect(display.report.closed).toBe(true);expect(display.report.signedVolumeMm3).toBeLessThan(analysis.signedVolumeMm3)}
  expect(JSON.stringify(body)).toBe(before)
 }
})
it('transforms centroid and inertia with the authored body',()=>{
 const brep=createBrepBox([-1,-2,-3],[1,2,3]),mesh=tessellateNurbsBrep(brep,2),before=analyzeNurbsBrep(brep)
 const moved=transformSelection({version:1,sketches:[],bodies:[{id:'b',name:'box',brep,mesh}]},['b'],[10,-20,30],[0,0,1],90,2).bodies[0]
 const after=analyzeNurbsBrep(moved.brep!)
 expect(after.centroid).toEqual([10,-20,30]);expect(after.signedVolumeMm3).toBeCloseTo(before.signedVolumeMm3*8)
 expect(after.inertiaMm5[0][0]).toBeCloseTo(before.inertiaMm5[1][1]*32)
 expect(after.inertiaMm5[1][1]).toBeCloseTo(before.inertiaMm5[0][0]*32)
 expect(()=>analyzeNurbsBrep(brep,1e-7,100)).toThrow(/budget/)
 expect(()=>analyzeNurbsBrep(brep,0)).toThrow(/tolerance/)
})

it('retains B-rep identities and outward orientation through affine reflections',async()=>{
 const {transformNurbsBrep}=await import('../src/services/geometry/brep')
 const source=createBrepSphere(3),before=JSON.stringify(source)
 const reflected=transformNurbsBrep(source,[[-2,0,0,10],[0,3,0,-5],[0,0,4,6],[0,0,0,1]])
 expect(reflected.topologyIds).toEqual(source.topologyIds)
 expect(tessellateNurbsBrep(reflected,8).report.signedVolumeMm3).toBeGreaterThan(0)
 expect(analyzeNurbsBrep(reflected).signedVolumeMm3).toBeCloseTo(36*Math.PI*24,5)
 expect(JSON.stringify(source)).toBe(before)
 expect(()=>transformNurbsBrep(source,[[0,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]])).toThrow(/singular/)
})
