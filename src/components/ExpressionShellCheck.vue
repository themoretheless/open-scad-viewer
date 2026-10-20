<script setup lang="ts">
import {computed,toRef,watch,ref,nextTick,onMounted,onUnmounted,useId} from 'vue'
import type {ExpressionInspectionOptions} from '../services/expressionShellInspection'
import type {ExpressionDisplayReport} from '../services/expressionDisplay'
import type {BoundaryMeshReport} from '../services/expressionBoundaryMesh'
import type {ExpressionMeshReport} from '../services/expressionMesh'
import {useExpressionInspection} from '../services/useExpressionInspection'
import {alignedChordParameterErrors} from '../services/expressionAlignedChords'
const props=defineProps<{options?:ExpressionInspectionOptions;active:boolean;ru:boolean}>()
const emit=defineEmits<{radiusFaces:[faces:number[]];boundaryMesh:[mesh:BoundaryMeshReport|null];faces:[faces:number[]];display:[display:ExpressionDisplayReport|null];mesh:[mesh:ExpressionMeshReport|null];meshRegion:[region:ExpressionMeshReport['pending'][number]|null]}>()
const requestBoundaryMesh=ref(false),showBoundaryMesh=ref(false),requestRefinement=ref(false)
const requestMesh=ref(false),showMesh=ref(false)
const requestSupportTangency=ref(false),supportTangencyBudget=ref('saved')
const requestNeighborTangency=ref(false),neighborTangencyBudget=ref('saved')
const requestAlignedChords=ref(false),alignedChordBudget=ref('saved')
const alignedChordMinimum=ref(props.options?.alignedChordCheck?.minimumMm??1)
const alignedChordTolerance=ref(props.options?.alignedChordCheck?.toleranceUv??0.05)
const alignedChordAngle=ref(props.options?.alignedChordCheck?.maxSineSquared??1e-4)
watch(()=>[props.options?.alignedChordCheck,alignedChordBudget.value],()=>{
 const saved=props.options?.alignedChordCheck
 if(alignedChordBudget.value==='saved'&&saved){
  alignedChordMinimum.value=saved.minimumMm
  alignedChordTolerance.value=saved.toleranceUv
  alignedChordAngle.value=saved.maxSineSquared
 }
},{flush:'sync'})
const alignedChordSettings=computed(()=>{
 if(!requestAlignedChords.value&&!props.options?.alignedChordCheck)return undefined
 if(alignedChordBudget.value==='saved'&&props.options?.alignedChordCheck)return props.options.alignedChordCheck
 return {minimumMm:alignedChordMinimum.value,maxSineSquared:alignedChordAngle.value,toleranceUv:alignedChordTolerance.value,controls:1000,
  cells:alignedChordBudget.value==='extended'?100000:10000,
  spans:alignedChordBudget.value==='extended'?1000000:100000,
  domainCells:alignedChordBudget.value==='extended'?1000000:100000}
})
const meshBudget=ref('standard')
const selectedMeshRegion=ref(0),showMeshRegion=ref(false)
const inspectionOptions=computed(()=>props.options?{...props.options,...(requestNeighborTangency.value||neighborTangencyBudget.value!=='saved'?{neighborTangencyCheck:neighborTangencyBudget.value==='extended'?{controls:1000,cells:100000,spans:1000000,maxSineSquared:props.options.neighborTangencyCheck?.maxSineSquared??1e-6}:neighborTangencyBudget.value==='standard'?{controls:1000,cells:10000,spans:100000,maxSineSquared:props.options.neighborTangencyCheck?.maxSineSquared??1e-6}:props.options.neighborTangencyCheck??{controls:1000,cells:10000,spans:100000,maxSineSquared:1e-6}}:{}),...(requestSupportTangency.value||supportTangencyBudget.value!=='saved'?{supportTangencyCheck:supportTangencyBudget.value==='extended'?{controls:1000,cells:100000,spans:1000000}:supportTangencyBudget.value==='standard'?{controls:1000,cells:10000,spans:100000}:props.options.supportTangencyCheck??{controls:1000,cells:10000,spans:100000}}:{}),...(alignedChordSettings.value?{alignedChordCheck:alignedChordSettings.value}:{}),displayLimits:props.options.displayLimits??{
 divisions:4,cells:10000,controlsPerEnclosure:1000,spans:10000,toleranceUv:1e-8,domainCells:10000,domainCellsPerCell:256,
},...(requestBoundaryMesh.value?{boundaryMeshLimits:{maxDeviationMm:0.05,cells:4096,spans:100000,maxDepth:24,...(requestRefinement.value?{refinement:{maxDeviationMm:0.05,cells:512,spans:20000,spansPerQuery:64,controls:1000,domainCells:10000,domainCellsPerTriangle:128,toleranceUv:1e-8,maxDepth:8,regionalSpans:10000}}:{})}}:{}),...(requestMesh.value?{meshLimits:{maxDeviationMm:0.05,cells:meshBudget.value==='extended'?32768:8192,controlsPerEnclosure:1000,spans:100000,toleranceUv:1e-8,domainCells:meshBudget.value==='extended'?100000:30000,domainCellsPerCell:256,maxDepth:meshBudget.value==='extended'?18:14}}:{})}:undefined)
const {result,pending,error,errorCode,run,cancel}=useExpressionInspection(inspectionOptions,toRef(props,'active'))
const errorHint=computed(()=>{
 switch(errorCode.value){
  case 'CAD_REQUEST':return label('Некорректные параметры проверки. Исправьте их и запустите расчёт снова.','Invalid inspection parameters. Correct them and run the calculation again.')
  case 'CAD_TIMEOUT':return label('Расчёт превысил время ожидания. Уменьшите бюджет и повторите проверку.','The calculation timed out. Reduce the work budget and retry.')
  case 'CAD_STARTUP':case 'CAD_CRASH':return label('Вычислитель остановлен или не запустился. Повторите проверку.','The computation worker stopped or could not start. Retry the inspection.')
  case 'CAD_DISPOSED':return label('Вычислитель закрыт. Сохраните проект и перезагрузите страницу.','The computation worker is closed. Save the project and reload the page.')
  case 'CAD_PROTOCOL':return label('Ответ расчёта не прошёл проверку. Повторите проверку.','The calculation response failed validation. Retry the inspection.')
  case 'CAD_TRANSPORT':return label('Не удалось передать запрос или получить результат. Повторите проверку.','The request or result could not be transferred. Retry the inspection.')
  default:return label('Расчёт геометрии не выполнен. Проверьте исходную геометрию и параметры.','The geometry calculation failed. Check the source geometry and parameters.')
 }
})
const parameterErrorId=useId()
const chordParameterErrors=computed(()=>alignedChordParameterErrors(alignedChordSettings.value))
const chordMinimumInput=ref<HTMLInputElement|null>(null),chordToleranceInput=ref<HTMLInputElement|null>(null),chordAngleInput=ref<HTMLInputElement|null>(null)
const chordBudgetInput=ref<HTMLSelectElement|null>(null)
async function startInspection(){
 if(!props.active || !props.options || pending.value)return
 if(alignedChordSettings.value){
  const errors=chordParameterErrors.value
  const field=errors.minimumMm?chordMinimumInput:errors.toleranceUv?chordToleranceInput:errors.maxSineSquared?chordAngleInput:null
  if(field){
   cancel()
   await nextTick()
   if(alignedChordBudget.value==='saved'&&props.options?.alignedChordCheck)chordBudgetInput.value?.focus()
   else field.value?.focus()
   return
  }
 }
 await run()
}

