import assert from 'node:assert/strict'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {dirname,join} from 'node:path'
import {createHash} from 'node:crypto'
import {encodeBinary} from '../src/services/valueBinaryCodec'
import {writeLinear,decodePacked} from '../src/services/wasmHost'
import {DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
import {correctedC2Path} from './corrected-c2-sources.mjs'

const [root,output,buildReceipt]=process.argv.slice(2)
const exactPlanarCaps=process.argv.includes('--exact-planar-caps')
assert.ok(root&&output&&buildReceipt)
const sha=(v:Uint8Array|string)=>createHash('sha256').update(v).digest('hex')
const build=JSON.parse(readFileSync(buildReceipt,'utf8'));assert.equal(build.passed,true)
const report:any={schema:'corrected-c2-staged-native/1',passed:false,
 scope:'Actual raw/packed native C2 source value/error, filled-boundary and fresh retained-body Solid audits. Receiving Rush/viewport/UI/STEP and source/retained G1/G2 remain separate.',
 buildReceiptSha256:sha(readFileSync(buildReceipt)),runnerSha256:sha(readFileSync(new URL(import.meta.url))),
 fixtureSha256:sha(readFileSync(new URL('./corrected-c2-sources.mjs',import.meta.url))),
 nativeSourceSha256:sha(readFileSync(join(root,'crates/nurbs-core/src/sweeps/progressive_sweep/authored_error/corrected_regular.rs'))),
 nativeSilenceLimitMs:30000,exactPlanarCaps,artifacts:[]}
assert.equal(report.nativeSourceSha256,'c1931a0d33066c25f87bf0c2297785232dc8c08aff1ac213d8ed120e7a8597e7')
report.nativeCapProjectionSourceSha256=sha(readFileSync(join(root,'crates/nurbs-core/src/construct/surfaces/sections/section_projection.rs')))
if(exactPlanarCaps)assert.equal(report.nativeCapProjectionSourceSha256,'53ee88ea4be1d6d046994a03d55f45a83f550250530fb6df592cbda661a2f292')
mkdirSync(dirname(output),{recursive:true})
const save=()=>writeFileSync(output,JSON.stringify(report,null,2)+'\n')
const line=(a:number[],b:number[])=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
for(const kind of ['raw','packed']){
 const bytes=readFileSync(join(root,kind==='raw'?'crates/target/wasm32-unknown-unknown/release/geometry_wasm.wasm':'public/wasm/geometry-kernel.wasm'))
 assert.deepEqual({sha256:sha(bytes),byteLength:bytes.length},build.artifacts[kind])
 const w=(await WebAssembly.instantiate(bytes)).instance.exports as any
 const item:any={kind,...build.artifacts[kind],passed:false,calls:[],cases:[]};report.artifacts.push(item);save()
 function call(name:string,args:any){
  const before=JSON.stringify(args),encoded=encodeBinary(args),ptr=writeLinear(w.memory,w.abi_alloc,encoded);assert.ok(ptr)
  let response:any,milliseconds:number
  try{const start=performance.now();response=decodePacked<any>(w.memory,w.abi_free,w.abi_request(0,ptr,encoded.length));milliseconds=performance.now()-start}
  finally{w.abi_free(ptr,encoded.length)}
  item.calls.push({name,args,requestSha256:sha(before),response,milliseconds:milliseconds!});save()
  assert.ok(milliseconds!<=30000);assert.equal(JSON.stringify(args),before)
  assert.equal(response.ok,true,JSON.stringify(response.error));return response.value
 }
 const path=structuredClone(correctedC2Path),scale=line([1,0,0],[1,0,0]),twist=line([0,0,0],[0,0,0])
 assert.equal(call('original-frenet-regularity',{op:'sweep_frenet_frame_regularity',path,twist,maxCells:10000}).regularityCertified,true)
 const outer=call('original-outer',{op:'curve_circle',center:[0,0,0],normal:[1,0,0],radius:.1})
 const hole=call('original-hole',{op:'curve_circle',center:[0,0,0],normal:[-1,0,0],radius:.05})
 for(const [spacing,count] of [['parameter',17],['arc_length',65]] as const){
  const common={path,scale,twist,normal:[0,0,1],orientation:'corrected_frenet',spacing,
   length_tolerance:.001,length_max_cells:100000,initial_sections:3,max_sections:129,max_deviation:.05}
  const surface=call(spacing+'-retained-source',{op:'surface_progressive_sweep_level',profiles:[line([0,.1,0],[0,.2,0])],...common,preview_sections:count})
  assert.equal(surface.report.continuousBound,true);assert.equal(surface.report.accepted,true)
  assert.ok(surface.report.continuousErrorUpper>=0&&surface.report.continuousErrorUpper<=.05)
  const frame=call(spacing+'-frame-second-jets',{op:'surface_progressive_sweep_frame_smoothness',profiles:[line([0,.1,0],[0,.2,0])],...common,order:2,maxCells:10000,maxExactWork:1000000})
  assert.equal(frame.sourceFrameSmoothnessCertified,false,'C2 value eligibility must not promote source second jets')
  const body=call(spacing+'-owned-body',{op:'brep_nurbs_progressive_profile_auto_body',...common,loops:[[outer],[hole]],
   circle_correction_tolerance:1e-9,circle_correction_quantum:2**-40,circle_correction_max_work:1000000,
   ...(exactPlanarCaps?{cap_correction_tolerance:1e-9,cap_correction_quantum:2**-40,cap_correction_max_work:1000000}:{}),
   annular_cap_max_work:1000000,annular_cap_max_products:100000,annular_cap_max_pairs:10000,annular_cap_max_cells:100000,
   smoothness_max_work:2000000,geometric_g2_max_cells:128})
  assert.equal(body.boundaryContinuousBound,true);assert.equal(body.boundaryErrorWithinBudget,true)
  const savedProfile=body.retainedSmoothness.walls
  assert.equal(savedProfile.profileG1Certified,true)
  assert.equal(savedProfile.profile.exactG1G2Certified,true)
  assert.deepEqual(savedProfile.profile.unresolvedSeams,[])
  assert.equal(savedProfile.profile.seams.length,body.model.faces.length-body.capFaces.length)
  if(exactPlanarCaps){assert.equal(body.capRepresentation,'legacy');assert.ok(body.capCorrectionErrorUpper>=0&&body.capCorrectionErrorUpper<=1e-9)}
  assert.ok(body.boundaryErrorUpper>=0&&body.boundaryErrorUpper<=.05)
  assert.equal(body.filledCapErrorUpper.length,2)
  assert.ok(body.filledCapErrorUpper.every((v:number)=>v>=0&&v<=.05))
  const volume=call(spacing+'-fresh-solid',{op:'brep_sweep_volume_charts_audit',model:body.model,capFaces:body.planarCapFaces,...DEFAULT_SWEEP_VOLUME_BUDGETS})
  for(const field of ['solidGeometryCertified','allFacesInjective','allPairsClassified','boundaryEmbeddingCertified'])assert.equal(volume[field],true,field)
  assert.equal(volume.nesting.rolesConsistent,true);assert.equal(volume.nextPair,null)
  assert.ok(volume.orientations.every((s:any)=>s.outward===s.expectedOutward))
  assert.equal(volume.individualPairs+volume.groupedPairs,body.model.faces.length*(body.model.faces.length-1)/2)
  const refused=call(spacing+'-exhausted-solid',{op:'brep_sweep_volume_audit',model:body.model,capFaces:body.planarCapFaces,
   ...DEFAULT_SWEEP_VOLUME_BUDGETS,maxPairs:1,maxCells:1,maxDomainCells:1,maxExactWork:1})
  assert.equal(refused.solidGeometryCertified,false)
  item.cases.push({spacing,sourceReport:surface.report,frame,body,volume});save()
 }
 item.passed=true;save()
}
assert.deepEqual(report.artifacts[0].calls.map(({milliseconds,...c}:any)=>c),report.artifacts[1].calls.map(({milliseconds,...c}:any)=>c))
report.passed=true;save();console.log('Corrected C2 parameter/arc source, filled caps, fresh Solid and refusal raw/packed parity passed')
