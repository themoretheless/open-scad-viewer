<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { stringifyMeshJson } from '../services/meshJson'
import { compileModelGraph } from '../services/modelGraphCompiler'
import { createMechanicalDocument, GEAR_DEFAULTS, PLANETARY_DEFAULTS, THREAD_DEFAULTS } from '../services/mechanicalGeneratorContract'
const props=defineProps<{open:boolean;locale:'ru'|'en'}>()
const emit=defineEmits<{close:[];generate:[source:string,name:string];downloadCurrent:[]}>()
const dialog=ref<HTMLDialogElement|null>(null)
const kind=ref<'gear'|'planetary_gears'|'thread'>('gear')
const values=ref<Record<string,number|boolean>>({...GEAR_DEFAULTS})
const error=ref('')
const generated=ref<ReturnType<typeof compileModelGraph>|null>(null)
const ru=computed(()=>props.locale==='ru')
const names:Record<string,[string,string]>={
  teeth:['Число зубьев','Teeth'],module:['Модуль, мм','Module, mm'],pressure_angle:['Угол давления, °','Pressure angle, °'],thickness:['Толщина, мм','Thickness, mm'],bore:['Отверстие, мм','Bore diameter, mm'],backlash:['Боковой зазор на шестерню, мм','Backlash per gear, mm'],clearance:['Зазор, мм','Clearance, mm'],internal:['Внутренний вариант','Internal'],rim_width:['Толщина обода, мм','Rim width, mm'],flank_segments:['Сегменты боковой линии зуба','Flank segments'],sun_teeth:['Зубья солнечной шестерни','Sun teeth'],planet_teeth:['Зубья сателлита','Planet teeth'],planet_count:['Число сателлитов','Planets'],carrier_angle:['Поворот водила, °','Carrier angle, °'],diameter:['Диаметр резьбы, мм','Thread diameter, mm'],pitch:['Шаг резьбы, мм','Thread pitch, mm'],length:['Длина, мм','Length, mm'],wall:['Стенка втулки, мм','Sleeve wall, mm'],starts:['Число заходов','Starts'],left_handed:['Левая резьба','Left handed'],segments_per_turn:['Сегменты на оборот','Segments per turn'],
}
watch(kind,()=>{values.value={...(kind.value==='gear'?GEAR_DEFAULTS:kind.value==='thread'?THREAD_DEFAULTS:PLANETARY_DEFAULTS)};generated.value=null;error.value=''})
watch(()=>values.value.internal,internal=>{if(kind.value==='gear'&&internal){values.value.bore=0;if(Number(values.value.teeth)<36)values.value.teeth=72}})
watch(values,()=>{generated.value=null;error.value=''},{deep:true})
watch(()=>props.open,async open=>{await nextTick();if(open&&!dialog.value?.open)dialog.value?.showModal();else if(!open&&dialog.value?.open)dialog.value.close()})
function generate(){
  error.value=''
  try {generated.value=compileModelGraph(createMechanicalDocument({kind:kind.value,...values.value}))}
  catch(e){generated.value=null;error.value=e instanceof Error?e.message:String(e)}
}
function download(){
  if(!generated.value)return
  const url=URL.createObjectURL(new Blob([stringifyMeshJson(generated.value.document, 2)],{type:'application/json'}))
  const a=document.createElement('a');a.href=url;a.download=`${kind.value}.modelgraph.json`;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)
}
function openEditor(save:boolean){
  if(!generated.value)return
  if(save)emit('downloadCurrent')
  emit('generate',generated.value.source,`${kind.value}.scad`);emit('close')
}
</script>
<template>
  <Teleport to="body">
    <dialog ref="dialog" class="mechanical-dialog" aria-labelledby="mechanical-title" @cancel.prevent="emit('close')">
      <header><h2 id="mechanical-title">{{ru?'Генераторы деталей':'Part generators'}}</h2><button type="button" :aria-label="ru?'Закрыть':'Close'" @click="emit('close')">×</button></header>
      <form @submit.prevent="generate">
        <label class="kind-label">{{ru?'Что построить':'Part type'}}
          <select v-model="kind" :aria-label="ru?'Что построить':'Part type'"><option value="gear">{{ru?'Шестерня':'Gear'}}</option><option value="planetary_gears">{{ru?'Планетарная передача':'Planetary gearset'}}</option><option value="thread">{{ru?'Резьба':'Thread'}}</option></select>
        </label>
        <p class="hint">{{kind==='planetary_gears'?(ru?'Солнце, сателлиты и неподвижный венец. Водило и оси нужно моделировать отдельно.':'Sun, planets and fixed ring. Model the carrier and shafts separately.'):kind==='thread'?(ru?'Наружный стержень или втулка с внутренней резьбой. Зазор подбирайте под свой принтер.':'Externally threaded rod or internally threaded sleeve. Adjust clearance for your printer.'):(ru?'Прямозубая эвольвентная шестерня или внутренний венец.':'Involute spur gear or internal ring.')}}</p>
        <div class="fields"><label v-for="(value,key) in values" :key="key" :class="{check:typeof value==='boolean'}">
          <span>{{names[key]?.[ru?0:1]??key}}</span>
          <input v-if="typeof value==='boolean'" v-model="values[key]" type="checkbox">
          <input v-else v-model.number="values[key]" type="number" step="any" required>
        </label></div>
        <p v-if="error" class="error" role="alert">{{error}}</p>
        <button class="primary" type="submit">{{ru?'Создать модель':'Generate model'}}</button>
      </form>
      <section v-if="generated" class="generated" aria-live="polite">
        <p>{{ru?'Модель создана. Открытие заменит текущий текст в редакторе.':'Model generated. Opening it replaces the current editor text.'}}</p>
        <div class="actions"><button type="button" @click="download">{{ru?'Скачать ModelGraph':'Download ModelGraph'}}</button><button type="button" @click="openEditor(true)">{{ru?'Скачать текущую и открыть':'Save current and open'}}</button><button class="primary" type="button" @click="openEditor(false)">{{ru?'Открыть в редакторе':'Open in editor'}}</button></div>
      </section>
    </dialog>
  </Teleport>
