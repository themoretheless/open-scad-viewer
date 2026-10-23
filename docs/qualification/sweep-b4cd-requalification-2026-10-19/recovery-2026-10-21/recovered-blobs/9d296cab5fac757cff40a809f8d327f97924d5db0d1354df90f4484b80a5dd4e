<script setup lang="ts">
import {computed,onUnmounted,ref,shallowRef,watch} from 'vue'
import type {MeshData} from '../core/mesh'
import type {LighteningOptions} from '../services/solidLightening'
import type {NominalLatticeGraph} from '../services/latticeGraphProtocol'
import type {TrussLoadCase,TrussScenario} from '../services/trussScenario'
import {computeNominalLatticeGraph,computeTrussScenario} from '../services/mainSolidWorker'
import {trussFieldMeshes} from '../services/trussFieldMeshes'
import {screenTrussMembers,validatePrintStrengthProfile,type ValidatedPrintProfile,type ThermalEditorState,type ThermalEvaluation} from '../services/trussScreening'
import ThermalStrengthEditor from './ThermalStrengthEditor.vue'
import type {LatticePrintSettings} from '../services/latticePrintSettings'

const props=defineProps<{meshes:MeshData[];selection:number[];source:string;ready:boolean;locale:string;options:LighteningOptions;printSettings:LatticePrintSettings;previewEpoch?:number}>()
const emit=defineEmits<{preview:[meshes:MeshData[]|null]}>()
const showField=ref(false),markerMm=ref(1)
function clearField(){if(showField.value){showField.value=false;emit('preview',null)}}
function updateField(){
  if(!showField.value){emit('preview',null);return}
  if(!result.value){clearField();return}
  try{emit('preview',trussFieldMeshes(result.value.model,result.value.result,markerMm.value));error.value=''}
  catch(cause){clearField();error.value=String(cause)}
}
watch(()=>props.previewEpoch,clearField,{flush:'sync'})
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
const graph=shallowRef<NominalLatticeGraph|null>(null)
const entries=ref<{loadCase:TrussLoadCase;factor:number}[]>([])
const active=ref(0),mode=ref<'case'|'combination'>('case')
const youngMpa=ref<number|string>(''),areaMm2=ref<number|string>('')
const phase=ref<'graph'|'solve'|null>(null),error=ref('')
const result=shallowRef<Awaited<ReturnType<typeof computeTrussScenario>>|null>(null)
const tensionMpa=ref<number|string>(''),compressionMpa=ref<number|string>(''),safetyFactor=ref(2)
const material=ref(''),grade=ref(''),propertySource=ref('')
const nozzleTempC=ref<number|string>(''),bedTempC=ref<number|string>('')
const solvedPrintProfile=shallowRef<ValidatedPrintProfile|null>(null)
const thermalState=shallowRef<ThermalEditorState>({enabled:false,evaluation:null,error:''})
const solvedThermal=shallowRef<ThermalEvaluation|null>(null)
const effectiveYoung=computed(()=>thermalState.value.enabled?thermalState.value.evaluation?.result.youngMpa:youngMpa.value)
function updateThermal(value:ThermalEditorState){thermalState.value=value;invalidateResult()}
const printProfile=computed(()=>({material:material.value,grade:grade.value,propertySource:propertySource.value,
  nozzleMm:props.printSettings.nozzle,lineWidthMm:props.options.lineWidth,layerHeightMm:props.printSettings.layer,
  nozzleTempC:Number(nozzleTempC.value),bedTempC:bedTempC.value===''?NaN:Number(bedTempC.value)}))
const screening=computed(()=>{
  const thermal=solvedThermal.value?.result
  if(!result.value||(!thermal&&(tensionMpa.value===''||compressionMpa.value==='')))return {rows:[],error:''}
  try{return {rows:screenTrussMembers(result.value.model,[result.value.result],{
    tensionMpa:thermal?.tensionMpa??Number(tensionMpa.value),compressionMpa:thermal?.compressionMpa??Number(compressionMpa.value),safetyFactor:Number(safetyFactor.value)}),error:''}}
  catch(cause){return {rows:[],error:String(cause)}}
})
const current=computed(()=>entries.value[active.value]?.loadCase??null)
const load=computed(()=>current.value?.loads[0]??null)
const selectedMesh=computed(()=>props.selection.length===1?props.meshes[props.selection[0]]??null:null)
const available=computed(()=>props.ready&&!!selectedMesh.value)
const bulkFace=ref('z-min'),bulkMask=ref<[boolean,boolean,boolean]>([true,true,true])
const axes=['X','Y','Z']
let revision=0,controller:AbortController|undefined,nextCase=1
let binding:{sourceBodyIndex:number;graphOptions:LighteningOptions}|null=null

