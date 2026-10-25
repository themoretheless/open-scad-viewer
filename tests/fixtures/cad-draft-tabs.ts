import {createApp,h,ref} from 'vue'
import {createSolidPreviewWorker} from '../../src/services/solidPreviewWorker'
import {warmGeometryKernel} from '../../src/services/geometry/kernel'
import {DirectHistory,parseDirectDocument,serializeDirectDocument} from '../../src/services/directModeling'
import {transformSelection} from '../../src/services/directSolidTools'
import {readSolidDraftHead,writeSolidDraftHead,type SolidDraftHead} from '../../src/services/solidDraftHeadStore'
import baseline from '../../docs/qualification/cad-ui-history-2026-10-09/enclosure.json?raw'
// Isolated qualification key; never reads or replaces the user's active draft.
await warmGeometryKernel()
const key='qualification-enclosure-tabs-20261010-v1'
createApp({setup(){
 const state=ref('Not loaded'),revision=ref('none'),edits=ref(0),exact=ref(false),timings=ref<Record<string,number>>({})
 const measure=<T>(name:string,action:()=>T):T=>{const start=performance.now();try{return action()}finally{timings.value={...timings.value,[name]:performance.now()-start}}}
 let head:SolidDraftHead|null=null,history=new DirectHistory(parseDirectDocument(baseline))
 const worker=createSolidPreviewWorker()
 const duplicate=async()=>{
  const before=history.document,text=serializeDirectDocument(before),started=performance.now()
  state.value='Worker duplicate pending'
  try{
   const result=await worker.run({kind:'sceneEdit',document:before,options:{operation:'duplicate',id:'enclosure',ids:['enclosure'],copyIds:['qualification-copy'],createdId:'',x:10,y:10,z:0,axis:'z',angle:0,scale:1}})
   timings.value={...timings.value,workerDuplicateMs:performance.now()-started}
   if(serializeDirectDocument(before)!==text||result.bodies.length!==before.bodies.length+1||!result.bodies.some(body=>body.id==='qualification-copy'))throw Error('Worker duplicate preservation failed')
   state.value='Worker duplicate verified; saved draft unchanged'
  }catch(error){state.value=String(error)}finally{worker.cancel()}
 }
 const report=()=>{revision.value=head?.revision??'none';exact.value=head!==null&&serializeDirectDocument(parseDirectDocument(head.text))===head.text}
 const load=async()=>{head=await readSolidDraftHead(key);if(head)history=measure('parseAndHistoryMs',()=>new DirectHistory(parseDirectDocument(head!.text)));report();state.value=head?'Recovered CAD document':'No saved draft';edits.value=0}
 const publish=async()=>{try{const text=measure('serializeMs',()=>serializeDirectDocument(history.document));const started=performance.now();head=await writeSolidDraftHead(key,head?.revision??null,text);timings.value={...timings.value,writeMs:performance.now()-started};report();state.value='Published CAD document'}catch(error){state.value=error instanceof Error?error.message:String(error)}}
 const edit=()=>{const document=measure('documentCopyMs',()=>history.document);const next=measure('nativeTransformMs',()=>transformSelection(document,['enclosure'],[0,0,0.1],[0,0,1],0,1));measure('historyCommitMs',()=>history.commit(next));edits.value++;state.value='Local CAD translation'}
 void load().catch(error=>{state.value=String(error)})
 return()=>h('main',{style:'font:14px system-ui;max-width:760px;margin:32px auto'},[
 h('h1',{style:'font-size:18px'},'CAD document · two browser tabs'),
 h('p','Production CAD history and IndexedDB storage, isolated qualification key.'),
 h('button',{onClick:load},'Recover'),h('button',{onClick:edit},'Translate 0.1 mm'),h('button',{onClick:publish},'Publish'),h('button',{onClick:duplicate},'Verify worker duplicate'),
 h('p',{'aria-label':'Draft status'},state.value),h('p',{'aria-label':'Revision'},revision.value),
 h('p',{'aria-label':'Local edits'},String(edits.value)),h('p',{'aria-label':'Exact document roundtrip'},String(exact.value)),
 h('pre',{'aria-label':'Timing milliseconds'},JSON.stringify(timings.value,null,2)),
 ])
}}).mount('#app')
