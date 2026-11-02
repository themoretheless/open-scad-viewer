import {MAX_QUADRIC_EXACT_WORK,isOriginalQuadricCarrierReport,type OriginalQuadricCarrierQuery,type OriginalQuadricCarrierReport} from './originalQuadricCarrierProtocol'
import {MAX_PATCH_EXACT_WORK,isOriginalPatchContactsReport,type OriginalPatchContactsQuery,type OriginalPatchContactsReport} from './originalPatchContactsProtocol'
import {isSurfaceIsoTargetReport,type SurfaceIsoTargetReport,type SurfaceIsoTargetQuery} from './surfaceIsoTargetProtocol'
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
  if(!Number.isInteger(maxSpans)||maxSpans<1||maxSpans>100000)return Promise.reject(new Error('Invalid surface audit budget'))
  return this.run({type:'surface-self-audit/1',surface,maxSpans},(r):r is SurfaceSelfIntersectionReport=>isSurfaceAuditReport(r)&&r.maxSpans===maxSpans,options)
 }
 inspectIsoTarget(query:SurfaceIsoTargetQuery,options:{signal?:AbortSignal;deadlineMs?:number}={}):Promise<SurfaceIsoTargetReport>{
  if(!query||![0,1].includes(query.axis)||!Number.isFinite(query.parameter)||query.parameter<=0||query.parameter>=1
   ||!Number.isInteger(query.maxCells)||query.maxCells<1||query.maxCells>100000||!Array.isArray(query.window)||query.window.length!==3
   ||![0,1,2].every(i=>{const d=query.window[i];return Array.isArray(d)&&d.length===2&&d.every(Number.isFinite)&&d[0]<d[1]}))return Promise.reject(new Error('Invalid iso target query'))
  const {axis,parameter,maxCells}=query,window=query.window.map(d=>[...d])
  return this.run({...query,type:'surface-iso-target-window/1'},(r):r is SurfaceIsoTargetReport=>isSurfaceIsoTargetReport(r)&&r.axis===axis&&r.parameter===parameter&&r.maxCells===maxCells&&r.window.every((d,i)=>d[0]===window[i]![0]&&d[1]===window[i]![1]),options)
 }
 inspectOriginalPatchContacts(query:OriginalPatchContactsQuery,options:{signal?:AbortSignal;deadlineMs?:number}={}):Promise<OriginalPatchContactsReport>{
  if(!query||!Number.isFinite(query.tolerance)||query.tolerance<=0||!Number.isInteger(query.maxSpans)||query.maxSpans<1||query.maxSpans>100000
   ||!Number.isInteger(query.maxExactWork)||query.maxExactWork<0||query.maxExactWork>MAX_PATCH_EXACT_WORK)return Promise.reject(new Error('Invalid original patch query'))
  const {maxSpans,maxExactWork}=query
  return this.run({...query,type:'original-patch-contacts/1'},(r):r is OriginalPatchContactsReport=>isOriginalPatchContactsReport(r)&&r.maxSpans===maxSpans&&r.maxExactWork===maxExactWork,options)
 }
 inspectOriginalQuadricCarrierRoots(query:OriginalQuadricCarrierQuery,options:{signal?:AbortSignal;deadlineMs?:number}={}):Promise<OriginalQuadricCarrierReport>{
  if(!query||!Number.isFinite(query.tolerance)||query.tolerance<=0||!Number.isInteger(query.maxSpans)||query.maxSpans<1||query.maxSpans>100000
   ||!Number.isInteger(query.maxExactWork)||query.maxExactWork<0||query.maxExactWork>MAX_QUADRIC_EXACT_WORK
   ||!Array.isArray(query.quadric)||query.quadric.length!==4||![0,1,2,3].every(i=>{const row=query.quadric[i];return Array.isArray(row)&&row.length===4&&[0,1,2,3].every(j=>typeof row[j]==='number'&&Number.isFinite(row[j]))})
   ||!Array.isArray(query.constantProducts)||query.constantProducts.length>16||!Array.from(query.constantProducts).every(p=>Array.isArray(p)&&p.length===2&&[p[0],p[1]].every(v=>typeof v==='number'&&Number.isFinite(v))))return Promise.reject(new Error('Invalid original quadric query'))
  const {maxSpans,maxExactWork}=query
  return this.run({...query,type:'original-quadric-carrier-roots/1'},(r):r is OriginalQuadricCarrierReport=>isOriginalQuadricCarrierReport(r)&&r.maxSpans===maxSpans&&r.maxExactWork===maxExactWork,options)
 }
 private run<T>(request:Record<string,unknown>,validate:(r:unknown)=>r is T,options:{signal?:AbortSignal;deadlineMs?:number}):Promise<T>{
  if(this.active)return Promise.reject(new Error('Surface audit busy'))
  const deadline=options.deadlineMs??30000
  if(!Number.isFinite(deadline)||deadline<1||deadline>120000)return Promise.reject(new Error('Invalid surface audit deadline'))
  if(options.signal?.aborted)return Promise.reject(new Error('Surface audit cancelled'))
  this.active=true
  return new Promise((resolve,reject)=>{
   let worker:Worker|undefined,timer:ReturnType<typeof setTimeout>|undefined,settled=false
   const id=`surface-audit-${++this.sequence}`
   const finish=(error?:Error,result?:T)=>{
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
     if(!validate(reply.report))return finish(new Error('Invalid surface audit report'))
     finish(undefined,reply.report)
    }
    worker.onerror=()=>finish(new Error('Surface audit Worker failed'))
    worker.onmessageerror=()=>finish(new Error('Surface audit message failed'))
    options.signal?.addEventListener('abort',abort,{once:true})
    timer=setTimeout(()=>finish(new Error('Surface audit deadline')),deadline)
    worker.postMessage({...request,id,module:this.module})
   }catch(error){finish(error instanceof Error?error:new Error('Surface audit request failed'))}
  })
 }
}
