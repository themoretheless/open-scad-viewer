<script setup lang="ts">
import { computed, onUnmounted, ref, useId, watch } from 'vue'
import { parseCadQuantity, type QuantityKind } from '../services/cadQuantity'
defineOptions({ inheritAttrs: false })
const props = withDefaults(defineProps<{modelValue: number; kind?: QuantityKind; locale?: string; min?: number; max?: number}>(), {kind:'length',locale:'en'})
const emit = defineEmits<{'update:modelValue':[number]; validity:[boolean]}>()
const raw = ref<string | null>(null), failure = ref('')
const errorId=useId()+'-error'
let emittedValue:number|undefined
const hint = computed(() => props.kind === 'length' ? 'mm, cm, m, in' : props.kind === 'angle' ? 'deg, rad, °' : '')
const message = computed(() => !failure.value ? '' : props.locale === 'ru'
  ? failure.value === 'unit' ? `Допустимые единицы: ${hint.value || 'без единиц'}` : failure.value === 'range' ? 'Значение вне допустимого диапазона' : 'Введите конечное число'
  : failure.value === 'unit' ? `Allowed units: ${hint.value || 'unitless'}` : failure.value === 'range' ? 'Value is outside the allowed range' : 'Enter a finite number')
const text = computed({get:()=>raw.value ?? String(props.modelValue),set:(value:string|number)=>{
  raw.value=String(value)
  try {
    const result=parseCadQuantity(raw.value,props.kind,props.min,props.max)
    failure.value=result.valid?'':result.reason
    emit('validity',result.valid)
    if(result.valid){emittedValue=result.value;emit('update:modelValue',result.value)}
  } catch { failure.value='number';emit('validity',false) }
}})
watch(()=>props.modelValue,value=>{
  // Keep the user's units and spelling when the parent echoes our own edit.
  if(Object.is(value,emittedValue)){emittedValue=undefined;return}
  emittedValue=undefined;raw.value=null;failure.value='';emit('validity',true)
})
onUnmounted(()=>emit('validity',true))
</script>
<template>
  <span class="quantity-field">
    <input v-bind="$attrs" v-model="text" type="text" :aria-invalid="!!failure" :aria-describedby="[$attrs['aria-describedby'], message ? errorId : undefined].filter(Boolean).join(' ') || undefined" :aria-errormessage="message ? errorId : undefined" :title="message || hint" autocomplete="off" spellcheck="false" />
    <small v-if="message" :id="errorId" role="alert">{{ message }}</small>
  </span>
</template>
<style scoped>
.quantity-field{display:inline-flex;flex-direction:column;min-width:0;max-width:100%}
input{font:inherit;color:inherit;background:var(--surface-raised);border:1px solid var(--border);border-radius:5px;padding:7px 10px;min-width:0;width:100%;box-sizing:border-box}
input[aria-invalid=true]{border-color:var(--danger,#ff6978)}
small{color:var(--danger,#ff6978);font-size:11px;line-height:1.3;white-space:normal}
</style>
