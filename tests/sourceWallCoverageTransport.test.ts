import {describe,it,expect} from 'vitest'
import {coverageExpectation,validCoverage,type SourceWallCoverageOptions,type SourceWallCoverageResult} from '../src/services/sourceWallCoverageTransport'
const key=JSON.stringify
const request:SourceWallCoverageOptions={minimumMm:9.99,limits:{pairs:21,planeControls:100,normalSpans:100,gapCells:100,gapSpans:100,maxSineSquared:1e-6}}
function result():SourceWallCoverageResult {
 const pairs:SourceWallCoverageResult['pairs']=[]
 for(let a=0;a<6;a++)for(let b=a;b<6;b++)pairs.push({faces:[a,b],proven:true,reason:a===b?'source-wall-planar-self-excluded':'source-face-gap-qualified'})
 return {request:structuredClone(request),wholeWallQualified:true,lowerMm:9.99,enumerationComplete:true,totalPairs:21,pairs,planeControls:24,normalSpans:30,cells:3,spans:3,reason:'source-whole-wall-lower-qualified'}
}
describe('continuous original wall coverage transport',()=>{
 it('requires explicit normal product accounting and bounded requests',()=>{
  const option={...structuredClone(request),normalProducts:{cells:500,cellsPerPair:128}}
  expect(coverageExpectation(option)).toEqual(option)
  const r=result();r.request=option
  expect(validCoverage(option,r,6,key)).toBe(false)
  r.normalProductCells=500;expect(validCoverage(option,r,6,key)).toBe(true)
  r.normalProductCells=501;expect(validCoverage(option,r,6,key)).toBe(false)
  for(const cells of [0,-1,100001,NaN,1.5])expect(()=>coverageExpectation({...option,normalProducts:{cells,cellsPerPair:128}})).toThrow()
  const legacy=result();legacy.normalProductCells=1
  expect(validCoverage(request,legacy,6,key)).toBe(false)
 })
 it('binds threshold and complete canonical pairs',()=>{
  expect(validCoverage(request,result(),6,key)).toBe(true)
  for(const mutate of [(r:SourceWallCoverageResult)=>r.request.minimumMm++,r=>r.lowerMm=10,r=>r.pairs[3].faces=[1,1],r=>r.pairs[3].proven=false,r=>r.cells=101,r=>delete r.pairs[3]]) {
   const r=result();mutate(r);expect(validCoverage(request,r,6,key)).toBe(false)
  }
 })
 it('checks native proof reason against pair class and proven status',()=>{
  const r=result();r.pairs[0].reason='source-wall-curved-self-excluded'
  expect(validCoverage(request,r,6,key)).toBe(true)
  for(const [index,reason] of [[0,'source-face-gap-qualified'],[1,'source-wall-curved-self-excluded'],[1,'source-wall-pair-unproven'],[0,'source-invented-exclusion']] as const){
   const changed=result();changed.pairs[index].reason=reason
   expect(validCoverage(request,changed,6,key)).toBe(false)
  }
 })
 it('preserves complete unresolved and truncated refusals',()=>{
  const r=result();r.pairs[1].proven=false;r.pairs[1].reason='source-face-gap-work-limit';r.wholeWallQualified=false;r.lowerMm=null;r.reason='source-wall-coverage-unproven'
  expect(validCoverage(request,r,6,key)).toBe(true)
  r.pairs[1].reason='source-face-gap-chart-close'
  expect(validCoverage(request,r,6,key)).toBe(true)
  const partial=structuredClone(request);partial.limits.pairs=20
  r.request=partial;r.pairs.pop();r.enumerationComplete=false;r.reason='source-wall-coverage-pair-limit'
  expect(validCoverage(partial,r,6,key)).toBe(true)
  r.wholeWallQualified=true;expect(validCoverage(partial,r,6,key)).toBe(false)
 })
 it('clones bounded requests and rejects unrequested certificates',()=>{
  const e=coverageExpectation(request);e.limits.pairs=1;expect(request.limits.pairs).toBe(21)
  expect(validCoverage(undefined,result(),6,key)).toBe(false)
  for(const minimum of [0,-1,Infinity,NaN])expect(()=>coverageExpectation({...request,minimumMm:minimum})).toThrow()
  expect(()=>coverageExpectation({...request,limits:{...request.limits,gapCells:0}})).toThrow()
 })
})

