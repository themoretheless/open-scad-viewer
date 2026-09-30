import type {SolidInstanceBatchCache} from './solidInstanceBatchCache'
import type {DirectDocument,DirectBody} from './directModeling'
import {normalizePolygonMesh} from './geometry/polygon'
import {callGeometryRust} from './geometry/kernel'
import {stringifyMeshJson} from './meshJson'
export interface SolidInstanceLink {sourceId:string;matrix:number[][]}
/** Relative world-space edit about the current mesh bounds center. */
export function transformSolidInstance(document:DirectDocument,id:string,delta:number[],axis:number[],angle:number,scale:number):DirectDocument {
 const next=resolveSolidInstances(structuredClone(document)),body=next.bodies.find(body=>body.id===id)
 if(!body?.instance)throw Error('Select a linked instance.')
 body.instance.matrix=callGeometryRust('cad_instance_transform',{mesh:body.mesh,matrix:body.instance.matrix,delta,axis,angle,scale})
 return resolveSolidInstances(next)
}
/** References point directly to an independent source; geometry math stays in Rust. */
export function resolveSolidInstances(document:DirectDocument,cache?:SolidInstanceBatchCache):DirectDocument {
 const bodies=new Map(document.bodies.map(body=>[body.id,body]))
 const groups=new Map<string,DirectBody[]>()
 for(const body of document.bodies){
  if(!body.instance)continue
  const link=body.instance,source=bodies.get(link.sourceId)
  if(!source||source===body)throw Error('Instance source is missing or refers to itself. Make the instance independent before deleting its source.')
  if(source.instance)throw Error('Instance sources must be independent bodies; reference the original source.')
  const m=link.matrix
  if(!Array.isArray(m)||m.length!==4||!m.every(row=>Array.isArray(row)&&row.length===4&&row.every(Number.isFinite))||m[3].some((x,i)=>x!==(i===3?1:0)))throw Error('Instance placement must be a finite affine 4×4 matrix.')
  const group=groups.get(source.id)??[];group.push(body);groups.set(source.id,group)
 }
 const geometry=new Map<string,Pick<DirectBody,'mesh'|'brep'>>()
 for(const [sourceId,group] of groups){
  const source=bodies.get(sourceId)!
  const batchSize=Math.max(1,Math.min(32,Math.floor(1_000_000/stringifyMeshJson({mesh:source.mesh,brep:source.brep}).length)))
  // Bound temporary native Value/transport allocations, not just the final document.
  // A single 1000-body packet caused a 37-second first allocation in qualification.
  for(let start=0;start<group.length;start+=batchSize){
   const batch=group.slice(start,start+batchSize)
   const input={mesh:source.mesh,...(source.brep?{brep:source.brep}:{}),matrices:batch.map(body=>body.instance!.matrix)}
   const key=cache?stringifyMeshJson(input):undefined
   let result=key===undefined?undefined:cache!.get(key)
   if(!result){result=callGeometryRust<Pick<DirectBody,'mesh'|'brep'>[]>('cad_instances',input);if(key!==undefined&&result.length===batch.length)cache!.set(key,result)}
   if(result.length!==batch.length)throw Error('Instance batch returned an incomplete result.')
   result.forEach((item,i)=>{normalizePolygonMesh(item.mesh);geometry.set(batch[i].id,item)})
  }
 }
 const resolved=document.bodies.map(body=>{
  if(!body.instance)return body
  const {mesh:_mesh,brep:_brep,...identity}=body
  return {...identity,...geometry.get(body.id)!}
 })
 return {...document,bodies:resolved}
}
export function createSolidInstance(document:DirectDocument,sourceId:string,id:string,matrix:number[][]):DirectDocument {
 const source=document.bodies.find(body=>body.id===sourceId)
 if(!source||source.instance)throw Error('Choose an independent source body.')
 if(!id||[...document.bodies,...document.sketches,...document.curves??[],...document.surfaces??[]].some(item=>item.id===id))throw Error('Instance needs a unique object ID.')
 const next=structuredClone(document)
 next.bodies.push({...structuredClone(source),id,name:source.name.slice(0,89)+' · instance',instance:{sourceId,matrix:structuredClone(matrix)}})
 return resolveSolidInstances(next)
}
export function detachSolidInstances(document:DirectDocument,ids:readonly string[]):DirectDocument {
 const next=resolveSolidInstances(structuredClone(document))
 for(const body of next.bodies)if(ids.includes(body.id))delete body.instance
 return next
}
