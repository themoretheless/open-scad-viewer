import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname,join} from 'node:path'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {performance} from 'node:perf_hooks'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

const [root,output]=process.argv.slice(2)
assert.ok(root&&output,'Staged root and output are required')
const rawBuilt=process.argv.includes('--raw-built')
const unpackedOptimized=process.argv.find(a=>a.startsWith('--unpacked-optimized='))?.slice('--unpacked-optimized='.length)
assert.ok(!(rawBuilt&&unpackedOptimized),'Choose one explicit unpublished artifact mode')
const artifactMode=unpackedOptimized?'unpacked-optimized':rawBuilt?'raw-built-before-optimization':'packed-staged'
const jointMaterialStep=process.argv.includes('--joint-material-step')
const checkUnbudgeted=process.argv.includes('--unbudgeted-source')
const directory=dirname(output)
const stepCases:any[]=[]
const artifactPath=unpackedOptimized??(rawBuilt?`${root}/crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm`:`${root}/public/wasm/geometry-kernel.wasm`)
const bytes=readFileSync(artifactPath)
const hash=(b:Uint8Array)=>createHash('sha256').update(b).digest('hex')
const sha256=hash(bytes)
if(artifactMode==='packed-staged'){
 assert.equal(hash(readFileSync(`${root}/src/generated/geometry-kernels/kernel_bg.wasm`)),sha256)
 assert.ok(readFileSync(`${root}/src/generated/geometry-kernels/identity.ts`,'utf8').includes(sha256))
}
const wasm=(await WebAssembly.instantiate(bytes)).instance.exports as unknown as {
 memory:WebAssembly.Memory;abi_alloc:(n:number)=>number;abi_free:(p:number,n:number)=>void;
 abi_request:(op:number,p:number,n:number)=>bigint}
