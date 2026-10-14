import {expect,it} from 'vitest'
import {createNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
// Presentation fixture only. These flags never authorize a Solid.
const report={globalEmbeddingCertified:false,boundaryErrorScope:'constructor-owned-retained-wall-and-cap-union',
 closedPath:false,budget:.01,boundaryErrorUpper:.002,boundaryContinuousBound:true,boundaryErrorWithinBudget:true,
 retainedWalls:{certified:true,faceCoverageCertified:true,coefficientFamilyCertified:true,inspectedFaces:16,exactWork:100},
 retainedCaps:null,capProjection:null,capRepresentation:'radial-annular',capFaceGroups:[[16,17,18,19],[20,21,22,23]],
 capFaces:[16,17,18,19,20,21,22,23],planarCapFaces:[],filledCapErrorUpper:[.001,.002],
 retainedAnnularFilling:{method:'authored-smooth-endpoint-annular-filling',sourceFillingCovered:true,filledCapErrorUpper:[.001,.002],
  withinBudget:true,exactWork:300,constructionExactWork:100,fillingExactWork:200,products:100,endpointCells:100,
  sourceUvFacesInjective:true,sourceUvPairsClassified:true,solidGeometryCertified:false}}
const read=(changes:object={})=>readSweepBodyBoundaryViewportEvidence(createNativeGeometryArtifact('sweep','brep',{
 geometry:{},sweepBodyBoundaryEvidence:{...report,...changes}},{author:'transport-only'}))
it('presents native complete annular coverage and refuses malformed cap ownership or ledger',()=>{
 expect(read()).toMatchObject({continuousBound:true,errorUpper:.002,budget:.01,withinBudget:true})
 for(const change of [{capRepresentation:'legacy'},{capFaceGroups:[[16,17,18,19],[]]},{capFaces:[16,17,18,19,20,21,22,22]},
  {capFaces:[16,17,18,19,20,21,22,1024]},{planarCapFaces:[23]},{retainedCaps:{exact:true}},
  {filledCapErrorUpper:[.001,.003]},{boundaryErrorUpper:.001},{budget:null},{boundaryErrorWithinBudget:false}]){
  expect(read(change)?.continuousBound,JSON.stringify(change)).toBe(false)
 }
 for(const change of [{method:'frame-only'},{sourceFillingCovered:false},{sourceUvFacesInjective:false},{sourceUvPairsClassified:false},
  {solidGeometryCertified:true},{exactWork:301},{constructionExactWork:-1},{fillingExactWork:1000001},{products:100001},
  {endpointCells:100001},{withinBudget:false},{filledCapErrorUpper:[.001,.003]}]){
  expect(read({retainedAnnularFilling:{...report.retainedAnnularFilling,...change}})?.continuousBound,JSON.stringify(change)).toBe(false)
 }
})
