import type {MeshData} from '../core/mesh'
import {compileOpenSCAD} from './openscadCompiler'
import {directBodiesScad,type DirectDocument} from './directModeling'
import {sceneBody} from './mainModeling'
/** Replace only authored top-level calls owning changed bodies; keep declarations/comments intact. */
export function patchMainSource(source:string,meshes:MeshData[],result:DirectDocument):string {
 const roots=compileOpenSCAD(source).filter(s=>s.type==='call')
 const ownership=meshes.map(mesh=>{
  const refs=mesh.provenance.map(r=>r.source).filter(r=>r!==null)
  const candidates=roots.filter(root=>refs.length>0&&refs.every(r=>r.start>=root.p&&r.end<=root.end))
  return candidates.length===1?candidates[0]:null
 })
 const old=meshes.map(sceneBody),byId=new Map(result.bodies.map(b=>[b.id,b]))
 const changed=old.map((b,i)=>({b,i})).filter(({b})=>JSON.stringify(b.mesh)!==JSON.stringify(byId.get(b.id)?.mesh))
 const affected=new Set(changed.map(({i})=>{const root=ownership[i];if(!root)throw Error('Cannot safely map this body to a source expression. Export the result instead of replacing unrelated source.');return root}))
 const edits=[...affected].map(root=>({start:root.p,end:root.end,text:directBodiesScad({version:1,sketches:[],bodies:old.filter((_,i)=>ownership[i]===root).flatMap(b=>{const next=byId.get(b.id);return next?[next]:[]})})})).sort((a,b)=>b.start-a.start)
 let output=source
 for(const edit of edits)output=output.slice(0,edit.start)+edit.text+output.slice(edit.end)
 const added=result.bodies.filter(b=>!old.some(o=>o.id===b.id))
 if(added.length)output+='\n'+directBodiesScad({version:1,sketches:[],bodies:added})
 return output
}
export interface SourceEdit {before:string;after:string}
export interface SourceHistory {past:SourceEdit[];future:SourceEdit[]}
export function restoreSourceHistory(text:string|null,current:string):SourceHistory {
 try{const d=JSON.parse(text??'');if(!Array.isArray(d.past)||!Array.isArray(d.future)||JSON.stringify(d).length>2_000_000)throw Error();
 const valid=(v:unknown):v is SourceEdit=>!!v&&typeof (v as SourceEdit).before==='string'&&typeof (v as SourceEdit).after==='string'
 if(!d.past.every(valid)||!d.future.every(valid))throw Error()
 if(d.past.some((e:SourceEdit,i:number)=>i>0&&d.past[i-1].after!==e.before)||d.future.some((e:SourceEdit,i:number)=>i>0&&d.future[i-1].before!==e.after))throw Error()
 if(d.past.length&&d.past.at(-1).after!==current||d.future.length&&d.future.at(-1).before!==current)throw Error()
 return d}catch{return {past:[],future:[]}}
}
export function boundedSourceHistory(d:SourceHistory):SourceHistory {
 const out={past:[...d.past],future:[...d.future]}
 while(out.past.length+out.future.length>80||JSON.stringify(out).length>2_000_000){if(out.past.length)out.past.shift();else if(out.future.length)out.future.shift();else break}
 return out
}
