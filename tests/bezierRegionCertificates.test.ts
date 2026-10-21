import {expect,it} from 'vitest'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {certifyBrepAlgebraicIntersections,verifyBrepAlgebraicIntersections,certifyBrepMultispanAgreement,verifyBrepMultispanAgreement,offsetBrepRegionsCertified,verifyBrepOffsetRegions,normalizeBrepProfile} from '../src/services/geometry/brepProfile'
import {bezierLine} from './fixtures/bezier-algebraic-cases'
import {irrationalQuartic,irrationalContactLine,nondyadicTangency,nondyadicContactLine,irrationalLoop,irrationalCusp,multipleFiberFirst,multipleFiberSecond,multispanFirst,multispanSecond,regionBox,curvedRegion} from './fixtures/bezier-region-cases'
it('isolates irrational crossings, non-dyadic multiple contacts and separated fibers',()=>{
 for(const [a,b,n] of [[irrationalQuartic,irrationalContactLine,2],[nondyadicTangency,nondyadicContactLine,1],[multipleFiberFirst,multipleFiberSecond,2]] as const){
  const c=certifyBrepAlgebraicIntersections(a,b),r=verifyBrepAlgebraicIntersections(a,b,c)
  expect(r.events.length).toBe(n)
  for(const e of r.events)for(const p of e)expect(p[1]-p[0]).toBeLessThanOrEqual(1.01e-12)
  const bad=structuredClone(c);bad.factors.find((f:any)=>f.roots.length)!.roots.pop();expect(()=>verifyBrepAlgebraicIntersections(a,b,bad)).toThrow()
 }
})
it('distinguishes irrational self crossings and diagonal stationary roots',()=>{
 const c=certifyBrepAlgebraicIntersections(irrationalLoop,irrationalLoop,true)
 expect(verifyBrepAlgebraicIntersections(irrationalLoop,irrationalLoop,c).events.length).toBe(1)
 const cusp=certifyBrepAlgebraicIntersections(irrationalCusp,irrationalCusp,true)
 expect(verifyBrepAlgebraicIntersections(irrationalCusp,irrationalCusp,cusp).singularParameters.length).toBe(2)
 const profile=normalizeBrepProfile([[irrationalLoop,bezierLine([3,1.5],[3,-1.5])]])
 expect(profile.loops.length).toBeGreaterThanOrEqual(2)
})
it('covers unequal source knots with an irrational nonlinear preimage',()=>{
 const c=certifyBrepMultispanAgreement(multispanFirst,multispanSecond,[0,1],[0,1])
 expect(verifyBrepMultispanAgreement(multispanFirst,multispanSecond,[0,1],[0,1],c).commonSpans).toBe(2)
 const bad=structuredClone(c);bad.steps.pop();expect(()=>verifyBrepMultispanAgreement(multispanFirst,multispanSecond,[0,1],[0,1],bad)).toThrow()
})
it('regularizes overlapping offsets and rejects forged global boundaries',()=>{
 const source={loops:[regionBox(),regionBox(5)],toleranceMm:1e-7}
 const r=offsetBrepRegionsCertified(source,1,.05,'round',4)
 expect(r.components.length).toBe(1)
 expect(verifyBrepOffsetRegions(source,1,.05,4,r.certificate).status).toBe('verified')
 const bad=structuredClone(r.certificate);bad.exactRings[0].pop();expect(()=>verifyBrepOffsetRegions(source,1,.05,4,bad)).toThrow()
 for(const p of r.components){const model=callGeometryRust('brep_nurbs_extrude_curves',{loops:p.loops,zMin:0,zMax:2});const mesh=callGeometryRust<{report:object}>('brep_nurbs_tessellate',{model,segments:32});expect(mesh.report).toMatchObject({closed:true,nonManifoldEdges:0,degenerateTriangles:0,orientationConflicts:0})}
},120000)
it('cleans inward artifacts and supports curved source regions',()=>{
 const r=offsetBrepRegionsCertified({loops:[regionBox()],toleranceMm:1e-7},-1,.05,'round',4)
 expect(r.components.length).toBe(1);expect(r.components[0].areaMm2).toBeCloseTo(4,10)
 const curved=offsetBrepRegionsCertified({loops:[curvedRegion],toleranceMm:1e-7},.5,.05,'round',4)
 expect(curved.components.length).toBe(1);expect(curved.components[0].areaMm2).toBeGreaterThan(16)
},120000)
