import {expect,it} from 'vitest'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveMiterBrepProfileBody,transformCertifiedMiterBody} from '../src/services/geometry/brep'
const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
const shear=[[1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]]
const options={quantum:2**-40,maxWork:100000,maxDeviation:1e-9}
const source=()=>createProgressiveMiterBrepProfileBody([[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]],[[0,0,0],[0,0,10]],law(1),law(0),{normal:[1,0,0],maxDeviation:.01,maxSteps:1,circleCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:100000}})
it('places corrected hollow geometry exactly and scales the complete boundary bound',()=>{
 const original=source(),before=structuredClone(original.model)
 for(const matrix of [shear,[[-1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]]]){
  const placed=transformCertifiedMiterBody(original,matrix,options)
  expect(placed.placement).toMatchObject({reason:'exact-binary-lattice-placement',arithmeticErrorUpper:0})
  expect(placed.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
  expect(placed.boundaryCertificate.errorUpper).toBeGreaterThan(original.boundaryCertificate.errorUpper!)
  expect(placed.profileSmoothness.profile).toMatchObject({exactG1G2Certified:true,certifiedOrder:2})
  expect(placed.volume.solidGeometryCertified).toBe(true)
 }
 expect(original.model).toEqual(before)
})
it('refuses unsupported arithmetic, exhausted work, stale ownership and boundary budget',()=>{
 const original=source()
 expect(()=>transformCertifiedMiterBody(original,[[1,0,0,0],[0,1,0,0],[.1,1,1,0],[0,0,0,1]],options)).toThrow(/unsupported-matrix/)
 expect(()=>transformCertifiedMiterBody(original,shear,{...options,maxWork:1})).toThrow(/work-limit/)
 expect(()=>transformCertifiedMiterBody(original,shear,{...options,maxDeviation:1e-15})).toThrow(/complete boundary error/)
 expect(()=>transformCertifiedMiterBody(structuredClone(original),shear,options)).toThrow(/constructor-owned/)
 original.model.faces[0]!.surface.controlPoints[0]![0]![0]!+=1e-6
 expect(()=>transformCertifiedMiterBody(original,shear,options)).toThrow(/constructor-owned/)
})

it('keeps transformed-bound evidence through actual Rush and repeated placements',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileModelGraphText}=await import('../src/services/modelGraphText')
 const {buildOwnNurbs,buildOwnNurbsAsync}=await import('../src/services/modelGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const text=readFileSync('examples/rush/progressive-miter-oblique-circle-corrected-hollow.r','utf8')
 const compiled=compileModelGraphText(text)
 const built=buildOwnNurbs(compiled.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG2Certified:true,continuousBound:true,solidGeometryCertified:true})
 const asyncBuilt=await buildOwnNurbsAsync(compiled.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(asyncBuilt.nativeGeometry).toEqual(built.nativeGeometry)
 expect(asyncBuilt.report.construction).toEqual(built.report.construction)
 const reflected=text.replace('.brep_tessellate(2)', '.transform(matrix: [[-1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]).brep_tessellate(2)')
 const second=buildOwnNurbs(compileModelGraphText(reflected).document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(readSweepViewportEvidence(second.nativeGeometry)).toMatchObject({profileG2Certified:true,continuousBound:true,solidGeometryCertified:true})
})
