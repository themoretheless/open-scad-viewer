<script setup lang="ts">
import {onUnmounted,ref,shallowRef,watch} from 'vue'
import {computeBondedSolid} from '../services/mainSolidWorker'
import type {BondedSolidResult} from '../services/bondedSolidProtocol'
import {bondedSolidExample} from './bondedSolidExample'
const props=defineProps<{locale:string}>()
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
const input=ref(''),busy=ref(false),error=ref(''),url=ref('')
const result=shallowRef<BondedSolidResult|null>(null)
let revision=0,controller:AbortController|undefined
function clear(){revision++;controller?.abort();controller=undefined;busy.value=false;error.value='';result.value=null;if(url.value)URL.revokeObjectURL(url.value);url.value=''}
watch(input,clear,{flush:'sync'})
onUnmounted(clear)
async function solve(){
  clear();const ticket=revision,source=input.value;controller=new AbortController();busy.value=true
  try{
    const response=await computeBondedSolid(source,{signal:controller.signal})
    if(ticket!==revision)return
    result.value=response
    url.value=URL.createObjectURL(new Blob([JSON.stringify({version:1,model:JSON.parse(source),result:response},null,2)],{type:'application/json'}))
  }catch(e){if(ticket===revision)error.value=String(e)}
  finally{if(ticket===revision){busy.value=false;controller=undefined}}
}
async function load(event:Event){
  const file=(event.target as HTMLInputElement).files?.[0];if(!file)return
  clear();const ticket=revision
  if(file.size>262144){error.value=label('Файл больше 256 KiB.','File exceeds 256 KiB.');return}
  try{const text=await file.text();if(ticket===revision)input.value=text}
  catch(e){if(ticket===revision)error.value=String(e)}
}
</script>
<template>
  <details class="bonded-solid"><summary>{{label('Соединения оболочки и заполнения','Shell–infill connections')}}</summary>
    <p>{{label('Явная объёмная сетка Tet4 и соединения. Автоматической сетки из CAD пока нет.','Explicit Tet4 mesh and bonds. CAD meshing is not available yet.')}}</p>
    <p>{{label('Свойства связей для заданного профиля: жёсткость MPa/mm, пределы MPa.','Measured bond properties for the declared profile: stiffness MPa/mm, strengths MPa.')}}</p>
    <label>{{label('Загрузить сетку','Load mesh')}}<input type="file" accept=".json,application/json" @change="load"></label>
    <button @click="input=JSON.stringify(bondedSolidExample,null,2)">{{label('Синтетический пример — не свойства пластика','Synthetic example — not plastic properties')}}</button>
    <label>{{label('Расчётная модель JSON','Assembly JSON')}}<textarea v-model="input" maxlength="262144" rows="8" spellcheck="false"/></label>
    <button :disabled="busy||!input.trim()" @click="solve">{{label('Рассчитать соединения','Solve connections')}}</button>
    <button v-if="busy" @click="clear">{{label('Отмена','Cancel')}}</button>
    <p v-if="error" role="alert">{{error}}</p>
    <section v-if="result" aria-label="Bonded assembly results">
      <p>{{label('Максимальное перемещение, mm:','Maximum displacement, mm:')}} {{result.maxDeflectionMm.toPrecision(6)}}</p>
      <p :role="result.limitReached?'alert':undefined">{{result.limitReached?label('Предел соединения достигнут с учётом запаса. Расчёт предполагает целые связи; повреждение не моделируется.','A bond limit is reached with the safety factor. Intact bonds assumed; damage is not modeled.'):label('Пределы связей не достигнуты. Прочность всей детали не подтверждена.','Bond limits not reached. Whole-part strength is not established.')}}</p>
      <div class="scroll"><table><thead><tr><th>#</th><th>A, mm²</th><th>Fx, N</th><th>Fy, N</th><th>Fz, N</th><th>{{label('Использование','Utilization')}}</th></tr></thead>
        <tbody><tr v-for="(b,i) in result.bonds" :key="i"><td>{{i+1}}</td><td>{{b.areaMm2.toPrecision(4)}}</td><td v-for="(f,k) in b.forceOnShellN" :key="k">{{f.toPrecision(4)}}</td><td>{{b.utilization.toPrecision(4)}}</td></tr></tbody></table></div>
      <p>{{label('Силы: заполнение → оболочка. Использование ≥ 1 — предел достигнут.','Forces: infill → shell. Utilization ≥ 1 reaches the limit.')}}</p>
      <p>{{label('Изотропная линейная модель. Нужна проверка сходимости сетки; устойчивость не рассчитана.','Isotropic linear model. Mesh convergence needs checking; buckling is not calculated.')}}</p>
      <p v-if="result.profileWarnings.length">{{label('Проверьте высоту слоя и ширину линии.','Check layer height and line width.')}}</p>
      <a :href="url" download="bonded-assembly.json">bonded-assembly.json</a>
    </section>
  </details>
</template>
<style scoped>
.bonded-solid{font-size:12px;margin:10px 0;min-width:0}p{line-height:1.4;overflow-wrap:anywhere}summary{font-weight:600;cursor:pointer}label{display:flex;flex-direction:column;gap:4px;margin:6px 0}textarea,input,button{font:inherit;background:var(--surface-raised);color:var(--text);border:1px solid var(--border);border-radius:4px;padding:5px;min-width:0;box-sizing:border-box;max-width:100%}textarea{width:100%;resize:vertical;font-family:monospace}button{white-space:normal;cursor:pointer}button:disabled{opacity:.5}.scroll{overflow:auto;max-height:240px}table{border-collapse:collapse;width:100%}th,td{padding:4px;text-align:right;white-space:nowrap;border-bottom:1px solid var(--border)}a{color:var(--accent)}[role=alert]{color:var(--danger)}
</style>
