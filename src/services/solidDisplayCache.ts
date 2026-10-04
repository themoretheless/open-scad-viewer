import {adaptiveCacheBytes} from './adaptiveMemoryBudget'
import {stringifyMeshJson} from './meshJson'
import type {PolygonMesh} from './geometry/polygon'
export interface DisplayMesh {closed?:boolean[]|null;workClosed?:boolean[]|null;mesh:PolygonMesh;map:number[]|null;normals:number[][];flat?:{positions:Float32Array;normals:Float32Array}}
/** Bounded accounting estimate, not a measurement of JavaScript heap usage. */
export class SolidDisplayCache {
 private entries=new Map<string,{key:string;value:DisplayMesh;weight:number}>()
 private weight=0
 private maxWeight:number
 private budget:()=>number
 constructor(private maxEntries=1200,maxWeight?:number,budget?:()=>number){
  this.maxWeight=maxWeight??64_000_000
  this.budget=budget??(maxWeight===undefined?()=>adaptiveCacheBytes(64_000_000):()=>this.maxWeight)
  maxWeight=this.maxWeight
  if(!Number.isSafeInteger(maxEntries)||maxEntries<1||!Number.isSafeInteger(maxWeight)||maxWeight<1)throw Error('Invalid display cache limits')
 }
 get size(){return this.entries.size}
 get retainedWeight(){return this.weight}
 get capacityBytes(){return this.budget()}
 private trim(limit=this.capacityBytes){
  while(this.weight>limit){
   const oldest=this.entries.keys().next().value!
   this.weight-=this.entries.get(oldest)!.weight;this.entries.delete(oldest)
  }
 }
 get(key:string){
  this.trim()
  const entry=this.entries.get(key)
  if(entry){this.entries.delete(key);this.entries.set(key,entry)}
  return entry?.value
 }
 canonicalKey(key:string){return this.entries.get(key)?.key}
 clear(){this.entries.clear();this.weight=0}
 set(key:string,value:DisplayMesh){
  const limit=this.capacityBytes
  this.trim(limit)
  const previous=this.entries.get(key)
  if(previous){this.weight-=previous.weight;this.entries.delete(key)}
  // Reserve a flat GPU positions/normals pair even before its lazy construction.
  const weight=(value.closed?.length??0)*8+(value.workClosed?.length??0)*8+256+key.length*2+value.mesh.positions.byteLength+value.mesh.indices.byteLength+(value.mesh.uv?.byteLength??0)
   +value.normals.reduce((n,row)=>n+32+row.length*8,0)+(value.map?.length??0)*8+value.mesh.indices.length*24
  if(weight>limit)return
  while(this.entries.size>=this.maxEntries||this.weight+weight>limit){
   const oldest=this.entries.keys().next().value!
   this.weight-=this.entries.get(oldest)!.weight;this.entries.delete(oldest)
  }
  this.entries.set(key,{key,value,weight});this.weight+=weight
 }
}

export function solidDisplayKey(body:{mesh:PolygonMesh;brep?:unknown}):string{return stringifyMeshJson({mesh:body.mesh,brep:body.brep})}