function invalidateResult(){
  clearField()
  revision++;controller?.abort();controller=undefined;phase.value=null;result.value=null;solvedPrintProfile.value=null;solvedThermal.value=null;error.value=''
}
function invalidateGraph(){invalidateResult();graph.value=null;entries.value=[];binding=null;active.value=0}
watch([()=>props.source,()=>props.ready,selectedMesh,()=>selectedMesh.value?.vertices,
  ()=>selectedMesh.value?.indices,()=>selectedMesh.value?.transform],invalidateGraph,{flush:'sync'})
watch([()=>props.selection,()=>props.options],invalidateGraph,{deep:true,flush:'sync'})
watch([entries,youngMpa,areaMm2,active,mode],invalidateResult,{deep:true,flush:'sync'})
watch(printProfile,()=>{youngMpa.value='';tensionMpa.value='';compressionMpa.value='';invalidateResult()},{flush:'sync'})
// The report Blob and its object URL are built lazily on download click, not on
// every keystroke; the URL is revoked immediately after the click is dispatched.
function downloadReport(){
  if(!result.value||!solvedPrintProfile.value||!graph.value||!binding)return
  const assessment=screening.value.rows.length?{limits:{tensionMpa:solvedThermal.value?.result.tensionMpa??tensionMpa.value,compressionMpa:solvedThermal.value?.result.compressionMpa??compressionMpa.value,safetyFactor:safetyFactor.value},rows:screening.value.rows}:null
  const report={version:3,modelKind:graph.value.modelKind,...binding,...result.value,printProfile:solvedPrintProfile.value,thermal:solvedThermal.value,axialScreening:assessment}
  const url=URL.createObjectURL(new Blob([JSON.stringify(report,null,2)],{type:'application/json'}))
  const a=document.createElement('a');a.href=url;a.download='nominal-truss.json';a.click()
  URL.revokeObjectURL(url)
}
onUnmounted(invalidateResult)

