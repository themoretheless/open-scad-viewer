<script setup lang="ts">
import CadQuantityInput from './CadQuantityInput.vue'
interface Values {x:number;y:number;z:number;end:'start'|'end';profileGap:number}
const props=defineProps<{advanced:Values;locale:string;dimension:number;pick:{radius:number}|null;cut?:number[]}>()
const emit=defineEmits<{'update:advanced':[Partial<Values>];validity:[string,boolean];numeric:[]}>()
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
function update<K extends keyof Values>(key:K,value:Values[K]){emit('update:advanced',{[key]:value})}
</script>
<template>
                <small>{{ label('Оранжевый — исходная кривая, зелёный — сохраняемая часть. Щёлкните по исходной кривой или задайте координаты точки разреза.','Orange is the original curve; green is the retained part. Click the original curve or enter the cut point coordinates.') }}</small>
                <template v-if="!pick"><label v-for="axis in (dimension===2?['x','y']:['x','y','z']) as ('x'|'y'|'z')[]" :key="axis">{{ axis.toUpperCase() }}<CadQuantityInput :model-value="advanced[axis]" @update:model-value="value=>update(axis,value)" :locale="locale" :aria-label="label('Точка разреза ','Cut point ')+axis.toUpperCase()" @validity="emit('validity','pointTrim'+axis,$event)" /></label></template>
                <template v-else><output data-testid="point-trim-picked">{{ cut?.map(v=>v.toFixed(5)).join(', ') }} · {{ pick.radius }} px</output><button @click="emit('numeric')">{{ label('Ввести координаты','Enter coordinates') }}</button></template>
                <label>{{ label('Сохранить конец','Retain endpoint') }}<select :value="advanced.end" @change="update('end',($event.target as HTMLSelectElement).value as 'start'|'end')" :aria-label="label('Сохранить конец','Retain endpoint')"><option value="start">{{ label('Начало','Start') }}</option><option value="end">{{ label('Конец','End') }}</option></select></label>
                <label v-if="!pick">{{ label('Допуск захвата, мм','Capture distance, mm') }}<CadQuantityInput :model-value="advanced.profileGap" @update:model-value="value=>update('profileGap',value)" :locale="locale" :min="0.000000001" :max="1000000" :aria-label="label('Допуск захвата, мм','Capture distance, mm')" @validity="emit('validity','pointTrimDistance',$event)" /></label>
</template>
