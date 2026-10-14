import {expect,it} from 'vitest'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveMiterBrepProfileBody,reconstructCertifiedMiterStations,transformCertifiedMiterBody} from '../src/services/geometry/brep'
const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
const shear=[[1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]]
const options={quantum:2**-40,maxWork:100000,maxDeviation:1e-9}
const source=()=>createProgressiveMiterBrepProfileBody([[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]],[[0,0,0],[0,0,10]],law(1),law(0),{normal:[1,0,0],maxDeviation:.01,maxSteps:1,circleCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:100000}})
it('replays native station reconstruction before successive affine placements',()=>{
 const original=createProgressiveMiterBrepProfileBody([[circleNurbsCurve([0,0,0],[0,0,1],.5)]],
  [[0,0,0],[0,0,10]],law(1),law(0),{normal:[1,0,0],initialSteps:2,maxSteps:2,maxDeviation:.01,
   circleCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:100000}})
 const before=structuredClone(original.model)
 const smoothed=reconstructCertifiedMiterStations(original,{quantum:2**-40,maxWork:100000,wallTolerance:.01,maxDeviation:.01})
 const placed=transformCertifiedMiterBody(smoothed,shear,{...options,maxDeviation:.01})
 const reflected=transformCertifiedMiterBody(placed,[[-1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],{...options,maxDeviation:.01})
 expect(reflected.volume.solidGeometryCertified).toBe(true)
 expect(reflected.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(reflected.boundaryCertificate.errorUpper).toBeGreaterThanOrEqual(smoothed.boundaryCertificate.errorUpper!)
 expect(original.model).toEqual(before)
 expect(()=>transformCertifiedMiterBody(smoothed,shear,{...options,maxDeviation:0})).toThrow(/complete boundary error/)
})
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
 const alteredCertificate=source()
 alteredCertificate.boundaryCertificate.filledCapErrorUpper![0]+=1e-6
 expect(()=>transformCertifiedMiterBody(alteredCertificate,shear,options)).toThrow(/constructor-owned/)
 original.model.faces[0]!.surface.controlPoints[0]![0]![0]!+=1e-6
 expect(()=>transformCertifiedMiterBody(original,shear,options)).toThrow(/constructor-owned/)
})
it('checks intermediate model and certificate against native placement replay',()=>{
 const placed=transformCertifiedMiterBody(source(),shear,options)
 const identity=[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]
 const certificate=structuredClone(placed.boundaryCertificate)
 placed.boundaryCertificate.wallErrorUpper!+=1e-6
 expect(()=>transformCertifiedMiterBody(placed,identity,options)).toThrow(/constructor-owned replay certificate/)
 placed.boundaryCertificate=certificate
 placed.model.faces[0]!.surface.controlPoints[0]![0]![0]!+=.125
 expect(()=>transformCertifiedMiterBody(placed,identity,options)).toThrow(/constructor-owned replay model/)
})

it('keeps transformed-bound evidence through actual Rush and repeated placements',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs,buildOwnNurbsAsync}=await import('../src/services/rushGraphNurbsKernel')
 const {readSweepViewportEvidence}=await import('../src/services/sweepViewportEvidence')
 const text=readFileSync('examples/rush/progressive-miter-oblique-circle-corrected-hollow.r','utf8')
 const compiled=compileRushFrontend(text)
 const built=buildOwnNurbs(compiled.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG2Certified:true,continuousBound:true,solidGeometryCertified:true})
 const asyncBuilt=await buildOwnNurbsAsync(compiled.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(asyncBuilt.nativeGeometry).toEqual(built.nativeGeometry)
 expect(asyncBuilt.report.construction).toEqual(built.report.construction)
 const reflected=text.replace('.brep_tessellate(2)', '.transform(matrix: [[-1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]).brep_tessellate(2)')
 const second=buildOwnNurbs(compileRushFrontend(reflected).document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(readSweepViewportEvidence(second.nativeGeometry)).toMatchObject({profileG2Certified:true,continuousBound:true,solidGeometryCertified:true})
})

