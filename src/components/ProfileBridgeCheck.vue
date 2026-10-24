<script setup lang="ts">
import {computed,ref,toRaw,watch,nextTick} from 'vue'
import type {NurbsSurface} from '../services/nurbsSurface'
import type {ProfileBridgeChordOptions} from '../services/nurbsProfileBridgeChords'
import {useProfileBridgeInspection} from '../services/useProfileBridgeInspection'
const props=defineProps<{surfaces:[NurbsSurface,NurbsSurface];active:boolean;ru:boolean}>()
const label=(ru:string,en:string)=>props.ru?ru:en
const opened=ref(false),axes=ref<[0|1,0|1]>([0,0]),parameters=ref<[number,number]>([0,1]),scales=ref<[number,number]>([1,1])
const sine=ref(1e-4),whole=ref(false),budget=ref(10000)
function range(s:NurbsSurface,axis:number):[number,number]{
 const knots=axis===0?s.knotsU:s.knotsV,degree=axis===0?s.degreeU:s.degreeV
 const count=axis===0?s.controlPoints.length:s.controlPoints[0]?.length
 return [knots[degree],knots[count]]
}
watch(()=>[props.surfaces,axes.value[0],axes.value[1]],()=>{
 parameters.value=[range(props.surfaces[0],axes.value[0])[0],range(props.surfaces[1],axes.value[1])[1]]
},{immediate:true})
// Transport options intentionally contain raw surfaces for structuredClone.
// Track nested reactive source edits separately so a completed or pending
// certificate cannot survive an in-place control-point/weight/knot change.
const sourceRevision=ref(0)
watch(()=>props.surfaces,()=>{sourceRevision.value++},{deep:true,flush:'sync'})
const options=computed<ProfileBridgeChordOptions|undefined>(()=>{
 sourceRevision.value
 if(!opened.value)return undefined
 const free=range(props.surfaces[0],1-axes.value[0]),domain:[ [number,number],[number,number] ]=[free,[0,1]]
 return {surfaces:props.surfaces.map(s=>toRaw(s)) as [NurbsSurface,NurbsSurface],pivots:[[0,0],[0,0]],
  axes:[...axes.value],parameters:[...parameters.value],scales:[...scales.value],pair:[domain,domain],
  maxSineSquared:sine.value,maxControls:100000,maxSpans:100000,
  ...(whole.value?{coverage:{domain,cells:budget.value,spans:100000}}:{})}
})
const active=computed(()=>props.active&&opened.value)
const {result,pending,error,errorCode,run,cancel}=useProfileBridgeInspection(options,active)
const errorHelp=computed(()=>{
 if(error.value.includes('Unknown NURBS operation'))return label('Текущее ядро ещё не поддерживает эту проверку. Обновите приложение после сборки ядра.','The current kernel does not support this check yet. Reload after the kernel build.')
 switch(errorCode.value){
  case 'CAD_REQUEST':return label('Некорректные параметры проверки. Исправьте параметры сечений и повторите.','Invalid inspection parameters. Correct the section parameters and Retry.')
  case 'CAD_TIMEOUT':return label('Расчёт превысил время ожидания. Уменьшите бюджет и повторите проверку.','The calculation timed out. Reduce the work budget and Retry.')
  case 'CAD_STARTUP':case 'CAD_CRASH':return label('Вычислитель остановлен или не запустился. Повторите проверку.','The computation worker stopped or could not start. Retry the inspection.')
  case 'CAD_DISPOSED':return label('Вычислитель закрыт. Сохраните проект и перезагрузите страницу.','The computation worker is closed. Save the project and reload the page.')
  case 'CAD_TRANSPORT':return label('Не удалось передать запрос или получить результат. Повторите проверку.','The request or result could not be transferred. Retry the inspection.')
  case 'CAD_PROTOCOL':return label('Ответ ядра несовместим с этой проверкой. Обновите приложение и повторите.','The kernel response is incompatible with this check. Reload the application and Retry.')
  default:return label('Не удалось проверить переход. Проверьте параметры сечений и общую область профилей, затем повторите.','Transition inspection failed. Check section parameters and the common profile domain, then Retry.')
 }
})
const selectedPair=ref(1)
watch(result,()=>{selectedPair.value=1},{flush:'sync'})
const unresolvedPair=computed(()=>Number.isSafeInteger(selectedPair.value)&&selectedPair.value>0
 ?result.value?.coverage?.unresolvedPairs[selectedPair.value-1]:undefined)
