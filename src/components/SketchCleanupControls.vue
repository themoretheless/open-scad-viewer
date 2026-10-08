<script setup lang="ts">
import {ref,onUnmounted} from 'vue'
import {serializeDirectDocument,type DirectDocument} from '../services/directModeling'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
const props=defineProps<{document:DirectDocument;sourceId:string;locale?:string}>(),emit=defineEmits<{apply:[document:DirectDocument]}>()
const tolerance=ref(.1),pending=ref(false),error=ref(''),worker=createSolidPreviewWorker(),label=(ru:string,en:string)=>props.locale==='en'?en:ru
async function clean(){pending.value=true;error.value='';const before=serializeDirectDocument(props.document)
 try{const result=await worker.run({kind:'transaction',document:JSON.parse(before),script:JSON.stringify({version:1,actions:[{kind:'sketchCleanup',sourceId:props.sourceId,createdId:crypto.randomUUID(),tolerance:tolerance.value}]})});if(serializeDirectDocument(props.document)!==before)throw Error(label('Модель изменилась. Повторите очистку.','Model changed. Run cleanup again.'));emit('apply',result)}catch(e){error.value=String(e)}finally{pending.value=false}}
onUnmounted(()=>worker.dispose())
</script>
<template><div class="modeling-feature"><label>{{label('Допуск очистки, мм','Cleanup tolerance, mm')}}<input v-model.number="tolerance" type="number" min=".0001" max="10" step=".01"></label><button type="button" :disabled="pending" @click="clean">{{label('Очистить контур','Clean contour')}}</button><p v-if="error" role="alert">{{error}}</p></div></template>
<style scoped>div{display:grid;gap:6px;font-size:11px}label{display:flex;justify-content:space-between;gap:6px}input{width:70px;min-width:0}button{font-size:11px}p{color:#ef8c7e}</style>
