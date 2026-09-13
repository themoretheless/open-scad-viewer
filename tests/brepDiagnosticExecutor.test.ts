import {Worker} from 'node:worker_threads'
import {describe, expect, it} from 'vitest'
import {sha256Hex} from '../src/core/sha256'
import {GEOMETRY_MANIFEST_ARCHIVE, geometryProviderAdmissionForManifest} from '../src/core/geometryExecution'
import {BrepDiagnosticExecutor, type BrepDiagnosticWorkerPort} from '../src/services/brepDiagnosticExecutor'
import {BREP_DIAGNOSTIC_IDENTITY, BREP_DIAGNOSTIC_LIMITS, diagnosticEnvelope, isBrepDiagnosticMessage,
  isBrepDiagnosticRequest, type BrepDiagnosticRequest} from '../src/services/brepDiagnosticProtocol'
import {createBrepDiagnosticSupervisor} from '../src/mcp/brepDiagnosticSupervisor'
import {buildBrepSemanticScene} from '../src/services/brepSemanticScene'
import {lowerOpenSCADToSemanticProgram} from '../src/services/semanticProgramLowerer'

const source = '// @language openscad-viewer/brep-1\nlinear_extrude(3)difference(){circle(4);circle(2);}'
const policy = {quality: 'preview' as const, segments: 4}
const request = (): BrepDiagnosticRequest => ({type:'evaluate', workerEpoch:1, jobId:1, source,
  sourceSha256:sha256Hex(source), policy, identity:BREP_DIAGNOSTIC_IDENTITY})

class FakePort implements BrepDiagnosticWorkerPort {
  callbacks!: Parameters<BrepDiagnosticWorkerPort['listen']>[0]
  request!: BrepDiagnosticRequest
  terminated = 0
  unreferenced = false
  onRequest: (request: BrepDiagnosticRequest) => void = () => {}
  onTerminate: () => void | Promise<void> = () => {}
  postMessage(request: BrepDiagnosticRequest) {this.request = request; this.onRequest(request)}
  listen(callbacks: Parameters<BrepDiagnosticWorkerPort['listen']>[0]) {this.callbacks=callbacks; return () => {}}
  terminate() {this.terminated++; return this.onTerminate()}
  unref() {this.unreferenced=true}
  started() {this.callbacks.message({...diagnosticEnvelope(this.request), status:'started'})}
}

/** Infinite synchronous WebAssembly execution, to exercise actual hard termination. */
function hangingWasmPort(onStarted: () => void): BrepDiagnosticWorkerPort {
  const worker = new Worker(`
    const {parentPort}=require('node:worker_threads');
    const {createHash}=require('node:crypto');
    parentPort.once('message',r=>{
      const displayPolicyHash=createHash('sha256').update('brep-display-policy-v1\\n'+JSON.stringify({version:1,sampling:'uniform-patch',quality:r.policy.quality,segments:r.policy.segments})).digest('hex');
      parentPort.postMessage({workerEpoch:r.workerEpoch,jobId:r.jobId,sourceSha256:r.sourceSha256,identity:r.identity,displayPolicyHash,status:'started'});
      const bytes=Uint8Array.from([0,97,115,109,1,0,0,0,1,4,1,96,0,0,3,2,1,0,7,7,1,3,114,117,110,0,0,10,9,1,7,0,3,64,12,0,11,11]);
      new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports.run();
    });`, {eval:true, execArgv:[]})
  return {postMessage:r=>worker.postMessage(r), terminate:async()=>{await worker.terminate()}, unref:()=>worker.unref(),
    listen:callbacks=>{
      const message=(v:unknown)=>{callbacks.message(v); onStarted()}
      worker.on('message',message); worker.on('error',callbacks.error); worker.on('exit',callbacks.exit)
      return ()=>{worker.off('message',message); worker.off('error',callbacks.error); worker.off('exit',callbacks.exit)}
    }}
}

