<script setup lang="ts">
import {computed,ref,toRaw,watch} from 'vue'
import type {NurbsSurface} from '../services/nurbsSurface'
import type {OffsetDeviationOptions} from '../services/nurbsOffsetDeviation'
import {useOffsetDeviationInspection} from '../services/useOffsetDeviationInspection'
const props=defineProps<{surfaces:[NurbsSurface,NurbsSurface];active:boolean;ru:boolean}>()
const label=(ru:string,en:string)=>props.ru?ru:en
const opened=ref(false),distance=ref(1),tolerance=ref(.001),budget=ref(10000),revision=ref(0)
watch(()=>props.surfaces,()=>revision.value++,{deep:true,flush:'sync'})
const valid=computed(()=>Number.isFinite(distance.value)&&Number.isFinite(tolerance.value)&&tolerance.value>0)
const options=computed<OffsetDeviationOptions|undefined>(()=>{
 revision.value
 if(!opened.value||!valid.value)return undefined
 return {source:toRaw(props.surfaces[0]),candidate:toRaw(props.surfaces[1]),distance:distance.value,
  tolerance:tolerance.value,maxCells:budget.value,maxDepth:24}
})
const active=computed(()=>props.active&&opened.value)
const {result,pending,error,run,cancel}=useOffsetDeviationInspection(options,active)
</script>
<template>
 <section class="body-diagnostics offset-deviation-check" @keydown.esc.stop.prevent="cancel" @keydown.enter.prevent="run">
  <button type="button" :aria-expanded="opened" @click="opened=!opened">{{ label('Проверить смещение A → B','Inspect offset A → B') }}</button>
  <div v-if="opened">
   <label>{{ label('Смещение, мм','Offset, mm') }}<input v-model.number="distance" type="number" step="any" :aria-label="label('Смещение, мм','Offset, mm')" :aria-invalid="!Number.isFinite(distance)" /></label>
   <label>{{ label('Допуск, мм','Tolerance, mm') }}<input v-model.number="tolerance" type="number" min="0" step="any" :aria-label="label('Допуск, мм','Tolerance, mm')" :aria-invalid="!Number.isFinite(tolerance)||tolerance<=0" /></label>
   <label>{{ label('Объём расчёта','Calculation budget') }}<select v-model.number="budget" :aria-label="label('Объём расчёта offset','Offset calculation budget')"><option :value="10000">{{ label('Обычный','Standard') }}</option><option :value="100000">{{ label('Расширенный','Extended') }}</option></select></label>
   <p>{{ label('A — исходная поверхность, B — проверяемая. Знак смещения задаёт сторону нормали A.','A is the source surface; B is the candidate. The sign selects the side of A’s normal.') }}</p>
   <button v-if="!pending" type="button" :disabled="!valid||!active" @click="run">{{ error?label('Повторить · Enter','Retry · Enter'):label('Проверить · Enter','Inspect · Enter') }}</button>
   <button v-else type="button" @click="cancel">{{ label('Отменить · Esc','Cancel · Esc') }}</button>
   <p v-if="pending" role="status">{{ label('Проверяю всю поверхность…','Checking the whole surface…') }}</p>
   <p v-if="error" role="alert">{{ label('Не удалось проверить поверхности. Проверьте геометрию и повторите расчёт.','Could not inspect the surfaces. Check the geometry and retry.') }}</p>
   <div v-if="result" data-offset-deviation>
    <p v-if="result.decision==='within_tolerance'" role="status">{{ label('Допуск достигнут. Ошибка не более','Tolerance reached. Error at most') }} <output>{{ result.errorUpper }}</output> {{ label('мм','mm') }}.</p>
    <p v-else-if="result.decision==='exceeds_tolerance'" role="status">{{ label('Допуск превышен. Ошибка не менее','Tolerance exceeded. Error at least') }} <output>{{ result.errorLower }}</output> {{ label('мм','mm') }}.</p>
    <p v-else role="status">{{ label('Покрытие не завершено. Увеличьте объём расчёта или проверьте меньший участок.','Coverage is incomplete. Increase the calculation budget or inspect a smaller patch.') }}</p>
    <p v-if="result.decision==='within_tolerance'">{{ result.idealLocalFoldFree?label('Локальные складки исключены вдоль смещения.','Local folds excluded along the offset.'):label('Отсутствие локальных складок не доказано.','Absence of local folds is unproven.') }}</p>
    <p v-if="result.witnessUv">UV: {{ result.witnessUv.join(', ') }}</p>
   </div>
  </div>
 </section>
</template>
<style scoped>
.offset-deviation-check{margin-top:6px}.offset-deviation-check label{display:flex;align-items:center;justify-content:space-between;gap:4px;font-size:12px}.offset-deviation-check input{width:100px}.offset-deviation-check p{font-size:12px;margin:6px 0}.offset-deviation-check input[aria-invalid="true"]{outline:1px solid #b84a35}
</style>