it('binds adaptive face certificates and rejects forged ownership or work',()=>{
 const e=structuredClone(request);e.adaptiveSelf={planeControls:100,cells:10,spans:10,cellsPerFace:2,spansPerFace:2}
 const r=result();r.request=e;r.adaptiveSelf={faces:Array.from({length:6},(_,face)=>({face,qualified:true,reason:'source-self-wall-planar-excluded',cells:0,spans:0,pending:0,uncertain:null})),planeControls:24,cells:0,spans:0,allSelfPairsQualified:true}
 for(const p of r.pairs)if(p.faces[0]===p.faces[1])p.reason='source-wall-self-certified'
 expect(validCoverage(e,r,6,key)).toBe(true)
 for(const mutate of [(r:SourceWallCoverageResult)=>r.adaptiveSelf!.faces[0].face=1,r=>r.adaptiveSelf!.cells=1,r=>r.adaptiveSelf!.faces[0].pending=1,r=>r.adaptiveSelf!.allSelfPairsQualified=false,r=>r.adaptiveSelf=null,r=>r.pairs[0].reason='source-wall-planar-self-excluded']){
  const changed=structuredClone(r);mutate(changed);expect(validCoverage(e,changed,6,key)).toBe(false)
 }
 const unrequested=result();unrequested.pairs[0].reason='source-wall-self-certified';expect(validCoverage(request,unrequested,6,key)).toBe(false)
 for(const k of ['planeControls','cells','spans','cellsPerFace','spansPerFace'] as const){const bad=structuredClone(e);bad.adaptiveSelf![k]=0;expect(()=>coverageExpectation(bad)).toThrow()}
})

it('retains adaptive pending rectangles without promoting whole-wall qualification',()=>{
 const e=structuredClone(request);e.adaptiveSelf={planeControls:100,cells:10,spans:10,cellsPerFace:2,spansPerFace:2}
 const r=result();r.request=e;r.adaptiveSelf={faces:Array.from({length:6},(_,face)=>({face,qualified:true,reason:'source-self-wall-planar-excluded',cells:0,spans:0,pending:0,uncertain:null})),planeControls:24,cells:1,spans:1,allSelfPairsQualified:false}
 r.adaptiveSelf.faces[0]={face:0,qualified:false,reason:'source-self-wall-work-limit',cells:1,spans:1,pending:1,uncertain:[[[0,1],[0,1]],[[0,1],[0,1]]]}
 for(const p of r.pairs)if(p.faces[0]===p.faces[1])p.reason='source-wall-self-certified'
 r.pairs[0]={faces:[0,0],proven:false,reason:'source-wall-curved-self-unproven'};r.wholeWallQualified=false;r.lowerMm=null;r.reason='source-wall-coverage-unproven'
 expect(validCoverage(e,r,6,key)).toBe(true)
 for(const bad of [[],[[[1,0],[0,1]],[[0,1],[0,1]]],[[[0,Infinity],[0,1]],[[0,1],[0,1]]]]){
  const changed=structuredClone(r);changed.adaptiveSelf!.faces[0].uncertain=bad;expect(validCoverage(e,changed,6,key)).toBe(false)
 }
 const face=r.adaptiveSelf.faces[0]
 face.unresolvedPairs=[structuredClone(face.uncertain!)];face.endpointExchangeSymmetric=true
 expect(validCoverage(e,r,6,key)).toBe(true)
 for(const mutate of [
  (r:SourceWallCoverageResult)=>r.adaptiveSelf!.faces[0].unresolvedPairs=[],
  (r:SourceWallCoverageResult)=>delete r.adaptiveSelf!.faces[0].endpointExchangeSymmetric,
  (r:SourceWallCoverageResult)=>r.adaptiveSelf!.faces[0].unresolvedPairs![0][0][0][1]=Infinity,
  (r:SourceWallCoverageResult)=>r.adaptiveSelf!.faces[0].unresolvedPairs![0][0][0][1]=0.5,
 ]){
  const changed=structuredClone(r);mutate(changed);expect(validCoverage(e,changed,6,key)).toBe(false)
 }
 r.pairs[0].proven=true;r.pairs[0].reason='source-wall-self-certified';expect(validCoverage(e,r,6,key)).toBe(false)
})

 describe('factored normal accounting',()=>{
  it('requires bounded counters and rejects unrequested factored proofs',()=>{
   const option={...structuredClone(request),factoredNormals:{exactWork:100000000,queries:100,cells:100,cellsPerPair:32}}
   const key=(v:unknown)=>JSON.stringify(v)
   const r=result();r.request=structuredClone(option)
   expect(validCoverage(option,r,6,key)).toBe(false)
   r.factoredExactWork=10;r.factoredQueries=2;r.factoredProductCells=3
   r.pairs[1].reason='source-wall-factored-normal-pair-excluded'
   expect(validCoverage(option,r,6,key)).toBe(true)
   for(const counter of ['factoredExactWork','factoredQueries','factoredProductCells'] as const){
    const missing=structuredClone(r);delete missing[counter];expect(validCoverage(option,missing,6,key)).toBe(false)
    const excessive=structuredClone(r);excessive[counter]=100000001;expect(validCoverage(option,excessive,6,key)).toBe(false)
   }
   const legacy=result();legacy.pairs[1].reason=r.pairs[1].reason
   expect(validCoverage(request,legacy,6,key)).toBe(false)
   legacy.pairs[1].reason='source-wall-normal-pair-excluded';legacy.factoredQueries=1
   expect(validCoverage(request,legacy,6,key)).toBe(false)
   for(const n of [0,-1,NaN,1.5,100000001])expect(()=>coverageExpectation({...option,factoredNormals:{...option.factoredNormals,exactWork:n}})).toThrow()
  })
 })

