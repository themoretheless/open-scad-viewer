<script setup lang="ts">
import {computed,onUnmounted,ref,shallowRef,watchEffect} from 'vue'
import type {NurbsBrep} from '../services/geometry/brep'
import type {SolidDistanceResult,VolumeValidityLimits} from '../services/solidDistance'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
const props=defineProps<{active:boolean;ru:boolean;a?:NurbsBrep;b?:NurbsBrep;same:boolean;names:[string,string]}>()
const emit=defineEmits<{state:[active:boolean,point:[number,number,number]|null]}>()
const label=(a:string,b:string)=>props.ru?a:b
const enabled=ref(false),extended=ref(false),retry=ref(0),pending=ref(false),error=ref('')
const result=shallowRef<SolidDistanceResult|null>(null),worker=createSolidPreviewWorker()
onUnmounted(()=>{worker.dispose();emit('state',false,null)})
watchEffect(()=>{
 const active=props.active&&enabled.value,box=active?result.value?.contact?.pointIntervalMm:null
 emit('state',active,box?box.map(([lo,hi])=>lo/2+hi/2) as [number,number,number]:null)
})
watchEffect(onCleanup=>{
 const active=props.active&&enabled.value,a=props.a,b=props.b,same=props.same,budget=extended.value?1000000:100000
 void retry.value
 let current=true
 onCleanup(()=>{current=false;worker.cancel()})
 result.value=null;error.value='';pending.value=false
 if(!active)return
 if(!a||!b||same){error.value=label('Выберите два разных тела B-rep в полях A и B.','Choose two different B-rep bodies in A and B.');return}
 const validityLimits:VolumeValidityLimits={exactWork:1000000,trimPairs:10000,trimCells:100000,trimDomainCells:1000000,spans:10000,
  facePairs:10000,faceCells:budget,faceDomainCells:budget===100000?1000000:8000000,faceCellsPerPair:1000,faceDomainCellsPerPair:10000,
  nestingPairs:1000,nestingCells:budget,nestingDomainCells:budget===100000?1000000:8000000,
  orientationCells:budget,orientationDomainCells:budget===100000?1000000:8000000,orientationSpans:10000}
 pending.value=true
 void worker.run({kind:'solidDistance',options:{a,b,toleranceMm:0.001,toleranceUv:1e-7,maxPairs:10000,maxContactPairs:10000,maxCells:budget,maxDomainCells:budget===100000?1000000:8000000,validityLimits}})
  .then(r=>{if(current)result.value=r})
  .catch(()=>{if(current)error.value=label('Расчёт не выполнен. Повторите или проверьте геометрию выбранных тел.','Calculation failed. Retry or check the selected bodies’ geometry.')})
  .finally(()=>{if(current)pending.value=false})
})
const failures=computed(()=>result.value?.validity.flatMap((v,i)=>v.proven?[]:[{
 name:props.names[i],stage:!v.exactAgreement||!v.exactJoins?label('границы не подтверждены точно','exact boundaries unproven'):
 !v.trimValid?label('контуры граней не подтверждены','face regions unproven'):
 !v.selfIntersectionAbsent?label('самопересечения не исключены','self-intersections not excluded'):
 v.nestingRolesConsistent!==true?label('вложение оболочек не подтверждено','shell nesting unproven'):
 label('ориентация оболочек не подтверждена','shell orientation unproven')}])??[])
</script>
<template>
 <button type="button" :aria-pressed="enabled" @click="enabled=!enabled">{{label('Расстояние между объёмами','Distance between volumes')}}</button>
 <fieldset v-if="enabled" class="solid-volume-distance" aria-label="solid-volume-distance" @keydown.esc.stop="enabled=false">
  <legend>{{label('Заполненные тела','Filled bodies')}}</legend>
  <small>{{names[0]}} ↔ {{names[1]}}</small>
  <label><input v-model="extended" type="checkbox">{{label('Расширенный расчёт','Extended calculation')}}</label>
  <p v-if="pending" role="status">{{label('Проверяю объёмы и расстояние…','Checking volumes and distance…')}}</p>
  <template v-if="result">
   <output v-if="result.distanceIntervalMm" data-solid-distance>{{result.distanceIntervalMm.map(x=>Number(x.toPrecision(12))).join(' … ')}} mm</output>
   <p v-if="result.converged" role="status">{{result.materialOverlap?label('Общие точки тел подтверждены. Расстояние — 0 мм.','The bodies share points. Distance is 0 mm.'):label('Допуск расстояния достигнут: 0,001 мм.','Distance tolerance reached: 0.001 mm.')}}</p>
   <p v-else role="status">{{label('Проверка не завершена. Увеличьте объём расчёта; при повторном отказе проверьте геометрию.','Check incomplete. Increase the calculation budget; if it remains unresolved, inspect the geometry.')}}</p>
   <ul v-if="failures.length"><li v-for="f in failures" :key="f.name">{{f.name}}: {{f.stage}}</li></ul>
   <small v-if="result.contact">{{label('Контакт на гранях A / B: ','Contact on faces A / B: ')}}{{result.contact.faces.map(f=>f+1).join(' / ')}}</small>
  </template>
  <p v-if="error" role="alert">{{error}}</p>
  <button v-if="error||result&&!result.converged" type="button" @click="retry++">{{label('Повторить','Retry')}}</button>
  <button type="button" @click="enabled=false">{{label('Закрыть','Close')}} · Esc</button>
 </fieldset>
</template>
<style scoped>
button{padding:8px;border:1px solid var(--border);border-radius:5px;background:var(--surface-raised);color:var(--text);font:inherit;cursor:pointer}button:hover,button[aria-pressed=true]{border-color:var(--accent)}button:focus-visible{outline:2px solid var(--accent);outline-offset:2px}.solid-volume-distance{border:1px solid var(--border);border-radius:5px;display:grid;gap:8px;min-width:0}.solid-volume-distance output{overflow-wrap:anywhere}.solid-volume-distance small{color:var(--text-dim);line-height:1.5}.solid-volume-distance p{margin:4px 0}.solid-volume-distance ul{padding-inline-start:18px}
</style>
