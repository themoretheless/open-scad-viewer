import type {DirectBody} from './directModeling'
import {stringifyMeshJson} from './meshJson'
type Geometry=Pick<DirectBody,'mesh'|'brep'>
/** Worker-local exact batch results. Encoded JSON owns no caller objects; each hit is a fresh tree. */
export class SolidInstanceBatchCache {
 private entries=new Map<string,{encoded:Uint8Array;bytes:number}>()
 private encoder=new TextEncoder()
 private decoder=new TextDecoder('utf-8',{fatal:true})
 private bytes=0
 constructor(private limit=64_000_000){if(!Number.isSafeInteger(limit)||limit<0)throw Error('Invalid instance cache limit')}
 get size(){return this.entries.size}
 get retainedBytes(){return this.bytes}
 get(key:string):Geometry[]|undefined{
  const entry=this.entries.get(key)
  if(!entry)return
  this.entries.delete(key);this.entries.set(key,entry)
  return JSON.parse(this.decoder.decode(entry.encoded))
 }
 set(key:string,value:Geometry[]){
  const encoded=this.encoder.encode(stringifyMeshJson(value)),bytes=128+2*key.length+encoded.byteLength
  if(bytes>this.limit)return
  const old=this.entries.get(key)
  if(old){this.bytes-=old.bytes;this.entries.delete(key)}
  while(this.bytes+bytes>this.limit){const first=this.entries.keys().next().value!;this.bytes-=this.entries.get(first)!.bytes;this.entries.delete(first)}
  this.entries.set(key,{encoded,bytes});this.bytes+=bytes
 }
}
