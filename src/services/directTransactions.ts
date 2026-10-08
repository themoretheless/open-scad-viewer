import {cleanSketchContours,outlineSketchStroke} from './editableSketchOperations'
import {applyDirectExtrusionProfile,type DirectExtrusionOptions} from './directExtrusion'
import {parseDirectDocument,serializeDirectDocument,type DirectDocument} from './directModeling'
import {applySolidSceneEdit,type SolidSceneEditOptions} from './solidSceneEdit'
import {addSolidPrimitive,type SolidPrimitiveOptions} from './solidPrimitive'
import {setLiveBodyPattern,type LiveBodyPattern} from './liveBodyPattern'

export type DirectAction =
 | {kind:'sceneEdit';options:SolidSceneEditOptions}
 | {kind:'primitive';options:SolidPrimitiveOptions}
 | {kind:'pattern';sourceId:string;pattern:LiveBodyPattern|null}
 | {kind:'checkpoint';document:string}
 | {kind:'documentEdit';edits:DirectDocumentEdit[]}
 | {kind:'sketchCleanup';sourceId:string;createdId:string;tolerance:number}
 | {kind:'outlineStroke';sourceId:string;createdId:string;width:number}
 | {kind:'extrusion';options:DirectExtrusionOptions}
export interface DirectDocumentEdit {collection:'sketches'|'bodies'|'curves'|'surfaces'|'groups'|'metadata'|'order';id:string;before:unknown;after:unknown;index?:number}
export interface DirectActionScript {version:1;actions:DirectAction[]}
export const MAX_ACTION_SCRIPT_BYTES=16*1024*1024
/** Work on an isolated document. Only the final validated result may be committed. */
export function executeDirectTransaction(document:DirectDocument,text:string):DirectDocument {
 if(typeof text!=='string'||new TextEncoder().encode(text).length>MAX_ACTION_SCRIPT_BYTES)throw Error('Action script exceeds 16 MiB.')
 const script:DirectActionScript=JSON.parse(text)
 if(!script||script.version!==1||!Array.isArray(script.actions)||script.actions.length<1||script.actions.length>80)throw Error('Expected 1–80 actions in a version 1 script.')
 let next=parseDirectDocument(serializeDirectDocument(document))
 for(const [index,action] of script.actions.entries()){
  try{
   if(!action||typeof action!=='object')throw Error('Invalid action.')
   switch(action.kind){
    case 'sceneEdit':next=applySolidSceneEdit(next,action.options);break
    case 'primitive':next=addSolidPrimitive(next,action.options);break
    case 'pattern':next=setLiveBodyPattern(next,action.sourceId,action.pattern);break
    case 'sketchCleanup':{const source=next.sketches.find(s=>s.id===action.sourceId);if(!source)throw Error('Select an existing source sketch.');next={...next,sketches:[...next.sketches,cleanSketchContours(source,action.tolerance,action.createdId)]};break}
    case 'outlineStroke':{const source=next.sketches.find(s=>s.id===action.sourceId);if(!source)throw Error('Select an existing source sketch.');next={...next,sketches:[...next.sketches,outlineSketchStroke(source,action.width,action.createdId)]};break}
    case 'extrusion':{
     if(!action.options||!['new','union','difference'].includes(action.options.operation))throw Error('Invalid extrusion operation.')
     const target=next.bodies.find(b=>b.id===action.options.targetId)
     if(action.options.operation!=='new'&&target?.instance?.pattern)throw Error('Detach the complete pattern before editing a generated member.')
     next=applyDirectExtrusionProfile(next,action.options);break
    }
    case 'documentEdit':{
     if(!Array.isArray(action.edits)||!action.edits.length||action.edits.length>1200)throw Error('Invalid document edit command.')
     const value=JSON.parse(serializeDirectDocument(next)) as Record<string,any>
     for(const edit of action.edits){
      if(!edit||!['sketches','bodies','curves','surfaces','groups','metadata','order'].includes(edit.collection)||typeof edit.id!=='string')throw Error('Invalid entity edit.')
      const same=(a:unknown,b:unknown)=>JSON.stringify(a??null)===JSON.stringify(b??null)
      if(edit.collection==='order'){
       if(!['sketches','bodies','curves','surfaces','groups'].includes(edit.id)||!Array.isArray(edit.after)||edit.after.some(v=>typeof v!=='string'))throw Error('Invalid recorded entity order.')
       const key=edit.id==='groups'?'name':'id',original=(next as any)[edit.id]??[],list=value[edit.id]??[],ids=edit.after as string[]
       if(!same(original.map((e:any)=>e[key]),edit.before)||ids.length!==list.length||new Set(ids).size!==ids.length||ids.some(id=>!list.some((e:any)=>e[key]===id)))throw Error('Recorded entity order does not match the source.')
       value[edit.id]=ids.map(id=>list.find((e:any)=>e[key]===id))
      }else if(edit.collection==='metadata'){
       if(!['interchange','blenderProjectId'].includes(edit.id)||!same(value[edit.id],edit.before))throw Error('Recorded metadata no longer matches the source.')
       if(edit.after===null)delete value[edit.id];else value[edit.id]=edit.after
      }else{
       const list:any[]=value[edit.collection]??[],key=edit.collection==='groups'?'name':'id',index=list.findIndex(e=>e[key]===edit.id)
       if(!same(index<0?null:list[index],edit.before))throw Error('Recorded entity no longer matches the source: '+edit.id)
       if(edit.after===null){if(index>=0)list.splice(index,1)}
       else {if(typeof edit.after!=='object'||(edit.after as any)[key]!==edit.id)throw Error('Entity identity mismatch.');if(index>=0)list[index]=edit.after;else {if(edit.index!==undefined&&(!Number.isInteger(edit.index)||edit.index<0||edit.index>list.length))throw Error('Invalid entity insertion position.');list.splice(edit.index??list.length,0,edit.after)}}
       value[edit.collection]=list
      }
     }
     next=parseDirectDocument(JSON.stringify(value));break
    }
    case 'checkpoint':next=parseDirectDocument(action.document);break
    default:throw Error('Unsupported action kind.')
   }
   next=parseDirectDocument(serializeDirectDocument(next))
  }catch(error){throw Error(`Action ${index+1}: ${error instanceof Error?error.message:String(error)}`)}
 }
 return next
}
/** UI recording stores validated document checkpoints, including operations outside the command whitelist. */
export function appendDirectCheckpoint(script:DirectActionScript,document:DirectDocument):DirectActionScript {
 const text=serializeDirectDocument(document)
 if(script.actions.at(-1)?.kind==='checkpoint'&&(script.actions.at(-1) as Extract<DirectAction,{kind:'checkpoint'}>).document===text)return script
 const next:DirectActionScript={version:1,actions:[...script.actions,{kind:'checkpoint',document:text}]}
 if(next.actions.length>80||new TextEncoder().encode(JSON.stringify(next)).length>MAX_ACTION_SCRIPT_BYTES)throw Error('Recording limit reached (80 changes / 16 MiB).')
 return next
}