it('binds additional pole proofs to requested budgets and additive work',()=>{
 const e=structuredClone(request)
 e.adaptiveSelf={planeControls:100,cells:10,spans:10,cellsPerFace:2,spansPerFace:2,
  pole:{exactWork:100,cells:20,spans:30,cellsPerFace:5,spansPerFace:6}}
 const r=result();r.request=structuredClone(e)
 r.adaptiveSelf={faces:Array.from({length:6},(_,face)=>({face,qualified:true,reason:face===0?'source-self-wall-pole-qualified':'source-self-wall-planar-excluded',cells:face===0?7:0,spans:face===0?8:0,pending:0,uncertain:null})),planeControls:24,cells:7,spans:8,allSelfPairsQualified:true,poleExactWork:100,poleCells:5,poleSpans:6}
 for(const p of r.pairs)if(p.faces[0]===p.faces[1])p.reason='source-wall-self-certified'
 expect(validCoverage(e,r,6,key)).toBe(true)
 for(const [counter,max] of [['poleExactWork',100],['poleCells',20],['poleSpans',30]] as const){
  const missing=structuredClone(r);delete missing.adaptiveSelf![counter]
  expect(validCoverage(e,missing,6,key)).toBe(false)
  const excessive=structuredClone(r);excessive.adaptiveSelf![counter]=max+1
  expect(validCoverage(e,excessive,6,key)).toBe(false)
 }
 const negativeOrdinary=structuredClone(r);negativeOrdinary.adaptiveSelf!.poleCells=8
 expect(validCoverage(e,negativeOrdinary,6,key)).toBe(false)
 const ordinaryOver=structuredClone(r)
 ordinaryOver.adaptiveSelf!.cells=16;ordinaryOver.adaptiveSelf!.faces[1].cells=7;ordinaryOver.adaptiveSelf!.faces[2].cells=2
 expect(validCoverage(e,ordinaryOver,6,key)).toBe(false)
 const sumMismatch=structuredClone(r);sumMismatch.adaptiveSelf!.cells=6
 expect(validCoverage(e,sumMismatch,6,key)).toBe(false)
 const perFace=structuredClone(r);perFace.adaptiveSelf!.faces[0].cells=8;perFace.adaptiveSelf!.cells=8
 expect(validCoverage(e,perFace,6,key)).toBe(false)
 const unrequested=structuredClone(e);delete unrequested.adaptiveSelf!.pole
 const unsolicited=structuredClone(r);unsolicited.request=unrequested
 expect(validCoverage(unrequested,unsolicited,6,key)).toBe(false)
 const noAuthority=structuredClone(r);noAuthority.request=unrequested
 noAuthority.adaptiveSelf!.poleExactWork=0;noAuthority.adaptiveSelf!.poleCells=0;noAuthority.adaptiveSelf!.poleSpans=0
 noAuthority.adaptiveSelf!.cells=0;noAuthority.adaptiveSelf!.spans=0
 noAuthority.adaptiveSelf!.faces[0].cells=0;noAuthority.adaptiveSelf!.faces[0].spans=0
 expect(validCoverage(unrequested,noAuthority,6,key)).toBe(false)
 for(const name of ['exactWork','cells','spans','cellsPerFace','spansPerFace'] as const){
  for(const value of [0,-1,NaN,1.5,name==='exactWork'?100000001:100001]){
   const bad=structuredClone(e);bad.adaptiveSelf!.pole![name]=value
   expect(()=>coverageExpectation(bad)).toThrow()
  }
 }
 const clone=coverageExpectation(e);clone.adaptiveSelf!.pole!.cells=1
 expect(e.adaptiveSelf.pole!.cells).toBe(20)
})

