<script setup lang="ts">
import {computed,reactive,ref,shallowRef,watch} from 'vue'
import type {MeshData} from '../core/mesh'
import {downloadCad} from '../services/cadDrawing'
import {emitLaserFrame,emitLaserGrbl,preflightLaserPlan,type LaserMachineProfile,type LaserPlan,type LaserProgram,type LaserSummary} from '../services/geometry/laser'
import {gcodeMeshBounds} from '../services/gcodePreviewGeometry'
import {planLaserMeshSection} from '../services/laserSectionPlanning'
import {meshDataToPolygon} from '../services/solidBridge'

const props=defineProps<{meshes:MeshData[];selection:number[];source:string;ready:boolean;locale:string}>()
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
const selected=computed(()=>props.selection.length===1?props.meshes[props.selection[0]!]??null:null)
const bounds=computed(()=>{try{return selected.value?gcodeMeshBounds(selected.value):null}catch{return null}})
const machine=reactive<LaserMachineProfile>({widthMm:400,heightMm:400,maxPower:1000,estimatedRapidMmMin:6000,powerMode:'m4',laserModeConfirmed:false,supportsAirAssist:false,flipY:false,returnToOrigin:true})
const process=reactive({
  line:{output:true,speedMmMin:900,power:250,passes:1,airAssist:false},
  fill:{output:false,speedMmMin:1800,power:150,passes:1,airAssist:false,spacingMm:0.2},
  offset:[0,0] as [number,number],
})
const z=ref(0),error=ref(''),message=ref(''),filename=ref('laser-job.gcode')
const result=shallowRef<LaserProgram|null>(null),summary=shallowRef<LaserSummary|null>(null),planned=shallowRef<LaserPlan|null>(null)
const canvas=ref<HTMLCanvasElement|null>(null)
const unavailable=computed(()=>!props.ready?label('Сначала соберите текущий код.','Build the current source first.')
  :!selected.value?label('Выберите ровно одно тело.','Select exactly one body.')
  :!bounds.value?label('Нужна корректная треугольная сетка.','A valid triangle mesh is required.'):'')
function invalidate(note=''){result.value=null;summary.value=null;planned.value=null;error.value='';message.value=note}
watch([()=>props.meshes,()=>props.source,()=>props.ready,()=>props.selection.join(','),z,machine,process],()=>invalidate(),{deep:true,flush:'sync'})
watch(bounds,value=>{if(value)z.value=(value.min[2]+value.max[2])/2},{immediate:true})
function makePlan(){
  if(unavailable.value||!selected.value)throw new Error(unavailable.value)
  const mesh=meshDataToPolygon(selected.value)
  if(!mesh)throw new Error(label('Пустая сетка.','Empty mesh.'))
  return planLaserMeshSection(mesh,z.value,{...machine},{line:{...process.line},fill:{...process.fill},offset:[...process.offset]})
}
function compile(kind:'preflight'|'job'|'frame'){
  invalidate()
  try{
    const plan=makePlan();planned.value=plan
    if(kind==='preflight')summary.value=preflightLaserPlan(plan)
    else{
      result.value=kind==='job'?emitLaserGrbl(plan):emitLaserFrame(plan)
      summary.value=result.value.summary
      filename.value=`body-${props.selection[0]!+1}-${kind==='job'?'laser':'frame'}.gcode`
    }
    message.value=kind==='frame'?label('Рамка построена с выключенным лазером.','Laser-off frame generated.'):label('Проверка пройдена.','Preflight passed.')
    requestAnimationFrame(draw)
  }catch(reason){error.value=reason instanceof Error?reason.message:String(reason)}
}
function draw(){
  const element=canvas.value,plan=planned.value,box=summary.value?.bounds
  if(!element||!plan||!box)return
  const context=element.getContext('2d');if(!context)return
  const width=Math.max(box.max[0]-box.min[0],0.001),height=Math.max(box.max[1]-box.min[1],0.001),pad=18
  const scale=Math.min((element.width-pad*2)/width,(element.height-pad*2)/height)
  const point=(p:readonly [number,number])=>[pad+(p[0]-box.min[0])*scale,element.height-pad-(p[1]-box.min[1])*scale] as const
  context.clearRect(0,0,element.width,element.height);context.fillStyle='#fff';context.fillRect(0,0,element.width,element.height)
  for(const operation of plan.operations){if(!operation.output)continue;context.strokeStyle=operation.kind==='line'?'#7c3aed':'#dc5a42';context.lineWidth=1
    for(const path of operation.paths){const first=path.points[0];if(!first)continue;context.beginPath();context.moveTo(...point(first));for(const p of path.points.slice(1))context.lineTo(...point(p));if(path.closed)context.closePath();context.stroke()}}
}
function download(){if(result.value)downloadCad(result.value.gcode,'text/plain;charset=utf-8',filename.value)}
const number=(value:number,digits=2)=>value.toLocaleString(props.locale==='ru'?'ru-RU':'en-US',{maximumFractionDigits:digits})
</script>

