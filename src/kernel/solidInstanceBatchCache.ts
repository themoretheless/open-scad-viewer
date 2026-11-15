import type {DirectBody} from '../app/directModeling'
import {cloneDocumentTree,cloneDocumentTreeSized} from '../app/documentClone'
type Geometry=Pick<DirectBody,'mesh'|'brep'>
/** Worker-local exact transform results, LRU by retained bytes. Entries are private
 * copies: `set` stores a copy and every hit returns a fresh tree, so callers never share
 * geometry with the cache or with each other. Bytes are an in-memory size estimate. */
export class SolidInstanceBatchCache {
 private entries=new Map<string,{value:Geometry[];bytes:number}>()
 private bytes=0
 constructor(private limit=64_000_000){if(!Number.isSafeInteger(limit)||limit<0)throw Error('Invalid instance cache limit')}
 get size(){return this.entries.size}
 get retainedBytes(){return this.bytes}
 get(key:string):Geometry[]|undefined{
  const entry=this.entries.get(key)
  if(!entry)return
  this.entries.delete(key);this.entries.set(key,entry)
  return cloneDocumentTree(entry.value)
 }
 set(key:string,value:Geometry[]){
  const {copy,bytes:size}=cloneDocumentTreeSized(value),bytes=128+2*key.length+size
  if(bytes>this.limit)return
  const old=this.entries.get(key)
  if(old){this.bytes-=old.bytes;this.entries.delete(key)}
  while(this.bytes+bytes>this.limit){const first=this.entries.keys().next().value!;this.bytes-=this.entries.get(first)!.bytes;this.entries.delete(first)}
  this.entries.set(key,{value:copy,bytes});this.bytes+=bytes
 }
}