/** Record entity commands with source preconditions, rather than a complete document per edit. */
export function appendDirectDocumentEdit(script:DirectActionScript,before:DirectDocument,after:DirectDocument):DirectActionScript {
 before=JSON.parse(serializeDirectDocument(before));after=JSON.parse(serializeDirectDocument(after))
 const edits:DirectDocumentEdit[]=[]
 for(const collection of ['sketches','bodies','curves','surfaces','groups'] as const){
  const key=collection==='groups'?'name':'id',a=(before[collection]??[]) as any[],b=(after[collection]??[]) as any[]
  const ids=new Set([...a,...b].map(e=>e[key]))
  for(const id of ids){const old=a.find(e=>e[key]===id)??null,index=b.findIndex(e=>e[key]===id),value=index<0?null:b[index];if(JSON.stringify(old)!==JSON.stringify(value))edits.push({collection,id,before:structuredClone(old),after:structuredClone(value),index:index<0?undefined:index})}
  if(JSON.stringify(a.map(e=>e[key]))!==JSON.stringify(b.map(e=>e[key])))edits.push({collection:'order',id:collection,before:a.map(e=>e[key]),after:b.map(e=>e[key])})
 }
 for(const id of ['interchange','blenderProjectId'] as const)if(JSON.stringify(before[id]??null)!==JSON.stringify(after[id]??null))edits.push({collection:'metadata',id,before:structuredClone(before[id]??null),after:structuredClone(after[id]??null)})
 if(!edits.length)return script
 const next:DirectActionScript={version:1,actions:[...script.actions,{kind:'documentEdit',edits}]}
 if(next.actions.length>80||new TextEncoder().encode(JSON.stringify(next)).length>MAX_ACTION_SCRIPT_BYTES)throw Error('Recording limit reached (80 commands / 16 MiB).')
 return next
}
