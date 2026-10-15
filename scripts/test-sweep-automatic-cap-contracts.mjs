// Synthetic receipt controls only. This does not evaluate or certify geometry.
import assert from 'node:assert/strict'
import {smoothCapGapCompanions,checkAutomaticSmoothCapContract} from './sweep-automatic-cap-contracts.mjs'
const capFaces=[8,9,10,11,12,13,14,15]
const faceLoops=Array.from({length:16},(_,face)=>({outer:[0,1,2,3].map(edge=>4*face+edge),holes:[]}))
const item={file:[...smoothCapGapCompanions.keys()][0],faces:16,capFaces:[],physicalCapFaces:capFaces,
 requireNativeSolid:true,faceLoops,nativeVolume:{solidGeometryCertified:true},
 nativeExactUses:faceLoops.flatMap((loop,face)=>loop.outer.map((edge,coedge)=>({face,coedge,edge,status:'equal',exactIdentityCertified:true}))),
 construction:{bodyBoundaryEvidence:{capRepresentation:'radial-annular',capFaceGroups:[capFaces.slice(0,4),capFaces.slice(4)],
 capFaces,planarCapFaces:[],budget:.01,closedPath:false,capCorrectionErrorUpper:null,profileCorrectionErrorUpper:null,
 boundaryContinuousBound:true,boundaryErrorWithinBudget:true,boundaryErrorUpper:.003,
 boundaryErrorScope:'constructor-owned-retained-wall-and-cap-union',filledCapErrorUpper:[.001,.002],capSelectionExactWork:3,
 retainedAnnularFilling:{sourceFillingCovered:true,sourceUvFacesInjective:true,sourceUvPairsClassified:true,withinBudget:true,
  filledCapErrorUpper:[.001,.002],selectionExactWork:3,constructionExactWork:5,fillingExactWork:7,exactWork:15}}}}
for(const file of smoothCapGapCompanions.keys())assert.equal(checkAutomaticSmoothCapContract({...item,file}).file,file)
for(const [name,mutate] of [
 ['old cap representation',i=>{i.construction.bodyBoundaryEvidence.capRepresentation='legacy'}],
 ['changed authored tolerance',i=>{i.construction.bodyBoundaryEvidence.budget=.1}],
 ['added plane correction',i=>{i.construction.bodyBoundaryEvidence.capCorrectionErrorUpper=0}],
 ['added profile correction',i=>{i.construction.bodyBoundaryEvidence.profileCorrectionErrorUpper=0}],
 ['optional native Solid',i=>{i.requireNativeSolid=false}],
 ['missing physical patch',i=>{i.physicalCapFaces=i.physicalCapFaces.slice(1)}],
 ['missing actual coedge',i=>{i.nativeExactUses.pop()}],
 ['duplicate actual coedge',i=>{i.nativeExactUses[1]=i.nativeExactUses[0]}],
 ['rebound actual coedge',i=>{i.nativeExactUses[0].edge=1}],
 ['unproved actual coedge',i=>{i.nativeExactUses[0].status='unresolved'}],
 ['missing actual native Solid',i=>{i.nativeVolume.solidGeometryCertified=false}],
 ['lost source cap coverage',i=>{i.construction.bodyBoundaryEvidence.retainedAnnularFilling.sourceFillingCovered=false}],
]){
 const changed=structuredClone(item);mutate(changed)
 assert.throws(()=>checkAutomaticSmoothCapContract(changed),undefined,name)
}
console.log('Three declared automatic source contracts accepted; 12 corrupted receipt controls refused')
