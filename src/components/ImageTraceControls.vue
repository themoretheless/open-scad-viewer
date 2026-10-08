<script setup lang="ts">
import {onBeforeUnmount,ref} from 'vue'
import {imageTraceJob,imageTraceSketches,imageTraceColorJob,imageTraceColorSketches} from '../services/imageTrace'
import {createSvgWorkerClient} from '../services/svgWorkerClient'
import type {DirectSketch} from '../services/directModeling'
import type {SketchPlane} from '../services/directSketchGeometry'
const props=defineProps<{plane:SketchPlane;locale:string;authored?:boolean}>(),emit=defineEmits<{trace:[sketches:DirectSketch[]]}>()
const opened=ref(false),width=ref(100),threshold=ref(.5),resolution=ref(512),tolerance=ref(.1),mode=ref<'dark'|'alpha'|'color'>('dark'),colors=ref(4),ignoreWhite=ref(true),minArea=ref(.1),file=ref<File|null>(null),busy=ref(false),error=ref('')
const worker=createSvgWorkerClient();let generation=0
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
onBeforeUnmount(()=>{generation++;worker.dispose()})
function cancel(){generation++;worker.cancel();busy.value=false}
async function trace(){if(!file.value||busy.value)return;const token=++generation,basis=JSON.parse(JSON.stringify(props.plane)) as SketchPlane,image=file.value;busy.value=true;error.value=''
 try{
  if(image.size>2_800_000)throw Error('Image exceeds 2.8 MiB.')
  const bitmap=await createImageBitmap(image),w=bitmap.width,h=bitmap.height;bitmap.close()
  const dataUrl=await new Promise<string>((resolve,reject)=>{const reader=new FileReader();reader.onload=()=>resolve(String(reader.result));reader.onerror=()=>reject(Error('Could not read image.'));reader.readAsDataURL(image)})
  if(token!==generation)return
  const options={widthMm:width.value,pixelWidth:w,pixelHeight:h,threshold:threshold.value,resolution:resolution.value,tolerance:tolerance.value,mode:mode.value==='dark'?'dark' as const:'alpha' as const}
  const job=mode.value==='color'?imageTraceColorJob(dataUrl,options,colors.value,ignoreWhite.value,minArea.value):imageTraceJob(dataUrl,options)
  const result=await worker.run(job),sketches=mode.value==='color'?imageTraceColorSketches(result.layers??[],tolerance.value,basis,image.name):imageTraceSketches(result.contours??[],tolerance.value,basis,image.name)
  if(token!==generation||JSON.stringify(basis)!==JSON.stringify(props.plane))return
  emit('trace',props.authored?sketches:sketches.map(s=>{const {editablePath:_path,...polygon}=s;return polygon}));opened.value=false
 }catch(e){if(token===generation)error.value=e instanceof Error?e.message:String(e)}finally{if(token===generation)busy.value=false}
}
</script>
<template>
 <button @click="opened=!opened">Image Trace</button>
 <fieldset v-if="opened" class="trace-controls" :disabled="busy"><legend>Image Trace</legend>
  <input type="file" accept="image/png,image/jpeg,image/webp" :aria-label="label('Растровое изображение','Raster image')" @change="file=($event.target as HTMLInputElement).files?.[0]??null">
  <label>{{ label('Ширина, мм','Width, mm') }} <input v-model.number="width" type="number" min=".01" max="10000"></label>
  <label>{{ label('Порог','Threshold') }} <input v-model.number="threshold" type="number" min=".01" max=".99" step=".05"></label>
  <label>{{ label('Разрешение','Resolution') }} <select v-model.number="resolution"><option>128</option><option>256</option><option>512</option><option>1024</option><option>2048</option></select></label>
  <label>{{ label('Допуск, мм','Tolerance, mm') }} <input v-model.number="tolerance" type="number" min=".0001" max="10" step=".05"></label>
  <select v-model="mode" :aria-label="label('Режим трассировки','Trace mode')"><option value="dark">{{ label('Тёмные области','Dark areas') }}</option><option value="alpha">Alpha</option><option value="color">{{label('Цветная палитра','Color palette')}}</option></select>
  <template v-if="mode==='color'"><label>{{label('Цветов','Colors')}}<input v-model.number="colors" type="number" min="2" max="16"></label><label><input v-model="ignoreWhite" type="checkbox">{{label('Без белого фона','Ignore white background')}}</label><label>{{label('Минимальная область, мм²','Minimum region, mm²')}}<input v-model.number="minArea" type="number" min="0" step=".1"></label></template>
  <span>{{ label('Контуры отверстий импортируются отдельными границами.','Hole contours are imported as separate boundaries.') }}</span>
  <button :disabled="!file" @click="trace">{{ label('Создать контуры','Create contours') }}</button>
 </fieldset><button v-if="busy" @click="cancel">{{ label('Отмена','Cancel') }}</button><p v-if="error" role="alert">{{ error }}</p>
</template>
<style scoped>.trace-controls{display:flex;flex-wrap:wrap;gap:6px;align-items:center;border:1px solid var(--border)}.trace-controls input[type=number]{width:65px}.trace-controls input[type=file]{max-width:200px}.trace-controls span{font-size:11px}p{color:var(--danger)}</style>
