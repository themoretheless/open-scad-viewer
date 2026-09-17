import {expect,it} from 'vitest'
import {analyzeCertifiedNurbsBrep,analyzeNurbsBrep,auditedMultiEdgeFillet,auditedParallelFrameSweep,createBrepBox,createBrepCylinder,createBrepFrustum,createBrepSphere,createBrepTorus,createBrepTube,tessellateCertifiedNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
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

it('publishes finite certified mass and tessellation enclosures',()=>{
 for(const [body,volume] of [
  [createBrepBox([0,0,0],[2,3,4]),24],
  [createBrepCylinder(3,8),72*Math.PI],
  [createBrepTube(3,2,8),40*Math.PI],
 ] as const){
  const mass=analyzeCertifiedNurbsBrep(body)
  expect(mass.status).toBe('certified_enclosure')
  expect(mass.volumeMm3.lower).toBeLessThanOrEqual(volume)
  expect(mass.volumeMm3.upper).toBeGreaterThanOrEqual(volume)
  expect(mass.audit.ok).toBe(true)
  expect(mass.namingComplete).toBe(true)
 }
 for(const body of [createBrepBox([0,0,0],[2,3,4]),createBrepCylinder(3,8)]){
  const certified=tessellateCertifiedNurbsBrep(body,.02)
  expect(certified.surfaceToMeshDeviationMm).toBeLessThanOrEqual(.02)
  expect(certified.meshToSurfaceDeviationMm).toBe(certified.surfaceToMeshDeviationMm)
  expect(certified.coverage).toMatchObject({sharedEdgeIdentity:true,orientation:true,noTJunctions:true,noCracks:true})
  expect(certified.tessellation.report.closed).toBe(true)
 }
 const sphere=analyzeCertifiedNurbsBrep(createBrepSphere(2))
 expect(sphere.capability).toBe('certified-mass-properties/2')
 expect(sphere.volumeMm3.lower).toBeLessThanOrEqual(32*Math.PI/3)
 expect(sphere.volumeMm3.upper).toBeGreaterThanOrEqual(32*Math.PI/3)
 expect(()=>tessellateCertifiedNurbsBrep(createBrepCylinder(100,10),1e-12)).toThrow(/32|budget/i)
})

it('publishes audited finite feature successors and refuses bent frames',()=>{
 const box=createBrepBox([0,0,0],[10,8,6])
 const vertical=box.edges.map((edge,index)=>{
  const a=box.vertices[edge.vertices[0]].point,b=box.vertices[edge.vertices[1]].point
  return Math.abs(a[0]-b[0])<=1e-12&&Math.abs(a[1]-b[1])<=1e-12?index:-1
 }).filter(index=>index>=0).slice(0,2)
 const fillet=auditedMultiEdgeFillet(box,vertical,.5)
 expect(fillet).toMatchObject({certificate:{capability:'analytic-multi-edge-fillet/1',complete:true},audit:{ok:true},namingComplete:true})
 expect(fillet.evidenceClaimCount).toBeGreaterThan(0)
 const sweep=auditedParallelFrameSweep([[0,0],[2,0],[2,1],[0,1]],[[3,-1,0],[3,-1,4]],'rmf')
 expect(sweep).toMatchObject({certificate:{capability:'exact-parallel-frame-sweep/1',complete:true},audit:{ok:true},namingComplete:true})
 expect(()=>auditedParallelFrameSweep([[0,0],[2,0],[2,1],[0,1]],[[0,0,0],[0,0,2],[0,1,4]],'rmf')).toThrow(/straight|collinear|parallel/i)
})
