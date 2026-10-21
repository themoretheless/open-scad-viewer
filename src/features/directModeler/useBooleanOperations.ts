import {ref,shallowRef,watch,onUnmounted,type Ref} from 'vue'
import type {DirectDocument} from '../../services/directModeling'
import type {SolidBooleanOptions} from '../../services/solidBoolean'
import type {BrepBooleanOperation} from '../../services/geometry/brep'
import {createSolidPreviewWorker} from '../../services/solidPreviewWorker'
interface BooleanOptions<T> {
 document:Ref<DirectDocument>;snapDocument:Ref<DirectDocument>;selectedIds:Readonly<Ref<string[]>>;selection:Ref<string>;extraSelection:Ref<string[]>;ru:Readonly<Ref<boolean>>;mode:Ref<'2d'|'3d'>;brepSegments:Ref<number>;error:Ref<string>;props:Readonly<{open:boolean}>
 label:(ru:string,en:string)=>string;commit:(document:DirectDocument)=>void;fit:(pane:'3d')=>void;cancelCommand:()=>void;run:(action:()=>void)=>void
 setNotice:(message:string)=>void;captureCommand:()=>T;recordCommand:(command:T)=>void;calculationFailure:(error:unknown,retry:boolean)=>string
 download:(text:string,name:string)=>void;EMPTY_NOTICE:()=>string;FALLBACK_NOTICE:()=>string;TOLERANT_NOTICE:(mm:number)=>string
}
export function useBooleanOperations<T>(state:BooleanOptions<T>) {
 const {document,snapDocument,selectedIds,selection,extraSelection,ru,mode,brepSegments,error,props,label,commit,fit,cancelCommand,run,download,EMPTY_NOTICE,FALLBACK_NOTICE,TOLERANT_NOTICE}=state
const booleanWorker=createSolidPreviewWorker(),booleanPending=ref(false)
const booleanKind=ref<'boolean'|'minkowski3d'>('boolean')
const minkowskiDeviation=ref(1)
const minkowskiProof=shallowRef<import('../../services/solidMinkowski').SolidMinkowskiResult|null>(null)
watch(document,()=>{minkowskiProof.value=null},{flush:'sync'})
let booleanGeneration=0
function cancelBoolean(){booleanGeneration++;booleanWorker.cancel();booleanPending.value=false}
async function runBoolean(options:SolidBooleanOptions,guided=false){
 if(booleanPending.value)return
 cancelBoolean()
 booleanKind.value='boolean'
 const generation=booleanGeneration,snapshot=guided?state.captureCommand():null
 error.value='';state.setNotice('');booleanPending.value=true
 try{
  const result=await booleanWorker.run({kind:'boolean',document:document.value,options})
  if(generation!==booleanGeneration)return
  // Commit triggers the document watcher, so clear pending before publishing the transaction.
  booleanPending.value=false
  commit(result.document);selection.value=result.resultId??'';extraSelection.value=[]
  if(guided){subtract.value=null;if(snapshot)state.recordCommand(snapshot)}
  if(result.resultId===null)state.setNotice(EMPTY_NOTICE())
  else if(!result.exact)state.setNotice(FALLBACK_NOTICE())
  else if(result.toleranceMm>0)state.setNotice(TOLERANT_NOTICE(result.toleranceMm))
 }catch(e){if(generation===booleanGeneration)error.value=state.calculationFailure(e,false)}
 finally{if(generation===booleanGeneration)booleanPending.value=false}
}
async function applyMinkowski(){
 if(booleanPending.value)return
 const ids=selectedIds.value.filter(id=>document.value.bodies.some(b=>b.id===id))
 if(ids.length!==2){error.value=label('Выберите ровно два разных тела.','Select exactly two different bodies.');return}
 cancelCommand();cancelBoolean();booleanKind.value='minkowski3d'
 const generation=booleanGeneration
 error.value='';state.setNotice('');booleanPending.value=true
 try{
  const result=await booleanWorker.run({kind:'minkowski3d',document:document.value,options:{ids:[ids[0],ids[1]],createdId:crypto.randomUUID(),ru:ru.value,maximumSourceDeviationMm:minkowskiDeviation.value}})
  if(generation!==booleanGeneration)return
  booleanPending.value=false;commit(result.document);selection.value=result.resultId??'';extraSelection.value=[]
  minkowskiProof.value=result;mode.value='3d';fit('3d')
 }catch(e){if(generation===booleanGeneration)error.value=state.calculationFailure(e,false)}
 finally{if(generation===booleanGeneration)booleanPending.value=false}
}
function downloadMinkowskiProof(){if(minkowskiProof.value)download(JSON.stringify(minkowskiProof.value.proof),'minkowski-3d-proof.json')}
function applyBrepBoolean(operation:BrepBooleanOperation){run(()=>{
 if(booleanPending.value)return
 cancelCommand()
 const ids=selectedIds.value.filter(id=>document.value.bodies.some(b=>b.id===id))
 if(ids.length!==2)throw Error(label('Выберите ровно два тела: первое выбранное — A.','Select exactly two bodies; the first selected body is A.'))
 void runBoolean({operation,a:[ids[0]],b:[ids[1]],segments:brepSegments.value,ru:ru.value})
})}

/**
 * Guided subtraction: field A collects the bodies to keep, field B the bodies to remove.
 *
 * Bodies clicked in the viewport or the scene list go into the active field, Shift adds
 * or removes one. Each field is unioned first, then A minus B runs once, so several
 * cutters can be removed from several parts in a single step.
 */
const subtract=ref<{a:string[];b:string[];active:'a'|'b'}|null>(null)
function beginSubtract(){
 cancelCommand()
 const bodyIds=new Set(snapDocument.value.bodies.map(body=>body.id))
 const ids=selectedIds.value.filter(id=>bodyIds.has(id))
 subtract.value={a:ids.slice(0,1),b:ids.slice(1),active:ids.length?'b':'a'}
}
function subtractPick(id:string,add:boolean){
 const s=subtract.value;if(!s)return
 const field=s.active,other=field==='a'?'b':'a'
 s[field]=add?(s[field].includes(id)?s[field].filter(x=>x!==id):[...s[field],id]):[id]
 s[other]=s[other].filter(x=>!s[field].includes(x))
 const all=[...s.a,...s.b];selection.value=all[0]??'';extraSelection.value=all.slice(1)
}
function subtractNames(ids:string[]){
 const names=ids.map(id=>document.value.bodies.find(b=>b.id===id)?.name).filter((n):n is string=>!!n)
 return names.length?names.join(', '):label('пусто','empty')
}
function applySubtract(){
 const s=subtract.value;if(!s)return
 void runBoolean({operation:'difference',a:[...s.a],b:[...s.b],segments:brepSegments.value,ru:ru.value},true)
}
watch(()=>[props.open,document.value,JSON.stringify(selectedIds.value),JSON.stringify(subtract.value),brepSegments.value,ru.value],()=>{
 if(booleanPending.value)cancelBoolean()
},{flush:'sync'})
onUnmounted(()=>{cancelBoolean();booleanWorker.dispose()})
 return {booleanPending,booleanKind,minkowskiDeviation,minkowskiProof,cancelBoolean,runBoolean,applyMinkowski,downloadMinkowskiProof,applyBrepBoolean,subtract,beginSubtract,subtractPick,subtractNames,applySubtract}
}
