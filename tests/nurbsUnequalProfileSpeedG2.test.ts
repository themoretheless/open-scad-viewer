import {expect,it} from 'vitest'
import {bezierNurbsCurve} from '../src/services/nurbsConstructors'
import {createRationalBrepSectionLoft} from '../src/services/geometry/brep'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'
import {inspectSweepProjectiveSeams} from '../src/services/nurbsSweepAudit'

it.each([[1,2],[27,28]])('owns unequal retained profile speeds %j without smoothing sharp corners',(a,b)=>{
 const points=[[0,0],[a,0],[a+b,0],[a+b,1],[0,1],[0,0]]
 const sections=[0,1].map(z=>[points.slice(0,-1).map((p,i)=>{
  const q=points[i+1]!
  return bezierNurbsCurve([[p[0]!,p[1]!,z],[q[0]!,q[1]!,z]])
 })])
 const model=createRationalBrepSectionLoft(sections),caps=[model.faces.length-2,model.faces.length-1]
 const saved=JSON.stringify(model),proof=inspectMiterProfileSmoothness(model,caps)
 expect(proof).toMatchObject({extractionComplete:true,profileG1Certified:false,fullBoundarySmoothnessCertified:false})
 expect(proof.profile.seams[0]).toMatchObject({certified:true,exactIdentity:true,regularityCertified:true})
 expect(proof.profile.exactG1G2Certified).toBe(false)
 expect(proof.profile.seams.slice(1).some(s=>!s.certified)).toBe(true)
 const work=proof.profile.seams[0]!.work
 expect(work).toBeGreaterThan(0)
 expect(inspectMiterProfileSmoothness(model,caps,work-1).profile.seams[0]?.certified).toBe(false)
 expect(inspectMiterProfileSmoothness(model,caps,0).profile.seams.every(s=>!s.certified)).toBe(true)
 const kink=JSON.parse(saved)
 for(const p of kink.faces[0].surface.controlPoints[0])p[1]=Number.EPSILON
 expect(inspectMiterProfileSmoothness(kink,caps).profile.seams[0]?.certified).toBe(false)
 expect(JSON.stringify(model)).toBe(saved)
})

it('certifies the actual unequal-speed closing profile seam with explicit geometric scope',()=>{
 const points=[[1,0],[3,0],[3,1],[0,1],[0,0],[1,0]]
 const sections=[0,1].map(z=>[points.slice(0,-1).map((p,i)=>{
  const q=points[i+1]!
  return bezierNurbsCurve([[p[0]!,p[1]!,z],[q[0]!,q[1]!,z]])
 })])
 const model=createRationalBrepSectionLoft(sections),saved=JSON.stringify(model)
 const patches=model.faces.map(f=>f.surface)
 const seams=[{patches:[0,4] as [number,number],boundaries:['uMin','uMax'] as ['uMin','uMax'],order:2 as const,normalScale:.5,jetTolerance:0}]
 const proof=inspectSweepProjectiveSeams(patches,seams,2000000)
 expect(proof).toMatchObject({exactG1G2Certified:true,certifiedOrder:2})
 expect(proof.seams[0]).toMatchObject({certified:true,exactIdentity:true,regularityCertified:true})
 expect(inspectSweepProjectiveSeams(patches,seams,proof.exactWork-1).exactG1G2Certified).toBe(false)
 expect(inspectSweepProjectiveSeams(patches,seams,0).exactG1G2Certified).toBe(false)
 const kink=structuredClone(patches)
 for(const p of kink[0]!.controlPoints[1]!)p[1]=Number.EPSILON
 expect(inspectSweepProjectiveSeams(kink,seams,2000000).exactG1G2Certified).toBe(false)
 expect(JSON.stringify(model)).toBe(saved)
})
