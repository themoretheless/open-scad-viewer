<script setup lang="ts">
import {computed,onUnmounted,ref,shallowRef,watch,watchEffect} from 'vue'
import type {NurbsBrep} from '../services/geometry/brep'
import type {MaterialOptions,MaterialResult,MaterialOverlay} from '../services/solidMaterialVolume'
import {wallSearchCandidates,wholeWallSearchCandidates} from '../services/solidWallSearch'
import type {MaterialWallOptions,MaterialWallResult} from '../services/solidMaterialWall'
import type {WholeWallOptions,WholeWallResult} from '../services/solidWholeWall'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
import {evaluateNurbsSurface} from '../services/nurbsSurface'
const props=defineProps<{active:boolean;ru:boolean;model?:NurbsBrep}>()
const emit=defineEmits<{state:[active:boolean,overlay:MaterialOverlay|null]}>()
const label=(ru:string,en:string)=>props.ru?ru:en
const enabled=ref(false),mode=ref<'chord'|'segment'|'wall'|'whole'>('chord'),origin=ref([-2,5,5]),direction=ref([14,0,0]),extended=ref(false),normalAudit=ref(true)
const automatic=ref(false),searchProgress=ref(0),searchTotal=ref(0)
const faceGroups=ref<[number[],number[]]>([[],[]]),toleranceMm=ref(0.01)
const wallResult=shallowRef<MaterialWallResult|null>(null),wholeResult=shallowRef<WholeWallResult|null>(null)
const pending=ref(false),error=ref(''),result=shallowRef<MaterialResult|null>(null)
const request=shallowRef<{mode:'chord'|'segment'|'wall'|'whole';options:MaterialOptions|MaterialWallOptions|WholeWallOptions}|null>(null),worker=createSolidPreviewWorker()
watch(mode,next=>{if(next==='whole')automatic.value=true})
watch(()=>props.model,model=>{
 faceGroups.value=[[],[]]
 if(!model?.vertices.length)return
 const b=[0,1,2].map(()=>[Infinity,-Infinity])
 for(const v of model.vertices)for(let k=0;k<3;k++){b[k][0]=Math.min(b[k][0],v.point[k]);b[k][1]=Math.max(b[k][1],v.point[k])}
 const pad=Math.max(1,(b[0][1]-b[0][0])*.1)
 origin.value=[b[0][0]-pad,(b[1][0]+b[1][1])/2,(b[2][0]+b[2][1])/2]
 direction.value=[b[0][1]-b[0][0]+2*pad,0,0]
},{immediate:true})
watch(()=>[props.active,props.model,enabled.value,mode.value,extended.value,normalAudit.value,toleranceMm.value,JSON.stringify(faceGroups.value),automatic.value,...origin.value,...direction.value],()=>{
 request.value=null;wallResult.value=null;wholeResult.value=null;result.value=null;error.value='';pending.value=false;worker.cancel()
},{flush:'sync'})
function run(){
 if(!props.model){error.value=label('Выберите тело с исходной геометрией.','Choose a body with original geometry.');return}
 if(!((mode.value==='wall'||mode.value==='whole')&&automatic.value)&&(!origin.value.every(Number.isFinite)||!direction.value.every(Number.isFinite)||direction.value.every(x=>x===0))){
  error.value=label('Введите конечные координаты и ненулевое смещение XYZ.','Enter finite coordinates and a nonzero XYZ displacement.');return
 }
 if(mode.value==='wall'&&(!faceGroups.value.every(g=>g.length>0)||faceGroups.value[0].some(i=>faceGroups.value[1].includes(i))||!Number.isFinite(toleranceMm.value)||toleranceMm.value<=0)){error.value=label('Выберите две непересекающиеся группы граней и положительный допуск.','Choose two disjoint face groups and a positive tolerance.');return}
 if(mode.value==='whole'&&(!Number.isFinite(toleranceMm.value)||toleranceMm.value<=0)){error.value=label('Введите положительный допуск толщины.','Enter a positive thickness tolerance.');return}
 const budget=extended.value?1000000:100000,domains=extended.value?8000000:1000000
 request.value={mode:mode.value,options:{...((mode.value==='chord'&&normalAudit.value||mode.value==='wall'||mode.value==='whole')?{normalAudit:{maxSineSquared:1e-6,maxSpans:extended.value?10000:1000}}:{}),...((mode.value==='wall'||mode.value==='whole')?{toleranceMm:toleranceMm.value,maxDistanceCells:budget,maxDistanceDomainCells:domains}:{}),...(mode.value==='wall'?{faceGroups:structuredClone(faceGroups.value.map(g=>[...g])) as [number[],number[]]}:{}),...(mode.value==='whole'?{coverageLimits:{maxFacePairs:extended.value?100000:10000,maxPlaneControls:extended.value?1000000:100000,maxNormalSpans:extended.value?100000:10000}}:{}),model:props.model,origin:[...origin.value] as [number,number,number],direction:[...direction.value] as [number,number,number],toleranceUv:1e-7,
  limits:{pointCells:budget,pointDomainCells:domains,segmentCells:budget,segmentDomainCells:domains,validity:{exactWork:1000000,trimPairs:10000,trimCells:100000,trimDomainCells:1000000,spans:4096,
   facePairs:10000,faceCells:extended.value?1000000:150000,faceDomainCells:extended.value?8000000:1500000,faceCellsPerPair:1024,faceDomainCellsPerPair:100000,
   nestingPairs:1000,nestingCells:budget,nestingDomainCells:domains,orientationCells:budget,orientationDomainCells:domains,orientationSpans:10000}}}}
}
function acceptWall(r:MaterialWallResult|WholeWallResult){
 if(r.method==='bounded-whole-material-wall'){wholeResult.value=r;wallResult.value=null}else{wallResult.value=r;wholeResult.value=null}
 result.value=r.candidate
}
watchEffect(onCleanup=>{
 const job=request.value,active=props.active&&enabled.value,model=props.model
 let current=true
 onCleanup(()=>{current=false;worker.cancel()})
 if(!active||!job||job.options.model!==model)return
 wallResult.value=null;wholeResult.value=null;result.value=null;error.value='';pending.value=true
 if((job.mode==='wall'||job.mode==='whole')&&automatic.value){
  void (async()=>{
   try{
    const options=job.options as MaterialWallOptions|WholeWallOptions,candidates=job.mode==='whole'?wholeWallSearchCandidates(options.model):wallSearchCandidates(options.model,(options as MaterialWallOptions).faceGroups)
    searchProgress.value=0;searchTotal.value=candidates.length
    if(!candidates.length)throw Error('No regular samples')
    let best:MaterialWallResult|WholeWallResult|null=null
    for(const candidate of candidates){
     if(!current)return
     const line={origin:candidate.origin,direction:candidate.direction}
     const r=job.mode==='whole'?await worker.run({kind:'wholeWall',options:{...options as WholeWallOptions,...line}}):await worker.run({kind:'materialWall',options:{...options as MaterialWallOptions,...line}})
     if(!current)return
     searchProgress.value++
     if(!best||r.intervalMm&&(!best.intervalMm||r.intervalMm[1]<best.intervalMm[1]))best=r
     acceptWall(best)
     if(best.converged||r.candidate.reason==='volume-unproven')break
    }
   }catch{if(current)error.value=label('Поиск не выполнен. Проверьте геометрию и повторите.','Search failed. Inspect geometry and retry.')}
   finally{if(current)pending.value=false}
  })()
  return
 }
 const operation=job.mode==='whole'?worker.run({kind:'wholeWall',options:job.options as WholeWallOptions}):job.mode==='wall'?worker.run({kind:'materialWall',options:job.options as MaterialWallOptions}):worker.run({kind:job.mode==='chord'?'materialChord':'materialSegment',options:job.options})
 void operation.then(r=>{if(current){if(r.method==='bounded-material-wall'||r.method==='bounded-whole-material-wall')acceptWall(r);else result.value=r}})
  .catch(()=>{if(current)error.value=label('Проверка не выполнена. Повторите или проверьте геометрию тела.','Check failed. Retry or inspect the body geometry.')})
  .finally(()=>{if(current)pending.value=false})
})
const boundary=computed(()=>result.value?.method==='continuous-material-chord'?result.value.boundary:result.value?.segment??null)
const uncheckedFaces=computed(()=>{
 const r=wholeResult.value;if(!r||!props.model)return []
 if(!r.coverage.enumerationComplete)return props.model.faces.map((_,i)=>i)
 return [...new Set(r.coverage.pairs.filter(p=>p.reason==='self-pair-unresolved'||p.reason==='distance-work-limit'||(!r.converged&&p.lowerBoundMm===0)).flatMap(p=>p.faces))]
})
const issues=computed(()=>[...(boundary.value?.contacts.map(c=>({...c,unresolved:false}))??[]),...(boundary.value?.unresolved.map(c=>({...c,unresolved:true}))??[]),...uncheckedFaces.value.map(face=>{
 const s=props.model!.faces[face].surface
 return {face,unresolved:true,uv:[[s.knotsU[s.degreeU],s.knotsU[s.knotsU.length-s.degreeU-1]],[s.knotsV[s.degreeV],s.knotsV[s.knotsV.length-s.degreeV-1]]] as [[number,number],[number,number]]}
})])
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
 emit('state',true,{line:[r.origin as [number,number,number],r.origin.map((v,k)=>v+r.direction[k]) as [number,number,number]],marks,proven:wholeResult.value?wholeResult.value.converged:wallResult.value?wallResult.value.converged:r.proven})
})
onUnmounted(()=>{worker.dispose();emit('state',false,null)})
</script>
<template>
 <button type="button" :aria-pressed="enabled" @click="enabled=!enabled">{{label('Проверка материала вдоль линии','Material along a line')}}</button>
 <fieldset v-if="enabled" class="material-path-check" :aria-label="label('Материал вдоль линии','Material along a line')" @keydown.esc.stop="enabled=false" @keydown.enter.prevent.stop="run">
  <legend>{{label('Материал вдоль линии','Material along a line')}}</legend>
  <label>{{label('Проверка','Check')}}<select v-model="mode" :aria-label="label('Режим проверки материала','Material check mode')"><option value="chord">{{label('Между гранями','Between faces')}}</option><option value="whole">{{label('Минимальная толщина всей детали','Minimum thickness of the whole part')}}</option><option value="wall">{{label('Толщина между группами граней','Thickness between face groups')}}</option><option value="segment">{{label('Весь отрезок внутри','Entire segment inside')}}</option></select></label>
  <small>{{(mode==='whole'||mode==='wall')&&automatic?label('Линии поиска выбираются автоматически. Отключите поиск для ручного ввода.','Search lines are selected automatically. Disable search for manual input.'):mode!=='segment'?label('Начните снаружи тела и проведите линию через одну стенку.','Start outside the body and pass the line through one wall.'):label('Укажите начало внутри материала и смещение до конца отрезка.','Set a start inside material and displacement to the segment end.')}}</small>
  <div v-for="(values,name) in {origin,direction}" :key="name" class="coordinates" v-show="!((mode==='whole'||mode==='wall')&&automatic)"><span>{{name==='origin'?label('Начало, мм','Start, mm'):label('Смещение, мм','Displacement, mm')}}</span><label v-for="(axis,k) in ['X','Y','Z']" :key="axis">{{axis}}<input v-model.number="values[k]" type="number" step="any" :aria-label="label(name==='origin'?'Начало ':'Смещение ',name==='origin'?'Start ':'Displacement ')+axis" :aria-invalid="!Number.isFinite(values[k]) || name==='direction' && direction.every(x=>x===0)" aria-describedby="material-path-error"></label></div>
  <template v-if="mode==='wall'||mode==='whole'">
   <fieldset v-for="(_,group) in mode==='wall'?faceGroups:[]" :key="group"><legend>{{label('Группа граней','Face group')}} {{group+1}}</legend>
    <div class="face-options"><label v-for="(_,face) in model?.faces" :key="face"><input v-model="faceGroups[group]" type="checkbox" :value="face" :disabled="faceGroups[1-group].includes(face)">{{label('Грань','Face')}} {{face+1}}</label></div>
   </fieldset>
   <label><input v-model="automatic" type="checkbox">{{label('Автоматически искать тонкие участки','Automatically search thin regions')}}</label>
   <small v-if="automatic&&mode==='wall'">{{label('Поиск по регулярным точкам граней; участки между точками ещё не покрыты.','Search at regular face samples; regions between samples remain uncovered.')}} {{pending?searchProgress+' / '+searchTotal:''}}</small>
   <small v-if="mode==='whole'">{{label('Поиск по всем исходным граням с проверкой направления. Неподтверждённые участки остаются в результате.','Search all original faces with direction checks. Unproven regions remain in the result.')}} {{pending?searchProgress+' / '+searchTotal:''}}</small>
   <label>{{label('Допуск, мм','Tolerance, mm')}}<input v-model.number="toleranceMm" type="number" min="0" step="any"></label>
   <small v-if="mode==='wall'">{{label('Проверяются все пары выбранных граней. Для проверки всей детали нужно покрыть все её стенки.','Every selected face pair is checked. Checking the whole part requires covering every wall.')}}</small>
  </template>
  <label v-if="mode==='chord'"><input v-model="normalAudit" type="checkbox">{{label('Проверить направление к граням','Check direction against faces')}}</label>
  <label><input v-model="extended" type="checkbox">{{label('Расширенный расчёт','Extended calculation')}}</label>
  <button type="button" @click="run">{{result||error?label('Повторить проверку','Retry check'):label('Проверить','Check')}} · Enter</button>
  <p v-if="pending" role="status">{{label('Проверяю материал…','Checking material…')}}</p>
  <p v-if="error" id="material-path-error" role="alert">{{error}}</p>
  <template v-if="wholeResult">
   <output v-if="wholeResult.intervalMm" data-whole-wall-thickness>{{wholeResult.intervalMm.map(x=>Number(x.toPrecision(12))).join(' … ')}} mm</output>
   <p role="status" :data-whole-wall-converged="wholeResult.converged">{{wholeResult.converged?label('Минимальная толщина по всем исходным граням подтверждена в заданном допуске.','Minimum thickness over all original faces is confirmed within tolerance.'):!wholeResult.candidate.proven?label('Линия материала не подтверждена. Причина и следующий шаг указаны ниже.','The material chord is unproven. See the reason and next step below.'):!wholeResult.coverage.enumerationComplete?label('Не все пары граней проверены. Включите расширенный расчёт.','Face pair coverage is incomplete. Enable extended calculation.'):uncheckedFaces.length?label('Остались неподтверждённые участки. Выберите стенку для отдельной проверки по группам граней.','Some regions remain unproven. Select a wall for a separate face-group check.'):label('Минимум пока не подтверждён. Продолжите поиск или уточните линию.','The minimum is not yet confirmed. Continue search or refine the line.')}}</p>
   <small>{{label('Охвачены пары граней','Covered face pairs')}}: {{wholeResult.coverage.pairs.length}} / {{wholeResult.coverage.totalPairs}}</small>
   <small v-if="uncheckedFaces.length">{{label('Непроверенные грани','Unchecked faces')}}: {{uncheckedFaces.slice(0,16).map(i=>i+1).join(', ')}}</small>
  </template>
  <template v-if="wallResult">
   <output v-if="wallResult.intervalMm" data-wall-thickness>{{wallResult.intervalMm.map(x=>Number(x.toPrecision(12))).join(' … ')}} mm</output>
   <p role="status" :data-wall-converged="wallResult.converged">{{wallResult.converged?label('Толщина выбранной стенки подтверждена в заданном допуске.','Selected wall thickness is confirmed within tolerance.'):wallResult.intervalMm?label('Получены границы толщины. Расширьте расчёт или уточните группы граней.','Thickness bounds are available. Extend calculation or refine face groups.'):label('Толщина не подтверждена. Проверьте направление линии и выбранные грани.','Thickness is unproven. Inspect line direction and selected faces.')}}</p>
   <small>{{label('Уточнённые пары граней','Refined face pairs')}}: {{wallResult.clearance.evaluatedPairs}} / {{wallResult.clearance.totalPairs}}</small>
  </template>
  <template v-if="result">
   <p role="status" :data-material-proven="result.proven">{{message}}</p>
   <output v-if="result.method==='continuous-material-chord'&&result.lengthIntervalMm" data-material-length>{{result.lengthIntervalMm.map(x=>Number(x.toPrecision(12))).join(' … ')}} mm</output>
   <p v-if="result.method==='continuous-material-chord'&&result.normalAlignment!=='not-qualified'" data-material-normal role="status">{{result.normalAlignment==='angular-tolerance'?label('Направление перпендикулярно обеим граням с угловым допуском около 0,0573°.','Direction is normal to both faces within an angular tolerance of approximately 0.0573°.'):result.normalAlignment==='oblique'?label('Линия наклонена к грани. Измените направление для измерения поперёк стенки.','The line is oblique to a face. Change direction to measure across the wall.'):label('Направление не подтверждено. Измените линию или расширьте расчёт.','Direction is unproven. Change the line or extend the calculation.')}}</p>
   <small v-if="boundary">{{label('Пересечения','Crossings')}}: {{boundary.contacts.length}} · {{label('Непроверенные участки','Unchecked regions')}}: {{boundary.unresolved.length}}</small>
   <ul v-if="issues.length"><li v-for="(c,i) in issues.slice(0,16)" :key="i">{{label('Грань','Face')}} {{c.face+1}} · {{c.unresolved?label('не проверена','unchecked'):label('пересечение','crossing')}}</li></ul>
  </template>
  <small v-if="mode==='chord'||mode==='segment'">{{label('Длина вдоль заданной линии. Минимальная толщина требует проверки всей стенки.','Length along the specified line. Minimum thickness requires checking the entire wall.')}}</small>
  <button type="button" @click="enabled=false">{{label('Закрыть','Close')}} · Esc</button>
 </fieldset>
</template>
<style scoped>
.face-options{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));max-height:180px;overflow:auto}.face-options label{display:flex;gap:6px}button,input,select{font:inherit;color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:5px;padding:6px;min-width:0}button{cursor:pointer}button:focus-visible,input:focus-visible,select:focus-visible{outline:2px solid var(--accent);outline-offset:2px}.material-path-check{display:grid;gap:8px;min-width:0;border:1px solid var(--border);border-radius:5px}.coordinates{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:6px}.coordinates>span{grid-column:1/-1}.coordinates label{display:grid;gap:4px}.coordinates input{width:100%;box-sizing:border-box}small{color:var(--text-dim);line-height:1.5}p{margin:0}ul{margin:0;padding-inline-start:18px}output{overflow-wrap:anywhere}
</style>
