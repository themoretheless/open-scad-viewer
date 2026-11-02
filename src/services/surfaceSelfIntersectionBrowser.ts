import type {NurbsSurface} from './nurbsSurface'
import {isSurfaceAuditReport,type SurfaceSelfIntersectionReport} from './surfaceSelfIntersectionProtocol'
/** Read-only diagnostics on a caller-owned module; does not register a production provider. */
export class SurfaceSelfIntersectionBrowser {
 private active=false
 private started=0
 private terminated=0
 private sequence=0
 constructor(private readonly module:WebAssembly.Module){
  if(WebAssembly.Module.imports(module).length)throw new Error('Surface audit requires an import-free module')
 }
 snapshot(){return{active:this.active,workersStarted:this.started,workersTerminated:this.terminated}}
 inspect(surface:NurbsSurface,maxSpans:number,options:{signal?:AbortSignal;deadlineMs?:number}={}):Promise<SurfaceSelfIntersectionReport>{
  if(this.active)return Promise.reject(new Error('Surface audit busy'))
  if(!Number.isInteger(maxSpans)||maxSpans<1||maxSpans>100000)return Promise.reject(new Error('Invalid surface audit budget'))
  const deadline=options.deadlineMs??30000
  if(!Number.isFinite(deadline)||deadline<1||deadline>120000)return Promise.reject(new Error('Invalid surface audit deadline'))
  if(options.signal?.aborted)return Promise.reject(new Error('Surface audit cancelled'))
  this.active=true
  return new Promise((resolve,reject)=>{
   let worker:Worker|undefined,timer:ReturnType<typeof setTimeout>|undefined,settled=false
   const id=`surface-audit-${++this.sequence}`
   const finish=(error?:Error,result?:SurfaceSelfIntersectionReport)=>{
    if(settled)return
    settled=true
    if(timer!==undefined)clearTimeout(timer)
    options.signal?.removeEventListener('abort',abort)
    if(worker){worker.terminate();this.terminated++}
    this.active=false
    if(error)reject(error);else resolve(result!)
   }
   const abort=()=>finish(new Error('Surface audit cancelled'))
   try{
    worker=new Worker(new URL('../workers/surfaceSelfIntersection.worker.ts',import.meta.url),{type:'module'})
    this.started++
    worker.onmessage=event=>{
     const reply=event.data
     if(reply?.type!=='surface-self-result/1'||reply.id!==id)return finish(new Error('Invalid surface audit response'))
     if(reply.ok!==true)return finish(new Error(typeof reply.error==='string'?reply.error:'Surface audit refused'))
     if(!isSurfaceAuditReport(reply.report)||reply.report.maxSpans!==maxSpans)return finish(new Error('Invalid surface audit report'))
     finish(undefined,reply.report)
    }
    worker.onerror=()=>finish(new Error('Surface audit Worker failed'))
    worker.onmessageerror=()=>finish(new Error('Surface audit message failed'))
    options.signal?.addEventListener('abort',abort,{once:true})
    timer=setTimeout(()=>finish(new Error('Surface audit deadline')),deadline)
    worker.postMessage({type:'surface-self-audit/1',id,module:this.module,surface,maxSpans})
   }catch(error){finish(error instanceof Error?error:new Error('Surface audit request failed'))}
  })
 }
}