async function generate(){
  invalidateGraph()
  if(!available.value)return
  const ticket=revision,abort=new AbortController();controller=abort;phase.value='graph'
  const snapshot={sourceBodyIndex:props.selection[0],graphOptions:{...props.options}}
  try{
    const value=await computeNominalLatticeGraph(selectedMesh.value!,snapshot.graphOptions,{signal:abort.signal})
    if(ticket!==revision)return
    graph.value=value;binding=snapshot;nextCase=2
    entries.value=[{factor:1,loadCase:{id:'case-1',restrained:value.nodes.map(()=>[false,false,false]),
      loads:[{nodes:[],originMm:[0,0,0],forceN:[0,0,0],momentNmm:[0,0,0]}]}}]
  }catch(cause){if(ticket===revision&&!(cause instanceof Error&&cause.name==='AbortError'))error.value=String(cause)}
  finally{if(ticket===revision){phase.value=null;controller=undefined}}
}
const faceNodes=computed(()=>{
  if(!graph.value)return []
  const [axis,side]=bulkFace.value.split('-'),k='xyz'.indexOf(axis)
  const values=graph.value.nodes.map(point=>point[k]),bound=side==='min'?Math.min(...values):Math.max(...values)
  return values.flatMap((value,i)=>value===bound?[i]:[])
})
function setRestraints(){if(current.value)for(const i of faceNodes.value)current.value.restrained[i]=[...bulkMask.value]}
function setLoadNodes(){if(load.value)load.value.nodes=[...faceNodes.value]}
function centroid(){
  if(!graph.value||!load.value?.nodes.length)return
  const points=load.value.nodes.map(i=>graph.value!.nodes[i]),origin=points[0]
  load.value.originMm=axes.map((_,k)=>origin[k]+points.reduce((sum,point)=>sum+(point[k]-origin[k])/points.length,0)) as [number,number,number]
}
function duplicate(){
  if(!current.value||entries.value.length>=32)return
  const source=current.value
  entries.value.push({factor:0,loadCase:{id:`case-${nextCase++}`,restrained:source.restrained.map(mask=>[...mask]),
    loads:source.loads.map(value=>({nodes:[...value.nodes],originMm:[...value.originMm],forceN:[...value.forceN],momentNmm:[...value.momentNmm]}))}})
  active.value=entries.value.length-1
}
function remove(){if(entries.value.length>1){entries.value.splice(active.value,1);active.value=Math.min(active.value,entries.value.length-1)}}
async function solve(){
  invalidateResult()
  if(!graph.value||!binding||!current.value)return
  let profile:ValidatedPrintProfile
  try{profile=validatePrintStrengthProfile(printProfile.value)}
  catch(cause){error.value=String(cause);return}
  const thermal=thermalState.value.enabled?thermalState.value.evaluation:null
  if(thermalState.value.enabled&&!thermal){error.value=thermalState.value.error;return}
  const modulus=effectiveYoung.value
  if(typeof modulus!=='number'||!Number.isFinite(modulus)||modulus<=0
    ||typeof areaMm2.value!=='number'||!Number.isFinite(areaMm2.value)||areaMm2.value<=0){
    error.value=label('Модуль E и площадь должны быть положительными конечными числами.','E and area must be positive finite numbers.');return
  }
  const scenario:TrussScenario={nodesMm:graph.value.nodes,members:graph.value.edges.map(nodes=>({nodes,youngMpa:modulus,areaMm2:areaMm2.value as number})),
    cases:entries.value.map(entry=>entry.loadCase),combinations:mode.value==='combination'?[{id:'combination',terms:entries.value.map(entry=>({caseId:entry.loadCase.id,factor:entry.factor}))}]:[],
    activeId:mode.value==='combination'?'combination':current.value.id}
  const ticket=revision,abort=new AbortController();controller=abort;phase.value='solve'
  try{
    const value=await computeTrussScenario(scenario,{signal:abort.signal})
    if(ticket!==revision)return
    solvedPrintProfile.value=profile
    solvedThermal.value=thermal
    result.value=value
  }catch(cause){if(ticket===revision&&!(cause instanceof Error&&cause.name==='AbortError'))error.value=String(cause)}
  finally{if(ticket===revision){phase.value=null;controller=undefined}}
}
const number=(value:number)=>value===0?'0':value.toPrecision(5)
</script>

