import artifactIdentity from '../src/generated/geometry-kernels/identity'
import {verifyPackedWasmChunk} from './verify-packed-wasm.mjs'
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {Worker} from 'node:worker_threads'
import {MainSolidWorkerClient,type MainSolidPort,type MainSolidTiming} from '../src/services/mainSolidWorkerClient'
import {expressionInspectionExpectation,validExpressionInspection} from '../src/services/expressionShellInspection'

assert.ok(process.argv[2],'Provide a qualification output directory')
const directory=resolve(process.argv[2])
const timeoutArgument=process.argv.find(value=>value.startsWith('--timeout-ms='))
const timeoutMs=timeoutArgument?Number(timeoutArgument.slice('--timeout-ms='.length)):120000
assert(Number.isSafeInteger(timeoutMs)&&timeoutMs>=1&&timeoutMs<=120000,'Bound qualification timeout to 1..120000 ms')
const nativeReportArgument=process.argv.find(value=>value.startsWith('--native-wall-report='))
const nativeWallReport=nativeReportArgument?JSON.parse(readFileSync(resolve(nativeReportArgument.slice('--native-wall-report='.length)),'utf8')):null
const nativeReplayArgument=process.argv.find(value=>value.startsWith('--native-inspection-replay='))
const nativeReplay=nativeReplayArgument?JSON.parse(readFileSync(resolve(nativeReplayArgument.slice('--native-inspection-replay='.length)),'utf8')):null
const wasmPath='public/wasm/geometry-kernel.wasm'
const hash=()=>createHash('sha256').update(readFileSync(wasmPath)).digest('hex')
const wasmSha256=hash()
assert.equal(artifactIdentity.sha256,wasmSha256,'Packed identity must match published WASM')
assert.equal(artifactIdentity.byteLength,readFileSync(wasmPath).length)
verifyPackedWasmChunk(readFileSync('src/generated/geometry-kernels/bytes.ts','utf8'),readFileSync(wasmPath),'expression shell worker package')
const requestArgument=process.argv.find(value=>value.startsWith('--inspection-request='))
const requestBytes=requestArgument?readFileSync(resolve(requestArgument.slice('--inspection-request='.length))):null
const requestIdentity=requestBytes?{path:resolve(requestArgument!.slice('--inspection-request='.length)),sha256:createHash('sha256').update(requestBytes).digest('hex'),byteLength:requestBytes.length}:null
const fixture=requestBytes?{request:JSON.parse(requestBytes.toString('utf8'))}:JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-shell-inspection-native-2026-10-09.json','utf8'))
if(requestBytes){
 assert.equal(fixture.request.op,'cad_expression_shell_inspect')
 const mutatingFlags=['--aligned-chords','--aligned-chords-bounded','--radius','--radius-violation','--radius-unproven','--neighbor-tangency','--neighbor-tangency-unproven','--support-tangency','--support-tangency-unproven','--boundary-mesh','--refined-boundary','--display','--mesh','--extended-mesh']
 assert(!mutatingFlags.some(flag=>process.argv.includes(flag)),'Saved inspection requests must be replayed without option overrides')
}
const {op:_,...options}=fixture.request
if(process.argv.includes('--aligned-chords'))options.alignedChordCheck={
 minimumMm:1,maxSineSquared:1e-4,toleranceUv:0.05,controls:1000,
 cells:process.argv.includes('--aligned-chords-bounded')?10000:0,
 spans:process.argv.includes('--aligned-chords-bounded')?100000:0,
 domainCells:process.argv.includes('--aligned-chords-bounded')?100000:0,
}
const radiusState=process.argv.includes('--radius-violation')?'violation':process.argv.includes('--radius-unproven')?'unproven':'admitted'
if(process.argv.includes('--neighbor-tangency')){
 const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-neighbor-tangency-dispatch-2026-10-10.json','utf8'))
 options.neighborTangencyCheck=fixture[process.argv.includes('--neighbor-tangency-unproven')?'unproven':'admitted'].request.neighborTangencyCheck
}
if(process.argv.includes('--support-tangency')){
 const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-support-tangency-dispatch-2026-10-09.json','utf8'))
 options.supportTangencyCheck=fixture[process.argv.includes('--support-tangency-unproven')?'unproven':'admitted'].request.supportTangencyCheck
}
if(process.argv.includes('--radius')){
 const radiusFixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-radius-dispatch-2026-10-09.json','utf8'))
 options.radiusCheck=radiusFixture[radiusState].request.radiusCheck
}
if(process.argv.includes('--boundary-mesh'))options.boundaryMeshLimits={maxDeviationMm:0.05,cells:4096,spans:100000,maxDepth:24}
if(process.argv.includes('--refined-boundary')){
 assert(options.boundaryMeshLimits,'--refined-boundary requires --boundary-mesh')
 options.boundaryMeshLimits.refinement={maxDeviationMm:0.05,cells:512,spans:20000,spansPerQuery:64,controls:1000,domainCells:10000,domainCellsPerTriangle:128,toleranceUv:1e-8,maxDepth:8,regionalSpans:10000}
}
const displayRequested=process.argv.includes('--display')
const meshRequested=process.argv.includes('--mesh')
const extendedMesh=process.argv.includes('--extended-mesh')
assert(!extendedMesh||meshRequested,'--extended-mesh requires --mesh')
if(meshRequested){
  const precise=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-mesh-precision-native-2026-10-09.json','utf8'))
  options.meshLimits=precise.request.meshLimits
  if(extendedMesh)options.meshLimits={...options.meshLimits,cells:32768,domainCells:100000,maxDepth:18}
}
if(displayRequested)options.displayLimits={divisions:4,cells:10000,controlsPerEnclosure:1000,spans:10000,toleranceUv:1e-8,domainCells:10000,domainCellsPerCell:256}
assert(!nativeWallReport||options.alignedChordCheck,'Native wall parity requires an aligned-chord request')
if(nativeReplay){
 assert(requestIdentity,'Native inspection parity requires an unchanged saved request')
 assert.equal(nativeReplay.schema,'expression-inspection-native-replay/1')
 assert.equal(nativeReplay.requestSha256,requestIdentity.sha256,'Native and worker input bytes must match')
 assert.equal(nativeReplay.requestBytes,requestIdentity.byteLength)
 assert.deepEqual(nativeReplay.request,fixture.request,'Native and worker request options must match')
}
const refusedOptions=structuredClone(options);refusedOptions.profileLimits.controls=0
const realms:{port:MainSolidPort;entered:Promise<void>;exited:Promise<number>;terminated:boolean}[]=[]
const workerArtifacts:unknown[]=[]
const clientTimings:MainSolidTiming[]=[]
const onTiming=(timing:MainSolidTiming)=>{clientTimings.push(timing)}
const workerObservations:{realm:number;id:number;kind:string;postedAt:number;enteredAfterMs?:number;responseAfterMs?:number;timing?:{warmupMs:number;executeMs:number;prepareMs:number}}[]=[]
const client=new MainSolidWorkerClient(()=>{
  const worker=new Worker(new URL('./check-cad-whole-wall-worker-entry.mjs',import.meta.url),{workerData:{expectedArtifact:artifactIdentity}})
  const realmIndex=realms.length
  const observations=new Map<number,typeof workerObservations[number]>()
  let entered!:()=>void
  const realm={port:null as unknown as MainSolidPort,entered:new Promise<void>(resolve=>{entered=resolve}),
    exited:new Promise<number>(resolve=>worker.once('exit',resolve)),terminated:false}
  const port:MainSolidPort={onmessage:null,onerror:null,onmessageerror:null,
    postMessage(request){
      realm.entered=new Promise<void>(resolve=>{entered=resolve})
      const observation={realm:realmIndex,id:request.id,kind:request.job.kind,postedAt:performance.now()}
      observations.set(request.id,observation);workerObservations.push(observation)
      worker.postMessage(request)
    },
    terminate(){realm.terminated=true;void worker.terminate()},
  }
  worker.on('message',data=>{
    if(data?.artifact){assert.deepEqual(data.artifact,artifactIdentity);workerArtifacts.push(data.artifact);return}
    const observation=observations.get(data?.entered??data?.id)
    if(data?.entered){if(observation)observation.enteredAfterMs=performance.now()-observation.postedAt;entered();return}
    if(observation){
      observation.responseAfterMs=performance.now()-observation.postedAt
      if(data?.timing){
        assert(['warmupMs','executeMs','prepareMs'].every(k=>Number.isFinite(data.timing[k])&&data.timing[k]>=0),'Invalid worker stage timing')
        observation.timing={...data.timing}
      }
    }
    port.onmessage?.({data} as MessageEvent)
  })
  worker.on('error',error=>port.onerror?.({message:error.message} as ErrorEvent))
  worker.on('messageerror',()=>port.onmessageerror?.({} as MessageEvent))
  realm.port=port;realms.push(realm);return port
})
let warm:any,retry:any
let failureReport:Record<string,unknown>|undefined
let qualificationPhase='budget-refusal'
const qualificationStarted=performance.now()
const timingsMs={refusal:0,dispatchCancellation:0,retry:0}
try{
  let started=performance.now()
  warm=await client.run({kind:'expressionShellInspect',options:refusedOptions},{timeoutMs,onTiming})
  timingsMs.refusal=performance.now()-started
  assert(validExpressionInspection(expressionInspectionExpectation(refusedOptions),warm))
  assert.equal(warm.incidenceQualified,false)
  qualificationPhase='dispatch-cancellation'
  const abort=new AbortController()
  started=performance.now()
  const pending=client.run({kind:'expressionShellInspect',options},{signal:abort.signal,timeoutMs,onTiming})
  const refused=assert.rejects(pending,(e:any)=>e.name==='AbortError'&&e.code==='CAD_CANCELLED')
  await realms[0].entered;abort.abort();await refused
  assert.equal(realms[0].terminated,true);await realms[0].exited
  timingsMs.dispatchCancellation=performance.now()-started
  started=performance.now()
  qualificationPhase='fresh-worker-retry'
  retry=await client.run({kind:'expressionShellInspect',options},{timeoutMs,onTiming})
  timingsMs.retry=performance.now()-started
  assert.equal(realms.length,2)
  const completedObservations=workerObservations.filter(observation=>observation.responseAfterMs!==undefined)
  assert.equal(clientTimings.length,2,'Only the refusal and Retry emit client stage timing')
  assert.equal(completedObservations.length,2,'Refusal and Retry must each produce a response observation')
  assert(completedObservations.every(observation=>observation.timing!==undefined),'Every completed response must include requested worker stage timing')
  assert(validExpressionInspection(expressionInspectionExpectation(options),retry))
  if(nativeReplay)assert.deepEqual(retry,nativeReplay.response,'Complete worker response must match native replay')
  assert.equal(retry.incidenceQualified,true,retry.reason)
  assert.equal(retry.shell.faceCount,29);assert.equal(retry.shell.components,1)
  assert.equal(retry.embeddingQualified,true);assert.equal(retry.materialQualified,false)
  assert.equal(retry.shell.inspection.volumeQualified,true)
  assert.equal(retry.shell.inspection.contacts.pendingPairs,0)
  assert.equal(retry.shell.inspection.contacts.pairs.length,406)
  if(options.alignedChordCheck){
    const checked=retry.shell.alignedChordCheck
    assert(checked,'Native embedded geometry must return requested aligned-chord coverage')
    assert.equal(checked.wholeThicknessQualified,false);assert.equal(checked.solidQualified,false)
    assert.equal(checked.report.total_pairs,435)
    if(nativeWallReport)assert.deepEqual(checked.report,nativeWallReport,'Worker report must exactly match native coverage and work accounting')
    assert.equal(checked.report.minimum_mm,options.alignedChordCheck.minimumMm)
    for(const key of ['profile_transition_checked','profile_transition_excluded','profile_transition_inconclusive','profile_transition_unresolved'] as const){
      assert(Number.isSafeInteger(checked.report[key])&&checked.report[key]>=0,`Current WASM must expose nonnegative ${key}`)
    }
    assert.equal(checked.report.profile_transition_excluded+checked.report.profile_transition_inconclusive+checked.report.profile_transition_unresolved,checked.report.profile_transition_checked,'Every transition query must retain its native outcome')
    assert.equal(checked.qualified,checked.report.complete)
    assert(checked.report.intrinsic_spans<=checked.report.spans)
    if(process.argv.includes('--span-stages')){
      const stages=checked.report.span_stages
      assert(stages,'New WASM must expose native span accounting')
      const stageNames=['position','normal','shared_corner','original_pair','retained_pair','corrected_pair'] as const
      assert.equal(stageNames.reduce((sum,key)=>sum+stages[key],checked.report.intrinsic_spans+checked.report.profile_transition_spans),checked.report.spans)
    }
    if(options.alignedChordCheck.cells===0){
      assert.equal(checked.qualified,false);assert.equal(checked.report.pending.length,435)
      assert.equal(checked.report.cells,0);assert.equal(checked.report.spans,0)
    }
  }
  if(options.neighborTangencyCheck){
    const report=retry.shell.neighborTangencyCheck
    assert(report,'Requested actual-neighbor tangency report must be present')
    assert.equal(report.qualified,!process.argv.includes('--neighbor-tangency-unproven'))
    assert.equal(report.solidQualified,false)
    if(report.qualified){
      assert.equal(report.upperSineSquared.length,4)
      assert(report.upperSineSquared.every((v:number)=>v>=0&&v<=options.neighborTangencyCheck.maxSineSquared))
    }else assert.equal(report.upperSineSquared,null)
  }
  if(options.supportTangencyCheck){
    const report=retry.shell.supportTangencyCheck
    assert(report,'Requested support tangency report must be present')
    assert.equal(report.qualified,!process.argv.includes('--support-tangency-unproven'))
    assert.equal(report.neighborBodyQualified,false)
    assert.equal(report.solidQualified,false)
    if(report.qualified)assert.equal(report.boundaries.length,4)
  }
  if(options.radiusCheck){
    const radius=retry.shell.radiusCheck
    assert(radius,'Requested native radius report must be present')
    assert.equal(radius.qualified,radiusState==='admitted')
    assert.equal(radius.violationQualified,radiusState==='violation')
    assert.equal(radius.solidQualified,false)
    if(radiusState==='violation')assert(radius.errorLowerMm>options.radiusCheck.toleranceMm)
    if(radiusState==='unproven')assert.equal(radius.errorLowerMm,null)
  }
  const bounds=retry.shell.inspection.volume.positiveBounds
  assert(bounds[0]>0&&bounds[1]-bounds[0]<=options.inspectionLimits.volume.absoluteError)
  if(options.boundaryMeshLimits){
    const graph=retry.shell.boundaryMesh
    assert.equal(graph.materialQualified,false)
    assert.equal(graph.surfaceToleranceQualified,false)
    assert.equal(graph.closedOrientedIncidence,true)
    assert.equal(graph.pendingFaces.length,0);assert.equal(graph.pending.length,0)
    assert.equal(graph.openEdges,0);assert.equal(graph.nonmanifoldEdges,0)
    assert.equal(graph.orientationConflicts,0);assert.equal(graph.invalidVertexLinks,0)
    assert(graph.candidateTriangles.length>0)
    assert.equal(new Set(graph.candidateTriangles.map((t:any)=>t.face)).size,29)
    if(options.boundaryMeshLimits.refinement){
      const refined=graph.refinement,limits=options.boundaryMeshLimits.refinement
      assert(refined,'Native worker must return the requested refinement')
      assert.equal(refined.solidQualified,false)
      assert.deepEqual(refined.limits,limits)
      assert.equal(refined.initialTriangles,graph.candidateTriangles.length)
      assert.equal(refined.accepted.length+refined.pending.length,refined.initialTriangles+2*refined.splitTriangles)
      assert(refined.pending.length>0,'Unproven surface regions remain explicit')
      assert(refined.accepted.every((t:any)=>t.deviationUpperMm<=limits.maxDeviationMm))
      assert(refined.spans<=limits.spans&&refined.regional.spans<=limits.regionalSpans)
      assert.equal(refined.closedOrientedIncidence,true)
    }
  }
  if(displayRequested){
    const display=retry.shell.supportEnclosures
    assert.equal(display.trimQualified,false)
    assert.equal(display.cells.length,464)
    assert(display.inside>0&&display.unresolvedTrim>0)
    assert.equal(new Set(display.cells.map((cell:any)=>cell.face)).size,29)
    assert(display.domainCells>0&&display.domainCells<=options.displayLimits.domainCells)
  }
  if(meshRequested){
    const mesh=retry.shell.displayMesh
    assert.equal(mesh.watertightQualified,false)
    assert(mesh.tiles.length>0)
    assert(mesh.tiles.some((tile:any)=>retry.shell.faces[tile.face].kind==='transition'))
    assert(mesh.maxDeviationUpperMm>0&&mesh.maxDeviationUpperMm<=0.05)
    assert(mesh.pending.length>0,'Boundary coverage remains explicitly pending')
    assert.equal(mesh.leafCount,29+mesh.splitCells)
  }
}catch(error){
  failureReport={
    schema:'expression-worker-qualification-failure/1',passed:false,phase:qualificationPhase,
    elapsedMs:performance.now()-qualificationStarted,timeoutMs,wasmSha256,
    wasmUnchanged:hash()===wasmSha256,workersStarted:realms.length,timingsMs,requestIdentity,workerArtifacts,workerObservations,clientTimings,
    error:error instanceof Error?{name:error.name,message:error.message,code:(error as {code?:string}).code}:{message:String(error)},
    scope:'Failed qualification only; no geometry or latency admission',
  }
  throw error
}finally{
  client.dispose();await Promise.all(realms.map(r=>r.exited))
  if(failureReport){
    mkdirSync(directory,{recursive:true})
    writeFileSync(resolve(directory,'expression-worker-failure.json'),JSON.stringify({
      ...failureReport,workersJoined:realms.length,
    },null,2)+'\n',{flag:'wx'})
  }
}
assert.equal(hash(),wasmSha256,'WASM changed during qualification')
assert.equal(workerArtifacts.length,realms.length,'Every worker must report the expected loaded kernel identity')
const report={schema:'expression-shell-inspection-real-worker/1',passed:true,wasmSha256,requestIdentity,
 nativeInspectionParity:nativeReplay?{passed:true,source:resolve(nativeReplayArgument!.slice('--native-inspection-replay='.length))}:null,
 nativeWallParity:nativeWallReport?{passed:true,source:resolve(nativeReportArgument!.slice('--native-wall-report='.length))}:null,
 workersStarted:realms.length,workersJoined:realms.length,workerArtifacts,workerObservations,clientTimings,displayRequested,meshRequested,extendedMesh,timingsMs,warm,retry,
 optionalChecks:{alignedChords:!!options.alignedChordCheck,spanStages:process.argv.includes('--span-stages'),radius:!!options.radiusCheck,originalSupportTangency:!!options.supportTangencyCheck,actualNeighborTangency:!!options.neighborTangencyCheck},
 scope:'Actual Node worker production handler/client: native budget refusal, dispatch cancellation, terminated realm, fresh-worker Retry, exact annular shell embedding and signed-volume qualification. Optional radius, original-support and actual-neighbor tangency checks concern the saved controlled recipe only. The actual-neighbor check covers four patch joins, not all remaining shell seams. This does not admit a solid Body, thickness, general fillets or browser preview.'}
mkdirSync(directory,{recursive:true})
writeFileSync(resolve(directory,'expression-inspection-worker-report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'})
console.log(JSON.stringify({passed:true,workersStarted:realms.length,wasmSha256,faces:retry.shell.faceCount}))
