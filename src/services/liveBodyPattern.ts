import type {DirectBody, DirectDocument} from './directModeling'
import {callGeometryRust} from './geometry/kernel'

export type LiveBodyPattern =
 | {kind:'grid';rows:number;columns:number;spacing:[number,number]}
 | {kind:'radial';count:number;axis:'x'|'y'|'z';center:[number,number,number]}
 | {kind:'mirror';axis:'x'|'y'|'z';center:[number,number,number]}

/** Metadata and identities stay in the host; every placement is calculated natively. */
export function expandLiveBodyPatterns(document:DirectDocument):DirectDocument {
 for(const b of document.bodies)if(b.instance?.pattern!==undefined&&typeof b.instance.pattern!=='boolean')throw Error('Invalid live pattern membership.')
 const independent=document.bodies.filter(body=>!body.instance?.pattern)
 const ids=new Set([...independent,...document.sketches,...document.curves??[],...document.surfaces??[]].map(o=>o.id))
 const copies:DirectBody[]=[]
 for(const source of independent){
  if(source.livePattern===undefined)continue
  if(source.instance)throw Error('Live patterns require an independent source body.')
  const matrices=callGeometryRust<number[][][]>('cad_live_pattern',{pattern:source.livePattern})
  for(const [i,matrix] of matrices.entries()){
   const id=`${source.id}:pattern:${i+1}`
   if(id.length>200||ids.has(id))throw Error('Live pattern generated an invalid or conflicting object identity.')
   ids.add(id)
   const {livePattern:_pattern,...body}=source
   copies.push({...body,id,name:source.name.slice(0,80)+` · ${i+2}`,instance:{sourceId:source.id,matrix,pattern:true}})
  }
 }
 if(copies.length+independent.filter(b=>b.instance).length>1000)throw Error('Live patterns exceed 1000 linked instances.')
 // Orphaned derived placements must fail rather than silently disappear on source deletion.
 for(const body of document.bodies.filter(b=>b.instance?.pattern)){
  if(!independent.some(source=>source.id===body.instance!.sourceId&&source.livePattern!==undefined))throw Error('Remove or detach the live pattern before deleting its source.')
 }
 return {...document,bodies:[...independent,...copies]}
}

export function setLiveBodyPattern(document:DirectDocument,sourceId:string,pattern:LiveBodyPattern|null):DirectDocument {
 const next=structuredClone(document),source=next.bodies.find(b=>b.id===sourceId)
 if(!source||source.instance)throw Error('Select an independent source body.')
 next.bodies=next.bodies.filter(b=>!(b.instance?.pattern&&b.instance.sourceId===sourceId))
 if(pattern)source.livePattern=structuredClone(pattern);else delete source.livePattern
 return next
}

export function detachLiveBodyPattern(document:DirectDocument,sourceId:string):DirectDocument {
 const next=structuredClone(document),source=next.bodies.find(b=>b.id===sourceId)
 if(!source?.livePattern)throw Error('Select a live pattern source.')
 delete source.livePattern
 for(const body of next.bodies)if(body.instance?.pattern&&body.instance.sourceId===sourceId)delete body.instance
 return next
}
