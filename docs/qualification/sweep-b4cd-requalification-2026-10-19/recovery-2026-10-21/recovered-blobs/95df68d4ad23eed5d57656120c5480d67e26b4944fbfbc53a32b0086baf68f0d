<script setup lang="ts">
import {computed,ref,shallowRef,watch} from 'vue'
import {evaluateThermalStrength,type PrintStrengthProfile,type ThermalSample,type ThermalEditorState} from '../services/trussScreening'

const props=defineProps<{profile:PrintStrengthProfile;locale:string}>()
const emit=defineEmits<{change:[state:ThermalEditorState]}>()
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
const enabled=ref(false),serviceTempC=ref<number|string>('')
const calibrationProfile=shallowRef<PrintStrengthProfile|null>(null)
type DraftSample=Record<keyof ThermalSample,number|string>
const empty=():DraftSample=>({nozzleTempC:props.profile.nozzleTempC||'',serviceTempC:'',youngMpa:'',tensionMpa:'',compressionMpa:''})
const samples=ref<DraftSample[]>([empty(),empty()])
const fields=[{key:'nozzleTempC',ru:'Сопло, °C',en:'Nozzle, °C'},{key:'serviceTempC',ru:'Эксплуатация, °C',en:'Service, °C'},
  {key:'youngMpa',ru:'E, MPa',en:'E, MPa'},{key:'tensionMpa',ru:'Растяжение, MPa',en:'Tension, MPa'},
  {key:'compressionMpa',ru:'Сжатие, MPa',en:'Compression, MPa'}] as const
const state=computed<ThermalEditorState>(()=>{
  if(!enabled.value)return {enabled:false,evaluation:null,error:''}
  if(!calibrationProfile.value)return {enabled:true,evaluation:null,error:label('Заполните измерения и привяжите таблицу к профилю.','Enter measurements and bind the table to the profile.')}
  try{
    // Empty cells remain invalid transport values, never implicit zero measurements.
    const numeric=(v:number|string)=>v===''?NaN:Number(v)
    const request={profile:{...props.profile},calibrationProfile:calibrationProfile.value,serviceTempC:numeric(serviceTempC.value),
      samples:samples.value.map(s=>({nozzleTempC:numeric(s.nozzleTempC),serviceTempC:numeric(s.serviceTempC),youngMpa:numeric(s.youngMpa),tensionMpa:numeric(s.tensionMpa),compressionMpa:numeric(s.compressionMpa)}))}
    return {enabled:true,evaluation:{request,result:evaluateThermalStrength(request)},error:''}
  }catch(error){return {enabled:true,evaluation:null,error:String(error)}}
})
watch(state,value=>emit('change',value),{immediate:true,flush:'sync'})
function bind(){calibrationProfile.value={...props.profile}}
</script>

<template>
  <fieldset class="thermal-editor">
    <legend>{{label('Температурная модель','Temperature model')}}</legend>
    <label class="toggle"><input v-model="enabled" type="checkbox">{{label('Свойства по температурной таблице','Properties from temperature table')}}</label>
    <template v-if="enabled">
      <p>{{label('Измеренные E и пределы для выбранного пластика и процесса. Для каждой температуры сопла нужны все указанные температуры эксплуатации. Интерполяция только внутри таблицы; без ползучести, тепловых напряжений и анизотропии.','Measured E and limits for this plastic and process. Include every service temperature for every nozzle temperature. Interpolation stays inside the table; creep, thermal stresses and anisotropy are excluded.')}}</p>
      <label>{{label('Температура эксплуатации детали, °C','Part service temperature, °C')}}<input v-model.number="serviceTempC" type="number" step="1"></label>
      <div class="samples"><fieldset v-for="(sample,i) in samples" :key="i">
        <legend>{{label('Измерение','Sample')}} {{i+1}}</legend>
        <div class="fields"><label v-for="field in fields" :key="field.key">{{label(field.ru,field.en)}}<input v-model.number="sample[field.key]" type="number" step="any" :aria-label="`Sample ${i+1} ${field.key}`"></label></div>
        <button :disabled="samples.length<=2" @click="samples.splice(i,1)">{{label('Удалить измерение','Remove sample')}}</button>
      </fieldset></div>
      <div class="commands"><button :disabled="samples.length>=64" @click="samples.push(empty())">{{label('Добавить измерение','Add sample')}}</button>
        <button @click="bind">{{label('Привязать измерения к текущему профилю','Bind measurements to current profile')}}</button></div>
      <p>{{label('Привязка означает, что источник свойств в профиле относится к этим измерениям. Смена материала, сопла, линии, слоя или температуры стола требует другой калибровки.','Binding declares that the profile property source describes these measurements. A different material, nozzle, line, layer or bed temperature requires a different calibration.')}}</p>
      <p v-if="state.error" role="alert">{{state.error}}</p>
      <p v-if="state.evaluation" role="status">{{label('Интерполированные свойства','Interpolated properties')}}: E = {{state.evaluation.result.youngMpa}} MPa;
        {{label('растяжение','tension')}} = {{state.evaluation.result.tensionMpa}} MPa;
        {{label('сжатие','compression')}} = {{state.evaluation.result.compressionMpa}} MPa.</p>
    </template>
  </fieldset>
</template>

<style scoped>
.thermal-editor{border:0;border-top:1px solid var(--border);padding:8px 0;min-width:0}legend{font-weight:600}p{line-height:1.4;overflow-wrap:anywhere}
label{display:flex;flex-direction:column;gap:4px;margin:6px 0;min-width:0}.toggle{flex-direction:row;align-items:center}
input,button{font:inherit;background:var(--surface-raised);color:var(--text);border:1px solid var(--border);border-radius:4px;padding:5px;min-width:0;box-sizing:border-box}input:not([type=checkbox]){width:100%}
.fields{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:6px}.samples{max-height:320px;overflow:auto}.samples fieldset{border:1px solid var(--border);min-width:0;margin:8px 0;padding:6px}
.commands{display:flex;gap:6px;flex-wrap:wrap}button{cursor:pointer;white-space:normal}button:disabled{opacity:.5}[role=alert]{color:var(--danger)}
</style>