it('transports the whole boundary atomically in the native affine opcode',async()=>{
 const {callGeometryRust}=await import('../src/services/geometry/kernel')
 const original=source(),before=structuredClone(original)
 const request={model:original.model,sourceCertificate:original.boundaryCertificate,matrix:shear,
  quantum:options.quantum,maxWork:options.maxWork,budget:options.maxDeviation}
 type Result={reason:string;placement:{model:unknown|null;operatorNormUpper:number|null}|null;
  boundaryCertificate:{continuousBound:boolean;withinBudget:boolean|null;filledCapErrorUpper:[number,number]|null;wallErrorUpper:number|null}|null}
 const result=callGeometryRust<Result>('brep_miter_affine_boundary',request)
 expect(result.reason).toBe('exact-affine-complete-boundary')
 expect(result.placement?.model).not.toBeNull()
 expect(result.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(result.boundaryCertificate!.filledCapErrorUpper).toHaveLength(2)
 for(let i=0;i<2;i++)expect(result.boundaryCertificate!.filledCapErrorUpper![i]).toBeGreaterThanOrEqual(
  original.boundaryCertificate.filledCapErrorUpper![i]!*result.placement!.operatorNormUpper!)
 const budgetRefusal=callGeometryRust<Result>('brep_miter_affine_boundary',{...request,budget:1e-15})
 expect(budgetRefusal.placement?.model).toBeNull()
 expect(budgetRefusal.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:false})
 for(const maxWork of [0,1]){
  const denied=callGeometryRust<Result>('brep_miter_affine_boundary',{...request,maxWork})
  expect(denied.placement).toBeNull()
  expect(denied.reason).toBe('work-limit')
 }
 const missing=callGeometryRust<Result>('brep_miter_affine_boundary',{
  ...request,sourceCertificate:{...request.sourceCertificate,filledCapErrorUpper:null},
 })
 expect(missing).toMatchObject({placement:null,boundaryCertificate:null,reason:'source-bound-unproved'})
 expect(original).toEqual(before)
})

it.each(['miter-periodic-moving-axis-guide-affine-hollow.r','miter-periodic-moving-axis-guide-affine-hollow-authored-caps.r','closed-periodic-frame-guide-affine-hollow.r'])('admits %s shear and reflection through bound native replay',async(fixture)=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const text=readFileSync(`examples/rush/${fixture}`,'utf8')
 const transformed=text.replace('.brep_tessellate(4)',
  '.transform(matrix: [[1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]]).transform(matrix: [[-1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]).brep_tessellate(4)')
 expect(transformed).not.toBe(text)
 const graph=compileRushFrontend(transformed).document,before=structuredClone(graph)
 const transformedBody=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 const smoothReports=Object.values(transformedBody.report.construction!) as any[]
 const auditedProfiles=smoothReports.filter(r=>r.profileSmoothness)
 expect(auditedProfiles.length).toBe(3)
 expect(auditedProfiles.every(r=>r.profileSmoothness.profileG1Certified)).toBe(true)
 expect(auditedProfiles.every(r=>r.profileSmoothness.profile.exactG1G2Certified)).toBe(true)
 expect(auditedProfiles.every(r=>r.profileSmoothness.stationContinuity==='C0')).toBe(true)
 const actualModel=JSON.parse(transformedBody.nativeGeometry!.geometryJson).geometry
 expect(inspectProgressiveSweepSolidAdmission(transformedBody.nativeGeometry!,actualModel).solidGeometryCertified).toBe(true)
 const changed=structuredClone(actualModel)
 changed.faces[0].surface.controlPoints[0][0][0]+=.125
 expect(()=>inspectProgressiveSweepSolidAdmission(transformedBody.nativeGeometry!,changed)).toThrow(/final model differs/)
 const replay=JSON.parse(transformedBody.nativeGeometry!.geometryJson)
 expect(replay.sweepMiterReplay.followingPlacements).toHaveLength(1)
 replay.sweepMiterReplay.matrix[0][0]=-1
 expect(()=>inspectProgressiveSweepSolidAdmission({...transformedBody.nativeGeometry!,geometryJson:JSON.stringify(replay)},actualModel)).toThrow(/final model differs|replay Solid|binding/)

 const original=buildOwnNurbs(compileRushFrontend(text).document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(inspectProgressiveSweepSolidAdmission(original.nativeGeometry!,JSON.parse(original.nativeGeometry!.geometryJson).geometry).solidGeometryCertified).toBe(true)
 expect(graph).toEqual(before)
})

it('admits reflected automatic periodic caps with exact off-lattice native arithmetic',async()=>{
 const {readFileSync}=await import('node:fs')
 const {compileRushFrontend}=await import('../src/services/rushFrontend')
 const {buildOwnNurbs}=await import('../src/services/rushGraphNurbsKernel')
 const {inspectProgressiveSweepSolidAdmission}=await import('../src/services/sweepSolidAdmission')
 const source=readFileSync('examples/rush/miter-periodic-moving-axis-guide-affine-hollow.r','utf8')
 const reflected=source.replace('.brep_tessellate(4)', '.transform(matrix: [[-1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]).brep_tessellate(4)')
 const graph=compileRushFrontend(reflected).document,before=structuredClone(graph)
 const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(inspectProgressiveSweepSolidAdmission(built.nativeGeometry!,JSON.parse(built.nativeGeometry!.geometryJson).geometry).solidGeometryCertified).toBe(true)
 const reports=Object.values(built.report.construction!) as any[]
 expect(reports.some(r=>r.placement?.reason==='exact-binary-axis-permutation'&&r.placement.arithmeticErrorUpper===0&&r.boundaryCertificate.continuousBound&&r.volume.solidGeometryCertified)).toBe(true)
 expect(graph).toEqual(before)
})
