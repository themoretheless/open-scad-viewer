<script setup lang="ts">
import {computed,onUnmounted,ref,shallowRef,toRaw,watch} from 'vue'
import type {NurbsSurface} from '../services/nurbsSurface'
import {createNurbsSurfaceEvaluator,extractNurbsSurfaceIsocurvePoints} from '../services/nurbsSurface'
import type {SourceGraphPreview} from '../services/sourceGraphG2Preview'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
import CadQuantityInput from './CadQuantityInput.vue'
type Point=[number,number,number]
const props=defineProps<{surfaces:[NurbsSurface,NurbsSurface];active:boolean;ru:boolean}>()
const emit=defineEmits<{display:[{lines:Point[][];witness:Point|null}|null]}>()
const label=(ru:string,en:string)=>props.ru?ru:en
const opened=ref(false),width=ref(.25),radius=ref(1),tolerance=ref(.001),valid=ref<Record<string,boolean>>({})
const pending=ref(false),error=ref(''),result=shallowRef<SourceGraphPreview|null>(null)
const worker=createSolidPreviewWorker();let generation=0
function cancel(){generation++;worker.cancel();pending.value=false;result.value=null;error.value='';emit('display',null)}
watch(()=>[props.active,props.surfaces,width.value,radius.value,tolerance.value,opened.value,valid.value],cancel,{deep:true,flush:'sync'})
onUnmounted(()=>{cancel();worker.dispose()})
const canRun=computed(()=>props.active&&opened.value&&!pending.value&&Object.values(valid.value).every(Boolean)&&width.value>0&&width.value<=1&&Number.isFinite(radius.value)&&radius.value>0&&Number.isFinite(tolerance.value)&&tolerance.value>=0)
async function run(){
 if(!canRun.value)return
 cancel();const token=generation;pending.value=true
 try{
  const sources=structuredClone(props.surfaces.map(s=>toRaw(s))) as [NurbsSurface,NurbsSurface]
  const a=sources[0],lo=a.knotsU[a.degreeU],hi=a.knotsU[a.controlPoints.length]
  const value=await worker.run({kind:'sourceGraphG2Preview',sources,width:{degree:1,knots:[lo,lo,hi,hi],controlPoints:[[width.value,0,0],[width.value,0,0]],weights:[1,1]},options:{radiusMm:radius.value,radiusToleranceMm:tolerance.value}})
  if(token!==generation)return
  const p=value.patch,lines:Point[][]=[]
  for(const axis of ['u','v'] as const){
   const knots=axis==='u'?p.knotsU:p.knotsV,degree=axis==='u'?p.degreeU:p.degreeV,count=axis==='u'?p.controlPoints.length:p.controlPoints[0].length
   for(let i=0;i<=4;i++)lines.push(extractNurbsSurfaceIsocurvePoints(p,axis,knots[degree]+(knots[count]-knots[degree])*i/4,17))
  }
  let witness:Point|null=null
  const uv=value.report.radiusWitnessUv
  if(uv){
   const u=p.knotsU[p.degreeU]+(p.knotsU[p.controlPoints.length]-p.knotsU[p.degreeU])*(uv[0][0]+uv[0][1])/2
   const v=p.knotsV[p.degreeV]+(p.knotsV[p.controlPoints[0].length]-p.knotsV[p.degreeV])*(uv[1][0]+uv[1][1])/2
   const evaluate=createNurbsSurfaceEvaluator(p)
   try{witness=evaluate(u,v).point as Point}finally{evaluate.dispose()}
  }
  result.value=value;emit('display',{lines,witness})
 }catch(e){if(token===generation){
   const message=e instanceof Error?e.message:String(e)
   error.value=message.includes('required physical graph')
    ?label('Эта пара поверхностей пока не поддерживается. Выберите другую пару или измените порядок выбора.','This surface pair is not supported yet. Select another pair or reverse the selection order.')
    :label('Не удалось построить переход. Проверьте порядок поверхностей и ширину. ','Could not build the blend. Check the surface order and width. ')+message
  }}
 finally{if(token===generation)pending.value=false}
}
const status=computed(()=>{
 const r=result.value?.report;if(!r)return ''
 if(r.curvatureDecision==='exceeds')return label('Кривизна превышает 10 мм⁻¹. Увеличьте ширину перехода.','Curvature exceeds 10/mm. Increase the transition width.')
 if(r.curvatureDecision==='unresolved')return label('Кривизна не подтверждена. Измените ширину и повторите проверку.','Curvature remains unproven. Change the width and retry.')
 if(r.radiusDecision==='exceeds')return label('Целевой радиус не выдержан по всей поверхности. Красная отметка показывает найденное нарушение. Измените ширину или радиус.','The target radius is not maintained over the whole surface. The red marker shows a detected violation. Change the width or radius.')
 if(r.radiusDecision==='unresolved')return label('Радиус не подтверждён. Измените параметры и повторите проверку.','Radius remains unproven. Change the parameters and retry.')
 return label('Preview готов. Полная приёмка тела ещё не выполнена.','Preview ready. Full body acceptance remains incomplete.')
})
</script>
<template>
 <details :open="opened" @toggle="opened=($event.target as HTMLDetailsElement).open" data-testid="source-graph-preview-controls">
  <summary>{{ label('G2-переход между выбранными поверхностями','G2 blend between selected surfaces') }}</summary>
  <div v-if="opened" @keydown.esc.stop.prevent="cancel" @keydown.enter.stop.prevent="run">
   <p>{{ label('Каркас — только предварительный просмотр. Поддерживаются согласованные графовые поверхности; исходная деталь сохраняется.','The wireframe is a preview. Compatible graph supports are supported; the original part is preserved.') }}</p>
   <label>{{ label('Половина ширины, мм','Half width, mm') }}<CadQuantityInput v-model="width" :min="0.000001" :max="1" :locale="ru?'ru':'en'" @validity="valid.width=$event" /></label>
   <label>{{ label('Целевой радиус, мм','Target radius, mm') }}<CadQuantityInput v-model="radius" :min="0.000001" :locale="ru?'ru':'en'" @validity="valid.radius=$event" /></label>
   <label>{{ label('Допуск радиуса, мм','Radius tolerance, mm') }}<CadQuantityInput v-model="tolerance" :min="0" :locale="ru?'ru':'en'" @validity="valid.tolerance=$event" /></label>
   <button type="button" :disabled="!canRun" @click="run">{{ result||error?label('Повторить','Retry'):label('Предпросмотр','Preview') }}</button>
   <button type="button" :disabled="!pending&&!result&&!error" @click="cancel">{{ label('Отмена · Esc','Cancel · Esc') }}</button>
   <output aria-live="polite">{{ pending?label('Проверка…','Checking…'):error||status }}</output>
  </div>
 </details>
</template>
<style scoped>
summary{cursor:pointer}label{display:grid;gap:4px;margin:8px 0}button{margin:8px 6px 8px 0}output{display:block}p{font-size:12px;color:var(--text-muted)}
</style>
