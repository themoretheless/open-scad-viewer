// Qualification only: authenticate actual Rust ABI execution and prove that
// the UI destroys its Worker. Paused and running CPU-sampling probes have
// separate scopes; neither promises cooperative kernel polling latency.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
let qualifiedGeometryScriptUrl
function exportedFunctionIndex(bytes,name){
 let position=8
 const uint=()=>{let result=0,shift=0;for(let i=0;i<5;i++){const b=bytes[position++];result+=(b&127)*2**shift;if(!(b&128))return result;shift+=7}throw Error('Invalid WASM unsigned integer')}
 const string=()=>{const size=uint(),value=bytes.subarray(position,position+size).toString();position+=size;return value}
 while(position<bytes.length){const section=bytes[position++],size=uint(),end=position+size
  if(section===7){const count=uint();for(let i=0;i<count;i++){const exported=string(),kind=bytes[position++],index=uint();if(exported===name){assert.equal(kind,0);return index}}}
  position=end
 }
 throw Error('Missing geometry ABI export '+name)
}
export async function createNativeCancelProbe(browser,wasm){
 const root=await browser.newBrowserCDPSession(),pending=new Map(),scripts=new Map(),verified=new Map()
 const expectedSha=sha(wasm),requestIndex=exportedFunctionIndex(wasm,'abi_request')
 let sessionId,sequence=0,paused=null,detached=false,profilerArmed=false
 root.on('Target.receivedMessageFromTarget',event=>{
  if(event.sessionId!==sessionId)return
  const message=JSON.parse(event.message)
  if(message.id){const waiter=pending.get(message.id);if(waiter){pending.delete(message.id);clearTimeout(waiter.timer);message.error?waiter.reject(Error(message.error.message)):waiter.resolve(message.result)}}
  if(message.method==='Debugger.scriptParsed')scripts.set(message.params.scriptId,message.params)
  if(message.method==='Debugger.paused')paused=message.params
 })
 root.on('Target.detachedFromTarget',event=>{if(event.sessionId===sessionId)detached=true})
 const send=(method,params={})=>new Promise((resolve,reject)=>{
  const id=++sequence,timer=setTimeout(()=>{pending.delete(id);reject(Error('Native probe CDP timeout: '+method))},10000)
  pending.set(id,{resolve,reject,timer})
  root.send('Target.sendMessageToTarget',{sessionId,message:JSON.stringify({id,method,params})}).catch(error=>{clearTimeout(timer);pending.delete(id);reject(error)})
 })
 const targets=async()=> (await root.send('Target.getTargets')).targetInfos.filter(t=>t.type==='worker'&&t.url.includes('geometry.worker-'))
 const waitUntil=async(predicate,label,timeout=10000)=>{
  const until=Date.now()+timeout
  while(Date.now()<until){if(await predicate())return;await new Promise(resolve=>setTimeout(resolve,5))}
  throw Error('Native probe timeout: '+label)
 }
 return {
  async attachExisting(){const list=await targets();assert.equal(list.length,1,'Native build probe requires one isolated geometry Worker');await this.attach(list[0])},
  async attach(target){sessionId=(await root.send('Target.attachToTarget',{targetId:target.targetId,flatten:false})).sessionId;detached=false;paused=null;scripts.clear();verified.clear();await send('Debugger.enable');return target},
  async targetIds(){return new Set((await targets()).map(t=>t.targetId))},
  async attachNew(before){let target;await waitUntil(async()=>{target=(await targets()).find(t=>!before.has(t.targetId));return Boolean(target)},'new exact-solid Worker');await this.attach(target);return target},
  async pauseGeometryRequest(){
   // Cold base85/brotli/language initialization must run without repeated
   // debugger interruption. The URL only schedules observation; acceptance
   // still requires the complete geometry module SHA and active ABI frame.
   if(qualifiedGeometryScriptUrl){
    await waitUntil(()=>detached||[...scripts.values()].some(s=>s.url===qualifiedGeometryScriptUrl),
     'current geometry module initialization',60000)
    assert.ok(!detached,'Worker ended before the geometry module was observed')
   }
   const observations=[]
   for(let attempt=0;attempt<500;attempt++){
    paused=null;await send('Debugger.pause');await waitUntil(()=>paused||detached,'Rust/JS pause');assert.ok(!detached,'Worker finished before a native activation was observed')
    observations.push({top:paused.callFrames[0]?.functionName,scripts:[...scripts.values()].filter(s=>s.scriptLanguage==='WebAssembly').map(s=>s.url),verifiedGeometry:[...verified.values()].includes(expectedSha)})
    for(const frame of paused.callFrames){
     const script=scripts.get(frame.location.scriptId)
     if(script?.scriptLanguage!=='WebAssembly'&&!script?.url?.startsWith('wasm:'))continue
     let fingerprint=verified.get(script.scriptId)
     if(!fingerprint){const response=await send('Debugger.getScriptSource',{scriptId:script.scriptId});assert.ok(response.bytecode,'WASM bytecode must be available');fingerprint=sha(Buffer.from(response.bytecode,'base64'));verified.set(script.scriptId,fingerprint)}
     if(fingerprint!==expectedSha)continue
     const requestFrame=paused.callFrames.find(f=>f.location.scriptId===script.scriptId&&
      (f.functionName==='abi_request'||f.functionName==='$abi_request'||f.functionName===`$func${requestIndex}`||f.functionName===`wasm-function[${requestIndex}]`))
     if(requestFrame){qualifiedGeometryScriptUrl=script.url;return {
      geometryWasmSha256:fingerprint,abiRequestFunctionIndex:requestIndex,wasmScriptUrl:script.url,
      suspension:'Debugger paused an executing geometry abi_request',
      frames:paused.callFrames.map(f=>({functionName:f.functionName,location:f.location,url:f.url}))}}

    }
    await send('Debugger.resume');await new Promise(resolve=>setTimeout(resolve,5))
   }
   throw Error('No live geometry abi_request activation observed; cancellation is unqualified. Samples: '+JSON.stringify(observations.slice(-10)))
  },
  async armRunningGeometry(){
   assert.ok(!profilerArmed,'CPU profiler is already armed')
   assert.equal(paused,null,'Running cancellation must not pause the Worker')
   await send('Profiler.enable')
   await send('Profiler.setSamplingInterval',{interval:1000})
   await send('Profiler.start')
   profilerArmed=true
  },
  async observeRunningGeometry(){
   // Sampling observes execution without suspending it. Require a current
   // geometry ABI stack at the tail, not merely earlier initialization work.
   if(!profilerArmed)await this.armRunningGeometry()
   const observations=[]
   for(let attempt=0;attempt<100;attempt++){
    assert.ok(!detached,'Worker ended before running geometry was observed')
    assert.equal(paused,null,'Running cancellation must not pause the Worker')
    if(attempt>0)await send('Profiler.start')
    await new Promise(resolve=>setTimeout(resolve,20))
    const {profile}=await send('Profiler.stop')
    profilerArmed=false
    const profileReceivedAtEpochMs=Date.now()
    let authenticatedAfterProfile=false
    const byId=new Map(profile.nodes.map(node=>[node.id,node])),parents=new Map()
    for(const node of profile.nodes)for(const child of node.children??[])parents.set(child,node.id)
    const geometry=new Set()
    for(const node of profile.nodes){
     const frame=node.callFrame,script=scripts.get(frame.scriptId)
     if(script?.scriptLanguage!=='WebAssembly'&&!script?.url?.startsWith('wasm:'))continue
     let fingerprint=verified.get(script.scriptId)
     if(!fingerprint){const response=await send('Debugger.getScriptSource',{scriptId:script.scriptId});assert.ok(response.bytecode);fingerprint=sha(Buffer.from(response.bytecode,'base64'));verified.set(script.scriptId,fingerprint);authenticatedAfterProfile=true}
     if(fingerprint===expectedSha&&['abi_request','$abi_request',`$func${requestIndex}`,`wasm-function[${requestIndex}]`].includes(frame.functionName))geometry.add(node.id)
    }
    const isGeometry=id=>{while(id!==undefined){if(geometry.has(id))return true;id=parents.get(id)}return false}
    const samples=profile.samples??[],deltas=profile.timeDeltas??[]
    const nativeSamples=samples.filter(isGeometry).length
    observations.push({attempt,samples:samples.length,nativeSamples,
     tail:samples.length?byId.get(samples.at(-1))?.callFrame.functionName:null,
     authenticatedGeometryScripts:[...verified.values()].filter(value=>value===expectedSha).length,
     abiRequestFunctionIndex:requestIndex,
     authenticatedFrames:profile.nodes.filter(node=>verified.get(node.callFrame.scriptId)===expectedSha).slice(0,24).map(node=>node.callFrame.functionName),
     wasmFrames:profile.nodes.filter(node=>node.callFrame.url?.startsWith('wasm:')).slice(0,8).map(node=>({name:node.callFrame.functionName,scriptId:node.callFrame.scriptId,url:node.callFrame.url})),
     geometryRoots:geometry.size})
    // Last actual CPU observation must remain inside the authenticated ABI.
    // Idle/finished tails refuse qualification even if earlier samples match.
    // Fetching and hashing a module can outlive a short ABI activation. Once
    // its identity is cached, require a new profile rather than timestamping
    // that older observation as fresh after authentication.
    if(authenticatedAfterProfile)continue
    if(nativeSamples>=5&&samples.length&&isGeometry(samples.at(-1))){
     let lastTime=profile.startTime
     for(const delta of deltas)lastTime+=delta
     if((profile.endTime-lastTime)/1000>5)continue
     return {geometryWasmSha256:expectedSha,abiRequestFunctionIndex:requestIndex,
      observation:'CPU sampling of executing geometry abi_request; no Debugger pause',
      sampleCount:samples.length,nativeSampleCount:nativeSamples,
      profileDurationMs:(profile.endTime-profile.startTime)/1000,
      lastSampleToProfileEndMs:(profile.endTime-lastTime)/1000,
      profileReceivedAtEpochMs,profileProcessingMs:Date.now()-profileReceivedAtEpochMs,
      observedAtEpochMs:Date.now(),debuggerPaused:false}
    }
   }
   throw Error('No running geometry ABI at profile tail; cancellation is unqualified. Observations: '+JSON.stringify({first:observations.slice(0,4),last:observations.slice(-4),maxNativeSamples:Math.max(...observations.map(o=>o.nativeSamples))}))
  },
  async assertDestroyed(){await waitUntil(()=>detached,'UI must destroy the Worker containing the paused Rust activation');assert.ok(detached)},
  async close(){if(!detached&&sessionId){if(paused)await send('Debugger.resume').catch(()=>{});await root.send('Target.detachFromTarget',{sessionId}).catch(()=>{})}for(const waiter of pending.values()){clearTimeout(waiter.timer);waiter.reject(Error('Probe closed'))}pending.clear();await root.detach()},
 }
}
