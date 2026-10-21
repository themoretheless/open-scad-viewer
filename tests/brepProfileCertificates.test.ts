import {expect,it} from 'vitest'
import {authorBrepProfile,certifyBrepProfile,transformBrepProfile,validateBrepProfile,verifyBrepProfileCertificate} from '../src/services/geometry/brepProfile'
import {callGeometryRust} from '../src/services/geometry/kernel'
import type {NurbsCurve} from '../src/services/nurbsCurve'
const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
const cubic=(p:number[][],weights=[1,1,1,1]):NurbsCurve=>({degree:3,knots:[0,0,0,0,1,1,1,1],controlPoints:p,weights,periodic:false})
const reverse=(c:NurbsCurve):NurbsCurve=>({...c,controlPoints:[...c.controlPoints].reverse(),weights:[...c.weights].reverse()})

it('checks a portable certificate and rejects source, witness, schema and tolerance tampering in Rust',()=>{
 const p=authorBrepProfile({kind:'rectangle',size:[10,10],center:false})
 const certificate=certifyBrepProfile(p),before=JSON.stringify(p)
 expect(verifyBrepProfileCertificate(p,JSON.parse(JSON.stringify(certificate)))).toMatchObject({
  status:'verified',scope:'represented-profile-topology',spans:4,nestingDepth:[0],
 })
 for(const bad of [
  {...certificate,source:certificate.source+'0'},
  {...certificate,version:99},
  {...certificate,monotoneAxes:[]},
  {...certificate,separatingNormals:[]},
  {...certificate,orientation:[-1]},
  {...certificate,nestingDepth:[1]},
  {...certificate,solidValid:true},
 ])expect(()=>verifyBrepProfileCertificate(p,bad)).toThrow()
 expect(()=>verifyBrepProfileCertificate({...p,toleranceMm:1e-6},certificate)).toThrow()
 const changed=structuredClone(p);changed.loops[0][0].weights[0]=2
 expect(()=>verifyBrepProfileCertificate(changed,certificate)).toThrow()
 expect(JSON.stringify(p)).toBe(before)
 expect(p.geometryStatus).toBe('numerical_uncertified')
})

it('certifies a retained same-side cusp using algebraic separation in both coordinate frames',()=>{
 for(const rotated of [false,true]){
  const a=cubic([[0,0],[2,0],[4,2],[6,6]]),b=cubic([[0,0],[2,0],[6,2],[12,12]])
  const wire=[b,line([12,12],[6,6]),reverse(a)]
  if(rotated)for(const c of wire)for(const p of c.controlPoints){const[x,y]=p;p[0]=17+x-y;p[1]=-9+x+y}
  const p=validateBrepProfile([wire]),certificate=certifyBrepProfile(p)
  expect(certificate.separatingNormals).toContain(65535)
  expect(verifyBrepProfileCertificate(p,certificate).status).toBe('verified')
  expect(()=>verifyBrepProfileCertificate(p,{...certificate,orientationDepth:null})).toThrow()
 }
})

it('retains rational cubics and affine ellipses and refuses an unproved crossing',()=>{
 const c=cubic([[0,0],[2.5,0],[6,1],[10,5]],[3,4,5,6])
 const p=validateBrepProfile([[line([0,0],[10,0]),line([10,0],[10,5]),reverse(c)]])
 expect(p.areaMm2).toBeCloseTo(50/3,5)
 expect(verifyBrepProfileCertificate(p,certifyBrepProfile(p)).status).toBe('verified')
 const circle=authorBrepProfile({kind:'circle',radius:2})
 const ellipse=transformBrepProfile(circle,[3,0,0,0,1,2,0,0,0,0,1,0,0,0,0,1])
 expect(ellipse.areaMm2).toBeCloseTo(24*Math.PI,5)
 expect(ellipse.loops.flat().every(c=>c.degree===2)).toBe(true)
 expect(verifyBrepProfileCertificate(ellipse,certifyBrepProfile(ellipse)).status).toBe('verified')
 const points=[[0,0],[4,4],[0,4],[4,0]]
 expect(()=>callGeometryRust('brep_profile_certify',{loops:[points.map((p,i)=>line(p,points[(i+1)%4]))],toleranceMm:1e-7})).toThrow()
})
