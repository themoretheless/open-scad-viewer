import {expect,it} from 'vitest'
import {createNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
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
