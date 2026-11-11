import {Worker} from 'node:worker_threads'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {afterEach,expect,it} from 'vitest'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/kernel/mainSolidWorkerClient'
import type {MainSolidRequest,MainSolidJob} from '../src/kernel/mainSolidProtocol'
import identity from '../src/generated/geometry-kernels/identity'
const bytes=readFileSync('public/wasm/geometry-kernel.wasm')
const expectedArtifact={sha256:createHash('sha256').update(bytes).digest('hex'),byteLength:bytes.length}
const workers:Worker[]=[],clients:MainSolidWorkerClient[]=[]
class Port implements MainSolidPort {
 onmessage:MainSolidPort['onmessage']=null;onerror:MainSolidPort['onerror']=null;onmessageerror:MainSolidPort['onmessageerror']=null
 constructor(readonly worker:Worker){
  worker.on('message',data=>{if(data?.__webWorkerHarness==='artifact'){expect(data.identity).toEqual(expectedArtifact);return}this.onmessage?.({data} as MessageEvent)})
  worker.on('error',error=>this.onerror?.({message:error.message} as ErrorEvent))
  worker.on('messageerror',()=>this.onmessageerror?.({} as MessageEvent))
 }
 postMessage(message:MainSolidRequest){this.worker.postMessage(message)}
 terminate(){void this.worker.terminate()}
}
function client(){expect(identity).toEqual(expectedArtifact);const c=new MainSolidWorkerClient(()=>{
 const w=new Worker(new URL('./fixtures/web-worker-node-harness.mjs',import.meta.url),{workerData:{entryUrl:new URL('../src/workers/mainSolid.worker.ts',import.meta.url).href,announceReady:false,expectedArtifact}})
 workers.push(w);return new Port(w)
});clients.push(c);return c}
afterEach(async()=>{clients.splice(0).forEach(c=>c.dispose());await Promise.all(workers.splice(0).map(w=>w.terminate()))})
const artifact=JSON.parse(gunzipSync(readFileSync('tests/fixtures/frozen-graph-body.json.gz')).toString())
function job():Extract<MainSolidJob,{kind:'frozenGraphBodyRestore'}>{return {kind:'frozenGraphBodyRestore',artifact,options:{maxWork:10000000,maxBytes:4194304,maxPairs:100}}}
it('reproves the native exact body and original identities in a separate worker realm',async()=>{
 const c=client(),before=JSON.stringify(artifact),r=await c.run(job())
 expect(r.report.nativeVolumeCertified).toBe(true);expect(r.report.faces).toBe(9);expect(r.report.pairProducts).toBe(36)
 expect(r.artifact.geometry).toEqual(artifact.geometry);expect(r.report.applicationAdmitted).toBe(false)
 expect(JSON.stringify(artifact)).toBe(before)
},60000)
it('cancels a realm, rejects a corrupted identity and accepts a fresh Retry',async()=>{
 const c=client(),pending=c.run(job()),rejected=expect(pending).rejects.toThrow();c.cancel();await rejected
 const bad=structuredClone(artifact);bad.geometry.identityLinks.edges[0].sourceId='changed-id'
 await expect(c.run({...job(),artifact:bad})).rejects.toThrow()
 const r=await c.run(job());expect(r.report.nativeVolumeCertified).toBe(true);expect(r.artifact.geometry).toEqual(artifact.geometry)
 expect(workers.length).toBeGreaterThanOrEqual(2)
},60000)

it('samples every exact edge in the actual worker without altering its artifact',async()=>{
 const c=client(),base=job(),r=await c.run({kind:'frozenGraphBodyDisplay',artifact:base.artifact,segments:8,options:base.options})
 expect(r.display.edges).toHaveLength(r.report.edges)
 expect(r.display.edges.every(e=>e.points.length===9&&e.root===(artifact.geometry.edges[e.edge].root))).toBe(true)
 expect(r.display.polylineDeviationCertified).toBe(false)
 expect(r.artifact.geometry).toEqual(artifact.geometry)
},60000)

it('replays natural source cuts and displays their shared-root children in a separate worker',async()=>{
 const artifact=JSON.parse(gunzipSync(readFileSync('tests/fixtures/frozen-graph-natural-cuts.json.gz')).toString())
 const c=client(),before=JSON.stringify(artifact),base=job()
 const r=await c.run({kind:'frozenGraphBodyDisplay',artifact,segments:8,options:base.options})
 expect(r.artifact.format).toBe('native-convex-graph-volume/2')
 expect(r.report.edges).toBe(39);expect(r.report.openEdges).toBe(0)
 expect(r.report.globalEmbeddingCertified).toBe(true);expect(r.report.absoluteCurvatureUpper).toBeLessThanOrEqual(10)
 expect(r.display.edges).toHaveLength(39);expect(r.artifact.geometry).toEqual(artifact.geometry)
 expect(JSON.stringify(artifact)).toBe(before)
 const bad=structuredClone(artifact);bad.recipe.naturalCuts[1]=bad.recipe.naturalCuts[0]
 await expect(c.run({kind:'frozenGraphBodyRestore',artifact:bad,options:base.options})).rejects.toThrow()
 expect((await c.run({kind:'frozenGraphBodyRestore',artifact,options:base.options})).report.edges).toBe(39)
},60000)
