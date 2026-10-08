<script setup lang="ts">
import {ref,watch,onUnmounted} from 'vue'
import {serializeDirectDocument,type DirectDocument} from '../services/directModeling'
import {appendDirectCheckpoint,appendDirectDocumentEdit,type DirectActionScript} from '../services/directTransactions'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
const props=defineProps<{document:DirectDocument;locale?:string}>()
const emit=defineEmits<{apply:[document:DirectDocument]}>()
const open=ref(false),recording=ref(false),pending=ref(false),error=ref('')
const script=ref<DirectActionScript>({version:1,actions:[]}),text=ref('')
let previous=JSON.parse(serializeDirectDocument(props.document)) as DirectDocument
const worker=createSolidPreviewWorker()
const label=(ru:string,en:string)=>props.locale==='en'?en:ru
watch(()=>serializeDirectDocument(props.document),textValue=>{
 const next=JSON.parse(textValue) as DirectDocument
 if(recording.value)try{script.value=appendDirectDocumentEdit(script.value,previous,next);text.value=JSON.stringify(script.value,null,2)}catch(e){recording.value=false;error.value=String(e)}
 previous=next
},{flush:'sync'})
function record(){error.value='';recording.value=!recording.value;if(recording.value){previous=JSON.parse(serializeDirectDocument(props.document));script.value=appendDirectCheckpoint({version:1,actions:[]},previous);text.value=JSON.stringify(script.value,null,2)}}

async function replay(){
 const before=serializeDirectDocument(props.document);pending.value=true;error.value='';recording.value=false
 try{
  const result=await worker.run({kind:'transaction',document:JSON.parse(before),script:text.value})
  if(serializeDirectDocument(props.document)!==before)throw Error(label('Модель изменилась во время выполнения. Запустите снова.','Model changed during execution. Run again.'))
  emit('apply',result)
 }catch(e){error.value=e instanceof Error?e.message:String(e)}finally{pending.value=false}
}
function download(){const url=URL.createObjectURL(new Blob([text.value],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download='model-actions.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)}
async function load(event:Event){const f=(event.target as HTMLInputElement).files?.[0];if(!f)return;if(f.size>16*1024*1024){error.value='Maximum 16 MiB';return}text.value=await f.text()}
onUnmounted(()=>worker.dispose())
</script>
<template>
 <div class="direct-actions modeling-feature">
  <button type="button" :aria-expanded="open" @click="open=!open">{{label('Действия и запись','Actions and recording')}}</button>
  <fieldset v-if="open">
   <legend>{{label('Пакет операций · один Undo','Action batch · one Undo')}}</legend>
   <button type="button" :disabled="pending" :aria-pressed="recording" @click="record">{{recording?label('Остановить запись','Stop recording'):label('Записать изменения','Record changes')}}</button>
   <small>{{label('Запись сохраняет начальную модель и последовательность команд изменения объектов. При воспроизведении проверяется исходное состояние.','Recording stores the initial model and entity edit commands with source preconditions.')}}</small>
   <label>{{label('Файл действий','Action file')}}<input type="file" accept=".json,application/json" :disabled="pending||recording" @change="load"></label>
   <textarea v-model="text" :disabled="pending||recording" :aria-label="label('Пакет действий JSON','Action batch JSON')" rows="5" spellcheck="false" />
   <button type="button" :disabled="pending||!text" @click="replay">{{label('Воспроизвести','Replay')}}</button>
   <button type="button" :disabled="!text" @click="download">{{label('Сохранить запись','Save recording')}}</button>
   <button v-if="pending" type="button" @click="worker.cancel()">{{label('Отмена','Cancel')}}</button>
   <p v-if="error" role="alert">{{error}}</p>
  </fieldset>
 </div>
</template>
<style scoped>
.direct-actions{font-size:11px}fieldset{border:0;padding:6px 0;display:grid;gap:6px}textarea{width:100%;box-sizing:border-box;font:10px monospace}small{color:var(--text-dim);line-height:1.4}button{font-size:11px}p{color:#ef8c7e}
</style>
