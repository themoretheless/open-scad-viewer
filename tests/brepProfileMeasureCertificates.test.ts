import {expect,it} from 'vitest'
import {authorBrepProfile,certifyBrepProfile,verifyBrepProfileCertificate,certifyBrepProfileMeasure,verifyBrepProfileMeasureCertificate,certifyBrepCurveAgreement,verifyBrepCurveAgreementCertificate} from '../src/services/geometry/brepProfile'
import type {NurbsCurve} from '../src/services/nurbsCurve'

it('checks exact area ratios and rejects forged bounds and scope',()=>{
 const p=authorBrepProfile({kind:'rectangle',size:[8,8],center:false})
 const c=certifyBrepProfileMeasure(p,1e-9)
 expect(c.areaMm2).toEqual({lower:['64','1'],upper:['64','1']})
 expect(verifyBrepProfileMeasureCertificate(p,JSON.parse(JSON.stringify(c)))).toMatchObject({status:'verified',scope:'represented-profile-topology-and-area',areaMm2:c.areaMm2})
 for(const bad of [
  {...c,areaMm2:{lower:['65','1'],upper:['66','1']}},
  {...c,areaMm2:{lower:['64','0'],upper:['64','1']}},
  {...c,areaMm2:{lower:['128','2'],upper:['64','1']}},
  {...c,scope:'solid-validity'}, {...c,areaDepth:255}, {...c,solidValid:true},
 ])expect(()=>verifyBrepProfileMeasureCertificate(p,bad as typeof c)).toThrow()
 expect(()=>verifyBrepProfileMeasureCertificate({...p,toleranceMm:1e-6},c)).toThrow()
})

it('verifies a periodic cubic and a single stored multispan boundary without rewriting sources',()=>{
 const periodic:NurbsCurve={degree:3,knots:Array.from({length:11},(_,i)=>i),controlPoints:[[0,0],[8,0],[8,8],[0,8],[0,0],[8,0],[8,8]],weights:Array(7).fill(1),periodic:true}
 const polyline:NurbsCurve={degree:1,knots:[0,0,1,2,3,4,4],controlPoints:[[0,0],[8,0],[8,8],[0,8],[0,0]],weights:Array(5).fill(1),periodic:false}
 for(const curve of [periodic,polyline]){
  const p={loops:[[curve]],toleranceMm:1e-7},before=JSON.stringify(p)
  const c=certifyBrepProfile(p)
  expect(verifyBrepProfileCertificate(p,c)).toMatchObject({status:'verified',spans:4})
  const m=certifyBrepProfileMeasure(p,1e-9)
  expect(verifyBrepProfileMeasureCertificate(p,m).status).toBe('verified')
  expect(JSON.stringify(p)).toBe(before)
 }
})

it('certifies degree-eight boundaries with extended exact separator witnesses',()=>{
 const vertices=[[0,0],[8,8],[0,16],[-8,8]]
 const loops=[vertices.map((a,i):NurbsCurve=>{const b=vertices[(i+1)%4];return {
  degree:8,knots:[...Array(9).fill(0),...Array(9).fill(1)],
  controlPoints:Array.from({length:9},(_,j)=>[a[0]+(b[0]-a[0])*j/8,a[1]+(b[1]-a[1])*j/8]),weights:Array(9).fill(1),periodic:false,
 }})]
 const p={loops,toleranceMm:1e-7},c=certifyBrepProfile(p)
 expect(c.separatingNormals.some(n=>n>=256&&n<1280)).toBe(true)
 expect(verifyBrepProfileCertificate(p,c).status).toBe('verified')
 expect(certifyBrepProfileMeasure(p,1e-9).areaMm2).toEqual({lower:['128','1'],upper:['128','1']})
})

it('proves partial rational-cubic/quadratic agreement and refuses changed domains and points',()=>{
 const a:NurbsCurve={degree:3,knots:[0,0,0,0,1,1,1,1],controlPoints:[[0,0],[2.5,0],[6,1],[10,5]],weights:[3,4,5,6],periodic:false}
 const b:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[5,0],[10,5]],weights:[1,1,1],periodic:false}
 const c=certifyBrepCurveAgreement(a,b,[.25,.75],[.25,.75])
 expect(verifyBrepCurveAgreementCertificate(a,b,[.25,.75],[.25,.75],c)).toEqual({status:'verified',scope:'represented-curve-affine-parameter-agreement',commonSpans:1})
 const changed=structuredClone(b);changed.controlPoints[1][1]=Number.MIN_VALUE
 expect(()=>verifyBrepCurveAgreementCertificate(a,changed,[.25,.75],[.25,.75],c)).toThrow()
 expect(()=>certifyBrepCurveAgreement(a,changed,[.25,.75],[.25,.75])).toThrow()
 expect(()=>verifyBrepCurveAgreementCertificate(a,b,[0,1],[0,1],c)).toThrow()
 expect(()=>verifyBrepCurveAgreementCertificate(a,b,[.25,.75],[.25,.75],{...c,commonSpans:0})).toThrow()
})
