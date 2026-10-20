import {expect,it} from 'vitest'
import type {NurbsCurve} from '../src/services/nurbsCurve'
import {offsetBrepCurveCertified,verifyBrepCurveOffsetCertificate,certifyBrepCurveProjectiveAgreement,verifyBrepCurveProjectiveAgreementCertificate,certifyBrepCurveEndpointContacts,verifyBrepCurveEndpointContactsCertificate,validateBrepProfile,booleanBrepProfiles} from '../src/services/geometry/brepProfile'
import {callGeometryRust} from '../src/services/geometry/kernel'
const quadratic=(y:number):NurbsCurve=>({degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[1,y],[2,0]],weights:[1,1,1],periodic:false})
it('checks pointwise normal offsets and rejects moved output and changed tolerances',()=>{
 const curve:NurbsCurve={degree:4,knots:[0,0,0,0,0,1,1,1,1,1],controlPoints:[[0,0],[2,0],[4,1],[6,3],[8,4]],weights:[1,2,3,2,1],periodic:false}
 const before=JSON.stringify(curve),offset=offsetBrepCurveCertified(curve,.5,.05)
 expect(offset.geometryStatus).toBe('bounded_approximation')
 expect(verifyBrepCurveOffsetCertificate(curve,.5,.05,offset.certificate)).toMatchObject({status:'verified',segments:offset.curves.length,maximumDeviationMm:.05})
 const bad=structuredClone(offset.certificate);bad.points[1][0]+=1
 expect(()=>verifyBrepCurveOffsetCertificate(curve,.5,.05,bad)).toThrow()
 expect(()=>verifyBrepCurveOffsetCertificate(curve,.5,.01,offset.certificate)).toThrow()
 expect(JSON.stringify(curve)).toBe(before)
})
it('proves nonlinear projective speed and rejects a changed map',()=>{
 const a:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[5,0],[10,5]],weights:[1,1,1],periodic:false}
 const b={...a,weights:[1,.5,.25]},c=certifyBrepCurveProjectiveAgreement(a,b,[0,1],[0,1],2)
 expect(verifyBrepCurveProjectiveAgreementCertificate(a,b,[0,1],[0,1],2,c)).toMatchObject({status:'verified',commonSpans:1})
 expect(()=>verifyBrepCurveProjectiveAgreementCertificate(a,b,[0,1],[0,1],3,c)).toThrow()
 expect(()=>certifyBrepCurveProjectiveAgreement(a,b,[0,1],[0,1],-1)).toThrow()
})
it('proves two endpoint contacts without calling a touching compound a solid',()=>{
 const a=quadratic(-1),b=quadratic(1),c=certifyBrepCurveEndpointContacts(a,b)
 expect(verifyBrepCurveEndpointContactsCertificate(a,b,c)).toEqual({status:'verified',scope:'represented-endpoint-intersection-set',commonEndpoints:2})
 expect(()=>verifyBrepCurveEndpointContactsCertificate(a,b,{...c,commonEndpoints:0})).toThrow()
 expect(()=>certifyBrepCurveEndpointContacts(a,a)).toThrow()
})
it('models degrees four and eight through transverse booleans and closed meshes',()=>{
 const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
 for(const degree of [4,8]){
  const floor:NurbsCurve={degree,knots:[...Array(degree+1).fill(0),...Array(degree+1).fill(1)],controlPoints:Array.from({length:degree+1},(_,i)=>[8*i/degree,-Math.min(i,degree-i)]),weights:Array(degree+1).fill(1),periodic:false}
  const profile=validateBrepProfile([[floor,line([8,0],[8,8]),line([8,8],[0,8]),line([0,8],[0,0])]])
  const points=[[4,-4],[12,-4],[12,4],[4,4]],cutter=validateBrepProfile([points.map((p,i)=>line(p,points[(i+1)%4]))])
  const intersection=booleanBrepProfiles(profile,cutter,'intersection')
  expect(intersection.loops.flat().some(c=>c.degree===degree)).toBe(true)
  const model=callGeometryRust('brep_nurbs_extrude_curves',{loops:intersection.loops,zMin:0,zMax:3})
  const mesh=callGeometryRust<{report:{closed:boolean;degenerateTriangles:number;nonManifoldEdges:number;orientationConflicts:number}}>('brep_nurbs_tessellate',{model,segments:32})
  expect(mesh.report).toMatchObject({closed:true,degenerateTriangles:0,nonManifoldEdges:0,orientationConflicts:0})
 }
})
it('extrudes a certified two-arc lens with a nondegenerate cap mesh',()=>{
 const a=quadratic(-1),b=quadratic(1);b.controlPoints.reverse()
 const p=validateBrepProfile([[a,b]])
 expect(p.areaMm2).toBeCloseTo(4/3,9)
 const model=callGeometryRust('brep_nurbs_extrude_curves',{loops:p.loops,zMin:0,zMax:2})
 const mesh=callGeometryRust<{report:object}>('brep_nurbs_tessellate',{model,segments:32})
 expect(mesh.report).toMatchObject({closed:true,degenerateTriangles:0,nonManifoldEdges:0,orientationConflicts:0})
})
