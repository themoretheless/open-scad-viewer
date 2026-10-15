// Native-report transport controls. These do not certify any geometry.
import assert from 'node:assert/strict'
import {sweepStepCapLayout} from './sweep-step-cap-layout.mjs'
import {checkCompleteBoundaryContract} from './sweep-complete-boundary-contracts.mjs'

const report={capRepresentation:'radial-annular',capFaceGroups:[[8,9,10,11],[12,13,14,15]],
 capFaces:[8,9,10,11,12,13,14,15],planarCapFaces:[],capSelectionExactWork:3,
 retainedAnnularFilling:{selectionExactWork:3,constructionExactWork:5,fillingExactWork:7,exactWork:15,
  sourceFillingCovered:true,sourceUvFacesInjective:true,sourceUvPairsClassified:true,withinBudget:true,filledCapErrorUpper:[.001,.002]},
 budget:.01,closedPath:false,boundaryContinuousBound:true,boundaryErrorWithinBudget:true,
 boundaryErrorScope:'constructor-owned-retained-wall-and-cap-union',boundaryErrorUpper:.003,filledCapErrorUpper:[.001,.002]}
const body={...structuredClone(report),approximation:{report:{closedPath:false}},model:{faces:Array.from({length:16},()=>({holes:[]}))}}
const layout=sweepStepCapLayout(body)
assert.deepEqual(layout.capFaces,[])
assert.deepEqual(layout.physicalCapFaces,report.capFaces)
const item={file:'native-report-control.step',faces:16,...layout,construction:{bodyBoundaryEvidence:report}}
assert.equal(checkCompleteBoundaryContract(item).status,'certified')
const reconstructed={...structuredClone(body),stationReconstruction:{closedPath:false},
 boundaryCertificate:{method:'retained-sweep-boundary-union',scope:'boundary-set-hausdorff',reason:null,
  continuousBound:true,withinBudget:true,errorUpper:.003,wallErrorUpper:.003,filledCapErrorUpper:[.001,.002],budget:.01,closed:false}}
delete reconstructed.approximation
const stationItem={file:'reconstructed-native-report-control.step',faces:16,...sweepStepCapLayout(reconstructed),construction:reconstructed}
assert.equal(checkCompleteBoundaryContract(stationItem).status,'certified')
const lostStationOwnership=structuredClone(stationItem)
delete lostStationOwnership.construction.capFaceGroups
assert.throws(()=>checkCompleteBoundaryContract(lostStationOwnership),'Lost reconstructed cap ownership accepted')
for(const [name,mutate] of [
 ['missing endpoint',r=>{r.capFaceGroups[1]=[]}],
 ['duplicate physical face',r=>{r.capFaceGroups[1][0]=8;r.capFaces=r.capFaceGroups.flat()}],
 ['outside actual model',r=>{r.capFaceGroups[1][0]=16;r.capFaces=r.capFaceGroups.flat()}],
 ['wrong native planar mask',r=>{r.planarCapFaces=[14,15]}],
 ['reset selection ledger',r=>{r.retainedAnnularFilling.selectionExactWork=0}],
 ['reset combined ledger',r=>{r.retainedAnnularFilling.exactWork=12}],
]){
 const changed=structuredClone(body);mutate(changed)
 assert.throws(()=>sweepStepCapLayout(changed),undefined,name)
 const changedItem=structuredClone(item);mutate(changedItem.construction.bodyBoundaryEvidence)
 assert.throws(()=>checkCompleteBoundaryContract(changedItem),undefined,name)
}
for(const [name,mutate] of [
 ['missing source filling',r=>{r.sourceFillingCovered=false}],
 ['missing source UV injectivity',r=>{r.sourceUvFacesInjective=false}],
 ['unclassified source UV pairs',r=>{r.sourceUvPairsClassified=false}],
 ['wrong endpoint bounds',r=>{r.filledCapErrorUpper=[.001,.001]}],
]){
 const changed=structuredClone(item);mutate(changed.construction.bodyBoundaryEvidence.retainedAnnularFilling)
 assert.throws(()=>checkCompleteBoundaryContract(changed),undefined,name)
}
const legacy={...structuredClone(body),capRepresentation:'legacy',capFaceGroups:[[14],[15]],capFaces:[14,15],planarCapFaces:[14,15],retainedAnnularFilling:null}
assert.deepEqual(sweepStepCapLayout(legacy).capFaces,[14,15])
assert.equal(checkCompleteBoundaryContract({...item,...sweepStepCapLayout(legacy),
 construction:{bodyBoundaryEvidence:legacy}}).status,'certified')
const closed={...structuredClone(body),capRepresentation:'closed',capFaceGroups:[[],[]],capFaces:[],planarCapFaces:[],capSelectionExactWork:0,retainedAnnularFilling:null,approximation:{report:{closedPath:true}}}
assert.deepEqual(sweepStepCapLayout(closed).physicalCapFaces,[])
const closedItem={...item,...sweepStepCapLayout(closed),construction:{bodyBoundaryEvidence:{...closed,closedPath:true,filledCapErrorUpper:null}}}
assert.equal(checkCompleteBoundaryContract(closedItem).status,'certified')
const fabricatedClosed=structuredClone(closedItem)
fabricatedClosed.construction.bodyBoundaryEvidence.capFaces=[14,15]
assert.throws(()=>checkCompleteBoundaryContract(fabricatedClosed),'Fabricated closed physical caps accepted')
assert.equal(sweepStepCapLayout({}),null,'Old receipts retain their original policy')
console.log('Radial/legacy/closed and reconstructed native masks accepted; 12 corrupted report controls refused')
