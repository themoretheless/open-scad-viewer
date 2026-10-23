<script setup lang="ts">
import {computed,ref,watch} from 'vue'
import type {DirectDocument} from '../services/directModeling'
import {parseDirectDocument} from '../services/directModeling'
import {stringifyMeshJson} from '../services/meshJson'
import {setLiveBodyPattern,detachLiveBodyPattern,type LiveBodyPattern} from '../services/liveBodyPattern'
const props=defineProps<{document:DirectDocument;sourceId:string;locale:string}>()
const emit=defineEmits<{apply:[document:DirectDocument]}>()
const kind=ref<'grid'|'radial'|'mirror'>('grid'),rows=ref(2),columns=ref(3),count=ref(6),spacingX=ref(10),spacingY=ref(10),axis=ref<'x'|'y'|'z'>('z'),center=ref<[number,number,number]>([0,0,0]),error=ref('')
const source=computed(()=>props.document.bodies.find(b=>b.id===props.sourceId))
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
watch(()=>source.value?.livePattern,p=>{error.value='';if(!p)return;kind.value=p.kind;if(p.kind==='grid'){rows.value=p.rows;columns.value=p.columns;[spacingX.value,spacingY.value]=p.spacing}else{axis.value=p.axis;center.value=[...p.center];if(p.kind==='radial')count.value=p.count}},{immediate:true})
function apply(action:'set'|'remove'|'detach'){
 error.value=''
 try{
  const pattern:LiveBodyPattern=kind.value==='grid'?{kind:'grid',rows:rows.value,columns:columns.value,spacing:[spacingX.value,spacingY.value]}:kind.value==='radial'?{kind:'radial',count:count.value,axis:axis.value,center:[...center.value]}:{kind:'mirror',axis:axis.value,center:[...center.value]}
  const next=action==='detach'?detachLiveBodyPattern(props.document,props.sourceId):setLiveBodyPattern(props.document,props.sourceId,action==='remove'?null:pattern)
  emit('apply',parseDirectDocument(stringifyMeshJson(next)))
 }catch(e){error.value=e instanceof Error?e.message:String(e)}
}
</script>
<template>
 <fieldset v-if="source&&!source.instance" class="live-pattern modeling-feature">
  <legend>{{ label('Живой массив','Live pattern') }}</legend>
  <label>{{ label('Тип','Type') }} <select v-model="kind"><option value="grid">Grid XY</option><option value="radial">Radial</option><option value="mirror">Mirror</option></select></label>
  <template v-if="kind==='grid'">
   <label>{{ label('Ряды','Rows') }} <input v-model.number="rows" type="number" min="1" max="256"></label>
   <label>{{ label('Колонки','Columns') }} <input v-model.number="columns" type="number" min="1" max="256"></label>
   <label>{{ label('Шаг X, мм','X spacing, mm') }} <input v-model.number="spacingX" type="number"></label>
   <label>{{ label('Шаг Y, мм','Y spacing, mm') }} <input v-model.number="spacingY" type="number"></label>
  </template>
  <template v-else>
   <label v-if="kind==='radial'">{{ label('Количество, включая источник','Count, including source') }} <input v-model.number="count" type="number" min="2" max="256"></label>
   <label>{{ label(kind==='mirror'?'Нормаль плоскости':'Ось','Axis / plane normal') }} <select v-model="axis"><option>x</option><option>y</option><option>z</option></select></label>
   <label v-for="(name,i) in ['X','Y','Z']" :key="name">{{ name }}, mm <input v-model.number="center[i]" type="number"></label>
  </template>
  <button @click="apply('set')">{{ label('Применить массив','Apply pattern') }}</button>
  <template v-if="source.livePattern"><button @click="apply('remove')">{{ label('Удалить массив','Remove pattern') }}</button><button @click="apply('detach')">{{ label('Отделить все копии','Detach all copies') }}</button></template>
  <p v-if="error" role="alert">{{ error }}</p>
 </fieldset>
</template>
<style scoped>
.live-pattern{display:grid;gap:6px;border:0;padding:0;margin:8px 0}.live-pattern label{display:flex;justify-content:space-between;gap:8px}.live-pattern input{width:80px}.live-pattern input,.live-pattern select,.live-pattern button{font:inherit;color:var(--text);background:var(--surface);border:1px solid var(--border)}.live-pattern p{color:var(--danger)}
</style>
