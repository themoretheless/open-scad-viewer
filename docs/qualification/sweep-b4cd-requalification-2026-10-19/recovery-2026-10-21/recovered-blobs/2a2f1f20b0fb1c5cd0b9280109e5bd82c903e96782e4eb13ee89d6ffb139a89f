import type {DirectDocument} from './directModeling'
import {directCornerTool} from './directProfileTools'
import {circularDirectCopies} from './directModelingTools'

export interface SolidSketchEditOptions {
 operation:'fillet'|'dogear'|'array'
 id:string
 vertex:number
 radius:number
 count:number
 center:[number,number]
 sweep:number
 copyIds:string[]
}
/** Keep the source untouched and use caller-owned identities for preview and commit. */
export function applySolidSketchEdit(source:DirectDocument,p:SolidSketchEditOptions):DirectDocument {
 const document=structuredClone(source),index=document.sketches.findIndex(s=>s.id===p.id),sketch=document.sketches[index]
 if(!sketch)throw Error('Select an existing sketch.')
 if(p.operation==='array'){
  let i=0
  const copies=circularDirectCopies(sketch,p.count,p.center,p.sweep,()=>p.copyIds[i++])
  if(p.copyIds.length!==p.count-1||new Set(p.copyIds).size!==p.copyIds.length||p.copyIds.some(id=>!id||document.sketches.some(s=>s.id===id)))throw Error('Copies require distinct identities.')
  document.sketches.push(...copies)
 }else document.sketches[index]=directCornerTool(sketch,p.vertex,p.radius,p.operation)
 return document
}
