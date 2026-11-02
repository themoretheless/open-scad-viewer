import {expect,it} from 'vitest'
import {validFaceContacts} from '../src/services/solidFaceContacts'
function fixture(){
 const limits={maxPairs:1,maxCells:1000,maxDomainCells:1000,cellsPerPair:1000,domainCellsPerPair:1000,maxBoxes:0}
 const e:any={toleranceUv:1e-8,limits,domains:[0,1].map(()=>({uv:[[0,1],[0,1]],edges:[7],vertices:[3,4],periodic:false,carrierKey:'original-carrier'}))}
 const certificate={kind:'same-support-partition',certified:true,faces:[0,1],edge:7,vertex:null,cells:20,projection:[[1,0,0],[0,1,0]],rowWeights:[1,1]}
 const r:any={method:'interval-trimmed-face-contacts',scope:'distinct-face-pairs',solidGeometryStatus:'not-certified',totalPairs:1,visitedPairs:1,unvisitedPairs:0,nextPair:null,allPairsVisited:true,allPairsDisjoint:false,allPairsClassified:true,sharedBoundaryPairCount:1,contactPairCount:0,disjointPairCount:0,unresolvedPairCount:0,unresolvedBoxCount:0,exportedBoxCount:0,boxesTruncated:false,cells:20,domainCells:0,toleranceUv:e.toleranceUv,limits,pairs:[{faces:[0,1],status:'shared-boundary',sharedBoundary:certificate,witness:null,cells:0,hullCells:0,sourceChartCells:20,domainCells:0,unresolvedBoxes:[],unresolvedBoxCount:0}]}
 return {e,r,certificate}
}
it('requires exact boundary admission, common carrier, owned contact and bounded injectivity evidence',()=>{
 const {e,r,certificate}=fixture()
 expect(validFaceContacts(e,r,true)).toBe(true)
 expect(validFaceContacts(e,r)).toBe(false)
 const other=structuredClone(e);other.domains[1].carrierKey='other-carrier'
 expect(validFaceContacts(other,r,true)).toBe(false)
 for(const patch of [{certified:false},{edge:8},{edge:null,vertex:null},{edge:7,vertex:3},{faces:[1,0]},{cells:21},{projection:[[1,0,0],[2,0,0]]},{rowWeights:[0,1]}]){
  const bad=structuredClone(r);bad.pairs[0].sharedBoundary={...certificate,...patch}
  expect(validFaceContacts(e,bad,true)).toBe(false)
 }
 const vertex=structuredClone(r);vertex.pairs[0].sharedBoundary={...certificate,edge:null,vertex:3}
 expect(validFaceContacts(e,vertex,true)).toBe(true)
 vertex.pairs[0].sharedBoundary.vertex=999
 expect(validFaceContacts(e,vertex,true)).toBe(false)
})

it('validates projected trim contacts across different carriers with exact-scope and ownership gates',()=>{
 const {e,r}=fixture();e.domains[1].carrierKey='different-carrier'
 const c={kind:'projected-trim-partition',certified:true,faces:[0,1],edge:null,vertex:3,cells:20,axes:[0,1],determinantSigns:[1,-1]}
 r.pairs[0].sharedBoundary=c
 expect(validFaceContacts(e,r,true)).toBe(true)
 expect(validFaceContacts(e,r)).toBe(false)
 for(const patch of [{axes:[0,0]},{axes:[0,3]},{determinantSigns:[1,0]},{cells:21},{vertex:999},{edge:7,vertex:3},{certified:false}]){
  const bad=structuredClone(r);bad.pairs[0].sharedBoundary={...c,...patch}
  expect(validFaceContacts(e,bad,true)).toBe(false)
 }
})

it('requires exact scope and complete owned topology for a projected boundary network',()=>{
 const {e,r}=fixture()
 for(const domain of e.domains){domain.edges=[7,8,9];domain.vertices=[3,4,5,6]}
 const c={kind:'projected-trim-network',certified:true,faces:[0,1],edges:[7,8,9],vertices:[3,4,5,6],cells:20,axes:[0,1],determinantSigns:[-1,1]}
 r.pairs[0].sharedBoundary=c
 expect(validFaceContacts(e,r,true)).toBe(true)
 expect(validFaceContacts(e,r)).toBe(false)
 for(const patch of [{edges:[]},{edges:[7]},{edges:[7,7]},{edges:[7,999]},{vertices:[]},{vertices:[3,3]},{vertices:[3,999]},{axes:[1,1]},{determinantSigns:[0,1]},{faces:[1,0]},{cells:21},{certified:false}]){
  const bad=structuredClone(r);bad.pairs[0].sharedBoundary={...c,...patch}
  expect(validFaceContacts(e,bad,true)).toBe(false)
 }
 const unowned=structuredClone(e);unowned.domains[1].edges=[7,8]
 expect(validFaceContacts(unowned,r,true)).toBe(false)
})