<template>
  <details class="laser-panel">
    <summary>Laser CAM · GRBL</summary>
    <p>{{ label('Сечение выбранного тела превращается в Line/Fill операции. Только офлайн-проверка и экспорт; подключения к станку нет.','A section of the selected body becomes Line/Fill operations. Offline validation and export only; no machine connection.') }}</p>
    <p v-if="unavailable">{{ unavailable }}</p>
    <template v-else>
      <div class="laser-grid">
        <label>Z, mm<input v-model.number="z" type="number" step="0.1"></label>
        <label>Offset X<input v-model.number="process.offset[0]" type="number" step="0.1"></label>
        <label>Offset Y<input v-model.number="process.offset[1]" type="number" step="0.1"></label>
        <label>{{ label('Стол X, мм','Bed X, mm') }}<input v-model.number="machine.widthMm" type="number" min="1"></label>
        <label>{{ label('Стол Y, мм','Bed Y, mm') }}<input v-model.number="machine.heightMm" type="number" min="1"></label>
        <label>S max<input v-model.number="machine.maxPower" type="number" min="1" step="1"></label>
        <label>{{ label('Холостой ход, мм/мин','Rapid, mm/min') }}<input v-model.number="machine.estimatedRapidMmMin" type="number" min="1"></label>
        <label>{{ label('Мощность','Power mode') }}<select v-model="machine.powerMode"><option value="m4">M4 dynamic</option><option value="m3">M3 constant</option></select></label>
      </div>
      <label><input v-model="machine.laserModeConfirmed" type="checkbox">{{ label('Подтверждаю GRBL $32=1','Confirm GRBL $32=1') }}</label>
      <label><input v-model="machine.supportsAirAssist" type="checkbox">{{ label('Контроллер поддерживает M8/M9','Controller supports M8/M9') }}</label>
      <label><input v-model="machine.flipY" type="checkbox">Flip Y</label>
      <label><input v-model="machine.returnToOrigin" type="checkbox">{{ label('Вернуться в 0,0','Return to 0,0') }}</label>
      <fieldset><legend><label><input v-model="process.line.output" type="checkbox">Line</label></legend>
        <div class="laser-grid"><label>mm/min<input v-model.number="process.line.speedMmMin" type="number" min="1"></label><label>Power<input v-model.number="process.line.power" type="number" min="1"></label><label>Passes<input v-model.number="process.line.passes" type="number" min="1" max="100"></label><label><input v-model="process.line.airAssist" type="checkbox">Air</label></div>
      </fieldset>
      <fieldset><legend><label><input v-model="process.fill.output" type="checkbox">Fill</label></legend>
        <div class="laser-grid"><label>mm/min<input v-model.number="process.fill.speedMmMin" type="number" min="1"></label><label>Power<input v-model.number="process.fill.power" type="number" min="1"></label><label>Passes<input v-model.number="process.fill.passes" type="number" min="1" max="100"></label><label>Spacing<input v-model.number="process.fill.spacingMm" type="number" min="0.001" step="0.05"></label><label><input v-model="process.fill.airAssist" type="checkbox">Air</label></div>
      </fieldset>
      <div class="laser-actions"><button @click="compile('preflight')">Preflight</button><button @click="compile('frame')">{{ label('Рамка G0','Frame G0') }}</button><button @click="compile('job')">{{ label('Собрать GRBL','Generate GRBL') }}</button><button :disabled="!result" @click="download">{{ label('Скачать','Download') }}</button></div>
      <p v-if="error" role="alert">{{ error }}</p><p v-if="message" role="status">{{ message }}</p>
      <section v-if="summary"><p>{{ summary.operationCount }} op · {{ summary.pathCount }} paths · {{ summary.segmentCount }} segments<br>{{ number(summary.cutDistanceMm) }} mm cut · {{ number(summary.rapidDistanceMm) }} mm rapid · ≈ {{ number(summary.estimatedTimeS,1) }} s</p><canvas ref="canvas" width="300" height="220" role="img" :aria-label="label('Предпросмотр лазерных траекторий','Laser toolpath preview')"></canvas></section>
      <textarea v-if="result" :value="result.gcode" readonly aria-label="GRBL G-code" />
    </template>
  </details>
</template>

<style scoped>
.laser-panel{border:1px solid var(--border);border-radius:6px;margin:8px 0;padding:0 7px 7px}.laser-panel summary{cursor:pointer;padding:7px 0}.laser-panel p{line-height:1.4}.laser-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:5px}.laser-grid label{display:flex;flex-direction:column;align-items:stretch}.laser-grid input,.laser-grid select{width:auto!important}.laser-actions{display:flex;gap:5px;flex-wrap:wrap;margin:7px 0}fieldset{border:1px solid var(--border);margin:7px 0}canvas{width:100%;height:auto;border:1px solid var(--border);background:#fff}textarea{box-sizing:border-box;width:100%;min-height:140px;font:10px/1.35 monospace}[role=alert]{color:var(--danger)}
</style>