it('requires requested refinement accounting without granting whole-wall proof',()=>{
 const e=structuredClone(request)
 e.adaptiveSelf={planeControls:100,cells:1,spans:1,cellsPerFace:1,spansPerFace:1,
  refinement:{cells:10,spans:10,cellsPerFace:10,spansPerFace:10}}
 const r=result();r.request=structuredClone(e)
 r.adaptiveSelf={faces:Array.from({length:6},(_,face)=>({face,qualified:true,reason:face===0?'source-self-wall-refinement-qualified':'source-self-wall-planar-excluded',cells:face===0?3:0,spans:face===0?4:0,pending:0,uncertain:null})),planeControls:24,cells:3,spans:4,refinementCells:3,refinementSpans:4,allSelfPairsQualified:true}
 for(const p of r.pairs)if(p.faces[0]===p.faces[1])p.reason='source-wall-self-certified'
 r.pairs[1].proven=false;r.pairs[1].reason='source-wall-pair-unproven';r.wholeWallQualified=false;r.lowerMm=null;r.reason='source-wall-coverage-unproven'
 expect(validCoverage(e,r,6,key)).toBe(true)
 for(const counter of ['refinementCells','refinementSpans'] as const){
  for(const value of [undefined,11,-1,1.5]){
   const bad=structuredClone(r);bad.adaptiveSelf![counter]=value
   expect(validCoverage(e,bad,6,key)).toBe(false)
  }
 }
 const stale=structuredClone(e);delete stale.adaptiveSelf!.refinement
 const unrequested=structuredClone(r);unrequested.request=stale
 expect(validCoverage(stale,unrequested,6,key)).toBe(false)
 const forged=structuredClone(r);forged.wholeWallQualified=true;forged.lowerMm=e.minimumMm
 expect(validCoverage(e,forged,6,key)).toBe(false)
 for(const name of ['cells','spans','cellsPerFace','spansPerFace'] as const){
  const bad=structuredClone(e);bad.adaptiveSelf!.refinement![name]=0
  expect(()=>coverageExpectation(bad)).toThrow()
 }
})

