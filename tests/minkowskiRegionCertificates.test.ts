import {elevateNurbsCurve} from '../src/services/nurbsCurve'
import {certifyBrepCurveImageAgreement,verifyBrepCurveImageAgreement,booleanBrepProfiles} from '../src/services/geometry/brepProfile'
import {retracingImageFirst,retracingImageSecond,cuspImageFirst,cuspImageSecond} from './fixtures/minkowski-cases'
import {multispanFirst,multispanSecond} from './fixtures/bezier-region-cases'
import {expect,it} from 'vitest'
import {minkowskiBrepProfiles,minkowskiOffsetBrepProfile,verifyBrepMinkowskiCertificate,certifyBrepSpanIntersections,verifyBrepSpanIntersections,certifyBrepAlgebraicIntersections,verifyBrepAlgebraicIntersections,certifyBrepMultispanAgreement,verifyBrepMultispanAgreement,offsetBrepProfile,validateBrepProfile} from '../src/services/geometry/brepProfile'
import {minkowskiL,minkowskiKernel,minkowskiFrame,minkowskiSquare,minkowskiNeck,zeroJetRegion,retracedLine,monotoneStationary,spanFirst,spanSecond,rationalKnotFirst,rationalKnotSecond} from './fixtures/minkowski-cases'
it('computes an exact filled Minkowski sum for nonconvex polygons and holes',()=>{
 for(const [loops,area] of [[[minkowskiL],27],[minkowskiFrame,140]] as const){const a={loops:[...loops]},b={loops:[minkowskiKernel]},r=minkowskiBrepProfiles(a,b,0)
  expect(r.exactSetResult).toBe(true);expect(r.modelingStatus).toBe('validated');expect(r.components.reduce((n,p)=>n+p.areaMm2,0)).toBeCloseTo(area,8)
  expect(verifyBrepMinkowskiCertificate(a,b,r.certificate).status).toBe('verified')
  const bad=structuredClone(r.certificate);bad.outer[0].pop();expect(()=>verifyBrepMinkowskiCertificate(a,b,bad)).toThrow()
 }
})
it('proves collapsed erosion and splits a narrow neck into separate components',()=>{
 const a={loops:[minkowskiSquare]},r=minkowskiOffsetBrepProfile(a,-3,.1)
 expect(r.exactSetResult).toBe(true);expect(r.outerExactRings).toEqual([]);expect(r.components).toEqual([])
 expect(verifyBrepMinkowskiCertificate(a,{loops:[]},r.certificate).status).toBe('verified')
 const split=minkowskiOffsetBrepProfile({loops:[minkowskiNeck]},-.4,.1)
 expect(split.modelingStatus).toBe('validated');expect(split.components.length).toBe(2)
},120000)
it('encloses zero-jet curved source material and rejects deleted subdivision leaves',()=>{
 const a={loops:[zeroJetRegion]},b={loops:[minkowskiKernel]},r=minkowskiBrepProfiles(a,b,.2)
 expect(r.innerExactRings.length).toBeGreaterThan(0);expect(r.outerExactRings.length).toBeGreaterThan(0);expect(r.exactSetResult).toBe(false)
 const bad=structuredClone(r.certificate);bad.firstPartition[0].pop();expect(()=>verifyBrepMinkowskiCertificate(a,b,bad)).toThrow()
 expect(()=>verifyBrepMinkowskiCertificate({loops:[minkowskiSquare]},b,r.certificate)).toThrow()
 const line=minkowskiBrepProfiles({loops:[[retracedLine]]},b,0);expect(line.exactSetResult).toBe(true);expect(line.outerExactRings).toEqual([])
},120000)
it('keeps affine identity proofs for complete retracing without monotone coordinates',()=>{
 const c=certifyBrepMultispanAgreement(retracedLine,retracedLine,[0,1],[0,1]);expect(c.version).toBe(2)
 expect(verifyBrepMultispanAgreement(retracedLine,retracedLine,[0,1],[0,1],c).status).toBe('verified')
})
it('checks nondyadic stationary roots on the monotone self path',()=>{
 const c=certifyBrepAlgebraicIntersections(monotoneStationary,monotoneStationary,true),r=verifyBrepAlgebraicIntersections(monotoneStationary,monotoneStationary,c)
 expect(r.stationaryCensusComplete).toBe(true);expect(r.singularParameters.length).toBe(1);expect(r.singularParameters[0][0]).toBeLessThanOrEqual(1/3);expect(r.singularParameters[0][1]).toBeGreaterThanOrEqual(1/3)
 expect(()=>verifyBrepAlgebraicIntersections(monotoneStationary,monotoneStationary,{...c,factors:[]})).toThrow()
})
it('accounts for every span pair, including common components and isolated contacts',()=>{
 const c=certifyBrepSpanIntersections(spanFirst,spanSecond),r=verifyBrepSpanIntersections(spanFirst,spanSecond,c)
 expect(r.overlaps.length).toBeGreaterThanOrEqual(3);const bad=structuredClone(c);bad.pairs.pop();expect(()=>verifyBrepSpanIntersections(spanFirst,spanSecond,bad)).toThrow()
 const e=certifyBrepSpanIntersections(rationalKnotFirst,rationalKnotSecond);expect(verifyBrepSpanIntersections(rationalKnotFirst,rationalKnotSecond,e).events.length).toBe(2)
})
it('connects the retained general-curve offset to the filled-region operation',()=>{
 const p=validateBrepProfile([zeroJetRegion]),r=offsetBrepProfile(p,.5,.2)
 expect(r.geometryStatus).toBe('bounded_approximation');expect(r.minkowskiCertificate).toBeDefined();expect(r.areaMm2).toBeGreaterThan(p.areaMm2)
},120000)

it('proves nonmonotone nonlinear image equality, irrational cusp preimages and source-cell unions',()=>{
 for(const [a,b]of [[retracingImageFirst,retracingImageSecond],[cuspImageFirst,cuspImageSecond],[multispanFirst,multispanSecond]]){
  const c=certifyBrepCurveImageAgreement(a,b,[0,1],[0,1]);expect(verifyBrepCurveImageAgreement(a,b,[0,1],[0,1],c).status).toBe('verified')
  const bad=structuredClone(c);bad.forward[0].sections.pop();expect(()=>verifyBrepCurveImageAgreement(a,b,[0,1],[0,1],bad)).toThrow()
 }
 const larger=structuredClone(retracingImageSecond);larger.controlPoints[2][0]=9;expect(()=>certifyBrepCurveImageAgreement(retracingImageFirst,larger,[0,1],[0,1])).toThrow()
})
it('uses active retained spans in Boolean operations on a whole elevated NURBS circle',()=>{
 const a={degree:2,knots:[0,0,0,.25,.25,.5,.5,.75,.75,1,1,1],controlPoints:[[3,0],[3,3],[0,3],[-3,3],[-3,0],[-3,-3],[0,-3],[3,-3],[3,0]],weights:[1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1],periodic:false}
 const elevated=elevateNurbsCurve(a,3),p=validateBrepProfile([[elevated]])
 expect(booleanBrepProfiles(p,p,'difference').loops).toEqual([])
 expect(validateBrepProfile([[elevated]],'even-odd').loops[0][0]).toEqual(elevated)
})
