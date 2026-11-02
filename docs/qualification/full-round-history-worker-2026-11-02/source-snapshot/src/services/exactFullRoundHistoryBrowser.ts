import type {MeshData} from '../core/mesh'
import type {ExactFullRoundHistoryEdit,ExactHistoryBudget} from './exactFullRoundHistory'
import {isHistoryMesh,isHistoryRequest,type ExactHistoryWorkerRequest} from './exactFullRoundHistoryWorkerProtocol'

/** One browser Worker per edit. Input buffers are cloned, never transferred.
 * This lane does not claim a production engine/provider attestation.
 */
export class ExactFullRoundHistoryBrowser {
  private active=false
  private workersStarted=0
  private workersTerminated=0
  snapshot(){return {active:this.active,workersStarted:this.workersStarted,workersTerminated:this.workersTerminated}}
  async edit(mesh:MeshData,edit:ExactFullRoundHistoryEdit,segments=4,
    options:{signal?:AbortSignal;deadlineMs?:number;budget?:ExactHistoryBudget}={}):Promise<MeshData>{
    if(this.active)throw new Error('Exact history worker is busy')
    const deadline=options.deadlineMs??30000
    if(!Number.isSafeInteger(deadline)||deadline<1||deadline>30000)throw new RangeError('History deadline must be 1..30000 ms')
    if(options.signal?.aborted)throw new Error('Exact history edit cancelled')
    const request:ExactHistoryWorkerRequest=structuredClone({type:'exact-history-edit/1',id:crypto.randomUUID(),mesh,edit,segments,
      ...(options.budget?{budget:options.budget}:{})})
    if(!isHistoryRequest(request))throw new Error('Invalid bounded exact history edit request')
    const start=performance.now()
    this.active=true
    let worker:Worker
    try{worker=new Worker(new URL('../workers/exactFullRoundHistory.worker.ts',import.meta.url),{type:'module'})}
    catch(error){this.active=false;throw error}
    this.workersStarted++
    return new Promise((resolve,reject)=>{
      let settled=false
      let timer:ReturnType<typeof setTimeout>|undefined
      const finish=(error?:Error,result?:MeshData)=>{
        if(settled)return
        settled=true
        clearTimeout(timer)
        worker.removeEventListener('message',message)
        worker.removeEventListener('error',failure)
        worker.removeEventListener('messageerror',failure)
        options.signal?.removeEventListener('abort',abort)
        worker.terminate()
        this.workersTerminated++
        this.active=false
        if(options.signal?.aborted)error=new Error('Exact history edit cancelled')
        else if(performance.now()-start>=deadline)error=new Error('Exact history edit deadline exceeded')
        if(error)reject(error)
        else if(result)resolve(result)
        else reject(new Error('Missing exact history result'))
      }
      const abort=()=>finish(new Error('Exact history edit cancelled'))
      const failure=()=>finish(new Error('Exact history Worker failed'))
      const message=(event:MessageEvent<unknown>)=>{
        const v=event.data as any
        if(!v||v.type!=='exact-history-result/1'||v.id!==request.id||typeof v.ok!=='boolean'){
          finish(new Error('Uncorrelated exact history Worker result'));return
        }
        if(!v.ok){finish(new Error(typeof v.error==='string'?v.error.slice(0,1024):'Native history edit failed'));return}
        if(!isHistoryMesh(v.mesh)||v.mesh.entityId!==request.mesh.entityId
          ||v.mesh.nativeGeometry?.nodeId!==request.mesh.nativeGeometry?.nodeId
          ||v.mesh.transform.some((n:number,i:number)=>n!==request.mesh.transform[i])){
          finish(new Error('Invalid exact history Worker mesh'));return
        }
        finish(undefined,v.mesh)
      }
      worker.addEventListener('message',message)
      worker.addEventListener('error',failure)
      worker.addEventListener('messageerror',failure)
      options.signal?.addEventListener('abort',abort,{once:true})
      if(options.signal?.aborted){abort();return}
      timer=setTimeout(()=>finish(new Error('Exact history edit deadline exceeded')),Math.max(0,deadline-(performance.now()-start)))
      try{worker.postMessage(request)}catch{finish(new Error('Exact history request could not be sent'))}
    })
  }
}