const calls:{op:string;milliseconds:number}[]=[]
const invoke=(op:string,args:object):any=>{
 const encoded=encodeBinary({...args,op}),ptr=writeLinear(wasm.memory,wasm.abi_alloc,encoded)
 assert.ok(ptr)
 try{
  const started=performance.now()
  const reply=decodePacked<{ok:boolean;value:unknown;error:unknown}>(wasm.memory,wasm.abi_free,wasm.abi_request(0,ptr,encoded.length))
  const elapsed=performance.now()-started
  calls.push({op,milliseconds:elapsed})
  assert.ok(elapsed<=30000,`${op}: original 30-second native-silence limit exceeded`)
  assert.ok(reply.ok,JSON.stringify(reply.error))
  return reply.value
 }finally{wasm.abi_free(ptr,encoded.length)}
}
const volume=(model:unknown)=>{
 const started=performance.now()
 const start=invoke('brep_sweep_volume_begin',{model,capFaces:[],...DEFAULT_SWEEP_VOLUME_BUDGETS})
 try{
  for(let i=0;i<10000;i++){
   const step=invoke('brep_sweep_volume_step',{session:start.session,maxUnits:32})
   assert.ok(performance.now()-started<=120000,'Original 120-second Solid limit exceeded')
   if(step.done){assert.ok(step.report);return step.report}
  }
  throw Error('Native volume continuation did not finish')
 }finally{invoke('brep_sweep_volume_drop',{session:start.session})}
}
const materialAndStep=(body:any,key:string)=>{
 const exported=invoke('brep_nurbs_export_step_v5',{model:body.model})
 assert.equal(exported.certificate.complete,true)
 mkdirSync(join(directory,'step'),{recursive:true})
 const path=join(directory,'step',`${key}.step`)
 writeFileSync(path,exported.text)
 const proof=volume(body.model),faces=body.model.faces.length
 // Save each fresh result before assertions, including refusals. Failed
 // native material proofs must not disappear behind a final summary gate.
 writeFileSync(join(directory,`material-${key}.json`),JSON.stringify({key,faces,
  artifact:{sha256,mode:artifactMode},
  source:body.boundaryCertificate,volume:proof,stepPath:path,stepCertificate:exported.certificate},null,2)+'\n')
 assert.equal(proof.solidGeometryCertified,true,`${key}: native material proof refused`)
 assert.equal(proof.boundary.allPairsClassified,true,`${key}: unresolved pairs`)
 assert.equal(proof.boundary.totalPairs,faces*(faces-1)/2)
 assert.equal(proof.boundary.individualPairs+proof.boundary.groupedPairs,faces*(faces-1)/2)
 assert.equal(proof.boundary.nextPair,null)
 const model=body.model
 assert.ok(model.faces.every((face:any)=>face.holes.length===0),'STEP matrix requires full rectangular annular patches')
 const orientations=model.faces.map((_:any,index:number)=>{
  const uses=model.shells.flatMap((shell:any)=>shell.faces.filter((use:any)=>use.face===index))
  assert.equal(uses.length,1)
  return uses[0].reversed
 })
 stepCases.push({file:`step/${key}.step`,faces:faces,edges:model.edges.length,solids:1,shells:model.shells.length,
  capHoleFaces:0,capFaces:[],expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',
  outwardCapContours:[],wallSurfaces:model.faces.map((face:any)=>face.surface),wallFaceReversed:orientations,
  wallSamples:model.faces.map((face:any)=>[0,.17,.5,.83,1].flatMap(u=>[0,.13,.5,.87,1].map(v=>({u,v,...invoke('surface_evaluate',{surface:face.surface,u,v})})))),
  wallCoedges:model.faces.map((face:any)=>model.loops[face.outer].coedges.map((coedge:any)=>({edge:coedge.edge,reversed:coedge.reversed,pcurve:coedge.pcurve}))),
  faceLoops:model.faces.map((face:any)=>({outer:model.loops[face.outer].coedges.map((coedge:any)=>coedge.edge),holes:[]})),
  edgeCurves:model.edges.map((edge:any)=>({curve:edge.curve,samples:[0,.25,.5,.75,1].map(u=>({u,point:invoke('curve_evaluate',{curve:edge.curve,u}).point}))})),
  surfaceToleranceMm:1e-8,relativeVolumeTolerance:1e-7,sha256:hash(Buffer.from(exported.text)),requireNativeSolid:true,
  nativeVolume:proof,construction:{boundaryCertificate:body.boundaryCertificate,retainedAnnularFilling:body.retainedAnnularFilling}})
 return {proof,stepPath:path}
}
const outer=invoke('curve_circle',{center:[0,0,0],normal:[0,0,1],radius:.5})
const hole=invoke('curve_circle',{center:[0,0,0],normal:[0,0,-1],radius:.2})
// Same geometric data as miter-hollow-body.r, with an explicitly supplied
// 0.01 budget for this ownership test. The unchanged .r has no author budget.
const source={direct:true,loops:[[outer],[hole]],points:[[0,0,0],[0,0,10],[10,0,10],[10,10,15]],
 normal:[1,0,0],closed:false,miterLimit:2,maxDeviation:.01}