<template>
  <details class="nominal-truss">
    <summary>{{label('Осевая модель графа','Axial graph model')}}</summary>
    <p class="model-scope">{{label('Номинальная сеть по габаритам. Без обрезки поверхностью, оболочки, изгиба и потери устойчивости. Не оценка прочности детали.','Nominal bounding-box network. No surface clipping, shell, bending or buckling. Not a part-strength assessment.')}}</p>
    <button :disabled="!available||!!phase" @click="generate">{{label('Построить граф','Generate graph')}}</button>
    <span v-if="!available" role="status">{{label('Нужно одно тело и актуальная сборка.','One body and a current build are required.')}}</span>
    <template v-if="graph&&current&&load">
      <p>{{graph.nodes.length}} {{label('узлов','nodes')}} · {{graph.edges.length}} {{label('стержней','members')}}</p>
      <fieldset><legend>{{label('Пластик и условия печати','Plastic and print conditions')}}</legend>
        <label>{{label('Тип пластика','Plastic type')}}<select v-model="material" :aria-label="label('Тип пластика','Plastic type')"><option value="">{{label('Выберите пластик','Select plastic')}}</option><option v-for="kind in ['PLA','PETG','ABS','ASA','PA','PC','TPU','custom']" :key="kind" :value="kind">{{kind==='custom'?label('Другой / композит','Other / composite'):kind}}</option></select></label>
        <label>{{label('Марка / производитель / состав','Grade / manufacturer / composition')}}<input v-model="grade" maxlength="512"></label>
        <label>{{label('Источник E и пределов для этих условий','Source of E and limits for these conditions')}}<input v-model="propertySource" maxlength="512"></label>
        <div class="fields">
          <label>{{label('Температура сопла, °C','Nozzle temperature, °C')}}<input v-model.number="nozzleTempC" type="number" min="1"></label>
          <label>{{label('Температура стола, °C','Bed temperature, °C')}}<input v-model.number="bedTempC" type="number" min="0"></label>
        </div>
        <p>{{label('Из настроек FDM и геометрии: сопло','From FDM and geometry settings: nozzle')}} {{printSettings.nozzle}} mm · {{label('ширина линии','line width')}} {{options.lineWidth}} mm · {{label('слой','layer')}} {{printSettings.layer}} mm.</p>
        <p>{{label('Ручные свойства сбрасываются при изменении условий. Температурная таблица пересчитывает E и пределы только в измеренном диапазоне; анизотропия не моделируется.','Manual properties reset when conditions change. The temperature table recalculates E and limits only within the measured range; anisotropy is not modeled.')}}</p>
      </fieldset>
      <ThermalStrengthEditor :profile="printProfile" :locale="locale" @change="updateThermal"/>
      <div class="fields">
        <label>{{label('Модуль E, MPa','Young modulus, MPa')}}<input v-if="!thermalState.enabled" v-model.number="youngMpa" type="number" min="0" step="100"><input v-else :value="effectiveYoung??''" type="number" readonly></label>
        <label>{{label('Площадь стержня, mm²','Member area, mm²')}}<input v-model.number="areaMm2" type="number" min="0" step="0.1"></label>
      </div>
      <label>{{label('Редактируемый случай','Edit case')}}<select v-model.number="active" :aria-label="label('Редактируемый случай','Edit case')"><option v-for="(entry,i) in entries" :key="i" :value="i">{{entry.loadCase.id}}</option></select></label>
      <div class="commands"><button :disabled="entries.length>=32" @click="duplicate">{{label('Дублировать случай','Duplicate case')}}</button><button :disabled="entries.length===1" @click="remove">{{label('Удалить случай','Remove case')}}</button></div>
      <label>{{label('Имя случая','Case name')}}<input v-model="current.id" maxlength="128"></label>
      <fieldset><legend>{{label('Выбор узлов','Node selection')}}</legend>
        <label>{{label('Граница габаритов','Bounding plane')}}<select v-model="bulkFace" :aria-label="label('Граница габаритов','Bounding plane')"><option v-for="face in ['x-min','x-max','y-min','y-max','z-min','z-max']" :key="face">{{face}}</option></select></label>
        <div class="checks"><label v-for="(axis,k) in axes" :key="axis"><input v-model="bulkMask[k]" type="checkbox" :aria-label="`Bulk ${axis} restraint`">{{axis}}</label></div>
        <div class="commands"><button @click="setRestraints">{{label('Назначить закрепления','Set restraints')}}</button><button @click="setLoadNodes">{{label('Назначить нагрузку','Set load nodes')}}</button></div>
        <div class="table-scroll"><table><thead><tr><th>{{label('Узел','Node')}}</th><th>XYZ, mm</th><th v-for="axis in axes" :key="axis">{{axis}}</th><th>{{label('Нагрузка','Load')}}</th></tr></thead><tbody>
          <tr v-for="(point,i) in graph.nodes" :key="i"><th>{{i}}</th><td>{{point.map(number).join(', ')}}</td>
            <td v-for="(axis,k) in axes" :key="axis"><input v-model="current.restrained[i][k]" type="checkbox" :aria-label="`Node ${i} ${axis} restrained`"></td>
            <td><input v-model="load.nodes" type="checkbox" :value="i" :aria-label="`Node ${i} loaded`"></td></tr>
        </tbody></table></div>
      </fieldset>
      <fieldset><legend>{{label('Равнодействующая','Resultant load')}} · {{load.nodes.length}} {{label('узлов','nodes')}}</legend>
        <div v-for="field in [{key:'forceN' as const,ru:'Сила, N',en:'Force, N'},{key:'momentNmm' as const,ru:'Момент относительно начала, N mm',en:'Moment about origin, N mm'},{key:'originMm' as const,ru:'Начало отсчёта, mm',en:'Origin, mm'}]" :key="field.key">
          <span>{{label(field.ru,field.en)}}</span><div class="vector"><label v-for="(axis,k) in axes" :key="axis">{{axis}}<input v-model.number="load[field.key][k]" type="number" step="0.1" :aria-label="`${field.en} ${axis}`"></label></div>
        </div>
        <button :disabled="!load.nodes.length" @click="centroid">{{label('Начало в центре выбранных узлов','Use node centroid')}}</button>
      </fieldset>
      <label>{{label('Расчёт','Solve')}}<select v-model="mode" :aria-label="label('Режим расчёта','Result mode')"><option value="case">{{label('Текущий случай','Current case')}}</option><option value="combination">{{label('Линейная комбинация','Linear combination')}}</option></select></label>
      <div v-if="mode==='combination'" class="fields"><label v-for="(entry,i) in entries" :key="i">{{entry.loadCase.id}}<input v-model.number="entry.factor" type="number" step="0.1" :aria-label="`Factor ${entry.loadCase.id}`"></label></div>
      <div class="commands"><button :disabled="!!phase" @click="solve">{{label('Рассчитать модель','Solve model')}}</button><button v-if="phase" @click="invalidateResult">{{label('Отмена','Cancel')}}</button></div>
      <section v-if="result" class="results" aria-label="Axial graph results">
        <h4>{{label('Номинальная осевая модель','Nominal axial model')}}</h4>
        <p v-for="warning in solvedPrintProfile?.warnings" :key="warning" role="status">{{warning==='layer-above-80-percent-nozzle'?label('Высота слоя превышает 80% диаметра сопла — проверьте профиль печати.','Layer height exceeds 80% of nozzle diameter; check the print profile.'):label('Ширина линии не больше высоты слоя — проверьте профиль печати.','Line width does not exceed layer height; check the print profile.')}}</p>
        <dl><dt>{{label('Максимальное перемещение, mm','Maximum displacement, mm')}}</dt><dd>{{number(result.result.maxDeflectionMm)}}</dd><dt>{{label('Относительная невязка','Relative residual')}}</dt><dd>{{number(result.result.maxRelativeResidual)}}</dd><dt>{{label('Свободные степени свободы','Free DOFs')}}</dt><dd>{{result.result.freeDofs}}</dd></dl>
        <a href="nominal-truss.json" download="nominal-truss.json" @click.prevent="downloadReport">nominal-truss.json</a>
        <fieldset><legend>{{label('Поиск перегруженных стержней','Axial demand screening')}}</legend>
          <p>{{label('Только рассчитанный случай или комбинация. Допуски задаются для вашего материала и процесса печати; потеря устойчивости не проверяется.','Only the solved case or combination. Supply limits for your material and printing process; buckling is not checked.')}}</p>
          <div class="fields">
            <label>{{label('Предел растяжения, MPa','Tensile limit, MPa')}}<input v-if="!solvedThermal" v-model.number="tensionMpa" type="number" min="0" step="0.1"><input v-else :value="solvedThermal.result.tensionMpa" type="number" readonly></label>
            <label>{{label('Предел сжатия, MPa','Compressive limit, MPa')}}<input v-if="!solvedThermal" v-model.number="compressionMpa" type="number" min="0" step="0.1"><input v-else :value="solvedThermal.result.compressionMpa" type="number" readonly></label>
            <label>{{label('Коэффициент запаса','Safety factor')}}<input v-model.number="safetyFactor" type="number" min="1" step="0.1"></label>
          </div>
          <p v-if="screening.error" role="alert">{{screening.error}}</p>
          <template v-if="screening.rows.length">
            <p>{{label('Более 100% — превышение осевого допуска; менее 30% — малая осевая нагрузка. Уменьшение сечения требует повторного расчёта всех нагрузок, жёсткости и устойчивости.','Above 100% exceeds the axial limit; below 30% indicates low axial demand. Resizing requires rechecking all loads, stiffness and stability.')}}</p>
            <div class="table-scroll"><table aria-label="Axial demand screening"><thead><tr><th>{{label('Стержень','Member')}}</th><th>{{label('Использование, %','Utilization, %')}}</th><th>{{label('Оценка','Screening')}}</th></tr></thead><tbody>
              <tr v-for="row in screening.rows" :key="row.memberIndex"><td>{{result.model.members[row.memberIndex].nodes.join(' - ')}}</td><td>{{number(row.utilization*100)}}</td><td>{{row.status==='overloaded'?label('Перегружен','Overloaded'):row.status==='low-demand'?label('Малая нагрузка','Low demand'):label('В осевом допуске','Within axial limit')}}</td></tr>
            </tbody></table></div>
          </template>
        </fieldset>
        <label class="field-toggle"><input v-model="showField" type="checkbox" @change="updateField">{{label('Знак осевого усилия в предпросмотре','Preview axial force sign')}}</label>
        <label>{{label('Размер маркера, mm','Marker size, mm')}}<input v-model.number="markerMm" type="number" min="0.001" step="0.1" @input="showField&&updateField()"></label>
        <p v-if="showField" class="force-legend"><span><i class="compression"></i>{{label('Сжатие','Compression')}} (−N)</span><span><i class="zero"></i>0 N</span><span><i class="tension"></i>{{label('Растяжение','Tension')}} (+N)</span></p>
        <details><summary>{{label('Стержни','Members')}}</summary><div class="table-scroll"><table><thead><tr><th>{{label('Узлы','Nodes')}}</th><th>N</th><th>MPa</th></tr></thead><tbody><tr v-for="(member,i) in result.model.members" :key="i"><td>{{member.nodes.join(' - ')}}</td><td>{{number(result.result.axialForcesN[i])}}</td><td>{{number(result.result.axialStressesMpa[i])}}</td></tr></tbody></table></div></details>
        <details><summary>{{label('Перемещения и реакции','Displacements and reactions')}}</summary><div class="table-scroll"><table><thead><tr><th>{{label('Узел','Node')}}</th><th>XYZ, mm</th><th>XYZ, N</th></tr></thead><tbody><tr v-for="(point,i) in result.result.displacementsMm" :key="i"><th>{{i}}</th><td>{{point.map(number).join(', ')}}</td><td>{{result.result.reactionsN[i].map(number).join(', ')}}</td></tr></tbody></table></div></details>
      </section>
    </template>
    <button v-if="phase==='graph'" @click="invalidateResult">{{label('Отмена','Cancel')}}</button>
    <p v-if="phase" role="status">{{label('Расчёт…','Computing…')}}</p>
    <p v-if="error" role="alert">{{error}}</p>
  </details>
