import {createSourceBodyRecord} from '../src/services/sourceBodyArchive'
import {DirectHistory,emptyDirectDocument,parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {afterEach,expect,it} from 'vitest'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {gunzipSync} from 'node:zlib'
import {Worker} from 'node:worker_threads'
import {MainSolidWorkerClient,type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import type {MainSolidRequest} from '../src/services/mainSolidProtocol'
import {sourceBodyExpectation,validSourceBody,type SourceBodyOptions,type SourceBodyResult} from '../src/services/sourceBody'
const request=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-body-request.json.gz',import.meta.url))).toString())
const options=():SourceBodyOptions=>{const {op:_op,...value}=structuredClone(request);return value}
class Port implements MainSolidPort {
 onmessage:MainSolidPort['onmessage']=null;onerror:MainSolidPort['onerror']=null;onmessageerror:MainSolidPort['onmessageerror']=null
 requests:MainSolidRequest[]=[];terminated=false
 postMessage(request:MainSolidRequest){this.requests.push(structuredClone(request))}
 terminate(){this.terminated=true}
}
const timings:unknown[]=[]
class NodePort implements MainSolidPort {
 onmessage:MainSolidPort['onmessage']=null;onerror:MainSolidPort['onerror']=null;onmessageerror:MainSolidPort['onmessageerror']=null
 constructor(readonly worker:Worker){
  worker.on('message',data=>{if(data.kind==='sourceBodyRestore'&&data.timing)timings.push({id:data.id,ok:data.ok,admitted:data.result?.admitted,...data.timing});this.onmessage?.({data} as MessageEvent)})
  worker.on('error',e=>this.onerror?.({message:e.message} as ErrorEvent))
  worker.on('messageerror',()=>this.onmessageerror?.({} as MessageEvent))
 }
 postMessage(value:MainSolidRequest){this.worker.postMessage({...value,traceTiming:true})}
 terminate(){void this.worker.terminate()}
}
const clients:MainSolidWorkerClient[]=[],workers:Worker[]=[]
afterEach(async()=>{clients.splice(0).forEach(c=>c.dispose());await Promise.all(workers.splice(0).map(w=>w.terminate()))})
function realClient(){
 const client=new MainSolidWorkerClient(()=>{
  const worker=new Worker(new URL('./fixtures/web-worker-node-harness.mjs',import.meta.url),{
   workerData:{entryUrl:new URL('../src/workers/mainSolid.worker.ts',import.meta.url).href,announceReady:false}})
  workers.push(worker);return new NodePort(worker)
 });clients.push(client);return client
}
const denied:SourceBodyResult={admitted:false,sourceBody:null,edges:[],diagnostics:{reason:'source-volume-initial-work-limit',incidence:{},embedding:{},volume:{}}}
function reply(port:Port,result:SourceBodyResult){port.onmessage?.({data:{version:1,id:port.requests.at(-1)!.id,kind:'sourceBodyRestore',ok:true,result}} as MessageEvent)}
it('restores original source definitions through the real WASM CAD worker',async()=>{
 const o={...options(),displaySegments:8,faceDisplay:{divisions:4,toleranceUv:1e-8,domainCellsPerFace:10000}},before=JSON.stringify(o),client=realClient()
 const start=performance.now()
 const r=await client.run({kind:'sourceBodyRestore',options:o})
 const restoreElapsedMs=performance.now()-start
 expect(r.admitted).toBe(true);expect(validSourceBody(sourceBodyExpectation(o),r)).toBe(true)
 // Transport-only fixtures: the packaged WASM does not yet expose seam qualification.
 const seamRequest={edge:0,limits:{maxSineSquared:1e-6,cells:100,curveSpans:100,normalSpans:100}}
 const seamOptions={...o,seamQualification:seamRequest},seamExpected=sourceBodyExpectation(seamOptions)
 const checked:SourceBodyResult={...r,seamQualification:{request:seamRequest,qualified:false,sineSquaredBounds:null,
  reason:'source-seam-endpoint-normal-unresolved',cells:1,curveSpans:2,normalSpans:2,acceptedCells:0,uncertainCanonical:[0,0]}}
 expect(validSourceBody(seamExpected,checked)).toBe(true)
 expect(validSourceBody(sourceBodyExpectation(o),checked)).toBe(false)
 expect(validSourceBody(seamExpected,r)).toBe(false)
 for(const patch of [{request:{...seamRequest,edge:1}},{cells:101},{uncertainCanonical:[-1,0]},
  {qualified:true},{sineSquaredBounds:[0,1e-7]},{reason:'source-seam-tangent-planes-qualified'}]){
  expect(validSourceBody(seamExpected,{...checked,seamQualification:{...checked.seamQualification,...patch}})).toBe(false)
 }
 const qualified={...checked,seamQualification:{...checked.seamQualification!,qualified:true,
  reason:'source-seam-tangent-planes-qualified',sineSquaredBounds:[0,1e-7],uncertainCanonical:null}}
 expect(validSourceBody(seamExpected,qualified)).toBe(true)
 expect(validSourceBody(seamExpected,{...qualified,seamQualification:{...qualified.seamQualification,sineSquaredBounds:[0,1e-5]}})).toBe(false)
 expect(r.sourceBody!.shell).toEqual(o.definition.shell)
 expect(r.edges.map(e=>e.definition)).toEqual(o.definition.shell.pairs.map(p=>p.edge))
 expect(r.edges.every(edge=>edge.displaySegments?.length===8)).toBe(true)
 expect(r.displayFaces).toHaveLength(r.faceCount!)
 expect(r.displayFaces!.every(face=>face.tiles.length===16&&face.unresolved.length===0&&face.unresolvedBoxes.length===0)).toBe(true)
 const incomplete=structuredClone(r);incomplete.displayFaces![0]!.tiles.pop()
 expect(validSourceBody(sourceBodyExpectation(o),incomplete)).toBe(false)

 const broken=structuredClone(r);broken.edges[0]!.displaySegments![0]![0][0]+=1
 expect(validSourceBody(sourceBodyExpectation(o),broken)).toBe(false)
 const anchors=new Map<number,number[]>()
 for(const edge of r.edges)for(const end of [0,1] as const){
  const point=edge.displaySegments![end===0?0:7]![1][end],id=edge.vertices[end]
  if(anchors.has(id))expect(point).toEqual(anchors.get(id))
  anchors.set(id,point)
 }
 const falseIdentity=structuredClone(r)
 expect(falseIdentity.edges[0]!.vertices[0]).not.toBe(falseIdentity.edges[0]!.vertices[1])
 falseIdentity.edges[0]!.vertices[0]=falseIdentity.edges[0]!.vertices[1]
 expect(validSourceBody(sourceBodyExpectation(o),falseIdentity)).toBe(false)
 const disconnected=structuredClone(r)
 disconnected.edges[0]!.displaySegments![0]![1][0][0]+=0.001
 expect(validSourceBody(sourceBodyExpectation(o),disconnected)).toBe(false)
 const unbounded=structuredClone(r)
 unbounded.edges[0]!.displaySegments![0]![2][0]=[1e6,1e6]
 expect(validSourceBody(sourceBodyExpectation(o),unbounded)).toBe(false)
 const missing=structuredClone(r);delete missing.edges[0]!.displaySegments
 expect(validSourceBody(sourceBodyExpectation(o),missing)).toBe(false)
 expect(()=>sourceBodyExpectation({...o,displaySegments:4097})).toThrow()

 const oracle=2*Math.PI/3;expect(r.volume![0]).toBeLessThanOrEqual(oracle);expect(r.volume![1]).toBeGreaterThanOrEqual(oracle)
 const changed=structuredClone(r);changed.edges[0]!.definition={changed:true}
 expect(validSourceBody(sourceBodyExpectation(o),changed)).toBe(false)
 const signedZero=structuredClone(r),points=(signedZero.edges[0]!.definition as {world:{controlPoints:number[][]}}).world.controlPoints
 let changedZero=false
 for(const point of points){const axis=point.findIndex(n=>Object.is(n,0));if(axis>=0){point[axis]=-0;changedZero=true;break}}
 expect(changedZero).toBe(true);expect(validSourceBody(sourceBodyExpectation(o),signedZero)).toBe(false)
 const low=options();low.limits.volume.cells=1
 const refusal=await client.run({kind:'sourceBodyRestore',options:low})
 expect(refusal.admitted).toBe(false);expect(refusal.sourceBody).toBeNull();expect(refusal.edges).toEqual([])
 expect(JSON.stringify(o)).toBe(before)
 expect(timings).toHaveLength(2)
 if(process.env.CAD_SOURCE_BODY_WORKER_REPORT){
  const wasm=readFileSync(new URL('../public/wasm/geometry-kernel.wasm',import.meta.url))
  writeFileSync(process.env.CAD_SOURCE_BODY_WORKER_REPORT,JSON.stringify({schema:'source-body-worker/1',passed:true,wasmBytes:wasm.length,wasmSha256:createHash('sha256').update(wasm).digest('hex'),restoreElapsedMs,timings,volume:r.volume,edgeCount:r.edges.length,faceCount:r.faceCount,scope:'Native-authored equal-radius capped canal through the shipped worker and WASM; executeMs includes the adapter and native gates, and does not isolate Rust or rendered UI latency.'},null,2)+'\n')
 }
},120000)
it('supersedes a source restore and ignores its late response',async()=>{
 const ports:Port[]=[],client=new MainSolidWorkerClient(()=>{const p=new Port();ports.push(p);return p});clients.push(client)
 const first=client.run({kind:'sourceBodyRestore',options:{...options(),seamQualification:{edge:0,limits:{maxSineSquared:1e-6,cells:128,curveSpans:256,normalSpans:256}}}}).catch(e=>e.code)
 const late=ports[0]!.onmessage!
 const second=client.run({kind:'sourceBodyRestore',options:{...options(),seamQualification:{edge:1,limits:{maxSineSquared:1e-6,cells:128,curveSpans:256,normalSpans:256}}}})
 expect(await first).toBe('CAD_CANCELLED');expect(ports[0]!.terminated).toBe(true)
 late({data:{version:1,id:1,kind:'sourceBodyRestore',ok:true,result:denied}} as MessageEvent)
 reply(ports[1]!,denied);expect(await second).toEqual(denied)
})
it('aborts a source restore and permits retry in a fresh worker',async()=>{
 const ports:Port[]=[],client=new MainSolidWorkerClient(()=>{const p=new Port();ports.push(p);return p});clients.push(client)
 const controller=new AbortController(),first=client.run({kind:'sourceBodyRestore',options:options()},{signal:controller.signal}).catch(e=>e.code)
 controller.abort();expect(await first).toBe('CAD_CANCELLED');expect(ports[0]!.terminated).toBe(true)
 const retried=client.run({kind:'sourceBodyRestore',options:options()});reply(ports[1]!,denied);expect(await retried).toEqual(denied)
})

it('loads a source archive through the real document worker and rejects saved false admission',async()=>{
 const record=createSourceBodyRecord('source-archive','Source archive',options())
 const document={...emptyDirectDocument(),sourceBodies:[record]},client=realClient()
 const loaded=await client.run({kind:'restoreDocument',text:serializeDirectDocument(document)})
 expect(loaded.sourceBodies).toEqual([record])
 const low=options();low.limits.volume.cells=1
 const invalid={...emptyDirectDocument(),sourceBodies:[{...createSourceBodyRecord('bad-source','Bad source',low),admitted:true}]}
 await expect(client.run({kind:'restoreDocument',text:serializeDirectDocument(invalid)})).rejects.toMatchObject({code:'CAD_SOURCE_BODY_RESTORE',message:expect.stringContaining('bad-source')})
},30000)

it('discards a late inverse proof after changing only the shear recipe',async()=>{
 const ports:Port[]=[],client=new MainSolidWorkerClient(()=>{const port=new Port();ports.push(port);return port});clients.push(client)
 const a=options();a.definition.inverseShear={axes:[0,2],coefficient:0.25}
 const first=client.run({kind:'sourceBodyRestore',options:a}).catch(error=>error.code)
 const late=ports[0]!.onmessage!,id=ports[0]!.requests[0]!.id
 const b=structuredClone(a);b.definition.inverseShear!.coefficient=0.5
 const second=client.run({kind:'sourceBodyRestore',options:b})
 expect(await first).toBe('CAD_CANCELLED');expect(ports[0]!.terminated).toBe(true)
 expect(ports[0]!.requests[0]).toMatchObject({job:{options:{definition:{inverseShear:{coefficient:0.25}}}}})
 expect(ports[1]!.requests[0]).toMatchObject({job:{options:{definition:{inverseShear:{coefficient:0.5}}}}})
 late({data:{version:1,id,kind:'sourceBodyRestore',ok:true,result:denied}} as MessageEvent)
 reply(ports[1]!,denied);expect(await second).toEqual(denied)
})

it('binds inverse shear axes and coefficient into the source request identity',()=>{
 const o=options();o.definition.inverseShear={axes:[0,2],coefficient:0.25}
 const expected=sourceBodyExpectation(o)
 const changed=structuredClone(o);changed.definition.inverseShear!.coefficient=0.5
 expect(sourceBodyExpectation(changed).definition).not.toEqual(expected.definition)
 changed.definition.inverseShear!.axes=[1,2]
 expect(sourceBodyExpectation(changed).definition).not.toEqual(expected.definition)
 changed.definition.inverseShear!.axes=[0,0]
 expect(()=>sourceBodyExpectation(changed)).toThrow('Invalid inverse shear recipe')
 changed.definition.inverseShear!.axes=[0,2];changed.definition.inverseShear!.coefficient=NaN
 expect(()=>sourceBodyExpectation(changed)).toThrow('Invalid inverse shear recipe')
})

it.each([
 ['source-irrational-body-request','Irrational root tetrahedron','Irrational split on a straight shared carrier of a planar tetrahedral Body; no geometric editing or closed mesh proof.','CAD_IRRATIONAL_BODY_WORKER_REPORT','CAD_IRRATIONAL_BODY_DOCUMENT_OUTPUT'],
 ['source-curved-shear-body-request','Curved root tetrahedron','Curved polynomial shear Body with original irrational root restrictions and fresh inverse contact proof; no geometric editing or closed mesh proof.','CAD_CURVED_SHEAR_BODY_WORKER_REPORT','CAD_CURVED_SHEAR_BODY_DOCUMENT_OUTPUT'],
])('restores %s through WASM, preview and archive history',async(fixture,name,scope,reportEnv,documentEnv)=>{
 const request=JSON.parse(gunzipSync(readFileSync(new URL(`../docs/qualification/rolling-ball-offset-foundation-2026-10-05/${fixture}.json.gz`,import.meta.url))).toString())
 const {op:_op,...base}=request as SourceBodyOptions & {op:string}
 const options={...base,displaySegments:8,faceDisplay:{divisions:8,toleranceUv:1e-8,domainCellsPerFace:10000}},client=realClient()
 const started=performance.now(),r=await client.run({kind:'sourceBodyRestore',options}),restoreElapsedMs=performance.now()-started
 expect(r.admitted).toBe(true);expect(validSourceBody(sourceBodyExpectation(options),r)).toBe(true)
 if(base.definition.inverseShear){
  expect(r.sourceBody!.inverseShear).toEqual(base.definition.inverseShear)
  const changed=structuredClone(r);changed.sourceBody!.inverseShear!.coefficient=0.5
  expect(validSourceBody(sourceBodyExpectation(options),changed)).toBe(false)
  const wrong=structuredClone(options);wrong.definition.inverseShear!.coefficient=0.5
  const refusal=await client.run({kind:'sourceBodyRestore',options:wrong})
  expect(refusal.admitted).toBe(false);expect(validSourceBody(sourceBodyExpectation(wrong),refusal)).toBe(true)
 }
 expect(r.edges).toHaveLength(7);expect(r.faceCount).toBe(4)
 expect(r.volume![0]).toBeLessThanOrEqual(1/6);expect(r.volume![1]).toBeGreaterThanOrEqual(1/6)
 const ends=new Map<number,[number,0|1][]>()
 for(const edge of r.edges)for(const end of [0,1] as const){const id=edge.vertices[end];ends.set(id,[...(ends.get(id)??[]),[edge.index,end]])}
 expect(ends.size).toBe(5)
 const split=[...ends.values()].filter(list=>list.length===2);expect(split).toHaveLength(1)
 let anchor:number[]|undefined
 for(const [index,end] of split[0]!){
  const edge=r.edges[index]!,range=edge.parameterBounds[end]
  expect(range[0]).toBeGreaterThan(0);expect(range[1]).toBeLessThan(1);expect(range[0]).toBeLessThan(range[1])
  expect(range[0]).toBeLessThanOrEqual(Math.SQRT1_2);expect(range[1]).toBeGreaterThanOrEqual(Math.SQRT1_2)
  const point=edge.displaySegments![end===0?0:7]![1][end]
  if(anchor)expect(point).toEqual(anchor);anchor=point
 }
 expect(r.displayFaces!.some(face=>face.tiles.length>0)).toBe(true)
 expect(r.displayFaces!.some(face=>face.unresolved.length>0)).toBe(true)
 const record=createSourceBodyRecord('source-irrational-root',name,base)
 const document={...emptyDirectDocument(),sourceBodies:[record]}
 const loaded=await client.run({kind:'restoreDocument',text:serializeDirectDocument(document)})
 expect(loaded.sourceBodies).toEqual([record])
 await warmGeometryKernel()
 const history=new DirectHistory(document),renamed=history.document
 renamed.sourceBodies![0]!.name='Renamed irrational root';history.commit(renamed)
 expect(history.undo().sourceBodies).toEqual([record])
 expect(history.redo().sourceBodies![0]!.source).toEqual(record.source)
 expect(parseDirectDocument(serializeDirectDocument(history.document)).sourceBodies![0]!.source).toEqual(record.source)
 const documentOutput=process.env[documentEnv],reportOutput=process.env[reportEnv]
 if(documentOutput)writeFileSync(documentOutput,serializeDirectDocument(document))
 if(reportOutput)writeFileSync(reportOutput,JSON.stringify({passed:true,wasmSha256:createHash('sha256').update(readFileSync(new URL('../public/wasm/geometry-kernel.wasm',import.meta.url))).digest('hex'),restoreElapsedMs,scenarioElapsedMs:performance.now()-started,edgeCount:r.edges.length,faceCount:r.faceCount,vertexCount:ends.size,volume:r.volume,rootEnds:split[0],displayUnresolved:r.displayFaces!.reduce((n,f)=>n+f.unresolved.length,0),metadataUndoRedo:true,scope},null,2)+'\n')
},120000)

it('exchanges original curved root carriers through real WASM and worker STEP gates',async()=>{
 const fixture=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-curved-shear-body-request.json.gz',import.meta.url))).toString())
 const {op:_op,...source}=fixture
 const o:SourceBodyOptions={...source,displaySegments:0,faceDisplay:undefined,stepExchange:{toleranceMm:1e-7,trimWork:100000,
  limits:{rootChecks:1000,mappingCells:10000,replayMappingPerUse:10000,exactWork:100_000_000,driverCells:10000,spans:14,endpoints:14}}}
 const before=JSON.stringify(o),client=realClient(),start=performance.now()
 const r=await client.run({kind:'sourceBodyRestore',options:o})
 expect(r.admitted).toBe(true);expect(validSourceBody(sourceBodyExpectation(o),r)).toBe(true)
 const step=r.stepExchange
 expect(step?.prepared).toBe(true)
 if(!step?.prepared)throw new Error('Missing native STEP candidate')
 expect([step.vertices,step.edges,step.faces]).toEqual([5,7,4])
 expect(step.endpointErrorUpper).toBeLessThanOrEqual(1e-7)
 expect(step.text).toContain('TRIMMED_CURVE');expect(step.text).toContain('MANIFOLD_SOLID_BREP')
 if(process.env.CAD_SOURCE_WORKER_STEP_OUTPUT)writeFileSync(process.env.CAD_SOURCE_WORKER_STEP_OUTPUT,step.text)
 const wrong=structuredClone(r);wrong.stepExchange!.request.toleranceMm=1e-5
 expect(validSourceBody(sourceBodyExpectation(o),wrong)).toBe(false)
 expect(validSourceBody(sourceBodyExpectation({...o,stepExchange:undefined}),r)).toBe(false)
 const missing=structuredClone(r);delete missing.stepExchange
 expect(validSourceBody(sourceBodyExpectation(o),missing)).toBe(false)
 const low=structuredClone(o);low.stepExchange!.limits.exactWork=1
 const refusal=await client.run({kind:'sourceBodyRestore',options:low})
 expect(refusal.admitted).toBe(true);expect(refusal.stepExchange?.prepared).toBe(false)
 expect(validSourceBody(sourceBodyExpectation(low),refusal)).toBe(true)
 expect(JSON.stringify(o)).toBe(before)
 if(process.env.CAD_SOURCE_WORKER_STEP_REPORT){
  const wasm=readFileSync(new URL('../public/wasm/geometry-kernel.wasm',import.meta.url))
  writeFileSync(process.env.CAD_SOURCE_WORKER_STEP_REPORT,JSON.stringify({passed:true,wasmBytes:wasm.length,wasmSha256:createHash('sha256').update(wasm).digest('hex'),scenarioElapsedMs:performance.now()-start,endpointErrorUpper:step.endpointErrorUpper,counts:{vertices:step.vertices,edges:step.edges,faces:step.faces},scope:'Fresh native Body restore and original carrier STEP through real worker; one quadratic-shear control body, no general editing or UI export admission.'},null,2)+'\n')
 }
},120000)

it('cancels a changed STEP request and rejects the late worker response',async()=>{
 const ports:Port[]=[],client=new MainSolidWorkerClient(()=>{const p=new Port();ports.push(p);return p});clients.push(client)
 const o:SourceBodyOptions={...options(),stepExchange:{toleranceMm:1e-7,trimWork:100000,
  limits:{rootChecks:1000,mappingCells:10000,replayMappingPerUse:10000,exactWork:100_000_000,driverCells:10000,spans:1000,endpoints:1000}}}
 const first=client.run({kind:'sourceBodyRestore',options:o}).catch(e=>e.code),late=ports[0]!.onmessage!
 const changed=structuredClone(o);changed.stepExchange!.toleranceMm=1e-6
 const second=client.run({kind:'sourceBodyRestore',options:changed})
 expect(await first).toBe('CAD_CANCELLED');expect(ports[0]!.terminated).toBe(true)
 changed.stepExchange!.toleranceMm=1
 expect((ports[1]!.requests[0]!.job as {options:SourceBodyOptions}).options.stepExchange!.toleranceMm).toBe(1e-6)
 late({data:{version:1,id:ports[0]!.requests[0]!.id,kind:'sourceBodyRestore',ok:true,result:denied}} as MessageEvent)
 reply(ports[1]!,denied);expect(await second).toEqual(denied)
})

it('qualifies a selected original seam through the rebuilt WASM worker',async()=>{
 const o:SourceBodyOptions={...options(),displaySegments:0,faceDisplay:undefined,stepExchange:undefined,
  seamQualification:{edge:0,limits:{maxSineSquared:1e-6,cells:128,curveSpans:256,normalSpans:256}}}
 const r=await realClient().run({kind:'sourceBodyRestore',options:o})
 expect(r.admitted).toBe(true)
 expect(validSourceBody(sourceBodyExpectation(o),r)).toBe(true)
 expect(r.seamQualification?.request).toEqual(o.seamQualification)
 expect(r.seamQualification?.reason).toMatch(/^source-seam-/)
 expect(r.seamQualification?.cells).toBeLessThanOrEqual(128)
},60000)

it('qualifies the annular middle rail and localizes its collapsed endpoint through WASM',async()=>{
 const request=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-annular-body-request.json.gz',import.meta.url))).toString())
 const {op:_op,...base}=request,client=realClient()
 for(const edge of [6,0]){
  const o:SourceBodyOptions={...base,seamQualification:{edge,limits:{maxSineSquared:1e-6,cells:1024,curveSpans:2048,normalSpans:2048}}}
  const r=await client.run({kind:'sourceBodyRestore',options:o})
  expect(r.admitted).toBe(true)
  expect(validSourceBody(sourceBodyExpectation(o),r)).toBe(true)
  expect(r.faceCount).toBe(27);expect(r.poleCount).toBe(2)
  expect(r.seamQualification?.qualified).toBe(edge===6)
  if(edge===0){
   expect(r.seamQualification?.reason).toBe('source-seam-endpoint-normal-unresolved')
   expect(r.seamQualification?.uncertainCanonical).toEqual([0,0])
   expect(r.seamQualification?.cells).toBe(1)
  }
 }
},240000)

it('certifies automatically searched annular wall thickness through real WASM worker',async()=>{
 const request=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-annular-body-request.json.gz',import.meta.url))).toString())
 const {op:_op,...base}=request
 const client=realClient(),started=performance.now(),results=[]
 for(const gapCells of [1000,1]){
  const o:SourceBodyOptions={...base,displaySegments:0,faceDisplay:undefined,wallQualification:{
   groups:[[4,9,14,18,22,26],[1,6,11,15,19,23]],minimumMm:5.99,toleranceMm:0.02,toleranceUv:1e-7,
   grid:3,maxAttempts:54,limits:{gapCells,gapSpans:2000,cells:10000,domainCells:10000,normalSpans:1000,maxSineSquared:1e-6}}}
  const before=JSON.stringify(o),r=await client.run({kind:'sourceBodyRestore',options:o})
  expect(JSON.stringify(o)).toBe(before);expect(r.admitted).toBe(true)
  expect(validSourceBody(sourceBodyExpectation(o),r)).toBe(true)
  const wall=r.wallQualification!;expect(wall.request).toEqual(o.wallQualification)
  expect(wall.search.attempts).toBe(54);expect(wall.search.refused).toBeLessThan(54)
  if(gapCells===1000){
   expect(wall.qualified).toBe(true);expect(wall.converged).toBe(true)
   expect(wall.intervalMm![0]).toBeLessThanOrEqual(6);expect(wall.intervalMm![1]).toBeGreaterThanOrEqual(6)
   expect(wall.intervalMm![1]-wall.intervalMm![0]).toBeLessThan(1e-5)
  }else{
   expect(wall.qualified).toBe(false);expect(wall.converged).toBe(false);expect(wall.intervalMm).toBeNull()
   expect(wall.reason).toBe('source-wall-clearance-unproven')
  }
  const changed=structuredClone(r);changed.wallQualification!.request.groups[0][0]=3
  expect(validSourceBody(sourceBodyExpectation(o),changed)).toBe(false)
  results.push(wall)
 }
 if(process.env.CAD_SOURCE_WALL_WORKER_REPORT)writeFileSync(process.env.CAD_SOURCE_WALL_WORKER_REPORT,JSON.stringify({
  wasmSha256:createHash('sha256').update(readFileSync(new URL('../public/wasm/geometry-kernel.wasm',import.meta.url))).digest('hex'),
  elapsedMs:performance.now()-started,results,scope:'Selected annular flat face groups only; no full body wall coverage or UI proof.'},null,2)+'\n')
},120000)

it('discards late wall results when opposing groups change and permits retry',async()=>{
 const ports:Port[]=[],client=new MainSolidWorkerClient(()=>{const p=new Port();ports.push(p);return p});clients.push(client)
 const wall={groups:[[0],[1]] as [number[],number[]],minimumMm:0.1,toleranceMm:0.02,toleranceUv:1e-7,
  grid:3,maxAttempts:9,limits:{gapCells:1000,gapSpans:2000,cells:10000,domainCells:10000,normalSpans:1000,maxSineSquared:1e-6}}
 const first=client.run({kind:'sourceBodyRestore',options:{...options(),wallQualification:wall}}).catch(e=>e.code)
 const late=ports[0]!.onmessage!,id=ports[0]!.requests[0]!.id
 const second=client.run({kind:'sourceBodyRestore',options:{...options(),wallQualification:{...wall,groups:[[0],[2]]}}})
 expect(await first).toBe('CAD_CANCELLED');expect(ports[0]!.terminated).toBe(true)
 late({data:{version:1,id,kind:'sourceBodyRestore',ok:true,result:denied}} as MessageEvent)
 reply(ports[1]!,denied);expect(await second).toEqual(denied)
 const controller=new AbortController()
 const aborted=client.run({kind:'sourceBodyRestore',options:{...options(),wallQualification:wall}},{signal:controller.signal}).catch(e=>e.code)
 controller.abort();expect(await aborted).toBe('CAD_CANCELLED');expect(ports[1]!.terminated).toBe(true)
 const retry=client.run({kind:'sourceBodyRestore',options:{...options(),wallQualification:wall}})
 reply(ports[2]!,denied);expect(await retry).toEqual(denied)
})
