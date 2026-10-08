import {describe,it,expect} from 'vitest'
import {sampleAssemblyAnimation,validateAssemblyAnimation,type AssemblyAnimation} from '../src/services/assemblyAnimation'
import {emptyDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
function seed(){const d=emptyDirectDocument(),brep=createBrepBox([0,0,0],[2,2,2]);d.bodies.push({id:'part',name:'Part',brep,mesh:tessellateNurbsBrep(brep,1)});return d}
const animation:AssemblyAnimation={version:1,frames:[{time:0,poses:[{id:'part',translation:[0,0,0],angle:0,scale:1}],camera:{yaw:350*Math.PI/180,pitch:0}},{time:2,poses:[{id:'part',translation:[10,0,0],angle:0,scale:1}],camera:{yaw:10*Math.PI/180,pitch:.4}}]}
describe('native assembly keyframes',()=>{
 it('samples body and camera together while preserving source and retained geometry',()=>{
  const doc=seed(),before=serializeDirectDocument(doc),frame=sampleAssemblyAnimation(doc,animation,1)
  expect(frame.document.bodies[0].brep).toBeDefined();expect(inspectPolygonMesh(frame.document.bodies[0].mesh).signedVolumeMm3).toBeCloseTo(8)
  expect(frame.document.bodies[0].mesh.positions[0]).toBeCloseTo(5);expect(frame.camera.yaw).toBeCloseTo(2*Math.PI);expect(frame.camera.pitch).toBeCloseTo(.2)
  expect(serializeDirectDocument(doc)).toBe(before)
 })
 it('refuses invalid timing, missing tracks and nonfinite poses',()=>{
  expect(()=>validateAssemblyAnimation(seed(),{...animation,frames:[animation.frames[1],animation.frames[0]]})).toThrow(/times/)
  const invalid=structuredClone(animation);invalid.frames[1].poses[0].translation[0]=NaN
  expect(()=>validateAssemblyAnimation(seed(),invalid)).toThrow(/poses/)
  expect(()=>validateAssemblyAnimation(emptyDirectDocument(),animation)).toThrow(/independent/)
 })
})

it('rotates a nonsymmetric body around all three axes without changing its volume or source',()=>{
 const doc=seed(),brep=createBrepBox([0,0,0],[2,4,6]);doc.bodies[0]={...doc.bodies[0],brep,mesh:tessellateNurbsBrep(brep,1)}
 const before=serializeDirectDocument(doc),clip=structuredClone(animation);clip.frames[0].poses[0].rotation=[0,0,0];clip.frames[1].poses[0].rotation=[90,90,90];clip.frames[1].poses[0].translation=[0,0,0]
 const final=sampleAssemblyAnimation(doc,clip,2).document.bodies[0]
 expect(inspectPolygonMesh(final.mesh).signedVolumeMm3).toBeCloseTo(48)
 expect(final.mesh.positions).not.toEqual(doc.bodies[0].mesh.positions);expect(final.brep).toBeDefined();expect(serializeDirectDocument(doc)).toBe(before)
 clip.frames[1].poses[0].rotation=[NaN,0,0];expect(()=>validateAssemblyAnimation(doc,clip)).toThrow(/poses/)
})