function retainedFixture(){
 const option=structuredClone(request)
 option.retainedGap={cells:100,spans:100,domainCells:1000,cellsPerPair:10,spansPerPair:20,domainCellsPerPair:100,domainCellsPerQuery:10,toleranceUv:1e-8}
 const r=result();r.request=structuredClone(option);r.pairs[1].reason='source-wall-retained-gap-qualified'
 r.retainedGap={cells:2,spans:2,domainCells:8,domainQueries:2,endpointQueries:8,pairs:[{faces:[0,1],qualified:true,reason:'source-retained-gap-qualified',cells:2,spans:2,domainCells:8,domainQueries:2,endpointQueries:8,outsideProducts:1,unresolvedProducts:[]}]}
 return {option,r}
}
it('binds retained domain proofs to owned pairs and exact additive budgets',()=>{
 const {option,r}=retainedFixture();expect(validCoverage(option,r,6,key)).toBe(true)
 expect(coverageExpectation(option)).toEqual(option)
 for(const mutate of [
  (r:SourceWallCoverageResult)=>r.retainedGap=null,
  (r:SourceWallCoverageResult)=>r.retainedGap!.cells++,
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].cells=11,
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].faces=[1,0],
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].faces=[0,0],
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].faces=[0,6],
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs.push(structuredClone(r.retainedGap!.pairs[0])),
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].domainQueries=5,
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].outsideProducts=3,
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].reason='source-retained-gap-work-limit',
  (r:SourceWallCoverageResult)=>r.pairs[1].reason='source-face-gap-qualified',
  (r:SourceWallCoverageResult)=>r.request.retainedGap!.toleranceUv=1e-7,
 ]){const changed=structuredClone(r);mutate(changed);expect(validCoverage(option,changed,6,key)).toBe(false)}
 const unrequested=result();unrequested.retainedGap=r.retainedGap;expect(validCoverage(request,unrequested,6,key)).toBe(false)
 for(const name of ['cells','spans','domainCells','cellsPerPair','spansPerPair','domainCellsPerPair','domainCellsPerQuery'] as const){
  for(const value of [0,-1,NaN,1.5,name==='domainCells'?1000001:100001]){
   const bad=structuredClone(option);bad.retainedGap![name]=value;expect(()=>coverageExpectation(bad)).toThrow()
  }
 }
 for(const value of [0,-1,NaN,Infinity]){const bad=structuredClone(option);bad.retainedGap!.toleranceUv=value;expect(()=>coverageExpectation(bad)).toThrow()}
})
it('retains unresolved original products and refuses forged retained success',()=>{
 const {option,r}=retainedFixture(),p=r.retainedGap!.pairs[0]
 p.qualified=false;p.reason='source-retained-gap-work-limit';p.unresolvedProducts=[[[[0,1],[0,1]],[[0,1],[0,1]]]]
 r.pairs[1].proven=false;r.pairs[1].reason='source-face-gap-chart-close';r.wholeWallQualified=false;r.lowerMm=null;r.reason='source-wall-coverage-unproven'
 expect(validCoverage(option,r,6,key)).toBe(true)
 for(const mutate of [
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].unresolvedProducts=[],
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].unresolvedProducts[0][0][0]=[1,0],
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].unresolvedProducts[0][0][0]=[0,Infinity],
  (r:SourceWallCoverageResult)=>r.retainedGap!.pairs[0].qualified=true,
  (r:SourceWallCoverageResult)=>{r.pairs[1].proven=true;r.pairs[1].reason='source-wall-retained-gap-qualified'},
  (r:SourceWallCoverageResult)=>{r.wholeWallQualified=true;r.lowerMm=option.minimumMm},
 ]){const changed=structuredClone(r);mutate(changed);expect(validCoverage(option,changed,6,key)).toBe(false)}
})

