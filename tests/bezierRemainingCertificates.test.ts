import {expect,it} from 'vitest'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {certifyBrepCurveNonlinearAgreement,verifyBrepCurveNonlinearAgreementCertificate,certifyBrepCurveIntersections,verifyBrepCurveIntersectionsCertificate,offsetBrepWireCertified,verifyBrepOffsetWireCertificate,validateBrepProfile,booleanBrepProfiles,normalizeBrepProfile} from '../src/services/geometry/brepProfile'
import {nonlinearFirst,nonlinearSecond,nonlinearWire,fourthOrderContact,contactLine,degreeEightLoop,nonprimitiveLoop,exactCusp,cornerWire,squareWire,bezierLine} from './fixtures/bezier-algebraic-cases'
it('proves a non-rational parameter correspondence and uses it in production booleans',()=>{
 const c=certifyBrepCurveNonlinearAgreement(nonlinearFirst,nonlinearSecond,[0,1],[0,1])
 expect(verifyBrepCurveNonlinearAgreementCertificate(nonlinearFirst,nonlinearSecond,[0,1],[0,1],c).status).toBe('verified')
 const bad=structuredClone(c);bad.relation[0][0]=['1','1']
 expect(()=>verifyBrepCurveNonlinearAgreementCertificate(nonlinearFirst,nonlinearSecond,[0,1],[0,1],bad)).toThrow()
 const a=validateBrepProfile([nonlinearWire(nonlinearFirst)]),b=validateBrepProfile([nonlinearWire(nonlinearSecond)])
 expect(booleanBrepProfiles(a,b,'union').areaMm2).toBeCloseTo(a.areaMm2,7)
 expect(booleanBrepProfiles(a,b,'difference').loops).toEqual([])
 expect(normalizeBrepProfile([...a.loops,...b.loops]).loops).toEqual([])
})
it('checks a complete fourth-order contact census and rejects a missing event',()=>{
 const c=certifyBrepCurveIntersections(fourthOrderContact,contactLine)
 expect(verifyBrepCurveIntersectionsCertificate(fourthOrderContact,contactLine,c).events).toEqual([[.5,.5]])
 expect(()=>verifyBrepCurveIntersectionsCertificate(fourthOrderContact,contactLine,{...c,events:[]})).toThrow()
 expect(()=>certifyBrepCurveIntersections(fourthOrderContact,contactLine,true)).toThrow()
})
it('distinguishes an eighth-degree self intersection from a cusp singularity',()=>{
 const c=certifyBrepCurveIntersections(degreeEightLoop,degreeEightLoop,true)
 expect(verifyBrepCurveIntersectionsCertificate(degreeEightLoop,degreeEightLoop,c).events).toEqual([[.25,.75]])
 const rational=certifyBrepCurveIntersections(nonprimitiveLoop,nonprimitiveLoop,true)
 expect(verifyBrepCurveIntersectionsCertificate(nonprimitiveLoop,nonprimitiveLoop,rational).events).toEqual([[.25,.75]])
 const cusp=certifyBrepCurveIntersections(exactCusp,exactCusp,true)
 expect(verifyBrepCurveIntersectionsCertificate(exactCusp,exactCusp,cusp)).toMatchObject({events:[],singularParameters:[.5]})
 expect(()=>verifyBrepCurveIntersectionsCertificate(exactCusp,exactCusp,{...cusp,singularParameters:[]})).toThrow()
})
it('normalizes the eighth-degree loop using the exact original parent census',()=>{
 const profile=normalizeBrepProfile([[degreeEightLoop,bezierLine([0,84],[0,0])]])
 expect(profile.loops.length).toBeGreaterThanOrEqual(2)
 expect(profile.loops.flat().some(c=>c.degree===8)).toBe(true)
})
it('checks signed bevel, miter and round joins and rejects vertex or source tampering',()=>{
 for(const distance of [-2,2])for(const style of ['bevel','miter','round'] as const){
  const r=offsetBrepWireCertified(cornerWire,distance,.001,false,style,5)
  expect(verifyBrepOffsetWireCertificate(cornerWire,distance,.001,5,r.certificate).status).toBe('verified')
  const bad=structuredClone(r.certificate);bad.joins[0][0][0]+=1
  expect(()=>verifyBrepOffsetWireCertificate(cornerWire,distance,.001,5,bad)).toThrow()
  expect(()=>verifyBrepOffsetWireCertificate(cornerWire,-distance,.001,5,r.certificate)).toThrow()
 }
})
it('preserves a zero-offset point image in the wire result',()=>{
 const r=offsetBrepWireCertified([bezierLine([2,3],[2,3])],0,.001,false,'bevel',1)
 expect(r.curves[0].controlPoints).toEqual([[2,3],[2,3]])
})
it('extrudes a certified closed round-joined outward offset into a closed mesh',()=>{
 const r=offsetBrepWireCertified(squareWire,-1,.01,true,'round',3)
 const profile=validateBrepProfile([r.curves]),model=callGeometryRust('brep_nurbs_extrude_curves',{loops:profile.loops,zMin:0,zMax:3})
 const mesh=callGeometryRust<{report:object}>('brep_nurbs_tessellate',{model,segments:32})
 expect(mesh.report).toMatchObject({closed:true,degenerateTriangles:0,nonManifoldEdges:0,orientationConflicts:0})
})
