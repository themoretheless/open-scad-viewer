import { countOperation } from '../dev/operationCounters'
import {isResidentDocumentRef,mainSolidExpectation, mainSolidResult, type MainSolidExpectation, type MainSolidJob, type MainSolidProgress, type MainSolidProgressMessage, type MainSolidRequest, type MainSolidResponse, type MainSolidResults} from './mainSolidProtocol'
import {decodeMainSolidResult} from './mainSolidWorkerTransport'
import {validWorkerErrorField, validWorkerErrorTarget} from '../core/workerHandlerRuntime'
import {applyDirectDocumentDelta,isDirectDocumentDelta} from './directDocumentDelta'
import type {DirectDocument} from '../app/directModeling'

export interface MainSolidPort {
  onmessage: ((event:MessageEvent) => void)|null
  onerror: ((event:ErrorEvent) => void)|null
  onmessageerror: ((event:MessageEvent) => void)|null
  postMessage(message:MainSolidRequest):void
  terminate():void
}
export interface MainSolidTiming {clientElapsedMs:number;warmupMs:number;executeMs:number;prepareMs:number}
export interface MainSolidRunOptions {signal?:AbortSignal | undefined; timeoutMs?:number | undefined; onTiming?:(timing:MainSolidTiming)=>void; onProgress?:(progress:MainSolidProgress)=>void
  /** `sceneEdit`/`pointEdit` only: keep `job.document` resident in the worker. The first request
   * sends it (as `text()` when that returns the document's canonical serialization, which is far
   * cheaper to clone than the object tree); later requests with the same object send only its
   * key, and answers arrive as deltas against it. Unchanged objects of the result are then the
   * base's own objects, so the base must not be mutated in place (published UI documents are not). */
  residentDocument?:{text?:()=>string|undefined}
  /** `boolean`/`minkowski3d` only: answer `result.document` as a delta against `job.document`,
   * rebuilt here from that same object. Same immutability contract as `residentDocument`. */
  documentDelta?:boolean
  /** `restoreDocument` with `job.base` only: the document whose canonical snapshot text is
   * `job.base`; the delta answer is rebuilt from it. Without it `base` is not sent. */
  restoreBase?:DirectDocument}
export class MainSolidWorkerError extends Error {
  /** Inspector field and scene ids carried by a coded refusal across the worker boundary. */
  declare readonly field?:string
  readonly targetIds:readonly string[]
  constructor(readonly code:string, message:string, name='MainSolidWorkerError', mark:{field?:string|undefined;target?:readonly string[]|undefined}={}) {
    super(message);this.name=name
    if(mark.field!==undefined)this.field=mark.field
    this.targetIds=mark.target?[...mark.target]:[]
  }
}
const WIRE_ERROR_KEYS=new Set(['name','message','code','field','target'])
const cancelled=()=>new MainSolidWorkerError('CAD_CANCELLED','Operation cancelled','AbortError')

/** One warm CAD realm; noncooperative calls are cancelled by terminating it. */
export class MainSolidWorkerClient {
  private worker:MainSolidPort|null=null
  private active:{fail:(error:Error, discard:boolean)=>void}|null=null
  private nextId=1
  private closed=false
  private resident:{document:WeakRef<object>;key:string}|null=null
  private readonly residentPrefix=crypto.randomUUID()
  private residentSerial=0
  constructor(private readonly factory:()=>MainSolidPort) {}

