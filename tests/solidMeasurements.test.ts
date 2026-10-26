import {expect,it} from 'vitest'
import {measureSolidVertices,measureSolidManifoldMetrics,measureSolidEdgeCurvature,measureSolidMassProperties} from '../src/services/solidMeasurements'
import {createBrepBox,createBrepCylinder,createBrepSphere,booleanNurbsBrep,analyzeNurbsBrep,inspectNurbsBrep,tessellateNurbsBrep,transformNurbsBrep} from '../src/services/geometry/brep'
const body=(brep:ReturnType<typeof createBrepBox>)=>({id:'body',name:'Body',brep,mesh:tessellateNurbsBrep(brep)})
it('measures vertices across bodies without modifying their documents',()=>{
 const a=body(createBrepBox([0,0,0],[1,1,1])),b=body(createBrepBox([3,4,12],[4,5,13]))
 const before=structuredClone([a,b])
 const ai=a.brep.vertices.findIndex(v=>v.point.every(x=>x===0)),bi=b.brep.vertices.findIndex(v=>v.point.every((x,i)=>x===[3,4,12][i]))
 expect(measureSolidVertices(a,ai,b,bi)).toMatchObject({distanceMm:13,deltaMm:[3,4,12]})
 expect(measureSolidVertices(a,ai,a,ai).distanceMm).toBe(0)
 expect([a,b]).toEqual(before)
 expect(()=>measureSolidVertices(a,-1,b,0)).toThrow('existing vertex')
})
it('measures the local curvature radius of a circular edge and reports a straight edge as infinite',()=>{
 const cylinder=body(createBrepCylinder(3,5)),index=cylinder.brep.edges.findIndex(edge=>edge.curve.degree===2)
 for(const t of [.125,.375,.625,.875])expect(measureSolidEdgeCurvature(cylinder,index,t).radiusMm).toBeCloseTo(3,8)
 const tilted=body(transformNurbsBrep(cylinder.brep,[[2,0,0,0],[0,0,-2,0],[0,2,0,0],[0,0,0,1]]))
 expect(measureSolidEdgeCurvature(tilted,index,.125).radiusMm).toBeCloseTo(6,8)
 const ellipse=body(transformNurbsBrep(cylinder.brep,[[2,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]))
 const measured=measureSolidEdgeCurvature(ellipse,index,.125)
 // Each authored cylinder edge is an arc; parameter t is not a whole-circle angle.
 const [x,y]=measured.point
 expect(measured.radiusMm).toBeCloseTo(Math.pow(36*(y/3)**2+9*(x/6)**2,1.5)/18,8)
 const box=body(createBrepBox([0,0,0],[1,1,1]))
 expect(measureSolidEdgeCurvature(box,0,.5).radiusMm).toBeNull()
 expect(()=>measureSolidEdgeCurvature(cylinder,index,1.1)).toThrow('between 0 and 1')
})


it.each(['wall sphere','drilled sphere','two stubs'] as const)('preserves tolerant Boolean mass identities within the original budget: %s',name=>{
 const translate=(model:ReturnType<typeof createBrepBox>,at:number[])=>transformNurbsBrep(model,[[1,0,0,at[0]],[0,1,0,at[1]],[0,0,1,at[2]],[0,0,0,1]])
 const a=name==='drilled sphere'?createBrepSphere(5):createBrepCylinder(3,10)
 const b=name==='drilled sphere'?translate(createBrepCylinder(1,20),[.3,.2,-10]):translate(createBrepSphere(name==='wall sphere'?2:4),name==='wall sphere'?[2.5,.4,5]:[.2,.1,5])
 const original=JSON.stringify([a,b]),results=(['union','difference','intersection'] as const).map(op=>booleanNurbsBrep(a,b,op))
 const mass=(model:typeof a)=>{
  const report=analyzeNurbsBrep(model,1e-5,2_000_000)
  expect(report.status).toBe('converged_estimate');expect(report.solidGeometryStatus).toBe('not_certified')
  expect(report.evaluations).toBeLessThanOrEqual(2_000_000);expect(report.signedVolumeMm3).toBeGreaterThan(0)
  return report.signedVolumeMm3
 }
 for(const result of results)expect(inspectNurbsBrep(result).topologyValid).toBe(true)
 const [union,difference,intersection]=results.map(mass),va=mass(a),vb=mass(b)
 expect(Math.abs(union-(va+vb-intersection))/Math.max(union,va+vb-intersection)).toBeLessThanOrEqual(2e-3)
 expect(Math.abs(difference-(va-intersection))/Math.max(difference,va-intersection)).toBeLessThanOrEqual(2e-3)
 expect(intersection).toBeLessThan(Math.min(va,vb))
 expect(results[1].bodies).toHaveLength(name==='two stubs'?2:1)
 expect(JSON.stringify([a,b])).toBe(original)
},120_000)

it('computes volume, surface area, center of mass, and principal inertia for B-Rep and mesh bodies',()=>{
 const box=body(createBrepBox([0,0,0],[2,4,6]))
 const mass=measureSolidMassProperties(box)
 expect(mass.volumeMm3).toBeCloseTo(48,4)
 expect(mass.surfaceAreaMm2).toBeCloseTo(2*(8+12+24),4)
 expect(mass.surfaceToVolumeRatioPerMm).toBeCloseTo(88/48,4)
 expect(mass.centerOfMass[0]).toBeCloseTo(1,3)
 expect(mass.centerOfMass[1]).toBeCloseTo(2,3)
 expect(mass.centerOfMass[2]).toBeCloseTo(3,3)
 expect(mass.dimensions).toEqual([2,4,6])
 expect(mass.principalInertiaMm5[0]).toBeGreaterThan(0)
 expect(mass.principalInertiaMm5[1]).toBeGreaterThanOrEqual(mass.principalInertiaMm5[0])
 expect(mass.principalInertiaMm5[2]).toBeGreaterThanOrEqual(mass.principalInertiaMm5[1])
 const meshOnly={id:'mesh',name:'Mesh',mesh:box.mesh}
 const meshMass=measureSolidMassProperties(meshOnly)
 expect(meshMass.volumeMm3).toBeCloseTo(48,4)
 expect(meshMass.centerOfMass[2]).toBeCloseTo(3,3)
})

it('computes manifold metrics for closed and open meshes in Rust',()=>{
 const box=body(createBrepBox([0,0,0],[2,4,6]))
 const closed=measureSolidManifoldMetrics(box.mesh)
 expect(closed).toMatchObject({vertexCount:8,triangleCount:12,edgeCount:18,eulerCharacteristic:2,componentCount:1,watertight:true})
 expect(closed.signedVolume).toBeCloseTo(48,8)
 expect(closed.surfaceArea).toBeCloseTo(88,8)
 expect(closed.components).toHaveLength(1)
 expect(closed.components[0]).toMatchObject({genus:0,boundaryEdges:0,eulerCharacteristic:2})

 const open=measureSolidManifoldMetrics({positions:new Float64Array([0,0,0,1,0,0,0,1,0]),indices:new Uint32Array([0,1,2])})
 expect(open).toMatchObject({vertexCount:3,triangleCount:1,edgeCount:3,eulerCharacteristic:1,componentCount:1,watertight:false,signedVolume:0})
 expect(open.surfaceArea).toBeCloseTo(.5,8)
 expect(open.components[0].genus).toBeNull()
})
