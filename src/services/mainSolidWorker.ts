import type {CadOptions} from './cadWorkbench'
import type {MeshData} from '../core/mesh'
import type {PickHit} from './rendererContracts'
import type {DirectDocument} from './directModeling'
import type {MainOperation,MainParameters} from './mainModeling'

// One warm worker serves all main-solid/CAD operations; the geometry kernel is
// initialized once instead of every call paying the WASM inflate+compile.
// A running computation cannot be interrupted in place, so cancellation (or a
// superseding call) terminates the worker and the next operation recreates it.
let shared:Worker|null=null
let current:{reject:(e:Error)=>void}|null=null

export function cancelMainSolid(){
 if(!current)return
 const job=current;current=null
 if(shared){shared.terminate();shared=null}
 job.reject(new DOMException('Operation cancelled','AbortError'))
}

function run<T>(message:unknown,pick:(data:any)=>T):Promise<T>{
 cancelMainSolid()
 return new Promise((resolve,reject)=>{
  if(!shared)shared=new Worker(new URL('../workers/mainSolid.worker.ts',import.meta.url),{type:'module'})
  const worker=shared
  const cleanup=()=>{worker.onmessage=null;worker.onerror=null}
  worker.onmessage=(event:MessageEvent)=>{
   cleanup()
   if(current?.reject===reject)current=null
   event.data.error?reject(Error(event.data.error)):resolve(pick(event.data as never))
  }
  worker.onerror=(event:ErrorEvent)=>{
   // The worker state is unknown; drop it so the next call starts fresh.
   cleanup();if(shared===worker)shared=null;worker.terminate()
   if(current?.reject===reject)current=null
   reject(Error(event.message||'Geometry worker failed'))
  }
  current={reject}
  try{worker.postMessage(message)}catch(e){cleanup();if(current?.reject===reject)current=null;reject(e)}
 })
}

export function computeMainSolid(meshes:MeshData[],selected:number,hit:PickHit|null,operation:MainOperation,parameters:MainParameters):Promise<DirectDocument>{
 return run({meshes,selected,hit,operation,parameters},data=>data.document)
}

export function computeCadOperation(document:DirectDocument,options:CadOptions):Promise<DirectDocument>{
 return run({kind:'cad',document,options},data=>data.document)
}

export function computeCadInspection(bodies:import('./directModeling').DirectBody[]):Promise<import('./cadInspection').CadPairReport[]>{
 return run({kind:'inspect',bodies},data=>data.report)
}