  run<K extends MainSolidJob['kind']>(job:Extract<MainSolidJob,{kind:K}>, options:MainSolidRunOptions={}):Promise<MainSolidResults[K]> {
    if(this.closed) return Promise.reject(new MainSolidWorkerError('CAD_DISPOSED','CAD worker is closed'))
    const signal=options.signal
    if(signal?.aborted) return Promise.reject(cancelled())
    const timeoutMs=options.timeoutMs??120000
    if(!Number.isFinite(timeoutMs)||timeoutMs<1||timeoutMs>120000) return Promise.reject(new RangeError('CAD timeout must be between 1 and 120000 ms'))
    const onTiming=options.onTiming,onProgress=options.onProgress,started=onTiming?performance.now():0
    // postMessage snapshots the payload. Retain only response dimensions here,
    // avoiding a second host-side copy of potentially large CAD meshes.
    let expected:MainSolidExpectation
    try {expected=mainSolidExpectation(job)}
    catch {return Promise.reject(new MainSolidWorkerError('CAD_TRANSPORT','Invalid CAD request shape'))}
    this.cancel()
    let worker:MainSolidPort
    try {worker=this.worker??=this.factory()}
    catch {return Promise.reject(new MainSolidWorkerError('CAD_STARTUP','Could not start CAD worker'))}
    // The previous request is settled (or its worker discarded) by now: choose the wire form.
    let wire:MainSolidJob=job,base:DirectDocument|undefined,retainAs:string|undefined
    const drag=job as MainSolidJob
    if(options.documentDelta&&(drag.kind==='boolean'||drag.kind==='minkowski3d')&&drag.document&&typeof drag.document==='object') {
      base=drag.document;wire={...drag,delta:true}
    } else if(drag.kind==='restoreDocument'&&drag.base!==undefined) {
      if(options.restoreBase)base=options.restoreBase
      else {const {base:_base,...full}=drag;wire=full}
    } else if(options.residentDocument&&(drag.kind==='sceneEdit'||drag.kind==='pointEdit')&&typeof drag.document==='object'&&!isResidentDocumentRef(drag.document)) {
      const edit=drag
      base=edit.document as DirectDocument
      const resident=this.resident
      if(resident&&resident.document.deref()===base)wire={...edit,document:{resident:resident.key},delta:true}
      else {
        retainAs=`${this.residentPrefix}:${++this.residentSerial}`
        let text:string|undefined
        try {text=options.residentDocument.text?.()} catch {text=undefined}
        wire={...edit,document:text??base,retainAs,delta:true}
      }
    }
    const id=this.nextId++
    return new Promise((resolve,reject)=>{
      let settled=false
      let timer:ReturnType<typeof setTimeout>|undefined
      const finish=(error?:Error,result?:MainSolidResults[K],discard=false,timing?:MainSolidResponse['timing'])=>{
        if(settled)return
        settled=true;clearTimeout(timer)
        signal?.removeEventListener('abort',abort)
        worker.onmessage=null;worker.onerror=null;worker.onmessageerror=null
        if(this.active?.fail===fail)this.active=null
        if(discard){if(this.worker===worker){this.worker=null;this.resident=null}worker.terminate()}
        // A new base is resident only once the worker answered for it; after a failed first
        // request (or a worker that lost the document) the next request sends the base again.
        else if(retainAs!==undefined)this.resident=error?null:{document:new WeakRef(base!),key:retainAs}
        else if(error instanceof MainSolidWorkerError&&error.code==='CAD_RESIDENT_MISSING')this.resident=null
        error?reject(error):resolve(result!)
        // Observers cannot change command settlement, cancellation or geometry.
        if(onTiming && timing && [timing.warmupMs,timing.executeMs,timing.prepareMs].every(value=>Number.isFinite(value)&&value>=0)) {
          try {onTiming({...timing,clientElapsedMs:performance.now()-started})} catch { /* Optional observation must not fail a CAD command. */ }
        }
      }
      const fail=(error:Error,discard:boolean)=>finish(error,undefined,discard)
      const abort=()=>fail(cancelled(),true)
      worker.onmessage=event=>{
        if(settled)return
        const data=event.data as Partial<MainSolidResponse|MainSolidProgressMessage>|null
        if(data?.version===1 && Number.isSafeInteger(data.id) && data.id!>0 && data.id!<id)return
        if(!data || data.version!==1 || data.id!==id || data.kind!==expected.kind){
          fail(new MainSolidWorkerError('CAD_PROTOCOL','Invalid CAD worker response'),true);return
        }
        if('progress' in data){
          const progress=data.progress
          if(!progress||typeof progress.phase!=='string'||!progress.phase.length||progress.phase.length>64
            ||!Number.isSafeInteger(progress.completed)||!Number.isSafeInteger(progress.total)
            ||progress.completed<0||progress.total<1||progress.completed>progress.total){
            fail(new MainSolidWorkerError('CAD_PROTOCOL','Invalid CAD worker progress'),true);return
          }
          if(onProgress)try{onProgress(progress)}catch{/* Observation cannot change command settlement. */}
          return
        }
        if('ok' in data&&data.ok===true && 'result' in data) {
          // Unpack transferred typed-array payloads before the plain-array protocol checks.
          let result=decodeMainSolidResult(expected.kind,data.result)
          // A malformed delta decodes to undefined and fails the protocol check below.
          if(base!==undefined&&isDirectDocumentDelta(result))result=applyDirectDocumentDelta(base,result)
          else if(base!==undefined&&result&&typeof result==='object'&&isDirectDocumentDelta((result as {document?:unknown}).document)) {
            const document=applyDirectDocumentDelta(base,(result as {document:unknown}).document)
            result=document?{...(result as object),document}:undefined
          }
          if(mainSolidResult(expected,result))finish(undefined,result as MainSolidResults[K],false,data.timing)
          else fail(new MainSolidWorkerError('CAD_PROTOCOL','Invalid CAD worker response'),true)
        } else if('ok' in data&&data.ok===false && 'error' in data && data.error
          && typeof data.error.name==='string' && typeof data.error.message==='string'
          && (data.error.code===undefined || typeof data.error.code==='string')
          && validWorkerErrorField(data.error.field) && validWorkerErrorTarget(data.error.target)
          && Object.keys(data.error).every(key=>WIRE_ERROR_KEYS.has(key))) {
          finish(new MainSolidWorkerError(data.error.code??'CAD_OPERATION',data.error.message,data.error.name,{field:data.error.field,target:data.error.target}),undefined,false,data.timing)
        } else fail(new MainSolidWorkerError('CAD_PROTOCOL','Invalid CAD worker response'),true)
      }
      worker.onerror=event=>fail(new MainSolidWorkerError('CAD_CRASH',event.message||'CAD worker failed'),true)
      worker.onmessageerror=()=>fail(new MainSolidWorkerError('CAD_TRANSPORT','CAD result could not be received'),true)
      this.active={fail}
      signal?.addEventListener('abort',abort,{once:true})
      timer=setTimeout(()=>fail(new MainSolidWorkerError('CAD_TIMEOUT','CAD operation exceeded its time limit'),true),timeoutMs)
      countOperation('solidWorkerRequests')
      try {worker.postMessage({version:1,id,job:wire,...(onTiming?{traceTiming:true}:{})})}
      catch {fail(new MainSolidWorkerError('CAD_TRANSPORT','CAD request could not be sent'),true)}
    })
  }
  cancel(){this.active?.fail(cancelled(),true)}
  dispose(){this.closed=true;this.cancel();this.worker?.terminate();this.worker=null;this.resident=null}
}
