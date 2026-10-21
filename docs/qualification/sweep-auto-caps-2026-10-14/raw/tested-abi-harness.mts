import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname,join} from 'node:path'
import {createHash} from 'node:crypto'
import assert from 'node:assert/strict'
import {performance} from 'node:perf_hooks'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

const [root,output,nativeRoot]=process.argv.slice(2)
assert.ok(nativeRoot,'Rust original-source capture root required')
assert.ok(root&&output,'Staged root and output are required')
const automatic=process.argv.includes('--automatic-caps')
const rawBuilt=process.argv.includes('--raw-built')
const unpackedOptimized=process.argv.find(a=>a.startsWith('--unpacked-optimized='))?.slice('--unpacked-optimized='.length)
assert.ok(!(rawBuilt&&unpackedOptimized),'Choose one explicit unpublished artifact mode')
const artifactMode=unpackedOptimized?'unpacked-optimized':rawBuilt?'raw-built-before-optimization':'packed-staged'
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
const volume=(model:unknown,capFaces:number[]=[])=>{
 const started=performance.now()
 const start=invoke('brep_sweep_volume_begin',{model,capFaces,...DEFAULT_SWEEP_VOLUME_BUDGETS})
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
// The Rust example exports exact original input coefficients. This runner
// transports them unchanged; every geometric decision is native WASM.
const capture=JSON.parse(readFileSync(join(nativeRoot,'manifest.json'),'utf8'))
const reports:any[]=[]
for(const entry of capture.cases){
 const sourceBytes=readFileSync(join(nativeRoot,entry.sourceRequestFile))
 assert.equal(hash(sourceBytes),entry.sourceRequestSha256)
 const source=JSON.parse(sourceBytes.toString())
 const {op,...args}=source
 const before=JSON.stringify(args)
 assert.ok(['brep_nurbs_progressive_profile_annular_body','brep_nurbs_progressive_profile_auto_body'].includes(op))
 const operation=automatic?'brep_nurbs_progressive_profile_auto_body':'brep_nurbs_progressive_profile_annular_body'
 const body=invoke(operation,{...args,smoothness_max_work:2000000})
 assert.equal(JSON.stringify(args),before)
 assert.equal(body.capRepresentation,'radial-annular')
 if(automatic)assert.ok(body.capSelectionExactWork>0)
 else assert.equal(body.capSelectionExactWork,0)
 assert.equal(body.boundaryContinuousBound,true)
 assert.equal(body.boundaryErrorWithinBudget,true)
 assert.ok(body.boundaryErrorUpper<=args.max_deviation)
 assert.equal(body.globalEmbeddingCertified,false)
 assert.deepEqual(body.planarCapFaces,[])
 assert.deepEqual(body.capFaceGroups.flat(),body.capFaces)
 assert.equal(new Set(body.capFaces).size,body.capFaces.length)
 assert.equal(body.retainedAnnularFilling.sourceFillingCovered,true)
 assert.equal(body.retainedAnnularFilling.sourceUvFacesInjective,true)
 assert.equal(body.retainedAnnularFilling.sourceUvPairsClassified,true)
 assert.equal(body.retainedAnnularFilling.exactWork,(body.retainedAnnularFilling.selectionExactWork??0)+body.retainedAnnularFilling.constructionExactWork+body.retainedAnnularFilling.fillingExactWork)
 assert.ok(body.retainedAnnularFilling.exactWork<=1000000)
 assert.equal(body.retainedSmoothness.capWallContinuity,'C0')
 assert.equal(body.retainedSmoothness.fullBoundarySmoothnessCertified,false)
 assert.ok(body.retainedSmoothness.totalExactWork<=2000000)
 const boundaryCertificate={method:'retained-sweep-boundary-union',scope:'boundary-set-hausdorff',
  continuousBound:body.boundaryContinuousBound,errorUpper:body.boundaryErrorUpper,
  filledCapErrorUpper:body.filledCapErrorUpper,budget:args.max_deviation,withinBudget:body.boundaryErrorWithinBudget}
 const key=entry.sourceRequestFile.replace('-source.json','')
 const audited=materialAndStep({...body,boundaryCertificate},key)
 reports.push({key,operation,sourceRequestSha256:entry.sourceRequestSha256,boundaryCertificate,
  retainedAnnularFilling:body.retainedAnnularFilling,retainedSmoothness:body.retainedSmoothness,volume:audited.proof})
 for(const injected of ['model','boundaryCertificate','retainedAnnularFilling','capFaces','retainedSmoothness']){
  assert.throws(()=>invoke(operation,{...args,[injected]:{certified:true}}),/original source/)
 }
 assert.throws(()=>invoke(operation,{...args,annular_cap_max_work:0}))
 const unproved=invoke(operation,{...args,annular_cap_max_products:0})
 assert.equal(unproved.boundaryContinuousBound,false)
 assert.equal(unproved.boundaryErrorUpper,null)
}
const curve=(value:number)=>({degree:1,knots:[0,0,1,1],controlPoints:[[value,0,0],[value,0,0]],weights:[1,1],periodic:false})
const outer=invoke('curve_circle',{center:[0,0,0],normal:[0,0,1],radius:.5})
const hole=invoke('curve_reverse',{curve:invoke('curve_circle',{center:[0,0,0],normal:[0,0,1],radius:.25})})
const request={loops:[[outer],[hole]],path:invoke('curve_line',{start:[0,0,0],end:[0,0,10]}),
 scale:curve(1),twist:curve(0),normal:[1,0,0],orientation:'fixed',spacing:'parameter',
 initial_sections:3,max_sections:3,max_deviation:.01,smoothness_max_work:2000000}
const ownedOperation=automatic?'brep_nurbs_progressive_profile_auto_body':'brep_nurbs_progressive_profile_annular_body'
const owned=invoke(ownedOperation,request)
assert.equal(owned.capRepresentation,automatic?'legacy':'radial-annular')
assert.equal(owned.capFaces.length,automatic?2:8)
assert.deepEqual(owned.planarCapFaces,automatic?owned.capFaces:[])
const repair={station_quantum:.125,station_tolerance:1e-9,station_max_work:1000000}
const reconstruct=(extra:object={})=>invoke(ownedOperation+'_reconstruct_stations',{
 ...request,...repair,expected_source_model:owned.model,...extra})
const repaired=reconstruct(),candidate=repaired.stationReconstruction
assert.equal(candidate.boundaryCertificate.continuousBound,true)
assert.equal(candidate.boundaryCertificate.withinBudget,true)
assert.equal(candidate.profileSmoothness.station.stationG2Certified,true)
assert.deepEqual(repaired.capFaces,owned.capFaces)
assert.deepEqual(repaired.planarCapFaces,owned.planarCapFaces)
for(const face of owned.capFaces)assert.deepEqual(candidate.reconstruction.model.faces[face],owned.model.faces[face])
const tainted=structuredClone(owned.model);tainted.vertices[0].point[0]+=1e-12
assert.throws(()=>reconstruct({expected_source_model:tainted}),/differs/)
if(automatic)assert.equal(reconstruct({station_max_work:0}).stationReconstruction.reconstruction,null)
else assert.throws(()=>reconstruct({station_max_work:0}),/Annular boundary identity is not exact/)
const repairedMaterial=volume(candidate.reconstruction.model,repaired.planarCapFaces)
assert.equal(repairedMaterial.solidGeometryCertified,true)
mkdirSync(directory,{recursive:true})
writeFileSync(join(directory,'manifest.json'),JSON.stringify({schema:'sweep-external-step/2',units:'mm',
 artifactProvenance:{mode:artifactMode,sha256,sourceCaptureManifestSha256:hash(readFileSync(join(nativeRoot,'manifest.json')))},cases:stepCases},null,2)+'\n')
writeFileSync(output,JSON.stringify({schema:'smooth-annular-owned-abi/1',passed:true,automatic,artifactPath,artifactMode,sha256,
 byteLength:bytes.length,scope:'Finite exact-artifact ABI, source-bound, fresh material, station jets and independent STEP preparation; no Rush/UI delivery claim',
 cases:reports,reconstruction:{boundaryCertificate:candidate.boundaryCertificate,profileSmoothness:candidate.profileSmoothness,
 volume:repairedMaterial,capFacesPreserved:true,mutatedSourceRefused:true,zeroWorkRefused:true},calls},null,2)+'\n')
console.log('Smooth annular ABI3, fresh all-face material, retained station G2 and refusal controls passed')