const cancelButton=ref<HTMLButtonElement|null>(null)
const errorRetryButton=ref<HTMLButtonElement|null>(null)
watch(error,async value=>{
 if(!value)return
 await nextTick()
 if(props.active && props.options && !pending.value && error.value===value)errorRetryButton.value?.focus()
},{flush:'post'})
function cancelPendingOnEscape(event:KeyboardEvent){
 if(event.key==='Escape'&&props.active&&pending.value){
  event.preventDefault()
  event.stopPropagation()
  cancel()
 }
}
onMounted(()=>window.addEventListener('keydown',cancelPendingOnEscape,{capture:true}))
onUnmounted(()=>window.removeEventListener('keydown',cancelPendingOnEscape,{capture:true}))
watch(pending,async value=>{
 if(!value)return
 await nextTick()
 if(pending.value)cancelButton.value?.focus()
},{flush:'sync'})
const showDisplay=ref(false),showRadiusViolation=ref(false)
const display=computed(()=>result.value?.shell?.supportEnclosures)
const mesh=computed(()=>result.value?.shell?.displayMesh)
const boundaryMesh=computed(()=>result.value?.shell?.boundaryMesh)
const alignedChords=computed(()=>result.value?.shell?.alignedChordCheck)
const chordBudgetHint=computed(()=>{
 const report=alignedChords.value?.report,limits=alignedChordSettings.value
 if(report&&limits&&report.cells>=limits.cells&&limits.cells>=100000)
  return label('Максимальный бюджет ячеек исчерпан. Просмотрите неподтверждённые области; порог для них не доказан.','Maximum cell budget exhausted. Review the unverified regions; their threshold is unproven.')
 if(report&&limits&&report.spans>=limits.spans&&limits.spans>=1000000)
  return label('Максимальный бюджет вычислений исчерпан. Просмотрите неподтверждённые области; порог для них не доказан.','Maximum span budget exhausted. Review the unverified regions; their threshold is unproven.')
 if(report&&limits&&report.domain_cells>=limits.domainCells&&limits.domainCells>=1000000)
  return label('Максимальный бюджет проверки обрезки исчерпан. Просмотрите неподтверждённые области; порог для них не доказан.','Maximum trim-domain budget exhausted. Review the unverified regions; their threshold is unproven.')
 return label('Исчерпан бюджет расчёта. Выберите расширенный бюджет и повторите проверку.','Calculation budget exhausted. Choose extended work and inspect again.')
})
const selectedChordRegion=ref(1)
const chordRegion=computed(()=>Number.isSafeInteger(selectedChordRegion.value)&&selectedChordRegion.value>0?alignedChords.value?.report.pending[selectedChordRegion.value-1]:undefined)
watch(()=>[result.value,pending.value],()=>{selectedChordRegion.value=1},{flush:'sync'})
const radiusCheck=computed(()=>result.value?.shell?.radiusCheck)
const neighborTangency=computed(()=>result.value?.shell?.neighborTangencyCheck)
const supportTangency=computed(()=>result.value?.shell?.supportTangencyCheck)
watch(()=>[result.value,pending.value,showBoundaryMesh.value],()=>emit('boundaryMesh',showBoundaryMesh.value&&!pending.value?boundaryMesh.value??null:null),{flush:'sync'})
watch(()=>[result.value,pending.value,showMesh.value],()=>emit('mesh',showMesh.value&&!pending.value?mesh.value??null:null),{flush:'sync'})
watch(()=>[result.value,pending.value],()=>{selectedMeshRegion.value=0;showMeshRegion.value=false},{flush:'sync'})
watch(()=>[mesh.value,pending.value,selectedMeshRegion.value,showMeshRegion.value],()=>emit('meshRegion',showMeshRegion.value&&!pending.value?mesh.value?.pending[selectedMeshRegion.value]??null:null),{flush:'sync'})
const radiusViolationFaces=computed(()=>radiusCheck.value?.violationQualified?
 result.value?.shell?.faces.filter(f=>f.kind==='transition').map(f=>f.id)??[]:[])
