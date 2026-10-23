<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import CadQuantityInput from './CadQuantityInput.vue'
import type { DirectSketch } from '../services/directModeling'
import { sketchDimensions, sketchDimensionStatus, editSketchDimension, type SketchDimension } from '../services/directDimensions'
const props=defineProps<{sketch:DirectSketch;locale:string}>()
const emit=defineEmits<{change:[sketch:DirectSketch]}>()
const kind=ref<'length'|'angle'|'horizontal'|'vertical'|'radius'|'diameter'>('length'),a=ref(0),b=ref(1),c=ref(2),error=ref('')
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
const values=ref<Record<number,number>>({}), valid=ref<Record<number,boolean>>({})
watch(()=>props.sketch,()=>{values.value={};valid.value={};error.value=''})
watch(()=>props.sketch.analytic,curve=>{kind.value=curve?'radius':'length'},{immediate:true})
const constraintState=computed(()=>{try{return sketchDimensionStatus(props.sketch)}catch{return null}})
const measurements=computed(()=>sketchDimensions(props.sketch).measurements)
function apply(work:()=>DirectSketch){try{const next=work();sketchDimensions(next);emit('change',next);error.value=''}catch(e){error.value=e instanceof Error?e.message:String(e);if(error.value.includes('conflict'))error.value=label('Размеры противоречат друг другу. Изменение не применено.','Dimensions conflict. The change was not applied.')}}
function add(){apply(()=>({...props.sketch,dimensions:[...(props.sketch.dimensions??[]),props.sketch.analytic && kind.value!=='diameter'?{kind:'radius'}:kind.value==='radius'||kind.value==='diameter'?{kind:kind.value}:{kind:kind.value,a:a.value,b:b.value,...(kind.value==='angle'?{c:c.value}:{})} as SketchDimension]}))}
function edit(index:number){if(valid.value[index]!==false&&values.value[index]!==undefined)apply(()=>editSketchDimension(props.sketch,index,values.value[index]))}
function remove(index:number){apply(()=>({...props.sketch,dimensions:props.sketch.dimensions?.filter((_,i)=>i!==index)}))}
</script>
<template>
  <section class="dimension-panel" aria-label="Sketch dimensions">
    <strong>{{ label('Размеры эскиза','Sketch dimensions') }}</strong>
    <small>{{ label('Длина, горизонталь, вертикаль или угол A–B–C. Номера точек показаны на эскизе.','Length, horizontal, vertical or angle A–B–C. Point numbers appear on the sketch.') }}</small>
    <div><select v-model="kind" aria-label="Dimension kind"><option v-if="!sketch.analytic" value="length">{{ label('Длина · мм','Length · mm') }}</option><option v-if="!sketch.analytic" value="horizontal">{{ label('Горизонталь · мм','Horizontal · mm') }}</option><option v-if="!sketch.analytic" value="vertical">{{ label('Вертикаль · мм','Vertical · mm') }}</option><option v-if="!sketch.analytic" value="angle">{{ label('Угол · °','Angle · °') }}</option><option v-if="sketch.analytic" value="radius">{{ label('Радиус · мм','Radius · mm') }}</option><option v-if="sketch.analytic" value="diameter">{{ label('Диаметр · мм','Diameter · mm') }}</option></select></div>
    <div v-if="!sketch.analytic && kind!=='radius'&&kind!=='diameter'" class="point-inputs"><label v-for="key in (kind==='angle'?['a','b','c']:['a','b'])" :key="key">{{ key.toUpperCase() }}<select :value="key==='a'?a:key==='b'?b:c" @change="key==='a'?a=Number(($event.target as HTMLSelectElement).value):key==='b'?b=Number(($event.target as HTMLSelectElement).value):c=Number(($event.target as HTMLSelectElement).value)"><option v-for="(_,i) in sketch.points" :key="i" :value="i">{{ i }}</option></select></label></div>
    <button @click="add">{{ label('Добавить размер','Add dimension') }}</button>
    <label v-for="(dimension,i) in sketch.dimensions??[]" :key="i" class="dimension-row">{{ dimension.kind==='length'?'↔':dimension.kind==='angle'?'∠':dimension.kind==='horizontal'?'↔h':dimension.kind==='vertical'?'↕':dimension.kind==='radius'?'R':'Ø' }} {{ 'a' in dimension ? dimension.a+'–'+dimension.b : '' }}{{ dimension.kind==='angle'?'–'+dimension.c:'' }}<CadQuantityInput :aria-label="'Dimension '+i" :model-value="values[i]??Number((measurements[i]?.value??0).toFixed(6))" :kind="dimension.kind==='angle'?'angle':'length'" :locale="locale" :min="dimension.kind==='radius' ? .01 : dimension.kind==='diameter' ? .02 : dimension.kind==='length' ? .000001 : dimension.kind==='angle'?-180:-1000000" :max="dimension.kind==='angle'?180:1000000" @update:model-value="values[i]=$event" @validity="valid[i]=$event" @keydown.enter.stop.prevent="edit(i)" />{{ dimension.kind==='angle'?'°':'mm' }}<button :disabled="valid[i]===false || values[i]===undefined" :aria-label="label('Применить размер ','Apply dimension ')+i" @click="edit(i)">✓</button><button :aria-label="label('Удалить размер','Delete dimension')" @click="remove(i)">×</button></label>
    <small>{{ label('Введите размер и нажмите ✓ или Enter. Одиночная длина перемещает B, угол — C; остальные измерения обновляются.','Enter a value, then press ✓ or Enter. A single length moves B; an angle moves C. Other measurements update.') }}</small>
    <small>{{ label('Несколько размеров длины решаются совместно для 3–16 точек. Горизонталь и вертикаль задают расстояние по оси, а не ограничение направления.', 'Multiple lengths are solved together for 3–16 points. Horizontal and vertical dimensions measure axis distances, not direction constraints.') }}</small>
    <small v-if="constraintState" role="status">{{ label('Свободных степеней: ','Degrees of freedom: ')+constraintState.degrees_of_freedom }} · {{ label('включая положение эскиза','including sketch placement') }}<template v-if="constraintState.redundant_equations"> · {{ label('Избыточных размеров: ','Redundant dimensions: ')+constraintState.redundant_equations }}</template></small>
    <p v-if="error" role="alert">{{ error }}</p>
  </section>
</template>
<style scoped>
.dimension-panel{display:grid;gap:8px}.dimension-panel small{color:var(--text-dim)}.point-inputs,.dimension-row{display:flex;gap:6px;align-items:center}.dimension-row input{width:85px}button,input,select{font:inherit;color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:4px;padding:5px}.point-inputs label{display:flex;align-items:center;gap:4px}p{color:var(--danger)}
</style>