it('binds mixed product proofs and preserves an earlier gap refusal',()=>{
 const {option,r}=retainedFixture()
 option.mixedProducts={...option.retainedGap!,normalSpans:100,normalSpansPerPair:20}
 r.request=structuredClone(option)
 const old=r.retainedGap!.pairs[0]
 old.qualified=false;old.reason='source-retained-gap-work-limit'
 old.unresolvedProducts=[[[[0,1],[0,1]],[[0,1],[0,1]]]]
 r.pairs[1].reason='source-wall-mixed-product-qualified'
 r.mixedProducts={cells:3,spans:4,domainCells:8,domainQueries:2,endpointQueries:8,normalSpans:6,normalProducts:1,
  pairs:[{...structuredClone(old),cells:3,spans:4,qualified:true,reason:'source-wall-pair-qualified',unresolvedProducts:[],normalSpans:6,normalProducts:1}]}
 expect(coverageExpectation(option)).toEqual(option)
 expect(validCoverage(option,r,6,key)).toBe(true)
 for(const mutate of [
  (r:SourceWallCoverageResult)=>r.mixedProducts=null,
  (r:SourceWallCoverageResult)=>r.mixedProducts!.normalSpans++,
  (r:SourceWallCoverageResult)=>r.mixedProducts!.normalProducts++,
  (r:SourceWallCoverageResult)=>r.mixedProducts!.pairs[0].normalSpans=21,
  (r:SourceWallCoverageResult)=>r.mixedProducts!.pairs[0].normalProducts=4,
  (r:SourceWallCoverageResult)=>r.mixedProducts!.pairs[0].faces=[1,0],
  (r:SourceWallCoverageResult)=>r.mixedProducts!.pairs.push(structuredClone(r.mixedProducts!.pairs[0])),
  (r:SourceWallCoverageResult)=>r.mixedProducts!.pairs[0].unresolvedProducts=structuredClone(old.unresolvedProducts),
  (r:SourceWallCoverageResult)=>r.mixedProducts!.pairs[0].reason='source-wall-pair-numeric-limit',
  (r:SourceWallCoverageResult)=>r.request.mixedProducts!.normalSpans++,
  (r:SourceWallCoverageResult)=>r.pairs[1].reason='source-face-gap-qualified',
 ]){const forged=structuredClone(r);mutate(forged);expect(validCoverage(option,forged,6,key)).toBe(false)}
 const missingRequest=structuredClone(option);delete missingRequest.mixedProducts
 expect(validCoverage(missingRequest,r,6,key)).toBe(false)
 for(const name of ['cells','spans','domainCells','cellsPerPair','spansPerPair','domainCellsPerPair','domainCellsPerQuery','normalSpans','normalSpansPerPair'] as const){
  const bad=structuredClone(option);bad.mixedProducts![name]=0;expect(()=>coverageExpectation(bad)).toThrow()
 }
 const unresolved=structuredClone(r),p=unresolved.mixedProducts!.pairs[0]
 p.qualified=false;p.reason='source-wall-pair-numeric-limit';p.unresolvedProducts=structuredClone(old.unresolvedProducts)
 unresolved.pairs[1].proven=false;unresolved.pairs[1].reason='source-face-gap-chart-close'
 unresolved.wholeWallQualified=false;unresolved.lowerMm=null;unresolved.reason='source-wall-coverage-unproven'
 expect(validCoverage(option,unresolved,6,key)).toBe(true)
})

it('validates the complete native annular mixed report without promoting its remaining pairs',async()=>{
 const {readFileSync}=await import('node:fs'),{gunzipSync}=await import('node:zlib')
 const {sourceBodyExpectation,validSourceBody}=await import('../src/services/sourceBody')
 const {op:_op,...options}=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/mixed-wall-bridge-request.json.gz',import.meta.url))).toString())
 const report=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/mixed-wall-bridge-native.json.gz',import.meta.url))).toString())
 expect(validSourceBody(sourceBodyExpectation(options),report.result)).toBe(true)
 const coverage=report.result.wallCoverage as SourceWallCoverageResult
 expect(coverage.pairs.filter(p=>!p.proven)).toHaveLength(32)
 expect(coverage.mixedProducts!.pairs.filter(p=>p.qualified)).toHaveLength(13)
 expect(coverage.wholeWallQualified).toBe(false)
 const forged=structuredClone(report.result);forged.wallCoverage.mixedProducts.pairs=[]
 expect(validSourceBody(sourceBodyExpectation(options),forged)).toBe(false)
})
