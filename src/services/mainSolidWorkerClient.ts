import {mainSolidExpectation, mainSolidResult, type MainSolidExpectation, type MainSolidJob, type MainSolidRequest, type MainSolidResponse, type MainSolidResults} from './mainSolidProtocol'
import {decodeMainSolidResult} from './mainSolidWorkerTransport'

export interface MainSolidPort {
  onmessage: ((event:MessageEvent) => void)|null
  onerror: ((event:ErrorEvent) => void)|null
  onmessageerror: ((event:MessageEvent) => void)|null
  postMessage(message:MainSolidRequest):void
  terminate():void
}
export interface MainSolidRunOptions {signal?:AbortSignal; timeoutMs?:number}
export class MainSolidWorkerError extends Error {
  constructor(readonly code:string, message:string, name='MainSolidWorkerError') {super(message);this.name=name}
}
const cancelled=()=>new MainSolidWorkerError('CAD_CANCELLED','Operation cancelled','AbortError')

/** One warm CAD realm; noncooperative calls are cancelled by terminating it. */
export class MainSolidWorkerClient {
  private worker:MainSolidPort|null=null
  private active:{fail:(error:Error, discard:boolean)=>void}|null=null
  private nextId=1
  private closed=false
  constructor(private readonly factory:()=>MainSolidPort) {}

  run<K extends MainSolidJob['kind']>(job:Extract<MainSolidJob,{kind:K}>, options:MainSolidRunOptions={}):Promise<MainSolidResults[K]> {
    if(this.closed) return Promise.reject(new MainSolidWorkerError('CAD_DISPOSED','CAD worker is closed'))
    const signal=options.signal
    if(signal?.aborted) return Promise.reject(cancelled())
    const timeoutMs=options.timeoutMs??120000
    if(!Number.isFinite(timeoutMs)||timeoutMs<1||timeoutMs>120000) return Promise.reject(new RangeError('CAD timeout must be between 1 and 120000 ms'))
    // postMessage snapshots the payload. Retain only response dimensions here,
    // avoiding a second host-side copy of potentially large CAD meshes.
    let expected:MainSolidExpectation
    try {expected=mainSolidExpectation(job)}
    catch {return Promise.reject(new MainSolidWorkerError('CAD_TRANSPORT','Invalid CAD request shape'))}
    this.cancel()
    let worker:MainSolidPort
    try {worker=this.worker??=this.factory()}
    catch {return Promise.reject(new MainSolidWorkerError('CAD_STARTUP','Could not start CAD worker'))}
    const id=this.nextId++
    return new Promise((resolve,reject)=>{
      let settled=false
      let timer:ReturnType<typeof setTimeout>|undefined
      const finish=(error?:Error,result?:MainSolidResults[K],discard=false)=>{
        if(settled)return
        settled=true;clearTimeout(timer)
        signal?.removeEventListener('abort',abort)
        worker.onmessage=null;worker.onerror=null;worker.onmessageerror=null
        if(this.active?.fail===fail)this.active=null
        if(discard){if(this.worker===worker)this.worker=null;worker.terminate()}
        error?reject(error):resolve(result!)
      }
      const fail=(error:Error,discard:boolean)=>finish(error,undefined,discard)
      const abort=()=>fail(cancelled(),true)
      worker.onmessage=event=>{
        if(settled)return
        const data=event.data as Partial<MainSolidResponse>|null
        if(data?.version===1 && Number.isSafeInteger(data.id) && data.id!>0 && data.id!<id)return
        if(!data || data.version!==1 || data.id!==id || data.kind!==expected.kind){
          fail(new MainSolidWorkerError('CAD_PROTOCOL','Invalid CAD worker response'),true);return
        }
        if(data.ok===true && 'result' in data) {
          // Unpack transferred typed-array payloads before the plain-array protocol checks.
          const result=decodeMainSolidResult(expected.kind,data.result)
          if(mainSolidResult(expected,result))finish(undefined,result as MainSolidResults[K])
          else fail(new MainSolidWorkerError('CAD_PROTOCOL','Invalid CAD worker response'),true)
        } else if(data.ok===false && 'error' in data && data.error
          && typeof data.error.name==='string' && typeof data.error.message==='string'
          && (data.error.code===undefined || typeof data.error.code==='string')) {
          finish(new MainSolidWorkerError(data.error.code??'CAD_OPERATION',data.error.message,data.error.name))
        } else fail(new MainSolidWorkerError('CAD_PROTOCOL','Invalid CAD worker response'),true)
      }
      worker.onerror=event=>fail(new MainSolidWorkerError('CAD_CRASH',event.message||'CAD worker failed'),true)
      worker.onmessageerror=()=>fail(new MainSolidWorkerError('CAD_TRANSPORT','CAD result could not be received'),true)
      this.active={fail}
      signal?.addEventListener('abort',abort,{once:true})
      timer=setTimeout(()=>fail(new MainSolidWorkerError('CAD_TIMEOUT','CAD operation exceeded its time limit'),true),timeoutMs)
      try {worker.postMessage({version:1,id,job})}
      catch {fail(new MainSolidWorkerError('CAD_TRANSPORT','CAD request could not be sent'),true)}
    })
  }
  cancel(){this.active?.fail(cancelled(),true)}
  dispose(){this.closed=true;this.cancel();this.worker?.terminate();this.worker=null}
}
