<script setup lang="ts">
import { computed, ref } from 'vue'
import type { DirectSketch } from '../services/directModeling'
import { sketchDimensions, editSketchDimension, type SketchDimension } from '../services/directDimensions'
const props=defineProps<{sketch:DirectSketch;locale:string}>()
const emit=defineEmits<{change:[sketch:DirectSketch]}>()
const kind=ref<'length'|'angle'|'horizontal'|'vertical'|'radius'|'diameter'>('length'),a=ref(0),b=ref(1),c=ref(2),error=ref('')
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
const measurements=computed(()=>sketchDimensions(props.sketch).measurements)
function apply(work:()=>DirectSketch){try{const next=work();sketchDimensions(next);emit('change',next);error.value=''}catch(e){error.value=e instanceof Error?e.message:String(e)}}
function add(){apply(()=>({...props.sketch,dimensions:[...(props.sketch.dimensions??[]),kind.value==='radius'||kind.value==='diameter'?{kind:kind.value}:{kind:kind.value,a:a.value,b:b.value,...(kind.value==='angle'?{c:c.value}:{})} as SketchDimension]}))}
function edit(index:number,event:Event){apply(()=>editSketchDimension(props.sketch,index,Number((event.target as HTMLInputElement).value)))}
function remove(index:number){apply(()=>({...props.sketch,dimensions:props.sketch.dimensions?.filter((_,i)=>i!==index)}))}
</script>
<template>
  <section class="dimension-panel" aria-label="Sketch dimensions">
    <strong>{{ label('Размеры эскиза','Sketch dimensions') }}</strong>
    <small>{{ label('Длина, горизонталь, вертикаль или угол A–B–C. Номера точек показаны на эскизе.','Length, horizontal, vertical or angle A–B–C. Point numbers appear on the sketch.') }}</small>
    <div><select v-model="kind" aria-label="Dimension kind"><option value="length">{{ label('Длина · мм','Length · mm') }}</option><option value="horizontal">{{ label('Горизонталь · мм','Horizontal · mm') }}</option><option value="vertical">{{ label('Вертикаль · мм','Vertical · mm') }}</option><option value="angle">{{ label('Угол · °','Angle · °') }}</option><option v-if="sketch.analytic" value="radius">{{ label('Радиус · мм','Radius · mm') }}</option><option v-if="sketch.analytic" value="diameter">{{ label('Диаметр · мм','Diameter · mm') }}</option></select></div>
    <div v-if="kind!=='radius'&&kind!=='diameter'" class="point-inputs"><label v-for="key in (kind==='angle'?['a','b','c']:['a','b'])" :key="key">{{ key.toUpperCase() }}<select :value="key==='a'?a:key==='b'?b:c" @change="key==='a'?a=Number(($event.target as HTMLSelectElement).value):key==='b'?b=Number(($event.target as HTMLSelectElement).value):c=Number(($event.target as HTMLSelectElement).value)"><option v-for="(_,i) in sketch.points" :key="i" :value="i">{{ i }}</option></select></label></div>
    <button @click="add">{{ label('Добавить размер','Add dimension') }}</button>
    <label v-for="(dimension,i) in sketch.dimensions??[]" :key="i" class="dimension-row">{{ dimension.kind==='length'?'↔':dimension.kind==='angle'?'∠':dimension.kind==='horizontal'?'↔h':dimension.kind==='vertical'?'↕':dimension.kind==='radius'?'R':'Ø' }} {{ 'a' in dimension ? dimension.a+'–'+dimension.b : '' }}{{ dimension.kind==='angle'?'–'+dimension.c:'' }}<input :aria-label="'Dimension '+i" type="number" step="any" :value="measurements[i]?.value" @change="edit(i,$event)">{{ dimension.kind==='angle'?'°':'mm' }}<button :aria-label="label('Удалить размер','Delete dimension')" @click="remove(i)">×</button></label>
    <small>{{ label('Правка перемещает B для длины или C для угла. Остальные размеры пересчитываются.','Editing moves B for length or C for angle. Other dimensions are remeasured.') }}</small>
    <small v-if="sketch.analytic">{{ label('Размеры дуг и окружностей доступны для измерения.','Arc and circle dimensions are read-only.') }}</small>
    <p v-if="error" role="alert">{{ error }}</p>
  </section>
</template>
<style scoped>
.dimension-panel{display:grid;gap:8px}.dimension-panel small{color:var(--text-dim)}.point-inputs,.dimension-row{display:flex;gap:6px;align-items:center}.dimension-row input{width:85px}button,input,select{font:inherit;color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:4px;padding:5px}.point-inputs label{display:flex;align-items:center;gap:4px}p{color:var(--danger)}
</style>
