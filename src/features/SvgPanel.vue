<script setup lang="ts">
import { computed, ref } from 'vue'
import type { MeshData } from '../core/mesh'
import type { PickHit } from '../services/rendererContracts'
import { svgContours, contoursSvg, contoursExtrusion, meshSvgContours, SVG_MAX_BYTES } from '../services/svgGeometry'
const props = defineProps<{ meshes: MeshData[]; hit: PickHit | null; available: boolean; locale: string }>()
const emit = defineEmits<{ append: [source: string] }>()
const ru = computed(() => props.locale === 'ru')
const text = ref('<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="30mm" viewBox="0 0 40 30"><rect x="2" y="2" width="36" height="26" rx="4"/></svg>')
const height = ref(5), axis = ref<'x'|'y'|'z'>('z'), error = ref(''), busy = ref(false), preview = ref('')
async function run(action: () => Promise<void>) { busy.value=true; error.value=''; try { await action() } catch(e) { error.value=e instanceof Error?e.message:String(e) } finally { busy.value=false } }
function download(svg: string) { const url=URL.createObjectURL(new Blob([svg],{type:'image/svg+xml'})); const a=document.createElement('a'); a.href=url; a.download='profile.svg'; a.click(); setTimeout(()=>URL.revokeObjectURL(url),1000) }
async function normalize() { const svg=contoursSvg(await svgContours(text.value)); preview.value=`data:image/svg+xml,${encodeURIComponent(svg)}`; return svg }
function create(shape: 'rect'|'circle') { text.value=`<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="40mm" viewBox="0 0 40 40">${shape==='rect'?'<rect x="2" y="2" width="36" height="36" rx="4"/>':'<circle cx="20" cy="20" r="18"/>'}</svg>`; preview.value='' }
async function file(event: Event) { const input=event.target as HTMLInputElement, f=input.files?.[0]; if(f) await run(async()=>{ if(f.size>SVG_MAX_BYTES) throw new Error('SVG exceeds 256 KiB.'); text.value=await f.text(); await normalize() }); input.value='' }
async function fromMesh(face: boolean) { const svg=contoursSvg(await meshSvgContours(props.meshes,{axis:axis.value,...(face && props.hit?{face:props.hit}:{})})); text.value=svg; preview.value=`data:image/svg+xml,${encodeURIComponent(svg)}`; download(svg) }
</script>
<template>
  <details class="svg-panel">
    <summary>SVG ↔ 3D</summary>
    <div class="svg-content">
      <p>{{ ru ? 'Создайте фигуру, загрузите SVG или вставьте разметку. Размеры — в мм.' : 'Create a shape, upload SVG or paste markup. Dimensions are in mm.' }}</p>
      <div class="actions"><button @click="create('rect')">{{ ru ? 'Прямоугольник' : 'Rectangle' }}</button><button @click="create('circle')">{{ ru ? 'Круг' : 'Circle' }}</button><label>{{ ru ? 'Открыть SVG' : 'Open SVG' }}<input type="file" accept=".svg,image/svg+xml" :disabled="busy" @change="file"></label></div>
      <textarea v-model="text" aria-label="SVG" spellcheck="false" :maxlength="SVG_MAX_BYTES" @input="preview=''" />
      <img v-if="preview" :src="preview" alt="SVG preview">
      <div class="actions"><button :disabled="busy" @click="run(async()=>{await normalize()})">{{ ru ? 'Предпросмотр' : 'Preview' }}</button><button :disabled="busy" @click="run(async()=>download(await normalize()))">{{ ru ? 'Скачать SVG' : 'Download SVG' }}</button></div>
      <div class="actions"><label>{{ ru ? 'Высота, мм' : 'Height, mm' }} <input v-model.number="height" type="number" min="0.01" max="100000" step="1"></label><button :disabled="busy" @click="run(async()=>emit('append',contoursExtrusion(await svgContours(text),height)))">{{ ru ? 'Добавить экструзию в модель' : 'Add extrusion to model' }}</button></div>
      <div class="actions"><select v-model="axis" :aria-label="ru?'Ось проекции':'Projection axis'"><option>x</option><option>y</option><option>z</option></select><button :disabled="busy || !available" @click="run(()=>fromMesh(false))">{{ ru ? 'Проекция модели → SVG' : 'Model projection → SVG' }}</button><button :disabled="busy || !available || !hit" @click="run(()=>fromMesh(true))">{{ ru ? 'Выбранная грань → SVG' : 'Selected face → SVG' }}</button></div>
      <p>{{ ru ? 'Грань должна быть плоской. Кривые поверхности можно проецировать; развёртка не выполняется. Текст в SVG сначала переведите в контуры.' : 'Face must be planar. Curved surfaces can be projected; no unwrapping. Convert SVG text to outlines first.' }}</p>
      <p v-if="error" role="alert">{{ error }}</p>
    </div>
  </details>
</template>
<style scoped>
.svg-panel{border:1px solid var(--border);border-radius:8px;margin:0;flex:0 0 auto;background:var(--surface);font-size:12px}.svg-panel summary{padding:9px;cursor:pointer}.svg-content{max-height:45vh;overflow:auto;padding:0 10px 10px;display:grid;gap:8px}.svg-content p{margin:0;opacity:.8}.actions{display:flex;gap:6px;flex-wrap:wrap;align-items:center}textarea{width:100%;height:90px;box-sizing:border-box;font:11px monospace}img{max-width:100%;height:100px;background:white}input[type=number]{width:75px}input[type=file]{max-width:190px}button,select,input,textarea{color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:4px;padding:5px}button:disabled{opacity:.4}button{cursor:pointer}[role=alert]{color:#d65b4a}
</style>
