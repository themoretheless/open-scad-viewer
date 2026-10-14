import {expect,it} from 'vitest'
import {callNurbsRust} from '../src/services/geometry/nurbs'
it('transports native retained face ownership without claiming embedding',()=>{
 const model={faces:[null,null],shells:[{closed:true,faces:[{face:0,reversed:false}]},{closed:true,faces:[{face:1,reversed:true}]}],bodies:[{outerShell:0,innerShells:[1]}]}
 const audit=(value:unknown,maxFaces=2)=>callNurbsRust('sweep_retained_body_coverage_audit',{model:value,maxFaces})
 const before=structuredClone(model)
 expect(audit(model)).toEqual({faceCoverageCertified:true,globalEmbeddingCertified:false})
 expect(audit(model,1)).toMatchObject({faceCoverageCertified:false})
 const repeated=structuredClone(model);repeated.shells[1]!.faces[0]!.face=0
 expect(audit(repeated)).toMatchObject({faceCoverageCertified:false})
 const wrongOwner=structuredClone(model);wrongOwner.bodies[0]!.innerShells=[0]
 expect(audit(wrongOwner)).toMatchObject({faceCoverageCertified:false})
 const open=structuredClone(model);open.shells[1]!.closed=false
 expect(audit(open)).toMatchObject({faceCoverageCertified:false})
 const invalid=structuredClone(model) as any;invalid.shells[0].faces[0].reversed=1
 expect(()=>audit(invalid)).toThrow()
 for(const face of [-1,.5,2]){
  const wrong=structuredClone(model);wrong.shells[0]!.faces[0]!.face=face
  if(face===2)expect(audit(wrong)).toMatchObject({faceCoverageCertified:false})
  else expect(()=>audit(wrong)).toThrow()
 }
 expect(model).toEqual(before)
})
