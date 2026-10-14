import {expect,it} from 'vitest'
import {createNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {readSweepViewportEvidence,readSweepPatchViewportEvidence} from '../src/services/sweepViewportEvidence'
it('keeps solid geometry independent of incomplete authored sweep guarantees',()=>{
 const artifact=createNativeGeometryArtifact('sweep','brep',{geometry:{},sweepEvidence:{
  volume:{solidGeometryCertified:true},continuousBound:false,
  profileRegularityCertified:true,wallRegularityCertified:true,
 }},{})
 expect(readSweepViewportEvidence(artifact)).toEqual({nodeId:'sweep',solidGeometryCertified:true,
  boundaryErrorUpper:null,boundaryErrorBudget:null,boundaryErrorWithinBudget:null,
  continuousBound:false,profileRegularityCertified:true,wallRegularityCertified:true})
 const unproved=createNativeGeometryArtifact('sweep','brep',{geometry:{},sweepEvidence:{
  volume:{solidGeometryCertified:false},continuousBound:'true',profileRegularityCertified:1,
 }},{})
 expect(readSweepViewportEvidence(unproved)).toMatchObject({solidGeometryCertified:false,
  continuousBound:false,profileRegularityCertified:false,wallRegularityCertified:false})
 expect(readSweepViewportEvidence(undefined)).toBeNull()
 expect(readSweepViewportEvidence(createNativeGeometryArtifact('edited','brep',{geometry:{}},{}))).toBeNull()
 expect(readSweepViewportEvidence(createNativeGeometryArtifact('mesh','mesh',{sweepEvidence:{volume:{solidGeometryCertified:true}}},{}))).toBeNull()
})
it('does not promote a certified retained-patch error to complete capped-body evidence',()=>{
 const patchReport={accepted:true,continuousBound:true,roundingCertified:true,
  continuousErrorUpper:.001,budget:.01,
  continuousErrorScope:'retained-patches-relative-to-original-profile-transport'}
 const artifact=createNativeGeometryArtifact('sweep','brep',{geometry:{},sweepEvidence:{
  ...patchReport,volume:{solidGeometryCertified:false},
  boundaryErrorUpper:.001,boundaryErrorBudget:.01,boundaryErrorWithinBudget:true,
  boundaryCertificate:patchReport,
 }},{})
 expect(readSweepViewportEvidence(artifact)).toMatchObject({
  continuousBound:false,solidGeometryCertified:false,boundaryErrorWithinBudget:true,
 })
})
it('shows a finite boundary budget separately and refuses inconsistent budget status',()=>{
 const artifact=(upper:unknown,budget:unknown,within:unknown)=>createNativeGeometryArtifact('sweep','brep',{geometry:{},sweepEvidence:{volume:{solidGeometryCertified:true},continuousBound:false,boundaryErrorUpper:upper,boundaryErrorBudget:budget,boundaryErrorWithinBudget:within}},{})
 expect(readSweepViewportEvidence(artifact(.25,.3,true))).toMatchObject({boundaryErrorUpper:.25,boundaryErrorBudget:.3,boundaryErrorWithinBudget:true,continuousBound:false})
 expect(readSweepViewportEvidence(artifact(.35,.3,false))).toMatchObject({boundaryErrorWithinBudget:false})
 for(const args of [[.35,.3,true],[.25,.3,'true'],[NaN,.3,true],[-1,.3,true]] as [unknown,unknown,unknown][])expect(readSweepViewportEvidence(artifact(...args))?.boundaryErrorWithinBudget).toBeNull()
})
it('presents complete boundary certification only with matching union scope and numerical evidence',()=>{
 const certificate={method:'retained-sweep-boundary-union',scope:'boundary-set-hausdorff',continuousBound:true,errorUpper:.25,budget:.3,withinBudget:true}
 const artifact=(boundaryCertificate:unknown)=>createNativeGeometryArtifact('sweep','brep',{geometry:{},sweepEvidence:{volume:{solidGeometryCertified:true},continuousBound:true,boundaryErrorUpper:.25,boundaryErrorBudget:.3,boundaryErrorWithinBudget:true,boundaryCertificate}},{})
 expect(readSweepViewportEvidence(artifact(certificate))).toMatchObject({continuousBound:true,boundaryErrorWithinBudget:true})
 for(const c of [null,{...certificate,scope:'wall'},{...certificate,errorUpper:.2},{...certificate,budget:1},{...certificate,withinBudget:false},{...certificate,continuousBound:false}])expect(readSweepViewportEvidence(artifact(c))?.continuousBound).toBe(false)
})
it('presents scoped profile G2 only with complete extraction and every exact seam, keeping path and cap C0',()=>{
 const smoothness={method:'retained-miter-profile-joins',scope:'wall-profile-seams',edgeIds:[3],extractionComplete:true,unclassifiedFaces:[],unpairedEdges:[],stationContinuity:'C0',capContinuity:'C0',fullBoundarySmoothnessCertified:false,
  profile:{method:'constant-projective-strip-jets',exactG1G2Certified:true,certifiedOrder:2,exactWork:100,unresolvedSeams:[],seams:[{certified:true,exactIdentity:true,regularityCertified:true,certifiedOrder:2}]}}
 const artifact=(profileSmoothness:unknown)=>createNativeGeometryArtifact('sweep','brep',{geometry:{},sweepEvidence:{volume:{solidGeometryCertified:true},profileSmoothness}},{})
 expect(readSweepViewportEvidence(artifact(smoothness))).toMatchObject({profileG2Certified:true,profileSeamCount:1,stationContinuity:'C0',capContinuity:'C0',continuousBound:false})
 for(const bad of [{...smoothness,extractionComplete:false},{...smoothness,unpairedEdges:[4]}, {...smoothness,edgeIds:[]},{...smoothness,profile:{...smoothness.profile,unresolvedSeams:[0]}},{...smoothness,profile:{...smoothness.profile,exactWork:1000001}},{...smoothness,profile:{...smoothness.profile,seams:[{certified:true,exactIdentity:false,regularityCertified:true,certifiedOrder:2}]}}])expect(readSweepViewportEvidence(artifact(bad))).toMatchObject({profileG2Certified:false,solidGeometryCertified:true,continuousBound:false})
})
it('presents independently proved G1 when G2 is unproved and rejects inconsistent shared budget evidence',()=>{
 const audit={method:'constant-projective-strip-jets',exactG1G2Certified:true,certifiedOrder:1,exactWork:40,unresolvedSeams:[],seams:[{certified:true,exactIdentity:true,regularityCertified:true,certifiedOrder:1}]}
 const smooth={method:'retained-miter-profile-joins',scope:'wall-profile-seams',edgeIds:[3],extractionComplete:true,unclassifiedFaces:[],unpairedEdges:[],stationContinuity:'C0',capContinuity:'C0',profileG1Certified:true,g1Method:'exact-projective-audit',exactWork:100,g1Audit:audit,
  profile:{method:'constant-projective-strip-jets',exactG1G2Certified:false,certifiedOrder:null,exactWork:60,unresolvedSeams:[0],seams:[{certified:false}]}}
 const artifact=(profileSmoothness:unknown)=>createNativeGeometryArtifact('sweep','brep',{geometry:{},sweepEvidence:{volume:{solidGeometryCertified:true},profileSmoothness}},{})
 expect(readSweepViewportEvidence(artifact(smooth))).toMatchObject({profileG1Certified:true,profileG2Certified:false,stationContinuity:'C0'})
 for(const bad of [{...smooth,exactWork:99},{...smooth,profileG1Certified:false},{...smooth,g1Method:'unproved'},{...smooth,extractionComplete:false},{...smooth,g1Audit:{...audit,certifiedOrder:2}},{...smooth,g1Audit:{...audit,exactWork:1000000},exactWork:1000060}])expect(readSweepViewportEvidence(artifact(bad))).toMatchObject({profileG1Certified:false,profileG2Certified:false})
})
it('does not display station G2 from flags alone or inconsistent aggregate work',()=>{
 const jet={certified:true,exactIdentity:true,regularityCertified:true,certifiedOrder:2}
 const audit={method:'constant-projective-strip-jets',exactG1G2Certified:true,certifiedOrder:2,exactWork:10,unresolvedSeams:[],seams:[jet]}
 const station={method:'retained-miter-station-joins',scope:'wall-station-seams',maxWork:90,exactWork:10,extractionComplete:true,unclassifiedFaces:[],unpairedEdges:[],capEdges:[],edgeIds:[4],g2:audit,g1Audit:null,stationG1Certified:true,stationG2Certified:true}
 const smooth={method:'retained-miter-profile-joins',scope:'wall-profile-seams',maxWork:100,exactWork:10,totalExactWork:20,edgeIds:[3],extractionComplete:true,unclassifiedFaces:[],unpairedEdges:[],profile:audit,g1Audit:null,station,stationContinuity:'G2'}
 const artifact=(profileSmoothness:unknown)=>createNativeGeometryArtifact('sweep','brep',{geometry:{},sweepEvidence:{volume:{solidGeometryCertified:true},profileSmoothness}},{})
 expect(readSweepViewportEvidence(artifact(smooth))).toMatchObject({stationG2Certified:true,stationContinuity:'G2'})
 for(const bad of [{...smooth,totalExactWork:19},{...smooth,station:{...station,maxWork:100}},{...smooth,station:{...station,unpairedEdges:[9]}},{...smooth,station:{...station,g2:{...audit,exactWork:-1},exactWork:-1,totalExactWork:9}},{...smooth,station:{...station,g2:{...audit,seams:[{...jet,regularityCertified:false}]}}}])expect(readSweepViewportEvidence(artifact(bad))).toMatchObject({stationG2Certified:false,stationContinuity:'C0',solidGeometryCertified:true})
})


it('keeps complete and partial retained-patch evidence scoped to surface transport',()=>{
 const report={method:'progressive-fourfold-section-refinement',accepted:true,continuousBound:true,roundingCertified:true,
  continuousErrorScope:'retained-patches-relative-to-original-profile-transport',continuousErrorUpper:.001,knownProfileErrorUpper:.001,
  budget:.01,errorCertificateCells:123,decompositionProducts:4,errorCertificateReason:null}
 const artifact=(r:unknown)=>createNativeGeometryArtifact('sweep','patches',{geometry:[],sweepPatchEvidence:r},{})
 expect(readSweepPatchViewportEvidence(artifact(report))).toMatchObject({continuousBound:true,errorUpper:.001,budget:.01})
 expect(readSweepViewportEvidence(artifact(report))).toBeNull()
 expect(readSweepPatchViewportEvidence(artifact({...report,errorCertificateMaxCells:100000,errorCertificateCells:36535}))).toMatchObject({continuousBound:true})
 for(const change of [{errorCertificateMaxCells:100001},{errorCertificateMaxCells:100.5},
  {errorCertificateMaxCells:122},{decompositionMaxProducts:3},{decompositionMaxProducts:1000001}]){
  expect(readSweepPatchViewportEvidence(artifact({...report,...change}))).toMatchObject({continuousBound:false})
 }

 for(const change of [{continuousErrorScope:'boundary-set-hausdorff'},{roundingCertified:false},{knownProfileErrorUpper:.002},
  {budget:.0001},{errorCertificateCells:10001},{decompositionProducts:1000001},{continuousErrorUpper:Infinity}]){
  expect(readSweepPatchViewportEvidence(artifact({...report,...change}))).toMatchObject({continuousBound:false,errorUpper:null})
 }
 expect(readSweepPatchViewportEvidence(artifact({...report,accepted:false}))).toBeNull()
 expect(readSweepPatchViewportEvidence(createNativeGeometryArtifact('body','brep',{sweepPatchEvidence:report},{}))).toBeNull()
 expect(readSweepPatchViewportEvidence(artifact({...report,continuousBound:false,continuousErrorUpper:null,
  errorCertificateReason:'arc-length-correspondence-unproved'}))).toMatchObject({continuousBound:false,errorUpper:null,reason:'arc-length-correspondence-unproved'})
})

it('keeps retained surface regularity independent from error and body admission',()=>{
 const r={method:'actual-retained-patch-shared-jacobian-regularity',surfaceRegularityCertified:true,cells:7,unresolvedPatches:[],
  continuousBound:false,globalEmbeddingCertified:false,solidCertified:false}
 const report={method:'progressive-fourfold-section-refinement',accepted:true,continuousBound:false,roundingCertified:false,
  budget:.1,retainedPatchRegularity:r}
 const artifact=(regularity:unknown)=>createNativeGeometryArtifact('sweep','patches',{geometry:[],sweepPatchEvidence:{...report,retainedPatchRegularity:regularity}},{})
 expect(readSweepPatchViewportEvidence(artifact(r))).toMatchObject({surfaceRegularityCertified:true,continuousBound:false,errorUpper:null})
 for(const change of [{method:'frame-regularity'},{cells:10001},{cells:-1},{unresolvedPatches:[0]},
  {unresolvedPatches:null},{continuousBound:true},{globalEmbeddingCertified:true},{solidCertified:true},{surfaceRegularityCertified:false}]){
  expect(readSweepPatchViewportEvidence(artifact({...r,...change}))).toMatchObject({surfaceRegularityCertified:false,continuousBound:false})
 }
 expect(readSweepPatchViewportEvidence(artifact(undefined))).toMatchObject({surfaceRegularityCertified:false})
})

it('keeps whole original frame cover separate from retained Jacobian and error proof',()=>{
 const frame={method:'original-path-adaptive-frenet-frame-regularity',regularityCertified:true,cells:11,certifiedIntervals:2,
  continuousBound:false,surfaceRegularityCertified:false,globalEmbeddingCertified:false}
 const artifact=(sourceFrameRegularity:unknown)=>createNativeGeometryArtifact('sweep','patches',{geometry:[],sweepPatchEvidence:{
  method:'progressive-fourfold-section-refinement',accepted:true,continuousBound:false,budget:1,sourceFrameRegularity,
 }},{})
 expect(readSweepPatchViewportEvidence(artifact(frame))).toMatchObject({sourceFrameRegularityCertified:true,surfaceRegularityCertified:false,continuousBound:false})
 for(const change of [{method:'retained-frame-samples'},{cells:10001},{certifiedIntervals:0},{certifiedIntervals:12},
  {regularityCertified:false},{continuousBound:true},{surfaceRegularityCertified:true},{globalEmbeddingCertified:true}]){
  expect(readSweepPatchViewportEvidence(artifact({...frame,...change}))).toMatchObject({sourceFrameRegularityCertified:false})
 }
 expect(readSweepPatchViewportEvidence(artifact(undefined))).toMatchObject({sourceFrameRegularityCertified:null})
})

it('requires original C2 scope, complete budgets and independent negative surface flags',()=>{
 const proof={method:'original-frame-continuity-and-nondegeneracy-cover',scope:'open-original-frame-only',requestedOrder:2,
  sourceFrameSmoothnessCertified:true,reason:null,cells:3,exactWork:0,retainedSeamsCertified:false,
  profileJoinsCertified:false,capJoinsCertified:false,continuousBound:false,solidCertified:false}
 const artifact=(sourceFrameSmoothness:unknown)=>createNativeGeometryArtifact('sweep','patches',{
  sweepPatchEvidence:{method:'progressive-fourfold-section-refinement',accepted:true,sourceFrameSmoothness}},{})
 expect(readSweepPatchViewportEvidence(artifact(proof))).toMatchObject({sourceFrameC2Certified:true,continuousBound:false})
 for(const change of [{scope:'retained-station-seams-only'},{requestedOrder:1},{cells:0},{cells:10001},{exactWork:1000001},
  {reason:'unproved'},{reason:undefined},{sourceFrameSmoothnessCertified:false},{continuousBound:true},{solidCertified:true},{retainedSeamsCertified:true},
  {profileJoinsCertified:true},{capJoinsCertified:true}]){
  expect(readSweepPatchViewportEvidence(artifact({...proof,...change}))?.sourceFrameC2Certified).toBe(false)
 }
 expect(readSweepPatchViewportEvidence(artifact(undefined))?.sourceFrameC2Certified).toBeNull()
})

it('keeps closed authored source-frame C2 independent of path and retained seam claims',()=>{
 const proof={method:'exact-original-authored-endpoint-jets-and-frame-cover',scope:'closed-original-authored-frame-only',
  requestedOrder:2,closedSourceFrameSmoothnessCertified:true,reason:null,cells:4,exactWork:104,
  pathSeamCertified:false,retainedSeamsCertified:false,profileJoinsCertified:false,
  capJoinsCertified:false,continuousBound:false,solidCertified:false}
 const artifact=(closedSourceFrameSmoothness:unknown)=>createNativeGeometryArtifact('sweep','patches',{
  sweepPatchEvidence:{method:'progressive-fourfold-section-refinement',accepted:true,closedSourceFrameSmoothness}},{})
 expect(readSweepPatchViewportEvidence(artifact(proof))).toMatchObject({closedSourceFrameC2Certified:true,continuousBound:false})
 const pathProof={...proof,method:'exact-original-path-twist-endpoint-jets-and-frame-cover',scope:'closed-original-path-frame-only'}
 expect(readSweepPatchViewportEvidence(artifact(pathProof))).toMatchObject({closedSourceFrameC2Certified:true,continuousBound:false})
 for(const change of [{method:proof.method},{scope:proof.scope},{retainedSeamsCertified:true},{solidCertified:true},{pathSeamCertified:true}]){
  expect(readSweepPatchViewportEvidence(artifact({...pathProof,...change}))?.closedSourceFrameC2Certified).toBe(false)
 }
 const guidedProof={...proof,method:'exact-original-guided-endpoint-jets-and-joint-frame-cover',scope:'closed-original-guided-frame-only'}
 expect(readSweepPatchViewportEvidence(artifact(guidedProof))).toMatchObject({closedSourceFrameC2Certified:true,continuousBound:false})
 for(const change of [{method:proof.method},{scope:proof.scope},{retainedSeamsCertified:true},{solidCertified:true},{pathSeamCertified:true}]){
  expect(readSweepPatchViewportEvidence(artifact({...guidedProof,...change}))?.closedSourceFrameC2Certified).toBe(false)
 }
 for(const change of [{scope:'open-original-frame-only'},{requestedOrder:1},{reason:'unproved'},
  {pathSeamCertified:true},{retainedSeamsCertified:true},{solidCertified:true},{continuousBound:true},
  {exactWork:0},{exactWork:1000001},{cells:0},{cells:10001}]){
  expect(readSweepPatchViewportEvidence(artifact({...proof,...change}))?.closedSourceFrameC2Certified).toBe(false)
 }
 expect(readSweepPatchViewportEvidence(artifact(undefined))?.closedSourceFrameC2Certified).toBeNull()
})

it('shows decomposition G2 only for complete native coverage with shared exact work',()=>{
 const join={leftPatch:0,rightPatch:1,certified:true,exactIdentity:true,regularityCertified:true,
  exactWork:37,reason:'exact-regular-projective-strip-jets'}
 const proof={method:'exact-retained-decomposition-strip-jets',scope:'within-source-profile-decomposition-only',
  requestedOrder:2,expectedJoins:1,checkedJoins:1,coverageComplete:true,decompositionJoinsCertified:true,
  exactWork:37,joins:[join],reason:null,allProfileJoinsCertified:false,closedProfileSeamsCertified:false,
  sourceFrameSmoothnessCertified:false,capJoinsCertified:false,continuousBound:false,solidCertified:false}
 const artifact=(retainedDecompositionSmoothness:unknown)=>createNativeGeometryArtifact('sweep','patches',{
  sweepPatchEvidence:{method:'progressive-fourfold-section-refinement',accepted:true,retainedDecompositionSmoothness}},{})
 expect(readSweepPatchViewportEvidence(artifact(proof))).toMatchObject({decompositionG2Certified:true,
  decompositionJoinCount:1,continuousBound:false,surfaceRegularityCertified:false})
 for(const change of [{coverageComplete:false},{checkedJoins:0},{reason:'work-unproved'},
  {decompositionJoinsCertified:false},{exactWork:38},{exactWork:1000001},{joins:[]},
  {allProfileJoinsCertified:true},{closedProfileSeamsCertified:true},{solidCertified:true},
  {requestedOrder:1},{scope:'all-profile-joins'},
  {expectedJoins:2,checkedJoins:2,joins:[join,join],exactWork:74},
  {joins:[{...join,regularityCertified:false}]},{joins:[{...join,exactIdentity:false}]},
  {joins:[{...join,leftPatch:1}]},{joins:[{...join,exactWork:-1}]}]){
  expect(readSweepPatchViewportEvidence(artifact({...proof,...change}))?.decompositionG2Certified).toBe(false)
 }
 expect(readSweepPatchViewportEvidence(artifact(undefined))?.decompositionG2Certified).toBeNull()
 expect(readSweepPatchViewportEvidence(artifact({...proof,expectedJoins:0,checkedJoins:0,joins:[],
  exactWork:0,decompositionJoinsCertified:false}))?.decompositionG2Certified).toBeNull()
})

it('shows independently proven G1 when native G2 fails without merging shared budgets',()=>{
 const join={leftPatch:0,rightPatch:1,certified:true,exactIdentity:true,regularityCertified:true,
  exactWork:17,reason:'exact-regular-projective-strip-jets'}
 const base={method:'exact-retained-decomposition-strip-jets',scope:'within-source-profile-decomposition-only',
  expectedJoins:1,checkedJoins:1,coverageComplete:true,decompositionJoinsCertified:true,
  exactWork:17,joins:[join],reason:null,allProfileJoinsCertified:false,closedProfileSeamsCertified:false,
  sourceFrameSmoothnessCertified:false,capJoinsCertified:false,continuousBound:false,solidCertified:false}
 const g1={...base,requestedOrder:1}
 const g2={...base,requestedOrder:2,decompositionJoinsCertified:false,exactWork:13,
  joins:[{...join,certified:false,exactIdentity:false,exactWork:13}],reason:'constant-projective-jet-relation-different'}
 const proof={...base,method:'exact-retained-decomposition-smoothness',g1,g2,
  decompositionG1Certified:true,exactWork:30,maxExactWork:100}
 const artifact=(retainedDecompositionG1Fallback:unknown)=>createNativeGeometryArtifact('sweep','patches',{
  sweepPatchEvidence:{method:'progressive-fourfold-section-refinement',accepted:true,
   retainedDecompositionSmoothness:g2,retainedDecompositionG1Fallback}},{})
 expect(readSweepPatchViewportEvidence(artifact(proof))).toMatchObject({decompositionG1Certified:true,
  decompositionG2Certified:false,decompositionJoinCount:1,continuousBound:false})
 for(const change of [{exactWork:31},{maxExactWork:20},{decompositionG1Certified:false},
  {allProfileJoinsCertified:true},{capJoinsCertified:true},{solidCertified:true},
  {g1:{...g1,coverageComplete:false}},{g1:{...g1,requestedOrder:2}},
  {g1:{...g1,exactWork:18}},{g1:{...g1,expectedJoins:2}},
  {g1:{...g1,joins:[{...join,regularityCertified:false}]}}]){
  expect(readSweepPatchViewportEvidence(artifact({...proof,...change}))?.decompositionG1Certified).toBe(false)
 }
})

it('presents an explicit closed source C1 proof without promoting C2 or Solid',()=>{
 const proof={method:'exact-original-path-twist-endpoint-jets-and-frame-cover',scope:'closed-original-path-frame-only',
  requestedOrder:1,closedSourceFrameSmoothnessCertified:true,reason:null,cells:4,exactWork:104,
  pathSeamCertified:false,retainedSeamsCertified:false,profileJoinsCertified:false,capJoinsCertified:false,continuousBound:false,solidCertified:false}
 const artifact=(closedSourceFrameSmoothnessC1:unknown)=>createNativeGeometryArtifact('sweep','patches',{
  sweepPatchEvidence:{method:'progressive-fourfold-section-refinement',accepted:true,closedSourceFrameSmoothnessC1}},{})
 expect(readSweepPatchViewportEvidence(artifact(proof))).toMatchObject({closedSourceFrameC1Certified:true,closedSourceFrameC2Certified:null,continuousBound:false})
 for(const change of [{requestedOrder:2},{scope:'closed-original-guided-frame-only'},{reason:'unproved'},{cells:0},{exactWork:0},{retainedSeamsCertified:true},{continuousBound:true},{solidCertified:true}]){
  expect(readSweepPatchViewportEvidence(artifact({...proof,...change}))?.closedSourceFrameC1Certified).toBe(false)
 }
})
