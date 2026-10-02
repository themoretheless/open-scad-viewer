<script setup lang="ts">
import {computed,onUnmounted,ref,shallowRef,watch,watchEffect} from 'vue'
import type {NurbsBrep} from '../services/geometry/brep'
import type {MaterialOptions,MaterialResult,MaterialOverlay} from '../services/solidMaterialVolume'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
import {evaluateNurbsSurface} from '../services/nurbsSurface'
const props=defineProps<{active:boolean;ru:boolean;model?:NurbsBrep}>()
const emit=defineEmits<{state:[active:boolean,overlay:MaterialOverlay|null]}>()
const label=(ru:string,en:string)=>props.ru?ru:en
const enabled=ref(false),mode=ref<'chord'|'segment'>('chord'),origin=ref([-2,5,5]),direction=ref([14,0,0]),extended=ref(false)
const pending=ref(false),error=ref(''),result=shallowRef<MaterialResult|null>(null)
const request=shallowRef<{mode:'chord'|'segment';options:MaterialOptions}|null>(null),worker=createSolidPreviewWorker()
watch(()=>props.model,model=>{
 if(!model?.vertices.length)return
 const b=[0,1,2].map(k=>[Math.min(...model.vertices.map(v=>v.point[k])),Math.max(...model.vertices.map(v=>v.point[k]))])
 const pad=Math.max(1,(b[0][1]-b[0][0])*.1)
 origin.value=[b[0][0]-pad,(b[1][0]+b[1][1])/2,(b[2][0]+b[2][1])/2]
 direction.value=[b[0][1]-b[0][0]+2*pad,0,0]
},{immediate:true})
watch(()=>[props.active,props.model,enabled.value,mode.value,extended.value,...origin.value,...direction.value],()=>{
 request.value=null;result.value=null;error.value='';pending.value=false;worker.cancel()
},{flush:'sync'})
function run(){
 if(!props.model){error.value=label('Выберите тело с исходной геометрией.','Choose a body with original geometry.');return}
 if(!origin.value.every(Number.isFinite)||!direction.value.every(Number.isFinite)||direction.value.every(x=>x===0)){
  error.value=label('Введите конечные координаты и ненулевое смещение XYZ.','Enter finite coordinates and a nonzero XYZ displacement.');return
 }
 const budget=extended.value?1000000:100000,domains=extended.value?8000000:1000000
 request.value={mode:mode.value,options:{model:props.model,origin:[...origin.value] as [number,number,number],direction:[...direction.value] as [number,number,number],toleranceUv:1e-7,
  limits:{pointCells:budget,pointDomainCells:domains,segmentCells:budget,segmentDomainCells:domains,validity:{exactWork:1000000,trimPairs:10000,trimCells:100000,trimDomainCells:1000000,spans:4096,
   facePairs:10000,faceCells:extended.value?1000000:150000,faceDomainCells:extended.value?8000000:1500000,faceCellsPerPair:1024,faceDomainCellsPerPair:100000,
   nestingPairs:1000,nestingCells:budget,nestingDomainCells:domains,orientationCells:budget,orientationDomainCells:domains,orientationSpans:10000}}}}
}
watchEffect(onCleanup=>{
 const job=request.value,active=props.active&&enabled.value,model=props.model
 let current=true
 onCleanup(()=>{current=false;worker.cancel()})
 if(!active||!job||job.options.model!==model)return
 result.value=null;error.value='';pending.value=true
 void worker.run({kind:job.mode==='chord'?'materialChord':'materialSegment',options:job.options})
  .then(r=>{if(current)result.value=r})
  .catch(()=>{if(current)error.value=label('Проверка не выполнена. Повторите или проверьте геометрию тела.','Check failed. Retry or inspect the body geometry.')})
  .finally(()=>{if(current)pending.value=false})
})
const boundary=computed(()=>result.value?.method==='continuous-material-chord'?result.value.boundary:result.value?.segment??null)
const issues=computed(()=>[...(boundary.value?.contacts.map(c=>({...c,unresolved:false}))??[]),...(boundary.value?.unresolved.map(c=>({...c,unresolved:true}))??[])])
const message=computed(()=>{
 const r=result.value;if(!r)return ''
 const texts:Record<string,[string,string]>={
  'material-chord':['Участок материала между двумя гранями подтверждён.','Material between two faces is confirmed.'],
  'interior-segment':['Весь отрезок находится внутри материала.','The entire segment is inside material.'],
  'volume-unproven':['Объём тела не подтверждён. Проверьте границы, самопересечения и ориентацию оболочек.','Body volume is unproven. Inspect boundaries, self-intersections and shell orientation.'],
  'seed-inside':['Начало линии внутри тела. Перенесите его наружу для измерения между гранями.','The line starts inside the body. Move it outside to measure between faces.'],
  'seed-outside':['Начало отрезка снаружи материала. Перенесите его внутрь.','The segment starts outside material. Move it inside.'],
  'seed-unresolved':['Положение начала не подтверждено. Отодвиньте его от границы или расширьте расчёт.','Start location is unproven. Move it away from the boundary or extend the calculation.'],
  'requires-two-crossings':['Нужны два пересечения. Измените линию; дополнительные пересечения могут указывать на полость.','Two crossings are required. Change the line; extra crossings may indicate a cavity.'],
  'boundary-contact':['Отрезок пересекает границу материала. Проверьте отмеченные грани.','The segment crosses a material boundary. Inspect the marked faces.'],
  'boundary-unresolved':['Часть границы не проверена. Измените линию или расширьте расчёт.','Some boundary regions remain unchecked. Change the line or extend the calculation.'],
  'segment-unresolved':['Часть отрезка не проверена. Отодвиньте концы от границы или расширьте расчёт.','Some segment regions remain unchecked. Move endpoints away from the boundary or extend the calculation.'],
  'overlapping-root-intervals':['Пересечения не разделены. Измените линию или расширьте расчёт.','Crossings are not separated. Change the line or extend the calculation.'],
 }
 const t=texts[r.reason];return t?label(...t):label('Проверка не завершена. Измените линию и повторите.','Check incomplete. Change the line and retry.')
})
watchEffect(()=>{
 if(!props.active||!enabled.value||!result.value||!props.model){emit('state',props.active&&enabled.value,null);return}
 const r=result.value,marks=issues.value.slice(0,32).map(c=>({face:c.face,unresolved:c.unresolved,
  point:evaluateNurbsSurface(props.model!.faces[c.face].surface,...c.uv.map(([a,b])=>a/2+b/2) as [number,number]).point as [number,number,number]}))
 emit('state',true,{line:[r.origin as [number,number,number],r.origin.map((v,k)=>v+r.direction[k]) as [number,number,number]],marks,proven:r.proven})
})
onUnmounted(()=>{worker.dispose();emit('state',false,null)})
</script>
<template>
 <button type="button" :aria-pressed="enabled" @click="enabled=!enabled">{{label('Проверка материала вдоль линии','Material along a line')}}</button>
 <fieldset v-if="enabled" class="material-path-check" :aria-label="label('Материал вдоль линии','Material along a line')" @keydown.esc.stop="enabled=false" @keydown.enter.prevent.stop="run">
  <legend>{{label('Материал вдоль линии','Material along a line')}}</legend>
  <label>{{label('Проверка','Check')}}<select v-model="mode" :aria-label="label('Режим проверки материала','Material check mode')"><option value="chord">{{label('Между гранями','Between faces')}}</option><option value="segment">{{label('Весь отрезок внутри','Entire segment inside')}}</option></select></label>
  <small>{{mode==='chord'?label('Начните снаружи тела и проведите линию через одну стенку.','Start outside the body and pass the line through one wall.'):label('Укажите начало внутри материала и смещение до конца отрезка.','Set a start inside material and displacement to the segment end.')}}</small>
  <div v-for="(values,name) in {origin,direction}" :key="name" class="coordinates"><span>{{name==='origin'?label('Начало, мм','Start, mm'):label('Смещение, мм','Displacement, mm')}}</span><label v-for="(axis,k) in ['X','Y','Z']" :key="axis">{{axis}}<input v-model.number="values[k]" type="number" step="any" :aria-label="label(name==='origin'?'Начало ':'Смещение ',name==='origin'?'Start ':'Displacement ')+axis" :aria-invalid="!Number.isFinite(values[k]) || name==='direction' && direction.every(x=>x===0)" aria-describedby="material-path-error"></label></div>
  <label><input v-model="extended" type="checkbox">{{label('Расширенный расчёт','Extended calculation')}}</label>
  <button type="button" @click="run">{{result||error?label('Повторить проверку','Retry check'):label('Проверить','Check')}} · Enter</button>
  <p v-if="pending" role="status">{{label('Проверяю материал…','Checking material…')}}</p>
  <p v-if="error" id="material-path-error" role="alert">{{error}}</p>
  <template v-if="result">
   <p role="status" :data-material-proven="result.proven">{{message}}</p>
   <output v-if="result.method==='continuous-material-chord'&&result.lengthIntervalMm" data-material-length>{{result.lengthIntervalMm.map(x=>Number(x.toPrecision(12))).join(' … ')}} mm</output>
   <small v-if="boundary">{{label('Пересечения','Crossings')}}: {{boundary.contacts.length}} · {{label('Непроверенные участки','Unchecked regions')}}: {{boundary.unresolved.length}}</small>
   <ul v-if="issues.length"><li v-for="(c,i) in issues.slice(0,16)" :key="i">{{label('Грань','Face')}} {{c.face+1}} · {{c.unresolved?label('не проверена','unchecked'):label('пересечение','crossing')}}</li></ul>
  </template>
  <small>{{label('Длина вдоль заданной линии. Для минимальной толщины нужна отдельная проверка направления и всей стенки.','Length along the specified line. Minimum thickness requires a separate check of direction and the entire wall.')}}</small>
  <button type="button" @click="enabled=false">{{label('Закрыть','Close')}} · Esc</button>
 </fieldset>
</template>
<style scoped>
button,input,select{font:inherit;color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:5px;padding:6px;min-width:0}button{cursor:pointer}button:focus-visible,input:focus-visible,select:focus-visible{outline:2px solid var(--accent);outline-offset:2px}.material-path-check{display:grid;gap:8px;min-width:0;border:1px solid var(--border);border-radius:5px}.coordinates{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:6px}.coordinates>span{grid-column:1/-1}.coordinates label{display:grid;gap:4px}.coordinates input{width:100%;box-sizing:border-box}small{color:var(--text-dim);line-height:1.5}p{margin:0}ul{margin:0;padding-inline-start:18px}output{overflow-wrap:anywhere}
</style>
