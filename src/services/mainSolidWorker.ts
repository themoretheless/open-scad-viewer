import type {CadOptions} from './cadWorkbench'
import type {MeshData} from '../core/mesh'
import type {PickHit} from './rendererContracts'
import type {DirectDocument} from './directModeling'
import type {MainOperation,MainParameters} from './mainModeling'
let current:{worker:Worker;reject:(e:Error)=>void}|null=null
export function cancelMainSolid(){if(current){const job=current;current=null;job.worker.terminate();job.reject(new DOMException('Operation cancelled','AbortError'))}}
export function computeMainSolid(meshes:MeshData[],selected:number,hit:PickHit|null,operation:MainOperation,parameters:MainParameters):Promise<DirectDocument>{
 cancelMainSolid()
 return new Promise((resolve,reject)=>{
  const worker=new Worker(new URL('../workers/mainSolid.worker.ts',import.meta.url),{type:'module'})
  current={worker,reject}
  const finish=()=>{worker.terminate();if(current?.worker===worker)current=null}
  worker.onmessage=event=>{finish();event.data.error?reject(Error(event.data.error)):resolve(event.data.document)}
  worker.onerror=event=>{finish();reject(Error(event.message||'Geometry worker failed'))}
  try{worker.postMessage({meshes,selected,hit,operation,parameters})}catch(e){finish();reject(e)}
 })
}

export function computeCadOperation(document:DirectDocument,options:CadOptions):Promise<DirectDocument>{
 cancelMainSolid()
 return new Promise((resolve,reject)=>{
  const worker=new Worker(new URL('../workers/mainSolid.worker.ts',import.meta.url),{type:'module'});current={worker,reject}
  const finish=()=>{worker.terminate();if(current?.worker===worker)current=null}
  worker.onmessage=e=>{finish();e.data.error?reject(Error(e.data.error)):resolve(e.data.document)};worker.onerror=e=>{finish();reject(Error(e.message||'Geometry worker failed'))}
  try{worker.postMessage({kind:'cad',document,options})}catch(e){finish();reject(e)}
 })
}

export function computeCadInspection(bodies:import('./directModeling').DirectBody[]):Promise<import('./cadInspection').CadPairReport[]>{
 cancelMainSolid();return new Promise((resolve,reject)=>{const worker=new Worker(new URL('../workers/mainSolid.worker.ts',import.meta.url),{type:'module'});current={worker,reject};const finish=()=>{worker.terminate();if(current?.worker===worker)current=null};worker.onmessage=e=>{finish();e.data.error?reject(Error(e.data.error)):resolve(e.data.report)};worker.onerror=e=>{finish();reject(Error(e.message))};try{worker.postMessage({kind:'inspect',bodies})}catch(e){finish();reject(e)}})
}