const parameterInputs:HTMLInputElement[]=[],scaleInputs:HTMLInputElement[]=[]
const angleInput=ref<HTMLInputElement|null>(null)
async function open(){opened.value=true;await nextTick();parameterInputs[0]?.focus()}
defineExpose({open})
const invalidParameters=computed(()=>parameters.value.map((x,i)=>{
 const [lo,hi]=range(props.surfaces[i],axes.value[i]);return !Number.isFinite(x)||x<lo||x>hi
}))
const invalidScales=computed(()=>scales.value.map(x=>!Number.isFinite(x)||x===0))
const invalidAngle=computed(()=>!Number.isFinite(sine.value)||sine.value<0||sine.value>=1)
async function start(){
 if(!active.value)return
 const side=invalidParameters.value.findIndex(Boolean),scale=invalidScales.value.findIndex(Boolean)
 const input=side>=0?parameterInputs[side]:scale>=0?scaleInputs[scale]:invalidAngle.value?angleInput.value:null
 if(input){cancel();await nextTick();input.focus();return}
 await run()
}
const cancelButton=ref<HTMLButtonElement|null>(null),runButton=ref<HTMLButtonElement|null>(null),toggleButton=ref<HTMLButtonElement|null>(null)
async function stop(){cancel();await nextTick();if(opened.value)runButton.value?.focus()}
async function close(){opened.value=false;await nextTick();toggleButton.value?.focus()}
watch(pending,async()=>{await nextTick();if(!active.value)return;if(pending.value)cancelButton.value?.focus();else runButton.value?.focus()})
function key(event:KeyboardEvent){
 if(event.key==='Escape'){event.preventDefault();event.stopPropagation();if(pending.value)void stop();else void close()}
 else if(event.key==='Enter'&&!pending.value&&event.target instanceof HTMLInputElement&&event.target.type==='number'){event.preventDefault();void start()}
}
</script>
<template>
 <section class="profile-bridge-check" @keydown="key">
  <button ref="toggleButton" type="button" :aria-expanded="opened" @click="opened=!opened">{{ label('Проверить переход профилей','Inspect profile transition') }}</button>
  <div v-if="opened">
   <p>{{ label('Предложенный переход между сечениями выбранных поверхностей.','Proposed transition between sections of the selected surfaces.') }}</p>
   <p v-if="!props.active" role="status">{{ label('Завершите текущую команду, чтобы проверить переход.','Finish the current command to inspect the transition.') }}</p>
   <div v-for="side in [0,1] as const" :key="side" class="profile-inputs">
    <strong>{{ side===0?'A':'B' }}</strong>
    <label>{{ label('Сечение','Section') }}<select v-model.number="axes[side]" :aria-label="label('Ось сечения ','Section axis ')+(side===0?'A':'B')" :disabled="pending||!active"><option :value="0">U</option><option :value="1">V</option></select></label>
    <label>{{ label('Параметр','Parameter') }}<input v-model.number="parameters[side]" :ref="el=>{parameterInputs[side]=el as HTMLInputElement}" :aria-invalid="invalidParameters[side]||undefined" type="number" step="any" :aria-label="label('Параметр сечения ','Section parameter ')+(side===0?'A':'B')" :disabled="pending||!active"></label>
    <label>{{ label('Масштаб касательной','Tangent scale') }}<input v-model.number="scales[side]" :ref="el=>{scaleInputs[side]=el as HTMLInputElement}" :aria-invalid="invalidScales[side]||undefined" type="number" step="any" :aria-label="label('Масштаб касательной ','Tangent scale ')+(side===0?'A':'B')" :disabled="pending||!active"></label>
   </div>
   <label>{{ label('Допуск sin²','Tolerance sin²') }}<input ref="angleInput" v-model.number="sine" :aria-invalid="invalidAngle||undefined" type="number" min="0" max="0.999999" step="any" :disabled="pending||!active"></label>
   <label><input v-model="whole" type="checkbox" :disabled="pending||!active">{{ label('Уточнить покрытие всей поверхности','Refine whole-surface coverage') }}</label>
   <label v-if="whole">{{ label('Объём расчёта','Calculation budget') }}<select v-model.number="budget" :disabled="pending||!active"><option :value="10000">{{ label('Обычный','Standard') }}</option><option :value="100000">{{ label('Расширенный','Extended') }}</option></select></label>
   <p v-if="invalidParameters.some(Boolean)||invalidScales.some(Boolean)||invalidAngle" role="alert">{{ label('Параметры сечений должны быть в области поверхности, масштабы — ненулевыми, sin² — от 0 включительно до 1 исключительно.','Section parameters must lie in the surface domain, scales must be nonzero, sin² must be at least 0 and less than 1.') }}</p>
   <button v-if="!pending" ref="runButton" type="button" :disabled="!active" @click="start">{{ error?label('Повторить · Enter','Retry · Enter'):label('Проверить · Enter','Inspect · Enter') }}</button>
   <button v-else ref="cancelButton" type="button" @click="stop">{{ label('Отмена · Esc','Cancel · Esc') }}</button>
   <p v-if="pending" role="status">{{ label('Проверяю переход…','Inspecting transition…') }}</p>
   <p v-if="error" role="alert">{{ errorHelp }} <small>{{ errorCode }}</small></p>
   <details v-if="error"><summary>{{ label('Подробности ошибки','Error details') }}</summary>{{ error }}</details>
   <div v-if="result" role="status">
    <p>{{ result.excluded===true?label('Исключение нормальных хорд без разбиения подтверждено.','Normal-chord exclusion without subdivision qualified.'):label('Исключение нормальных хорд без разбиения не подтверждено.','Normal-chord exclusion without subdivision remains unproven.') }}</p>
    <p v-if="result.coverage">{{ result.coverage.completeExclusion?label('Исключение нормальных хорд на всей поверхности подтверждено.','Normal-chord exclusion across the whole surface qualified.'):label('Непроверенные пары участков: ','Unresolved region pairs: ')+result.coverage.unresolvedPairs.length }} · {{ label('Ячейки: ','Cells: ')+result.coverage.cells }}</p>
    <details v-if="result.coverage?.unresolvedPairs.length">
     <summary>{{ label('Непроверенные области перехода','Unresolved transition regions') }}</summary>
     <label>{{ label('Пара участков','Region pair') }}<input v-model.number="selectedPair" type="number" min="1" :max="result.coverage.unresolvedPairs.length" :aria-label="label('Непроверенная пара перехода','Unresolved transition pair')" :aria-invalid="!unresolvedPair||undefined"></label>
     <p v-for="(rectangle,endpoint) in unresolvedPair" :key="endpoint">{{ endpoint===0?'A':'B' }} · s: [{{ rectangle[0].join(', ') }}] · t: [{{ rectangle[1].join(', ') }}]</p>
     <p>{{ label('Это непроверенные параметрические области, а не подтверждённые дефекты. Увеличьте объём расчёта или измените сечения перехода.','These are unresolved parameter regions, not confirmed defects. Increase the calculation budget or change the transition sections.') }}</p>
    </details>
    <p>{{ label('Проверены нормальные хорды предложенного перехода. Толщина, самопересечения и замкнутое тело требуют отдельных проверок.','Normal chords of the proposed transition were checked. Thickness, self-intersections and a closed solid require separate checks.') }}</p>
    <details v-if="result.profilePositionDeviationUpperMm"><summary>{{ label('Отклонение профилей от исходных поверхностей','Profile deviation from source surfaces') }}</summary><p v-for="(bounds,side) in result.profilePositionDeviationUpperMm" :key="side">{{ side===0?'A':'B' }} · XYZ ≤ {{ bounds.join(' / ') }} {{ label('мм','mm') }}</p></details>
   </div>
  </div>
 </section>
</template>
<style scoped>
.profile-inputs{display:flex;flex-wrap:wrap;gap:6px;align-items:center}.profile-inputs input{width:75px}.profile-inputs label{display:flex;align-items:center;justify-content:space-between;gap:4px;width:100%}.profile-bridge-check input[aria-invalid="true"]{outline:1px solid #b84a35}.profile-bridge-check p{font-size:12px;margin:6px 0}.profile-bridge-check label{font-size:12px}.profile-bridge-check{margin-top:6px}
</style>