describe('isolated B-rep diagnostic execution',()=>{
  it('executes the real Node worker and matches direct scene identities without a production descriptor',async()=>{
    const lane=createBrepDiagnosticSupervisor()
    const receipt=await lane.evaluate(source,policy)
    const direct=await buildBrepSemanticScene(lowerOpenSCADToSemanticProgram(source,{quality:'preview'}),policy)
    expect(receipt.scene.attestation).toEqual(direct.attestation)
    expect(receipt.scene.outputs).toEqual(direct.outputs)
    expect(receipt.scene.result.meshes[0].vertices).toEqual(direct.result.meshes[0].vertices)
    expect(receipt.scene.result.meshes[0].nativeGeometry).toEqual(direct.result.meshes[0].nativeGeometry)
    expect(receipt.identity).toEqual(BREP_DIAGNOSTIC_IDENTITY)
    expect(receipt).not.toHaveProperty('execution')
    expect(receipt.scene).toMatchObject({metrics:'display_mesh_estimates',deviationStatus:'not_certified'})
    expect(lane.snapshot()).toMatchObject({activeWorkerEpoch:null,workersStarted:1,workersTerminated:1,quarantined:false})
    expect(geometryProviderAdmissionForManifest(GEOMETRY_MANIFEST_ARCHIVE['brep-contract-v1']).allowed).toBe(false)
    const empty=await lane.evaluate('// @language openscad-viewer/brep-1\ndifference(){cube(1);cube(1);}',policy)
    expect(empty.scene.result.meshes).toEqual([])
    expect(empty.scene.outputs[0].empty).toBe(true)
    expect(lane.snapshot().workersTerminated).toBe(2)
  },15000)

  it('returns an actual native refusal, tears down, and permits a fresh successful run',async()=>{
    const lane=createBrepDiagnosticSupervisor()
    await expect(lane.evaluate('// @language openscad-viewer/brep-1\nhull(){sphere(1);translate([2,0,0])sphere(1);}',policy))
      .rejects.toMatchObject({code:'E_SEMANTIC_BACKEND_FAILURE'})
    expect(lane.snapshot()).toMatchObject({activeWorkerEpoch:null,workersTerminated:1})
    const result=await lane.evaluate('// @language openscad-viewer/brep-1\ncube(2);',policy)
    expect(result.scene.result.volume).toBeCloseTo(8)
  },15000)

  it('cancels the actual Node adapter and recovers in a fresh worker',async()=>{
    const lane=createBrepDiagnosticSupervisor(), controller=new AbortController()
    const pending=lane.evaluate(source,policy,{signal:controller.signal})
    controller.abort()
    await expect(pending).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_CANCELLED'})
    expect(lane.snapshot()).toMatchObject({activeWorkerEpoch:null,workersTerminated:1})
    const recovered=await lane.evaluate('// @language openscad-viewer/brep-1\ncube(2);',policy)
    expect(recovered.scene.result.volume).toBeCloseTo(8)
    expect(lane.snapshot()).toMatchObject({activeWorkerEpoch:null,workersStarted:2,workersTerminated:2})
  },15000)

  it('validates requests before worker admission and forbids hidden capability claims',async()=>{
    let starts=0
    const lane=new BrepDiagnosticExecutor(()=>{starts++; return new FakePort()})
    for(const invalid of ['cube(1);','// @language legacy/current\ncube(1);', 'x'.repeat(BREP_DIAGNOSTIC_LIMITS.sourceCharacters+1),
      '// @language openscad-viewer/brep-1\n// @requires geometry.brep\ncube(1);']) {
      await expect(lane.evaluate(invalid,policy)).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_INPUT'})
    }
    await expect(lane.evaluate(source,{...policy,segments:33})).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_INPUT'})
    const controller=new AbortController(); controller.abort()
    await expect(lane.evaluate(source,policy,{signal:controller.signal})).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_CANCELLED'})
    expect(starts).toBe(0)
    expect(isBrepDiagnosticRequest({...request(),sourceSha256:'0'.repeat(64)})).toBe(false)
  })

  it('refuses changed correlation, packed-artifact identities, policy, snapshots and oversized messages',async()=>{
    const lowered=lowerOpenSCADToSemanticProgram(source)
    const scene=await buildBrepSemanticScene(lowered,policy)
    const terminal={...diagnosticEnvelope(request()),status:'succeeded' as const,scene,programJson:JSON.stringify(lowered.program)}
    expect(isBrepDiagnosticMessage(terminal,request())).toBe(true)
    expect(isBrepDiagnosticMessage({...terminal,workerEpoch:2},request())).toBe(false)
    expect(isBrepDiagnosticMessage({...terminal,sourceSha256:'0'.repeat(64)},request())).toBe(false)
    expect(isBrepDiagnosticMessage({...terminal,identity:{...terminal.identity,kernelPackedBase64Sha256:'0'.repeat(64)}},request())).toBe(false)
    expect(isBrepDiagnosticMessage({...terminal,displayPolicyHash:'0'.repeat(64)},request())).toBe(false)
    const stale=structuredClone(terminal); stale.scene.outputs[0]={...stale.scene.outputs[0],snapshotRevision:'0'.repeat(64)}
    expect(isBrepDiagnosticMessage(stale,request())).toBe(false)
    const attest=structuredClone(terminal); attest.scene={...attest.scene,attestation:{...attest.scene.attestation,programHash:'0'.repeat(64)}}
    expect(isBrepDiagnosticMessage(attest,request())).toBe(false)
    const missing=structuredClone(terminal); missing.scene={...missing.scene,outputs:[]}; missing.scene.result.meshes=[]
    expect(isBrepDiagnosticMessage(missing,request())).toBe(false)
    const occurrence=structuredClone(terminal); occurrence.scene.outputs[0]={...occurrence.scene.outputs[0],nodeIndex:123}
    expect(isBrepDiagnosticMessage(occurrence,request())).toBe(false)
    const faces=structuredClone(terminal); faces.scene.outputs[0]={...faces.scene.outputs[0],topologyFaceIds:['face:forged']}
    expect(isBrepDiagnosticMessage(faces,request())).toBe(false)
    const indices=structuredClone(terminal); indices.scene.result.meshes[0].faceIds![0]=999999
    expect(isBrepDiagnosticMessage(indices,request())).toBe(false)
    expect(isBrepDiagnosticMessage({...terminal,scene:{...scene,outputs:Array.from({length:257},()=>scene.outputs[0])}},request())).toBe(false)
    const asset=structuredClone(terminal); asset.scene.result.meshes[0].vertices[0]+=123
    expect(isBrepDiagnosticMessage(asset,request())).toBe(false)
    const placement=structuredClone(terminal); placement.scene.result.meshes[0].transform[12]=123
    expect(isBrepDiagnosticMessage(placement,request())).toBe(false)
    const unbound=structuredClone(terminal); unbound.scene.result.meshes[0].provenance=[]
    expect(isBrepDiagnosticMessage(unbound,request())).toBe(false)
    const sourceSpan=structuredClone(terminal); sourceSpan.scene.result.meshes[0].provenance[0].source!.start++
    expect(isBrepDiagnosticMessage(sourceSpan,request())).toBe(false)
    const large=structuredClone(terminal); large.scene.result.meshes[0].vertices=new Float32Array(new ArrayBuffer(BREP_DIAGNOSTIC_LIMITS.transportBytes+16),0,3)
    expect(isBrepDiagnosticMessage(large,request())).toBe(false)
    const getter={...terminal}; Object.defineProperty(getter,'scene',{get:()=>{throw new Error('must not execute')},enumerable:true})
    expect(isBrepDiagnosticMessage(getter,request())).toBe(false)
  })

  it('refuses concurrent admission, stale startup and terminal-before-start without leaking workers',async()=>{
    const port=new FakePort(),lane=new BrepDiagnosticExecutor(()=>port)
    const first=lane.evaluate(source,policy)
    await expect(lane.evaluate(source,policy)).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_BUSY'})
    port.callbacks.message({...diagnosticEnvelope(port.request),status:'started',workerEpoch:99})
    await expect(first).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_PROTOCOL'})
    expect(port.terminated).toBe(1)
    const early=new FakePort(); early.onRequest=r=>early.callbacks.message({...diagnosticEnvelope(r),status:'failed',error:{name:'Error',code:null,message:'early'}})
    await expect(new BrepDiagnosticExecutor(()=>early).evaluate(source,policy)).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_PROTOCOL'})
    expect(early.terminated).toBe(1)
    const duplicate=new FakePort(); duplicate.onRequest=()=>{duplicate.started(); duplicate.started()}
    await expect(new BrepDiagnosticExecutor(()=>duplicate).evaluate(source,policy)).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_PROTOCOL'})
    expect(duplicate.terminated).toBe(1)
  })

  it('hard-cancels synchronous WASM and joins before rejection is observed',async()=>{
    const controller=new AbortController()
    const lane=new BrepDiagnosticExecutor(()=>hangingWasmPort(()=>setTimeout(()=>controller.abort(),20)),{deadlineMs:3000})
    await expect(lane.evaluate(source,policy,{signal:controller.signal})).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_CANCELLED'})
    expect(lane.snapshot()).toMatchObject({activeWorkerEpoch:null,workersStarted:1,workersTerminated:1})
  },10000)

  it('enforces a host deadline while synchronous WASM ignores messages',async()=>{
    let started=false
    const lane=new BrepDiagnosticExecutor(()=>hangingWasmPort(()=>{started=true}),{deadlineMs:3000})
    await expect(lane.evaluate(source,policy)).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_DEADLINE'})
    expect(started).toBe(true)
    expect(lane.snapshot()).toMatchObject({activeWorkerEpoch:null,workersTerminated:1})
  },10000)

  it('handles startup timeout, synchronous browser termination and cancellation during Node join',async()=>{
    const never=new FakePort(), startup=new BrepDiagnosticExecutor(()=>never,{startupTimeoutMs:10})
    await expect(startup.evaluate(source,policy)).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_STARTUP'})
    expect(never.terminated).toBe(1)
    const controller=new AbortController(), port=new FakePort()
    port.onTerminate=async()=>{controller.abort()}
    port.onRequest=r=>{port.started(); port.callbacks.message({...diagnosticEnvelope(r),status:'failed',error:{name:'Error',code:'REMOTE',message:'refused'}})}
    await expect(new BrepDiagnosticExecutor(()=>port).evaluate(source,policy,{signal:controller.signal})).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_CANCELLED'})
  })

  it('quarantines an unjoined realm and never publishes or starts a replacement',async()=>{
    const port=new FakePort(); port.onTerminate=()=>new Promise<void>(()=>{})
    const lane=new BrepDiagnosticExecutor(()=>port,{startupTimeoutMs:5,joinTimeoutMs:5})
    await expect(lane.evaluate(source,policy)).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_JOIN'})
    expect(port.unreferenced).toBe(true)
    expect(lane.snapshot()).toMatchObject({quarantined:true,workersStarted:1,workersTerminated:0})
    await expect(lane.evaluate(source,policy)).rejects.toMatchObject({code:'E_BREP_DIAGNOSTIC_QUARANTINED'})
  })
})