watch(()=>[result.value,pending.value,showRadiusViolation.value],()=>emit('radiusFaces',!pending.value&&showRadiusViolation.value?radiusViolationFaces.value:[]),{flush:'sync'})
watch(()=>[result.value,pending.value,showDisplay.value,showRadiusViolation.value],()=>{
 const available=display.value
 const focused=showRadiusViolation.value&&radiusViolationFaces.value.length>0
 emit('display',pending.value||!available?null:showDisplay.value?available:focused?{...available,cells:available.cells.filter(c=>radiusViolationFaces.value.includes(c.face))}:null)
},{flush:'sync'})
const label=(ru:string,en:string)=>props.ru?ru:en
function meshReason(reason:string){
 const reasons:Record<string,[string,string]>={
  'face-boundary-constraints-incomplete':['Границы грани проверены не полностью','Face boundaries are incomplete'],
  'face-triangulation-proposal-refused':['Не удалось построить сетку без пересечения границ','Boundary-preserving triangulation refused'],
  'face-triangulation-work-limit':['Исчерпан бюджет триангуляции','Triangulation budget exhausted'],
  'face-triangulation-vertex-identity-unproven':['Не подтверждено соответствие вершин','Vertex identity unproven'],
  'face-triangulation-boundary-incidence-unproven':['Не подтверждена связность границ сетки','Boundary incidence unproven'],
  'face-triangulation-orientation-unproven':['Не подтверждена ориентация сетки','Mesh orientation unproven'],
  'boundary-shared-endpoint-enclosure-conflict':['Расчёты общей вершины не согласованы','Shared vertex enclosures conflict'],
  'boundary-shared-rounding-error-limit':['Ошибка общей вершины превышает допуск','Shared vertex rounding exceeds tolerance'],
  'boundary-mesh-work-limit':['Исчерпан бюджет разбиения ребра','Edge partition budget exhausted'],
  'boundary-mesh-depth-limit':['Достигнута глубина разбиения ребра','Edge partition depth exhausted'],
  'boundary-mesh-end-order-unproven':['Не подтверждён порядок концов ребра','Edge endpoint order unproven'],
  'boundary-mesh-parameter-resolution':['Недостаточна точность параметра ребра','Edge parameter resolution exhausted'],
  'boundary-endpoint-work-limit':['Исчерпан бюджет проверки конца ребра','Endpoint budget exhausted'],
  'boundary-endpoint-enclosure-unproven':['Не подтверждены координаты конца ребра','Endpoint coordinates unproven'],
  'boundary-endpoint-source-map-unproven':['Не подтверждено отображение конца ребра','Endpoint source map unproven'],
  'boundary-sample-work-limit':['Исчерпан бюджет проверки ребра','Edge sample budget exhausted'],
  'boundary-sample-source-map-unproven':['Не подтверждено отображение ребра','Edge source map unproven'],
  'boundary-sample-retained-interior-unproven':['Не подтверждена принадлежность обрезанному ребру','Retained edge membership unproven'],
  'triangle-deviation-limit':['Достигнута глубина уточнения допуска','Tolerance refinement depth exhausted'],
  'triangle-exact-trim-unproven':['Не подтверждена точная принадлежность грани','Exact face membership unproven'],
  'triangle-midpoint-numeric-unproven':['Не подтверждены координаты внутренней вершины','Interior vertex coordinates unproven'],
  'triangle-outside-material':['Участок вне материала сохранён для диагностики','Outside-material region retained for diagnostics'],
  'triangle-parameter-resolution':['Недостаточна точность параметров треугольника','Triangle parameter resolution exhausted'],
  'triangle-refinement-work-limit':['Исчерпан бюджет уточнения','Refinement budget exhausted'],
  'triangle-support-numeric-unproven':['Не подтверждена численная оценка поверхности','Numeric surface enclosure unproven'],
  'triangle-cell-work-limit':['Исчерпан бюджет треугольников','Triangle work budget exhausted'],
  'triangle-span-work-limit':['Исчерпан бюджет оценки поверхности','Surface evaluation budget exhausted'],
  'triangle-trim-work-limit':['Исчерпан бюджет проверки обрезки','Trim evaluation budget exhausted'],
  'triangle-span-trim-work-limit':['Исчерпаны бюджеты поверхности и обрезки','Surface and trim budgets exhausted'],
  'triangle-cell-span-work-limit':['Исчерпаны бюджеты треугольников и поверхности','Triangle and surface budgets exhausted'],
  'triangle-cell-trim-work-limit':['Исчерпаны бюджеты треугольников и обрезки','Triangle and trim budgets exhausted'],
  'triangle-cell-span-trim-work-limit':['Исчерпаны все бюджеты уточнения','All refinement budgets exhausted'],
  'triangle-query-disabled':['Проверка отключена бюджетом','Query disabled by budget'],
  'mesh-work-limit':['Исчерпан бюджет расчёта','Calculation budget exhausted'],
  'mesh-support-unproven':['Не подтверждена область поверхности','Surface enclosure unproven'],
  'mesh-trim-unproven':['Не подтверждена граница обрезки','Trim boundary unproven'],
  'mesh-deviation-unproven':['Не подтверждён допуск сетки','Mesh tolerance unproven'],
  'mesh-vertex-unproven':['Не подтверждены координаты вершины','Vertex coordinates unproven'],
 }
 const text=reasons[reason]
 return text?label(...text):reason
}
const inspection=computed(()=>result.value?.shell?.inspection)
const volumeLabel=computed(()=>inspection.value?.volume?.positiveBounds?.map(v=>v.toLocaleString(props.ru?'ru-RU':'en-US',{maximumFractionDigits:3})).join(' … '))
const unresolved=computed(()=>inspection.value?.contacts?.pairs.filter(p=>p.proof==='pending')??[])
const selectedPair=ref(0)
const selectedContactRegion=ref(1)
const contactPair=computed(()=>Number.isSafeInteger(selectedPair.value)&&selectedPair.value>=0?unresolved.value[selectedPair.value]:undefined)
const contactRegion=computed(()=>Number.isSafeInteger(selectedContactRegion.value)&&selectedContactRegion.value>0?contactPair.value?.pendingProducts[selectedContactRegion.value-1]:undefined)
watch(()=>[result.value,pending.value,selectedPair.value],()=>{selectedContactRegion.value=1},{flush:'sync'})
watch(()=>[result.value,pending.value,props.options],()=>{selectedPair.value=0;emit('faces',[])},{flush:'sync'})
watch(()=>[selectedChordRegion.value,selectedPair.value],()=>emit('faces',[]),{flush:'sync'})
function sourceFaces(faces:number[]){
 const root=(props.options?.boundaryDefinition as {sides?:{ends?:{face?:number}[]}[]})?.sides?.[0]?.ends?.[0]?.face
 const original=faces.flatMap(id=>{
  const face=result.value?.shell?.faces[id]
  const source=face?.kind==='original'?face.source:face?.kind==='retained'?root:undefined
  return typeof source==='number'&&Number.isSafeInteger(source)&&source>=0?[source]:[]
 })
 return [...new Set(original)]
}
function selectPair(faces:number[]){emit('faces',sourceFaces(faces))}
const chordSourceFaces=computed(()=>sourceFaces(chordRegion.value?.faces??[]))
const chordHasTransition=computed(()=>chordRegion.value?.faces.some(id=>result.value?.shell?.faces[id]?.kind==='transition')??false)
const contactSourceFaces=computed(()=>sourceFaces(contactPair.value?.faces??[]))
const contactHasTransition=computed(()=>contactPair.value?.faces.some(id=>result.value?.shell?.faces[id]?.kind==='transition')??false)
</script>
<template>
 <details class="expression-shell-check" @keydown.esc.stop.prevent="cancel" @toggle="!($event.target as HTMLDetailsElement).open && cancel()">
  <summary>{{ label('Переход: проверка оболочки','Transition shell inspection') }}</summary>
  <p>{{ label('Сцена показывает исходное тело. Этот расчёт проверяет сохранённый рецепт предложенного перехода.','The scene shows the original body. This calculation checks the saved proposed transition recipe.') }}</p>
  <label v-if="!options?.alignedChordCheck"><input v-model="requestAlignedChords" type="checkbox" :disabled="pending">{{ label('Проверить расстояния вдоль нормалей','Check distances along normals') }}</label>
  <template v-if="requestAlignedChords || options?.alignedChordCheck">
   <label>{{ label('Порог расстояния, мм','Distance threshold, mm') }}<input ref="chordMinimumInput" v-model.number="alignedChordMinimum" :aria-invalid="chordParameterErrors.minimumMm || undefined" :aria-describedby="chordParameterErrors.minimumMm?parameterErrorId+'-minimum':undefined" type="number" min="0" step="any" :disabled="pending || (alignedChordBudget==='saved' && !!options?.alignedChordCheck)" :aria-label="label('Порог расстояния, мм','Distance threshold, mm')"><span v-if="chordParameterErrors.minimumMm" :id="parameterErrorId+'-minimum'" class="parameter-error" role="alert" data-chord-parameter-error="minimumMm">{{ label('Введите конечное число больше 0.','Enter a finite number greater than 0.') }}</span></label>
   <label>{{ label('Точность разбиения UV','UV subdivision tolerance') }}<input ref="chordToleranceInput" v-model.number="alignedChordTolerance" :aria-invalid="chordParameterErrors.toleranceUv || undefined" :aria-describedby="chordParameterErrors.toleranceUv?parameterErrorId+'-uv':undefined" type="number" min="0" max="1" step="any" :disabled="pending || (alignedChordBudget==='saved' && !!options?.alignedChordCheck)" :aria-label="label('Точность разбиения UV','UV subdivision tolerance')"><span v-if="chordParameterErrors.toleranceUv" :id="parameterErrorId+'-uv'" class="parameter-error" role="alert" data-chord-parameter-error="toleranceUv">{{ label('Введите конечное число больше 0 и не больше 1.','Enter a finite number greater than 0 and at most 1.') }}</span></label>
   <label>{{ label('Допуск sin² направления нормалей','Normal-direction squared sine tolerance') }}<input ref="chordAngleInput" v-model.number="alignedChordAngle" :aria-invalid="chordParameterErrors.maxSineSquared || undefined" :aria-describedby="chordParameterErrors.maxSineSquared?parameterErrorId+'-angle':undefined" type="number" min="0" max="1" step="any" :disabled="pending || (alignedChordBudget==='saved' && !!options?.alignedChordCheck)" :aria-label="label('Допуск sin² направления нормалей','Normal-direction squared sine tolerance')"><span v-if="chordParameterErrors.maxSineSquared" :id="parameterErrorId+'-angle'" class="parameter-error" role="alert" data-chord-parameter-error="maxSineSquared">{{ label('Введите конечное число от 0 до 1, не включая 1.','Enter a finite number from 0 up to, but excluding, 1.') }}</span></label>
   <label>{{ label('Бюджет проверки расстояний','Distance check budget') }}<select ref="chordBudgetInput" v-model="alignedChordBudget" :disabled="pending" :aria-label="label('Бюджет проверки расстояний','Distance check budget')">
    <option value="saved">{{ label('Сохранённый или стандартный','Saved or standard') }}</option><option value="standard">{{ label('Стандартный','Standard') }}</option><option value="extended">{{ label('Расширенный','Extended') }}</option>
   </select></label>
   <p>{{ label('Проверяется весь диапазон граней при заданном допуске направления нормалей. Минимальная толщина тела требует отдельного подтверждения.','The full face domains are checked at the specified normal-direction tolerance. Minimum body thickness requires separate qualification.') }}</p>
  </template>
  <p v-if="options?.radiusCheck">{{ label('Проверяется исходный закон радиуса. Допуск','The original radius law will be checked. Tolerance') }}: {{ options.radiusCheck.toleranceMm }} mm</p>
  <label v-if="!options?.neighborTangencyCheck"><input v-model="requestNeighborTangency" type="checkbox" :disabled="pending">{{ label('Проверить касательность соседних граней','Check neighboring face tangency') }}</label>
  <p v-if="options?.neighborTangencyCheck">{{ label('Сохранённая проверка соседних граней будет выполнена.','The saved neighboring-face check will run.') }}</p>
  <p v-if="inspectionOptions?.neighborTangencyCheck">{{ label('Допуск sin² угла соединения','Join squared angle sine tolerance') }}: {{ inspectionOptions.neighborTangencyCheck.maxSineSquared }}</p>
  <label v-if="requestNeighborTangency || options?.neighborTangencyCheck">{{ label('Бюджет соседних граней','Neighbor tangency budget') }}
   <select v-model="neighborTangencyBudget" :disabled="pending" :aria-label="label('Бюджет соседних граней','Neighbor tangency budget')">
    <option value="saved">{{ label('Сохранённый','Saved') }}</option>
    <option value="standard">{{ label('Обычный','Standard') }} · 10000 / 100000</option>
    <option value="extended">{{ label('Расширенный','Extended') }} · 100000 / 1000000</option>
   </select>
  </label>
  <label v-if="!options?.supportTangencyCheck"><input v-model="requestSupportTangency" type="checkbox" :disabled="pending">{{ label('Проверить касательность к исходной поверхности','Check tangency to original support') }}</label>
  <p v-if="options?.supportTangencyCheck">{{ label('Сохранённая проверка касательности к исходной поверхности будет выполнена.','The saved original-support tangency check will run.') }}</p>
  <label v-if="requestSupportTangency || options?.supportTangencyCheck">{{ label('Бюджет касательности','Tangency budget') }}
   <select v-model="supportTangencyBudget" :disabled="pending" :aria-label="label('Бюджет касательности','Tangency budget')">
    <option value="saved">{{ label('Сохранённый','Saved') }}</option>
    <option value="standard">{{ label('Обычный','Standard') }} · 10000 / 100000</option>
    <option value="extended">{{ label('Расширенный','Extended') }} · 100000 / 1000000</option>
   </select>
  </label>
  <label><input v-model="requestBoundaryMesh" type="checkbox" :disabled="pending">{{ label('Рассчитать общую сетку границ','Compute shared boundary mesh') }}</label>
  <label v-if="requestBoundaryMesh"><input v-model="requestRefinement" type="checkbox" :disabled="pending">{{ label('Уточнить допуск треугольников','Refine triangle tolerance') }}</label>
  <label><input v-model="requestMesh" type="checkbox" :disabled="pending">{{ label('Рассчитать preview · допуск 0,05 мм','Compute preview · tolerance 0.05 mm') }}</label>
  <label v-if="requestMesh" class="mesh-budget">{{ label('Бюджет preview','Preview budget') }}
   <select v-model="meshBudget" :disabled="pending" :aria-label="label('Бюджет preview','Preview budget')">
    <option value="standard">{{ label('Обычный','Standard') }} · 8192</option>
    <option value="extended">{{ label('Расширенный','Extended') }} · 32768</option>
   </select>
  </label>
  <button v-if="!pending" key="inspect" type="button" :disabled="!active || !options" @click="startInspection">{{ label('Проверить','Inspect') }}</button>
  <button v-else key="cancel" ref="cancelButton" type="button" @click="cancel">{{ label('Отмена · Esc','Cancel · Esc') }}</button>
  <span v-if="pending" role="status">{{ label('Восстанавливаю оболочку и проверяю контакты…','Rebuilding shell and checking contacts…') }}</span>
  <div v-if="error" role="alert">{{ errorHint }} <button ref="errorRetryButton" type="button" :disabled="!active || !options || pending" @click="startInspection">{{ label('Повторить','Retry') }}</button><details><summary>{{ label('Подробности','Details') }}</summary>{{ error }}</details></div>
  <div v-if="result" :role="result.embeddingQualified?'status':'alert'">
   <p>{{ label('Замкнутость','Closed incidence') }}: {{ result.incidenceQualified?label('подтверждена','qualified'):label('не подтверждена','unproven') }} · {{ result.shell?.faceCount ?? 0 }} {{ label('граней','faces') }}</p>
   <p>{{ label('Контакты граней','Face contacts') }}: {{ inspection?.contacts?.pairs.length ?? 0 }} · {{ label('не проверены','unverified') }}: {{ inspection?.contacts?.pendingPairs ?? '—' }}</p>
   <p>{{ label('Объём','Volume') }}: {{ inspection?.volumeQualified?'≈ '+volumeLabel:label('не подтверждён','unproven') }} <span v-if="inspection?.volumeQualified">{{ label('мм³','mm³') }}</span></p>
   <div v-if="alignedChords" role="status" :data-aligned-chord-state="alignedChords.qualified?'qualified':'unproven'">
    <p>{{ label('Расстояния вдоль нормалей','Distances along normals') }}: {{ alignedChords.qualified?label('порог подтверждён','threshold qualified'):label('проверка неполная','coverage incomplete') }}</p>
    <p>{{ label('Непроверенные области','Pending regions') }}: {{ alignedChords.report.pending.length }} · {{ label('Исходные пары','Initial pairs') }}: {{ alignedChords.report.initial_pairs_checked }} / {{ alignedChords.report.total_pairs }}</p>
    <p>{{ label('Ячейки','Cells') }}: {{ alignedChords.report.cells }} · {{ label('Участки','Spans') }}: {{ alignedChords.report.spans }} · {{ label('Проверки внутри грани','Within-face checks') }}: {{ alignedChords.report.intrinsic_spans }}</p>
    <details v-if="alignedChords.report.span_stages" data-chord-span-stages><summary>{{ label('Затраты по этапам','Work by stage') }}</summary>
     <p>{{ label('Координаты','Positions') }}: {{ alignedChords.report.span_stages.position }}</p>
     <p>{{ label('Нормали','Normals') }}: {{ alignedChords.report.span_stages.normal }}</p>
     <p>{{ label('Общие углы','Shared corners') }}: {{ alignedChords.report.span_stages.shared_corner }}</p>
     <p>{{ label('Пары исходной грани','Original face pairs') }}: {{ alignedChords.report.span_stages.original_pair }}</p>
     <p>{{ label('Пары обрезанной грани','Retained face pairs') }}: {{ alignedChords.report.span_stages.retained_pair }}</p>
     <p>{{ label('Пары переходной грани','Transition face pairs') }}: {{ alignedChords.report.span_stages.corrected_pair }}</p>
     <p>{{ label('Внутри грани','Within a face') }}: {{ alignedChords.report.intrinsic_spans }}</p>
     <p>{{ label('Обрезанная грань и переход','Retained face and transition') }}: {{ alignedChords.report.profile_transition_spans }}</p>
     <p>{{ label('Это число обработанных участков, а не время расчёта.','These counts measure processed spans, not elapsed time.') }}</p>
    </details>
    <label v-if="alignedChords.report.pending.length">{{ label('Непроверенная область','Pending region') }}<input v-model.number="selectedChordRegion" type="number" min="1" :max="alignedChords.report.pending.length" step="1" :aria-label="label('Непроверенная область','Pending region')"></label>
    <p v-if="chordRegion">{{ label('Грани','Faces') }}: {{ chordRegion.faces.map(f=>f+1).join(' · ') }}</p>
    <p v-if="chordRegion">{{ chordRegion.pending_reason==='budget'?chordBudgetHint:chordRegion.pending_reason==='enclosure'?label('Не подтверждены границы поверхности. Проверьте исходную геометрию.','Surface bounds are unproven. Check the source geometry.'):label('Достигнут предел разбиения области. Требуется более точная проверка.','Region subdivision limit reached. A more precise check is required.') }}</p>
    <button v-if="chordRegion" type="button" :disabled="!chordSourceFaces.length" @click="selectPair(chordRegion.faces)">{{ label('Показать исходные грани кандидатов','Show candidate source faces') }}</button>
    <p v-if="chordHasTransition" data-chord-transition-limitation>{{ label('Область включает переходную грань. Подсветка показывает только доступные исходные грани; переход в сцене пока не отображается.','The region includes a transition face. Highlighting shows available source faces only; the transition is not yet displayed in the scene.') }}</p>
    <details v-if="chordRegion?.domains" data-chord-uv-domains><summary>{{ label('UV-интервалы кандидата','Candidate UV intervals') }}</summary>
     <p v-for="(domain,side) in chordRegion.domains" :key="side">{{ label('Сторона','Side') }} {{ side+1 }}: U [{{ domain[0][0] }}, {{ domain[0][1] }}] · V [{{ domain[1][0] }}, {{ domain[1][1] }}]</p>
    </details>
    <p>{{ label('Минимальная толщина всего тела не подтверждена.','Minimum thickness of the whole body is unqualified.') }}</p>
   </div>
   <p v-else-if="inspectionOptions?.alignedChordCheck">{{ label('Проверка расстояний ожидает подтверждения геометрии оболочки.','Distance checks await shell geometry qualification.') }}</p>
   <div v-if="radiusCheck" :role="radiusCheck.violationQualified?'alert':'status'" :data-radius-state="radiusCheck.qualified?'qualified':radiusCheck.violationQualified?'violation':'unproven'">
    <p>{{ label('Радиус перехода','Transition radius') }}: {{ radiusCheck.qualified?label('допуск подтверждён','tolerance qualified'):radiusCheck.violationQualified?label('нарушение допуска подтверждено','tolerance violation qualified'):label('не проверен','unproven') }}</p>
    <p v-if="radiusCheck.errorUpperMm!==null">{{ label('Границы ошибки радиуса','Radial error bounds') }}: {{ radiusCheck.errorLowerMm ?? 0 }} … {{ radiusCheck.errorUpperMm }} mm</p>
    <label v-if="radiusCheck.violationQualified && display"><input v-model="showRadiusViolation" type="checkbox">{{ label('Показать области нарушения радиуса','Show radial mismatch regions') }}</label>
    <p v-if="radiusCheck.violationQualified">{{ label('Проверьте закон радиуса и форму перехода перед применением.','Check the radius law and transition shape before applying.') }}</p>
    <p v-else-if="!radiusCheck.qualified">{{ label('Полного доказательства нет. Повторите проверку с достаточным бюджетом.','Complete proof is unavailable. Retry with sufficient work budget.') }}</p>
   </div>
   <div v-if="neighborTangency" role="status" :data-neighbor-tangency-state="neighborTangency.qualified?'qualified':'unproven'">
    <p>{{ label('Касательность соседних граней','Neighboring face tangency') }}: {{ neighborTangency.qualified?label('четыре соединения подтверждены','four joins qualified'):label('не подтверждена','unproven') }}</p>
    <p>{{ label('Ячейки','Cells') }}: {{ neighborTangency.cells }} · {{ label('Участки','Spans') }}: {{ neighborTangency.spans }}</p>
    <p v-if="neighborTangency.upperSineSquared">{{ label('Верхние границы sin² угла','Upper bounds of squared angle sine') }}: {{ neighborTangency.upperSineSquared.join(' · ') }}</p>
    <p v-if="!neighborTangency.qualified">{{ label('Повторите с большим бюджетом или исправьте соединения перехода.','Retry with more work or correct the transition joins.') }}</p>
    <p>{{ label('Толщина, самопересечения и остальные швы требуют отдельных проверок.','Thickness, self-intersections and remaining seams require separate checks.') }}</p>
   </div>
   <div v-if="supportTangency" role="status" :data-support-tangency-state="supportTangency.qualified?'qualified':'unproven'">
    <p>{{ label('Касательность к исходной поверхности','Original support tangency') }}: {{ supportTangency.qualified?label('четыре границы подтверждены','four boundaries qualified'):label('не подтверждена','unproven') }}</p>
    <p>{{ label('Проверено границ','Qualified boundaries') }}: {{ supportTangency.boundaries.filter(b=>b.qualified).length }} / 4 · {{ label('Ячейки','Cells') }}: {{ supportTangency.cells }} · {{ label('Участки','Spans') }}: {{ supportTangency.spans }}</p>
    <p v-if="!supportTangency.qualified">{{ label('Проверка неполная. Требуется дополнительный бюджет или исправление рецепта перехода.','Coverage is incomplete. More work or a corrected transition recipe is required.') }}</p>
    <p v-if="!radiusCheck">{{ label('Радиус требует отдельной проверки.','Radius requires a separate check.') }}</p>
    <p v-if="!neighborTangency">{{ label('Соседние грани тела и толщина требуют отдельных проверок.','Neighboring body faces and thickness require separate checks.') }}</p>
   </div>
   <p v-if="!supportTangency && !neighborTangency">{{ radiusCheck?label('Касательность и толщина требуют отдельных проверок.','Tangency and thickness require separate checks.'):label('Радиус, касательность и толщина требуют отдельных проверок.','Radius, tangency and thickness require separate checks.') }}</p>
   <div v-if="boundaryMesh">
    <label><input v-model="showBoundaryMesh" type="checkbox">{{ label('Показать предварительную общую сетку','Show preliminary shared mesh') }}</label>
    <p>{{ boundaryMesh.candidateTriangles.length }} {{ label('треугольников','triangles') }} · {{ label('Замкнутость сетки','Mesh incidence') }}: {{ boundaryMesh.closedOrientedIncidence?label('подтверждена','qualified'):label('не подтверждена','unproven') }}</p>
    <p>{{ label('Допуск поверхности и обрезка ещё не подтверждены.','Surface tolerance and trim are not yet qualified.') }}</p>
    <p>{{ label('Непроверенные грани','Unverified faces') }}: {{ boundaryMesh.pendingFaces.length }} · {{ label('участки границ','boundary regions') }}: {{ boundaryMesh.pending.length }}</p>
    <button v-for="entry in boundaryMesh.pendingFaces" :key="entry.face" type="button" @click="selectPair([entry.face])">{{ label('Грань','Face') }} {{ entry.face }} · {{ meshReason(entry.reason) }}</button>
    <div v-if="boundaryMesh.refinement">
     <p>{{ label('Подтверждено','Qualified') }}: {{ boundaryMesh.refinement.accepted.length }} · {{ label('Не проверено','Unverified') }}: {{ boundaryMesh.refinement.pending.length }} · {{ label('Численные ошибки','Numeric errors') }}: {{ boundaryMesh.refinement.numericFailures.length }}</p>
     <p>{{ label('Зелёные участки: допуск и принадлежность грани подтверждены. Оранжевые: проверка не завершена. Радиус, касательность и толщина проверяются отдельно.','Green regions meet surface tolerance and face membership. Orange regions remain unproven. Radius, tangency and thickness require separate checks.') }}</p>
     <details v-if="boundaryMesh.refinement.pending.length"><summary>{{ label('Причины непроверенных треугольников','Unverified triangle reasons') }}</summary>
      <p v-for="reason in [...new Set(boundaryMesh.refinement.pending.map(t=>t.reason))]" :key="reason">{{ meshReason(reason) }}: {{ boundaryMesh.refinement.pending.filter(t=>t.reason===reason).length }}</p>
     </details>
    </div>
    <details v-if="boundaryMesh.pending.length"><summary>{{ label('Причины непроверенных границ','Unverified boundary reasons') }}</summary><p v-for="(entry,index) in boundaryMesh.pending" :key="index">{{ label('Ребро','Edge') }} {{ entry.pair }} · {{ meshReason(entry.reason) }}</p></details>
   </div>
   <div v-if="mesh">
    <label><input v-model="showMesh" type="checkbox">{{ label('Показать сетку перехода','Show transition mesh') }}</label>
    <p>{{ mesh.tiles.length*2 }} {{ label('треугольников','triangles') }} · {{ label('не проверены области','unverified regions') }}: {{ mesh.pending.length }}</p>
    <p>{{ label('Граница ошибки','Error upper bound') }}: {{ mesh.maxDeviationUpperMm }} mm · {{ label('Сетка ещё не замкнута.','Mesh is not yet watertight.') }}</p>
    <p v-if="mesh.pending.length">{{ label('Для непроверенных областей выберите расширенный бюджет и повторите проверку.','For unverified regions, choose the extended budget and run inspection again.') }}</p>
    <div v-if="mesh.pending.length" class="pair-controls">
     <label>{{ label('Непроверенная область','Unverified region') }}
      <select v-model.number="selectedMeshRegion" :aria-label="label('Непроверенная область','Unverified region')">
       <option v-for="(region,index) in mesh.pending" :key="index" :value="index">{{ region.face }} · UV {{ region.domain }} · {{ meshReason(region.reason) }}</option>
      </select>
     </label>
     <label><input v-model="showMeshRegion" type="checkbox" :disabled="!mesh.pending[selectedMeshRegion]?.bounds">{{ label('Показать область','Show region') }}</label>
     <p v-if="!mesh.pending[selectedMeshRegion]?.bounds">{{ label('Координаты области не подтверждены; подсветка недоступна.','Region coordinates are unproven; highlighting is unavailable.') }}</p>
    </div>
   </div>
   <div v-if="display">
    <label><input v-model="showDisplay" type="checkbox">{{ label('Области предложенной оболочки','Proposed shell regions') }}</label>
    <p>{{ label('Внутри грани','Inside face') }}: {{ display.inside }} · {{ label('вне грани','outside face') }}: {{ display.outside }} · {{ label('обрезка не проверена','trim unresolved') }}: {{ display.unresolvedTrim }}</p>
    <p>{{ label('Рамки — оценки областей поверхности. Оранжевым отмечена непроверенная обрезка; это ещё не сетка перехода.','Boxes enclose surface regions. Orange marks unresolved trim; a transition mesh is still pending.') }}</p>
   </div>
   <div v-if="unresolved.length" class="pair-controls">
    <label>{{ label('Непроверенная пара','Unverified pair') }}
     <select v-model.number="selectedPair" :aria-label="label('Непроверенная пара','Unverified pair')">
      <option v-for="(pair,index) in unresolved" :key="pair.faces.join(':')" :value="index">{{ pair.faces.join(' ↔ ') }}</option>
     </select>
    </label>
    <p>{{ label('Эти пары ещё не проверены. Самопересечение для них не подтверждено.','These pairs remain unverified. They are not confirmed self-intersections.') }}</p>
    <button type="button" :disabled="!active || !options || pending || !contactSourceFaces.length" @click="selectPair(contactPair?.faces??[])">{{ label('Показать исходные грани','Show source faces') }}</button>
    <p v-if="contactHasTransition" data-contact-transition-limitation>{{ label('Пара включает переходную грань. Подсветка показывает только доступные исходные грани; переход в сцене пока не отображается.','The pair includes a transition face. Highlighting shows available source faces only; the transition is not yet displayed in the scene.') }}</p>
    <label v-if="contactPair?.pendingProducts.length">{{ label('Непроверенная область контакта','Unverified contact region') }}<input v-model.number="selectedContactRegion" type="number" min="1" :max="contactPair.pendingProducts.length" step="1" :aria-label="label('Непроверенная область контакта','Unverified contact region')"></label>
    <details v-if="contactRegion" data-contact-uv-domains><summary>{{ label('UV-интервалы области контакта','Contact region UV intervals') }}</summary>
     <p v-for="(domain,side) in contactRegion" :key="side">{{ label('Сторона','Side') }} {{ side+1 }}: U [{{ domain[0][0] }}, {{ domain[0][1] }}] · V [{{ domain[1][0] }}, {{ domain[1][1] }}]</p>
     <p>{{ label('Интервалы относятся к граням предложенной оболочки.','Intervals refer to faces of the proposed shell.') }}</p>
    </details>
   </div>
   <button v-if="!result.embeddingQualified || !inspection?.volumeQualified" type="button" :disabled="!active || !options || pending" @click="startInspection">{{ label('Повторить','Retry') }}</button>
   <details><summary>{{ label('Подробности','Details') }}</summary>{{ inspection?.reason ?? result.reason }}<p v-if="inspection?.volume?.positiveBounds">{{ inspection.volume.positiveBounds.join(' … ') }} mm³</p></details>
  </div>
 </details>
</template>
<style scoped>
.expression-shell-check{font-size:12px;padding:6px 0;border-top:1px solid var(--border,#ddd);min-width:0;white-space:normal;overflow-wrap:anywhere}
p{margin:6px 0}button{font:inherit;margin:2px 4px 2px 0;padding:3px 7px}summary{cursor:pointer}.pair-controls label,.mesh-budget{display:flex;align-items:center;gap:6px}.pair-controls select,.mesh-budget select{font:inherit;min-width:0;max-width:100%;flex:1}.expression-shell-check>label{display:block}
input[aria-invalid="true"]{border-color:var(--danger,#b42318);outline-color:var(--danger,#b42318)}
.parameter-error{display:block;color:var(--danger,#b42318);margin:2px 0 5px}
</style>
