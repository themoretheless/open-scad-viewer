<script setup lang="ts">
import {frameArchive} from '../services/assemblyFrameExport'
import {ref,onUnmounted,watch} from 'vue'
import {serializeDirectDocument,type DirectDocument} from '../services/directModeling'
import {type AssemblyAnimation,validateAssemblyAnimation} from '../services/assemblyAnimation'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
const props=defineProps<{document:DirectDocument;camera:{yaw:number;pitch:number};sourceId?:string;canvas?:HTMLCanvasElement|null;locale?:string}>()
const emit=defineEmits<{preview:[value:{document:DirectDocument|null;camera:{yaw:number;pitch:number}}]}>()
const open=ref(false),time=ref(0),x=ref(0),y=ref(0),z=ref(0),angle=ref(0),rotationX=ref(0),rotationY=ref(0),scale=ref(1),frames=ref<AssemblyAnimation>({version:1,frames:[]}),playing=ref(false),error=ref(''),position=ref(0),videoUrl=ref(''),framesUrl=ref(''),framePreview=ref(''),exporting=ref(false),frameProgress=ref(0),exportFps=ref(12)
const worker=createSolidPreviewWorker(),label=(ru:string,en:string)=>props.locale==='en'?en:ru
let recorder:MediaRecorder|undefined,stream:MediaStream|undefined,destroyed=false,captureTimer:ReturnType<typeof setInterval>|undefined
let generation=0,timer:ReturnType<typeof setTimeout>|undefined,originalCamera:{yaw:number;pitch:number}|undefined
function add(){try{
 const ids=frames.value.frames[0]?.poses.map(p=>p.id)??(props.sourceId?props.document.bodies.filter(b=>!b.instance).map(b=>b.id):[])
 const previous=frames.value.frames.filter(f=>f.time<=time.value).at(-1)
 const poses=ids.map(id=>id===props.sourceId?{id,translation:[x.value,y.value,z.value] as [number,number,number],angle:angle.value,rotation:[rotationX.value,rotationY.value,angle.value] as [number,number,number],scale:scale.value}:(previous?.poses.find(p=>p.id===id)??{id,translation:[0,0,0] as [number,number,number],angle:0,scale:1}))
 const next:AssemblyAnimation={version:1,frames:[...frames.value.frames.filter(f=>f.time!==time.value),{time:time.value,poses,camera:{...props.camera}}].sort((a,b)=>a.time-b.time)}
 validateAssemblyAnimation(props.document,next);frames.value=next;time.value=Math.min(120,time.value+1);error.value=''
}catch(e){error.value=String(e)}}
function stop(){generation++;clearTimeout(timer);worker.cancel();playing.value=false;if(originalCamera){emit('preview',{document:null,camera:originalCamera});originalCamera=undefined}}
async function show(t:number,token:number){const result=await worker.run({kind:'animationSample',document:JSON.parse(serializeDirectDocument(props.document)),animation:JSON.parse(JSON.stringify(frames.value)),time:t});if(token!==generation)return;emit('preview',result);position.value=t}
async function play(skipStop=false){if(!skipStop)stop();error.value='';if(frames.value.frames.length<2){error.value=label('Добавьте минимум два кадра.','Add at least two frames.');return}originalCamera??={...props.camera};playing.value=true;const token=++generation,start=performance.now(),first=frames.value.frames[0].time,end=frames.value.frames.at(-1)!.time
 const tick=async()=>{if(token!==generation)return;const t=Math.min(end,first+(performance.now()-start)/1000);try{await show(t,token);if(token!==generation)return;if(t===end){playing.value=false;return}timer=setTimeout(tick,125)}catch(e){if(token!==generation)return;stop();error.value=String(e)}};await tick()
}
async function load(event:Event){const file=(event.target as HTMLInputElement).files?.[0];if(!file)return;stop();try{if(file.size>64*1024*1024)throw Error('Animation file exceeds 64 MiB.');const value=JSON.parse(await file.text());const animation=value.animation??value;validateAssemblyAnimation(props.document,animation);frames.value=animation;error.value=''}catch(e){error.value=String(e)}}
async function exportFrames(){
 stop();const token=generation;exporting.value=true;error.value='';frameProgress.value=0
 try{
  const prepared=await worker.run({kind:'animationFrames',document:JSON.parse(serializeDirectDocument(props.document)),animation:JSON.parse(JSON.stringify(frames.value)),fps:exportFps.value})
  const canvas=document.createElement('canvas');canvas.width=prepared.width;canvas.height=prepared.height
  const context=canvas.getContext('2d');if(!context)throw Error('Frame canvas unavailable.')
  const files:{name:string;bytes:Uint8Array}[]=[],manifest:{file:string;time:number}[]=[];let total=0
  for(const [i,frame] of prepared.frames.entries()){
   if(token!==generation||destroyed)return
   const image=new Image();await new Promise<void>((resolve,reject)=>{image.onload=()=>resolve();image.onerror=()=>reject(Error('Frame rendering failed.'));image.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(frame.svg)})
   context.clearRect(0,0,canvas.width,canvas.height);context.drawImage(image,0,0)
   const blob=await new Promise<Blob>((resolve,reject)=>canvas.toBlob(v=>v?resolve(v):reject(Error('PNG encoding failed.')),'image/png'))
   if(token!==generation||destroyed)return
   total+=blob.size;if(total>64*1024*1024)throw Error('Frame archive exceeds 64 MiB.')
   const name=`frame-${String(i+1).padStart(5,'0')}.png`;files.push({name,bytes:new Uint8Array(await blob.arrayBuffer())});manifest.push({file:name,time:frame.time});frameProgress.value=i+1
   if(i===0){if(framePreview.value)URL.revokeObjectURL(framePreview.value);framePreview.value=URL.createObjectURL(blob)}
  }
  files.push({name:'manifest.json',bytes:new TextEncoder().encode(JSON.stringify({version:1,fps:prepared.fps,requestedFps:exportFps.value,width:prepared.width,height:prepared.height,renderer:'native opaque mesh projection',frames:manifest},null,2))})
  if(framesUrl.value)URL.revokeObjectURL(framesUrl.value);framesUrl.value=URL.createObjectURL(new Blob([frameArchive(files) as BlobPart],{type:'application/zip'}))
 }catch(e){if(token===generation)error.value=String(e)}finally{exporting.value=false}
}
async function exportVideo(){
 stop();error.value=''
 try{
  if(!props.canvas||typeof MediaRecorder==='undefined')throw Error(label('Экспорт WebM требует активного WebGPU и поддержки MediaRecorder.','WebM export requires active WebGPU and MediaRecorder support.'))
  if(frames.value.frames.length<2)throw Error(label('Добавьте минимум два кадра.','Add at least two frames.'))
  validateAssemblyAnimation(props.document,frames.value)
  originalCamera={...props.camera}
  const warmToken=++generation;playing.value=true
  await show(frames.value.frames[0].time,warmToken)
  if(warmToken!==generation)return
  const source=props.canvas,output=document.createElement('canvas'),ratio=Math.min(1,1280/Math.max(source.width,source.height))
  output.width=Math.max(1,Math.round(source.width*ratio));output.height=Math.max(1,Math.round(source.height*ratio))
  const context=output.getContext('2d');if(!context)throw Error('Recording canvas unavailable.')
  const copy=()=>context.drawImage(source,0,0,output.width,output.height);copy()
  const copyTimer=captureTimer=setInterval(copy,60)
  const capture=stream=output.captureStream(15)
  const mimeType=['video/webm;codecs=vp9','video/webm;codecs=vp8','video/webm'].find(type=>MediaRecorder.isTypeSupported(type))
  if(!mimeType)throw Error('WebM recording is unavailable.')
  const chunks:Blob[]=[];const recording=recorder=new MediaRecorder(capture,{mimeType,videoBitsPerSecond:2000000})
  recorder.ondataavailable=e=>{if(e.data.size)chunks.push(e.data)}
  recorder.onerror=()=>{error.value=label('Ошибка записи WebM.','WebM recording failed.');stop()}
  recorder.onstop=()=>{clearInterval(copyTimer);if(captureTimer===copyTimer)captureTimer=undefined;capture.getTracks().forEach(t=>t.stop());if(stream===capture)stream=undefined;if(destroyed||!chunks.length)return;if(videoUrl.value)URL.revokeObjectURL(videoUrl.value);videoUrl.value=URL.createObjectURL(new Blob(chunks,{type:mimeType}));if(recorder===recording)recorder=undefined}
  recorder.start(1000);await play(true)
 }catch(e){clearInterval(captureTimer);captureTimer=undefined;if(recorder?.state==='recording')recorder.stop();stream?.getTracks().forEach(t=>t.stop());stream=undefined;stop();error.value=String(e)}
}
watch(playing,value=>{if(!value&&recorder?.state==='recording'){const current=recorder;setTimeout(()=>{if(current.state==='recording')current.stop()},120)}})
function download(){try{validateAssemblyAnimation(props.document,frames.value);const url=URL.createObjectURL(new Blob([JSON.stringify({document:JSON.parse(serializeDirectDocument(props.document)),animation:frames.value},null,2)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download='assembly-animation.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)}catch(e){error.value=String(e)}}
watch(()=>serializeDirectDocument(props.document),()=>stop())
onUnmounted(()=>{destroyed=true;stop();clearInterval(captureTimer);if(recorder?.state==='recording')recorder.stop();stream?.getTracks().forEach(t=>t.stop());if(videoUrl.value)URL.revokeObjectURL(videoUrl.value);if(framesUrl.value)URL.revokeObjectURL(framesUrl.value);if(framePreview.value)URL.revokeObjectURL(framePreview.value);worker.dispose()})
</script>
<template>
 <div class="modeling-feature"><button type="button" :aria-expanded="open" @click="open=!open">{{label('Анимация сборки','Assembly animation')}}</button>
  <fieldset v-if="open"><legend>{{label('Ключевые кадры тела и камеры','Body and camera keyframes')}}</legend>
   <small>{{label('Смещения относительно исходной модели. Повороты тела вокруг X, Y и Z. Кадр сохраняет текущий ракурс камеры.','Offsets from the source model. Body rotations around X, Y and Z. Each frame stores the current camera view.')}}</small>
   <label>{{label('Время кадра, с','Frame time, s')}}<input v-model.number="time" type="number" min="0" max="120" step=".1"></label>
   <label v-for="axis in ['x','y','z']" :key="axis">{{axis.toUpperCase()}}<input v-if="axis==='x'" v-model.number="x" type="number" aria-label="Animation X"><input v-else-if="axis==='y'" v-model.number="y" type="number" aria-label="Animation Y"><input v-else v-model.number="z" type="number" aria-label="Animation Z"></label>
   <label>{{label('Поворот X, °','Rotation X, °')}}<input v-model.number="rotationX" type="number" aria-label="Animation rotation X"></label>
   <label>{{label('Поворот Y, °','Rotation Y, °')}}<input v-model.number="rotationY" type="number" aria-label="Animation rotation Y"></label>
   <label>{{label('Поворот Z, °','Rotation Z, °')}}<input v-model.number="angle" type="number"></label>
   <label>{{label('Масштаб тела','Body scale')}}<input v-model.number="scale" type="number" min=".001" max="100" step=".1"></label>
   <button type="button" :disabled="playing" @click="add">{{label('Добавить кадр','Add keyframe')}}</button>
   <label>{{label('Файл кадров','Keyframe file')}}<input type="file" accept=".json,application/json" :disabled="playing" @change="load"></label>
   <span>{{frames.frames.length}} {{label('кадров','frames')}} · {{position.toFixed(2)}} s</span>
   <button type="button" :disabled="playing" @click="play()">{{label('Воспроизвести анимацию','Play animation')}}</button><button type="button" @click="stop">{{label('Вернуть исходный вид','Restore original view')}}</button>
   <button type="button" @click="download">{{label('Экспорт кадров JSON','Export keyframes JSON')}}</button><button type="button" :disabled="playing" @click="stop();frames={version:1,frames:[]}">{{label('Очистить кадры','Clear keyframes')}}</button>
   <label>FPS<input v-model.number="exportFps" type="number" min="1" max="30" :disabled="exporting"></label>
   <button type="button" :disabled="playing||exporting" @click="exportFrames">{{label('Экспорт PNG-кадров','Export PNG frames')}}</button>
   <template v-if="exporting"><span>{{frameProgress}} {{label('кадров подготовлено','frames prepared')}}</span><button type="button" @click="stop">{{label('Отменить экспорт','Cancel export')}}</button></template>
   <small>{{label('960 × 640, до 300 кадров. Непрозрачная сетка; шаг времени фиксирован, общий ракурс листа.','960 × 640, up to 300 frames. Opaque mesh; fixed time step and shared frame bounds.')}}</small>
   <img v-if="framePreview" :src="framePreview" :alt="label('Первый кадр экспорта','First exported frame')"><a v-if="framesUrl" :href="framesUrl" download="assembly-frames.zip">{{label('Скачать PNG-кадры ZIP','Download PNG frames ZIP')}}</a>
   <button type="button" :disabled="playing||exporting" @click="exportVideo">{{label('Экспорт WebM','Export WebM')}}</button><template v-if="videoUrl"><video :src="videoUrl" controls aria-label="Animation video preview"/><a :href="videoUrl" download="assembly-animation.webm">{{label('Скачать WebM','Download WebM')}}</a></template>
   <p v-if="error" role="alert">{{error}}</p>
  </fieldset>
 </div>
</template>
<style scoped>fieldset{border:0;display:grid;gap:6px;padding:6px 0;font-size:11px}label{display:flex;justify-content:space-between;gap:6px}input{min-width:0;max-width:100px}small{line-height:1.4}button{font-size:11px}p{color:#ef8c7e}video,img{width:100%}</style>
