<script setup lang="ts">
import {computed,ref,shallowRef,watch} from 'vue'
import type {MeshData} from '../core/mesh'
import type {LighteningOptions} from '../services/solidLightening'
import type {LatticePrintSettings} from '../services/latticePrintSettings'
import {analyzeLatticePrintability,optimizeLatticeForPrint,formatLatticePrintReport,latticeSlicerHintsJson,type LatticePrintReport} from '../services/latticePrintReport'
import {analyzeLatticeStrength,formatLatticeStrengthReport,latticeMaterials,latticeStructureHint,weakSpotMarkerMeshes,type LatticeLoadCase,type LatticeStrengthReport} from '../services/latticeStrength'

const props=defineProps<{meshes:MeshData[];selection:number[];ready:boolean;locale:string;options:LighteningOptions;printSettings:LatticePrintSettings;previewEpoch?:number}>()
const emit=defineEmits<{preview:[meshes:MeshData[]|null];options:[options:LighteningOptions]}>()
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
const error=ref(''),printText=ref(''),strengthText=ref('')
const printReport=shallowRef<LatticePrintReport|null>(null),strength=shallowRef<LatticeStrengthReport|null>(null)
const extra=ref({maxOverhangDeg:50,printSpeedMms:45,fanPct:100})
const materials=computed(()=>{try{return latticeMaterials()}catch{return []}})
const materialId=ref('pla'),loadCase=ref<LatticeLoadCase>('compression'),forceN=ref(100),safety=ref(2)
const service=ref({printAxis:'z' as 'x'|'y'|'z',temperatureC:23,humidityPct:50})
const showSpots=ref(false),markerMm=ref(1.6)
const hint=computed(()=>{try{return latticeStructureHint(props.options.pattern)}catch{return null}})
const selectedMesh=computed(()=>props.selection.length===1?props.meshes[props.selection[0]]??null:null)
function clearMarkers(){if(showSpots.value){showSpots.value=false;emit('preview',null)}}
let keepPrint=false
function invalidate(){clearMarkers();strength.value=null;strengthText.value='';if(keepPrint){keepPrint=false;return}printReport.value=null;printText.value=''}
watch(()=>props.previewEpoch,clearMarkers,{flush:'sync'})
watch([()=>props.options,()=>props.printSettings,()=>props.selection,selectedMesh],invalidate,{deep:true})
function guard(run:()=>void){try{run();error.value=''}catch(cause){error.value=String(cause)}}
const settings=()=>({...props.printSettings,...extra.value})
function runPrint(){guard(()=>{printReport.value=analyzeLatticePrintability(props.options,settings());printText.value=formatLatticePrintReport(printReport.value,props.locale)})}
function optimize(){guard(()=>{const r=optimizeLatticeForPrint(props.options,settings());keepPrint=true;emit('options',r.options);printReport.value=r.report;printText.value=formatLatticePrintReport(r.report,props.locale)+'\n'+r.changed.join(', ')})}
async function copyHints(){if(!printReport.value)runPrint();if(printReport.value)try{await navigator.clipboard.writeText(latticeSlicerHintsJson(printReport.value))}catch(cause){error.value=String(cause)}}
function runStrength(){guard(()=>{
  if(!selectedMesh.value)throw Error(label('Выберите одно тело.','Select one body.'))
  strength.value=analyzeLatticeStrength(selectedMesh.value,props.options,materialId.value,{case:loadCase.value,forceN:forceN.value,safety:safety.value},{service:{...service.value,perimeters:props.printSettings.lines,maxBridgeMm:props.printSettings.maxBridge}})
  strengthText.value=formatLatticeStrengthReport(strength.value,props.locale);updateMarkers()
})}
function updateMarkers(){
  if(!showSpots.value||!strength.value){emit('preview',null);return}
  guard(()=>emit('preview',weakSpotMarkerMeshes(strength.value!.weakSpots,markerMm.value)))
}
</script>

<template>
<p v-if="hint">{{ label(hint.ru,hint.en) }}</p>
<details><summary>{{ label('Печатаемость и подсказки слайсеру','Printability and slicer hints') }}</summary>
  <label>{{ label('Макс. свес, °','Max overhang, °') }}<input v-model.number="extra.maxOverhangDeg" type="number" min="20" max="80"></label>
  <label>{{ label('Скорость периметров, мм/с','Perimeter speed, mm/s') }}<input v-model.number="extra.printSpeedMms" type="number" min="1"></label>
  <label>{{ label('Обдув, %','Fan, %') }}<input v-model.number="extra.fanPct" type="number" min="0" max="100"></label>
  <button @click="runPrint">{{ label('Отчёт о печати','Printability report') }}</button>
  <button @click="optimize">{{ label('Оптимизировать под печать','Optimize for print') }}</button>
  <button @click="copyHints">{{ label('Копировать подсказки слайсеру (JSON)','Copy slicer hints (JSON)') }}</button>
  <pre v-if="printText">{{ printText }}</pre>
</details>
<details><summary>{{ label('Оценка прочности (сопромат)','Strength estimate (SoM)') }}</summary>
  <label>{{ label('Материал (типовые значения)','Material (typical values)') }}<select v-model="materialId"><option v-for="m in materials" :key="m.id" :value="m.id">{{ label(m.ru,m.en) }}</option></select></label>
  <label>{{ label('Нагрузка','Load case') }}<select v-model="loadCase"><option value="compression">{{ label('Сжатие','Compression') }}</option><option value="tension">{{ label('Растяжение','Tension') }}</option><option value="bending">{{ label('Изгиб','Bending') }}</option></select></label>
  <label>{{ label('Сила, Н','Force, N') }}<input v-model.number="forceN" type="number" min="0.1"></label>
  <label>{{ label('Запас','Safety factor') }}<input v-model.number="safety" type="number" min="1" step="0.1"></label>
  <label>{{ label('Ось печати','Print axis') }}<select v-model="service.printAxis"><option>x</option><option>y</option><option>z</option></select></label>
  <label>{{ label('Температура, °C','Temperature, °C') }}<input v-model.number="service.temperatureC" type="number"></label>
  <label>{{ label('Влажность, %','Humidity, %') }}<input v-model.number="service.humidityPct" type="number" min="0" max="100"></label>
  <button :disabled="!ready||!selectedMesh" @click="runStrength">{{ label('Оценить','Estimate') }}</button>
  <label><input v-model="showSpots" type="checkbox" :disabled="!strength" @change="updateMarkers">{{ label('Показать слабые места','Show weak spots') }}</label>
  <label>{{ label('Размер маркера, мм','Marker size, mm') }}<input v-model.number="markerMm" type="number" min="0.4" step="0.2" @change="updateMarkers"></label>
  <pre v-if="strengthText">{{ strengthText }}</pre>
</details>
<p v-if="error" class="error">{{ error }}</p>
</template>
