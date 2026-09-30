import type {DirectBody} from './directModeling'
import {stringifyMeshJson} from './meshJson'
type Geometry=Pick<DirectBody,'mesh'|'brep'>
/** Worker-local exact batch results. Text owns no caller objects; each hit is a fresh tree. */
export class SolidInstanceBatchCache {
 private entries=new Map<string,{text:string;bytes:number}>()
 private bytes=0
 constructor(private limit=64_000_000){if(!Number.isSafeInteger(limit)||limit<0)throw Error('Invalid instance cache limit')}
 get size(){return this.entries.size}
 get retainedBytes(){return this.bytes}
 get(key:string):Geometry[]|undefined{
  const entry=this.entries.get(key)
  if(!entry)return
  this.entries.delete(key);this.entries.set(key,entry)
  return JSON.parse(entry.text)
 }
 set(key:string,value:Geometry[]){
  const text=stringifyMeshJson(value),bytes=128+2*(key.length+text.length)
  if(bytes>this.limit)return
  const old=this.entries.get(key)
  if(old){this.bytes-=old.bytes;this.entries.delete(key)}
  while(this.bytes+bytes>this.limit){const first=this.entries.keys().next().value!;this.bytes-=this.entries.get(first)!.bytes;this.entries.delete(first)}
  this.entries.set(key,{text,bytes});this.bytes+=bytes
 }
}