const cases=[]
let budgetedRaw:any
for(const corrected of [false,true]){
 const args={...source,smoothnessMaxWork:2000000,...(corrected?{
  circleCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:1000000}}:{})}
 const before=JSON.stringify(args)
 const body=invoke('brep_miter_annular_owned_construct',args)
 if(!corrected)budgetedRaw=body
 assert.equal(JSON.stringify(args),before)
 assert.equal(body.capRepresentation,'radial-annular')
 assert.deepEqual(body.capFaceGroups,[[24,25,26,27],[28,29,30,31]])
 assert.equal(body.capFaces.length,8);assert.equal(body.model.faces.length,32)
 assert.deepEqual(body.planarCapFaces,[])
 assert.equal(body.boundaryCertificate.continuousBound,true)
 assert.equal(body.boundaryCertificate.withinBudget,true)
 assert.ok(body.boundaryCertificate.errorUpper<=source.maxDeviation)
 assert.equal(body.retainedAnnularFilling.sourceFillingCovered,true)
 assert.equal(body.solidGeometryCertified,false)
 assert.equal(body.globalEmbeddingCertified,false)
 assert.ok(body.retainedSmoothness.totalExactWork<=2000000)
 assert.equal(body.retainedSmoothness.fullBoundarySmoothnessCertified,false)
 assert.equal(body.retainedSmoothness.capWallContinuity,'C0')
 if(corrected){
  assert.equal(body.retainedSmoothness.walls.profile.exactG1G2Certified,true)
  assert.equal(body.retainedSmoothness.capInternalSeams.length,2)
  assert.ok(body.retainedSmoothness.capInternalSeams.every((c:any)=>c.g1Certified&&c.g2Certified&&c.edgeIds.length===4))
 }
 const material=jointMaterialStep?materialAndStep(body,corrected?'direct-corrected':'direct-raw'):null
 const proof=material?.proof??volume(body.model)
 assert.equal(proof.solidGeometryCertified,true)
 assert.equal(proof.boundary.allFacesInjective,true)
 assert.equal(proof.boundary.allPairsClassified,true)
 assert.equal(proof.boundary.totalPairs,496)
 assert.equal(proof.boundary.individualPairs+proof.boundary.groupedPairs,496)
 assert.equal(proof.boundary.nextPair,null)
 cases.push({corrected,faces:32,groups:body.capFaceGroups,boundary:body.boundaryCertificate,
  filling:body.retainedAnnularFilling,smoothness:body.retainedSmoothness,volume:proof,stepPath:material?.stepPath??null})
 console.log(corrected?'Corrected annular source, retained G2 and fresh material proof passed':'Raw annular source and fresh material proof passed')
}
const zero=invoke('brep_miter_annular_owned_construct',{...source,smoothnessMaxWork:0})
assert.equal(zero.boundaryCertificate.continuousBound,true)
assert.equal(zero.retainedSmoothness.walls.profile.exactG1G2Certified,false)
assert.equal(zero.retainedSmoothness.totalExactWork,0)
// Exercise the staged production build, including its existing bounded
// progressive correction path, for all eight joint law selectors.
const vector=(a:number[],b:number[])=>invoke('curve_bezier',{points:[a,b]})
const scale=vector([1,0,0],[1.125,0,0]),twist=vector([0,0,0],[.1,0,0])
const axes=vector([2,1,1],[2,1.25,1]),center=vector([.125,-.25,0],[.25,.125,0])
const axis=vector([0,0,2],[0,.5,2]),normal=vector([1,0,0],[1,0,0])
const guide=vector([1,0,0],[1,0,10])
const lawCombinations=[]
for(let mask=0;mask<8;mask++){
 const args={...source,direct:false,points:[[0,0,0],[0,0,10]],scale,twist,initialSteps:1,maxSteps:16,maxDeviation:.25,miterLimit:4,
  ...(mask&1?{axisScale:axes,centerLaw:center}:{}),...(mask&2?{frameAxis:axis,frameNormal:normal}:{}),
  ...(mask&4?{orientationGuide:guide}:{})}
 const body=invoke('brep_miter_annular_owned_construct',args)
 assert.equal(body.boundaryCertificate.continuousBound,true,`law mask ${mask}`)
 assert.equal(body.boundaryCertificate.withinBudget,true,`law mask ${mask}`)
 assert.equal(body.retainedAnnularFilling.sourceFillingCovered,true,`law mask ${mask}`)
 assert.deepEqual(body.capFaceGroups.map((group:number[])=>group.length),[4,4])
 assert.equal(new Set(body.capFaces).size,8)
 assert.ok(body.retainedAnnularFilling.exactWork<=1000000)
 const material=jointMaterialStep?materialAndStep(body,`law-${mask}`):null
 lawCombinations.push({mask,faces:body.model.faces.length,boundary:body.boundaryCertificate,groups:body.capFaceGroups,material})
 console.log('Staged joint miter law mask',mask,'passed')
}
for(const key of ['model','capFaces','capFaceGroups','retainedAnnularFilling','retainedSmoothness']){
 assert.throws(()=>invoke('brep_miter_annular_owned_construct',{...source,[key]:true}),/Owned miter construction accepts original source only/)
}
let unbudgetedSource:any=null
if(checkUnbudgeted){
 const {maxDeviation:_omitted,...original}=source
 const body=invoke('brep_miter_annular_owned_construct',original)
 assert.equal(body.budget,null)
 assert.equal(body.boundaryCertificate.budget,null)
 assert.equal(body.boundaryCertificate.withinBudget,null)
 assert.equal(body.retainedAnnularFilling.withinBudget,null)
 assert.equal(body.boundaryCertificate.continuousBound,true)
 assert.equal(body.boundaryCertificate.reason,'no-authored-tolerance')
 assert.deepEqual(body.levels,[])
 assert.equal(body.sectionCorrection,null)
 assert.deepEqual(body.model,budgetedRaw.model)
 assert.deepEqual(body.sections,budgetedRaw.sections)
 assert.deepEqual(body.sourceSections,budgetedRaw.sourceSections)
 assert.equal(body.boundaryCertificate.errorUpper,budgetedRaw.boundaryCertificate.errorUpper)
 const material=jointMaterialStep?materialAndStep(body,'direct-unbudgeted'):null
 const proof=material?.proof??volume(body.model)
 assert.equal(proof.solidGeometryCertified,true)
 assert.equal(proof.boundary.totalPairs,496)
 const straight={...original,points:[[0,0,0],[0,0,10]]}
 const preserved=invoke('brep_miter_annular_owned_construct',straight)
 const legacy=invoke('brep_miter_owned_construct',{...straight,maxDeviation:.01})
 assert.equal(preserved.capRepresentation,'legacy')
 assert.deepEqual(preserved.model,legacy.model)
 assert.equal(preserved.boundaryCertificate.withinBudget,null)
 const denied=invoke('brep_miter_annular_owned_construct',{...original,limits:{
  maxProducts:100000,maxFaces:1024,exactWork:1000000,domainExactWork:1,
  domainTolerance:.001,domainPairs:1000,domainCells:10000,projectionCells:10000,
  capRegions:DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets}})
 assert.equal(denied.boundaryCertificate.continuousBound,false)
 assert.equal(denied.boundaryCertificate.errorUpper,null)
 assert.equal(denied.boundaryCertificate.withinBudget,null)
 assert.equal(denied.boundaryCertificate.reason,'filled-cap-bound-unproved')
 assert.throws(()=>invoke('brep_miter_owned_construct',original))
 assert.throws(()=>invoke('brep_miter_annular_owned_construct',{...original,circleCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:1000000}}),/requires an authored maxDeviation/)
 unbudgetedSource={boundary:body.boundaryCertificate,filling:body.retainedAnnularFilling,
  capFaceGroups:body.capFaceGroups,volume:proof,legacyGeometryPreserved:true,unprovedCapsRefused:true,
  noAuthoredToleranceInvented:true,stepPath:material?.stepPath??null}
 console.log('Unbudgeted original source, full material, legacy preservation and missing-cap refusal passed')
}
if(jointMaterialStep){
 assert.equal(stepCases.length,checkUnbudgeted?11:10)
 writeFileSync(join(directory,'manifest.json'),JSON.stringify({schema:'sweep-external-step/2',units:'mm',
  artifactProvenance:{geometryWasmSha256:sha256,mode:artifactMode},cases:stepCases})+'\n')
}
writeFileSync(output,JSON.stringify({schema:'staged-annular-miter/1',artifact:{sha256,byteLength:bytes.length,mode:artifactMode},
 scope:'Private staged ABI checks; optional unbudgeted native source and STEP coverage are explicit fields; not Rush, viewport, UI, full mode matrix or main delivery',
 nativeSilenceLimitMs:30000,solidLimitMs:120000,jointMaterialStep,cases,lawCombinations,unbudgetedSource,calls,zeroSmoothnessBudgetRefused:true,injectedProofsRefused:true},null,2)+'\n')