</template>
<style scoped>
.mechanical-dialog{box-sizing:border-box;width:min(650px,calc(100vw - 32px));max-height:90vh;overflow:auto;border:1px solid var(--border);border-radius:14px;background:var(--surface);color:var(--text);padding:24px;box-shadow:0 18px 70px #0005}
.mechanical-dialog::backdrop{background:#0008}header{display:flex;align-items:center;justify-content:space-between;gap:16px}h2{font-size:20px;margin:0 0 16px}button,select,input{font:inherit;color:inherit;border:1px solid var(--border);border-radius:6px;background:var(--surface-raised);padding:8px}button{cursor:pointer}button:focus-visible,input:focus-visible,select:focus-visible{outline:2px solid var(--accent);outline-offset:2px}header button{font-size:24px;border:0;background:none}.kind-label{display:grid;gap:6px}.hint{font-size:13px;line-height:1.5;opacity:.8}.fields{display:grid;grid-template-columns:1fr 1fr;gap:14px;margin:18px 0}.fields label{display:grid;gap:6px;font-size:13px}.fields input{width:100%;box-sizing:border-box}.fields .check{display:flex;align-items:center;justify-content:space-between}.check input{width:auto}.primary{background:var(--accent);color:var(--accent-contrast,#fff)}.error{color:#df6262;overflow-wrap:anywhere}.generated{border-top:1px solid var(--border);margin-top:20px;padding-top:12px}.generated p{font-size:13px}.actions{display:flex;flex-wrap:wrap;gap:8px}@media(max-width:480px){.fields{grid-template-columns:1fr}.mechanical-dialog{padding:16px}}
</style>