</template>

<style scoped>
.nominal-truss{min-width:0;margin:10px 0;border-top:1px solid var(--border);padding-top:8px;font-size:12px;letter-spacing:0}
summary{cursor:pointer;font-weight:600;overflow-wrap:anywhere}.model-scope{color:var(--text-dim);line-height:1.4}
label{display:flex;flex-direction:column;gap:4px;margin:6px 0;min-width:0;overflow-wrap:anywhere}
input,select,button{font:inherit;background:var(--surface-raised);color:var(--text);border:1px solid var(--border);border-radius:4px;padding:5px;min-width:0;box-sizing:border-box}
input:not([type=checkbox]),select{width:100%}input[type=checkbox]{margin:0;accent-color:var(--accent)}button{cursor:pointer;white-space:normal}button:disabled{opacity:.5;cursor:default}
.fields{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px}.vector{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:6px}
.checks,.commands{display:flex;flex-wrap:wrap;gap:6px;align-items:center;margin:6px 0}.checks label{flex-direction:row;align-items:center}
.field-toggle{flex-direction:row;align-items:center}.force-legend{display:flex;flex-wrap:wrap;gap:8px}.force-legend span{display:flex;align-items:center;gap:4px}.force-legend i{width:10px;height:10px;display:inline-block}.compression{background:rgb(38,140,242)}.zero{background:rgb(153,153,153)}.tension{background:rgb(242,77,51)}
fieldset{border:0;border-top:1px solid var(--border);padding:6px 0;margin:10px 0;min-width:0}legend{font-weight:600;padding:0 4px 0 0}
.table-scroll{max-height:220px;overflow:auto;width:100%;margin:6px 0}table{border-collapse:collapse;width:100%;font-variant-numeric:tabular-nums}
th,td{padding:5px 4px;border-bottom:1px solid var(--border);text-align:right;white-space:nowrap}th:first-child,td:first-child{text-align:left}thead{position:sticky;top:0;background:var(--surface)}
.results{margin-top:10px;border-top:1px solid var(--border);padding-top:8px}h4{font-size:12px;margin:0 0 8px}dl{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:6px}dt{overflow-wrap:anywhere}dd{margin:0;font-variant-numeric:tabular-nums}a{color:var(--accent)}[role=alert]{color:var(--danger);overflow-wrap:anywhere}
</style>
