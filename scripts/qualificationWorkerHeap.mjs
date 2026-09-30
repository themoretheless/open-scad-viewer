/** Diagnostic CDP sessions belong only to the isolated qualification browser. */
export function workerHeapSampler(cdp){
 let nextId=0
 const pending=new Map()
 cdp.on('Target.receivedMessageFromTarget',({sessionId,message})=>{
  const response=JSON.parse(message),key=sessionId+':'+response.id,request=pending.get(key)
  if(!request)return
  pending.delete(key);clearTimeout(request.timer)
  if(response.error)request.reject(Error(response.error.message));else request.resolve(response.result)
 })
 function send(sessionId,method,params={}){
  const id=++nextId,key=sessionId+':'+id
  return new Promise((resolve,reject)=>{
   const timer=setTimeout(()=>{pending.delete(key);reject(Error('Worker heap command timed out: '+method))},5000)
   pending.set(key,{resolve,reject,timer})
   cdp.send('Target.sendMessageToTarget',{sessionId,message:JSON.stringify({id,method,params})}).catch(error=>{
    pending.delete(key);clearTimeout(timer);reject(error)
   })
  })
 }
 return async()=>{
  const {targetInfos}=await cdp.send('Target.getTargets'),samples=[]
  for(const target of targetInfos.filter(t=>t.type==='worker')){
   let sessionId
   try{
    ;({sessionId}=await cdp.send('Target.attachToTarget',{targetId:target.targetId,flatten:false}))
    await send(sessionId,'HeapProfiler.collectGarbage')
    const heap=await send(sessionId,'Runtime.getHeapUsage'),isolate=await send(sessionId,'Runtime.getIsolateId')
    if(typeof isolate.id!=='string'||['usedSize','totalSize'].some(key=>!Number.isFinite(heap[key])||heap[key]<0))throw Error('Invalid worker heap metrics')
    const group='qualification-memory'
    let wasmMemoryBytes
    try{
     const prototype=await send(sessionId,'Runtime.evaluate',{expression:'WebAssembly.Memory.prototype',objectGroup:group})
     if(!prototype.result?.objectId)throw Error('WASM memory prototype unavailable')
     const objects=await send(sessionId,'Runtime.queryObjects',{prototypeObjectId:prototype.result.objectId,objectGroup:group})
     const sizes=await send(sessionId,'Runtime.callFunctionOn',{objectId:objects.objects.objectId,functionDeclaration:'function(){return this.map(memory=>memory.buffer.byteLength)}',returnByValue:true})
     wasmMemoryBytes=sizes.result?.value
     if(!Array.isArray(wasmMemoryBytes)||wasmMemoryBytes.some(n=>!Number.isSafeInteger(n)||n<0))throw Error('Invalid WASM memory sizes')
    }finally{await send(sessionId,'Runtime.releaseObjectGroup',{objectGroup:group})}

    samples.push({targetId:target.targetId,url:target.url,isolateId:isolate.id,status:'measured',wasmMemoryBytes,...heap})
   }catch(error){
    const current=await cdp.send('Target.getTargets')
    if(current.targetInfos.some(t=>t.targetId===target.targetId))throw error
    samples.push({targetId:target.targetId,url:target.url,status:'ended'})
   }finally{if(sessionId)await cdp.send('Target.detachFromTarget',{sessionId}).catch(()=>{})}
  }
  return samples
 }
}
