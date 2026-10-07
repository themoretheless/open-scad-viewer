<script setup lang="ts">
import {computed,onBeforeUnmount,onMounted,ref,shallowRef,watch} from 'vue'
import type {SolidNurbsCurve} from '../services/solidNurbs'
import type {CurveOffsetDiagnostics} from '../services/curveOffsetDiagnostics'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
const emit=defineEmits<{result:[{curves:SolidNurbsCurve[];diagnostics:CurveOffsetDiagnostics}|null]}>()
const props=defineProps<{curves:SolidNurbsCurve[];ids:string[];locale:string}>()
const selected=computed(()=>props.ids.map(id=>props.curves.find(c=>c.id===id)))
const enabled=computed(()=>selected.value.length>0&&selected.value.every(c=>c?.curve.degree===1&&!c.curve.periodic))
const client=createSolidPreviewWorker(),pending=ref(false),error=ref(''),result=shallowRef<CurveOffsetDiagnostics|null>(null)
let generation=0
const text=(ru:string,en:string)=>props.locale==='ru'?ru:en
function cancel(){generation++;client.cancel();pending.value=false;result.value=null;error.value='';emit('result',null)}
watch(()=>[props.ids,selected.value.map(c=>c?.curve)],cancel,{deep:true,flush:'sync'})
async function inspect(){
 cancel();const current=generation;pending.value=true
 try{const value=await client.run({kind:'curveChainInspection',document:{version:1,bodies:[],sketches:[],curves:JSON.parse(JSON.stringify(selected.value.filter((c):c is SolidNurbsCurve=>!!c)))},ids:[...props.ids],maxPairs:1000000});if(current===generation){result.value=value;emit('result',{curves:selected.value.filter((c):c is SolidNurbsCurve=>!!c),diagnostics:value})}}
 catch(e){if(current===generation)error.value=text('Проверка не выполнена. Выберите соединённые отрезки по порядку в плоскости XY с постоянной Z.','Inspection failed. Select connected segments in order in XY with constant Z.')}
 finally{if(current===generation)pending.value=false}
}
function keydown(event:KeyboardEvent){if(event.key==='Escape'&&(pending.value||result.value))cancel()}
onMounted(()=>window.addEventListener('keydown',keydown))
onBeforeUnmount(()=>{window.removeEventListener('keydown',keydown);cancel();client.dispose()})
</script>
<template>
 <section style="display:grid;gap:6px" @keydown.esc.stop="cancel">
  <button :disabled="!enabled||pending" @click="inspect">{{ text('Проверить текущую цепочку','Inspect current chain') }}</button>
  <small v-if="!enabled">{{ text('Выберите отрезки цепочки в порядке соединения.','Select chain segments in connection order.') }}</small>
  <button v-if="pending" @click="cancel">{{ text('Проверяю… · Отмена','Inspecting… · Cancel') }}</button>
  <small v-if="error" role="alert">{{ error }}</small>
  <small v-if="result" data-testid="current-chain-report">{{ text('Текущая цепочка','Current chain') }}: {{ text('пересечения','crossings') }} {{ result.crossings.length }} · {{ text('контакты','contacts') }} {{ result.contacts.length }} · {{ text('неразрешённые пары','unresolved pairs') }} {{ result.uncertain.length }} · {{ result.complete?text('проверка завершена','inspection complete'):text('проверка не завершена','inspection incomplete') }}. {{ text('Корректность области не подтверждена.','Region validity is unverified.') }}</small>
 </section>
</template>
