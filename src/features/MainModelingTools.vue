<script setup lang="ts">
import {resolveSketchSupport} from '../services/mainSketchAssociation'
import {storageKeys,storageGet,storageSet} from '../services/safeStorage'
import {parseDirectDocument} from '../services/directModeling'
import {computed,ref,watch,onUnmounted} from 'vue'
import {sceneSnapPoints} from '../services/mainSnapping'
import {patchMainSource} from '../services/mainSourceEditing'
import {computeMainSolid,computeCadOperation,cancelMainSolid} from '../services/mainSolidWorker'
import CadDimensionsOverlay from './CadDimensionsOverlay.vue'
import type {CadOptions} from '../services/cadWorkbench'
import CadWorkbenchPanel from './CadWorkbenchPanel.vue'
import MainModelingOverlay from './MainModelingOverlay.vue'
import MainSketchTools from './MainSketchTools.vue'
import {facePlane} from '../services/directSolidTools'
import {booleanPolygonMeshes} from '../services/geometry/polygon'
import {xyPlane,type SketchPlane} from '../services/directSketchGeometry'
import type {DirectBody,DirectDocument} from '../services/directModeling'
import type {Ray3} from '../services/math3d'
import type {MeshData} from '../core/mesh'
import type {PickHit} from '../services/rendererContracts'
import {previewMeshes,primitiveSource,sceneFace,sceneBody,type MainOperation,type MainParameters} from '../services/mainModeling'
const props=defineProps<{meshes:MeshData[];selected:number|null;hit:PickHit|null;source:string;ready:boolean;locale:string;canUndo:boolean;canRedo:boolean;selectedIndices:number[];project:(p:readonly number[])=>[number,number]|null;ray:(x:number,y:number)=>Ray3|null;cameraRevision:number}>()
const emit=defineEmits<{append:[source:string];apply:[source:string];solid:[document:DirectDocument];preview:[meshes:MeshData[]|null];undo:[];redo:[];selectMany:[indices:number[]]}>()
const ru=computed(()=>props.locale==='ru'),l=(a:string,b:string)=>ru.value?a:b
const sketchAnchor=ref<{entity:string;face:number}|null>(null)
const snapPoints=computed(()=>sceneSnapPoints(props.meshes))
const sketchOpen=ref(false),sketchPlane=ref<SketchPlane>(xyPlane())
const working=ref(false)
let solidGeneration=0
onUnmounted(cancelMainSolid)
const dimensionOpen=ref(false)
const workbenchOpen=ref(false),workbenchAction=ref<import('../services/cadWorkbench').CadAction>('union')
const boxSelect=ref(false)
const size=ref(20),op=ref<MainOperation|null>(null),error=ref(''),previewing=ref(false)
const p=ref<MainParameters>({amount:2,x:0,y:0,z:0,axis:'z',edge:0,shape:'rectangle',width:10,height:10,cut:false})
const kinds=[['box','Куб','Box'],['cylinder','Цилиндр','Cylinder'],['cone','Конус','Cone'],['sphere','Сфера','Sphere']]
const ops:[MainOperation,string,string][]=[['push','Push/Pull','Push/Pull'],['fillet','Скругление','Fillet'],['chamfer','Фаска','Chamfer'],['shell','Оболочка','Shell'],['split','Разрез','Split'],['profile','Эскиз на грани','Face sketch'],['move','Двигать','Move'],['rotate','Вращать','Rotate'],['scale','Масштаб','Scale'],['duplicate','Копия','Duplicate'],['delete','Удалить','Delete']]
const topology=computed(()=>{try{return props.selected===null?null:sceneFace(props.meshes[props.selected],props.hit)}catch{return null}})
function cancel(){solidGeneration++;cancelMainSolid();working.value=false;emit('preview',null);previewing.value=false;op.value=null;error.value=''}
watch(()=>[props.source,props.selected,props.hit],()=>cancel())
watch(p,()=>{solidGeneration++;cancelMainSolid();working.value=false;if(previewing.value){emit('preview',null);previewing.value=false}},{deep:true,flush:'sync'})
async function run(fn:()=>void|Promise<void>){error.value='';try{await fn()}catch(e){if(e instanceof Error&&e.name==='AbortError')return;error.value=e instanceof Error?e.message:String(e)}}
function choose(kind:MainOperation){
 cancel();workbenchOpen.value=false;if(kind==='profile'){const t=topology.value;if(!t||t.face<0){error.value=l('Выберите грань в сцене','Pick a face in the scene');return}sketchPlane.value=facePlane(t.body,t.topology.faces[t.face]);sketchAnchor.value=props.selected!==null&&props.hit?.faceId!==null&&props.hit?.faceId!==undefined&&props.meshes[props.selected].entityId?{entity:props.meshes[props.selected].entityId!,face:props.hit.faceId}:null;sketchOpen.value=true;return}sketchOpen.value=false;op.value=kind;p.value.amount=kind==='scale'?1:kind==='rotate'?0:2;p.value.x=p.value.y=p.value.z=0
 const t=topology.value,hit=props.hit
 if(kind==='shell')p.value.openings=t&&t.face>=0?[t.face]:[]
 if(kind==='split'&&t){const axis=['x','y','z'].indexOf(p.value.axis),values=t.body.mesh.positions.filter((_,i)=>i%3===axis);p.value.normal=undefined;p.value.amount=(Math.min(...values)+Math.max(...values))/2}
 p.value.edge=0;p.value.edges=undefined;p.value.endRadius=undefined
 if(t&&hit){
  const points=t.body.mesh.positions
  let best=Infinity
  t.topology.edges.forEach((e,i)=>{
   const a=points.slice(e.a*3,e.a*3+3),b=points.slice(e.b*3,e.b*3+3),v=b.map((x,k)=>x-a[k]),w=hit.point.map((x,k)=>x-a[k])
   const along=Math.max(0,Math.min(1,w.reduce((s,x,k)=>s+x*v[k],0)/v.reduce((s,x)=>s+x*x,0)))
   const distance=Math.hypot(...w.map((x,k)=>x-along*v[k]))
   if(distance<best){best=distance;p.value.edge=i}
  })
 }
}
function edgeLabel(i:number){const t=topology.value,e=t?.topology.edges[i];if(!t||!e)return String(i+1);const point=(n:number)=>t.body.mesh.positions.slice(n*3,n*3+3).map(x=>Number(x.toFixed(1))).join(', ');return `${i+1}: (${point(e.a)}) → (${point(e.b)})`}

async function result(){
 if(!props.ready||props.selected===null||!op.value)throw Error('Build the current source and select a body.')
 const parameters={...p.value,selection:props.selectedIndices.length?props.selectedIndices:undefined}
 {
  const generation=++solidGeneration
  working.value=true
  try{return await computeMainSolid(props.meshes,props.selected,props.hit,op.value,JSON.parse(JSON.stringify(parameters)))}finally{if(generation===solidGeneration)working.value=false}
 }
}
function preview(){run(async()=>{const d=await result();emit('preview',previewMeshes(d));previewing.value=true})}
function apply(){run(async()=>{const source=patchMainSource(props.source,props.meshes,await result());emit('preview',null);emit('apply',source);op.value=null;previewing.value=false})}
function continueInSolid(){run(async()=>{const document=await result();emit('preview',null);emit('solid',document);op.value=null;previewing.value=false})}

watch(()=>props.meshes,()=>{
 for(const key of storageKeys('scad-main-sketches-v1:')){try{const anchor=JSON.parse(key.slice('scad-main-sketches-v1:'.length));if(!anchor.entity||!Number.isInteger(anchor.face))continue;const plane=resolveSketchSupport(props.meshes,anchor),d=parseDirectDocument(storageGet(key)??'');d.sketches=d.sketches.map(s=>({...s,plane}));storageSet(key,JSON.stringify(d))}catch{/* Missing supports remain stored for explicit reattachment. */}}
 const anchor=sketchAnchor.value;if(!anchor)return;try{sketchPlane.value=resolveSketchSupport(props.meshes,anchor)}catch(e){error.value=String(e)}
},{immediate:true})
function resizeDimensions(dimensions:number[]){run(async()=>{if(!props.ready)throw Error('Build the current source.');const before={version:1 as const,sketches:[],bodies:props.meshes.map(sceneBody)},options:CadOptions={action:'resize',ids:props.selectedIndices.map(String),sketches:[],axis:[0,0,1],origin:[0,0,0],width:dimensions[0],height:dimensions[1],depth:dimensions[2],amount:0,count:0,pitch:1,secondary:0,mode:'',pathId:'',profileIds:[]};emit('apply',patchMainSource(props.source,props.meshes,await computeCadOperation(before,options)))})}
function applySketch(body:DirectBody,cut:boolean){run(()=>{if(!props.ready)throw Error('Build the current source before applying the sketch.');if(sketchAnchor.value)resolveSketchSupport(props.meshes,sketchAnchor.value);const bodies=props.meshes.map(sceneBody);if(cut){if(props.selected===null)throw Error('Select a body to cut.');bodies[props.selected]={...bodies[props.selected],mesh:booleanPolygonMeshes(bodies[props.selected].mesh,body.mesh,'difference')}}else bodies.push(body);emit('apply',patchMainSource(props.source,props.meshes,{version:1,sketches:[],bodies}))})}
</script>
<template>
 <CadDimensionsOverlay v-if="dimensionOpen" :meshes="meshes" :selection="selectedIndices" :project="project" :revision="cameraRevision" @resize="resizeDimensions" />
 <CadWorkbenchPanel v-if="workbenchOpen" :key="workbenchAction" :initial-action="workbenchAction" :meshes="meshes" :selection="selectedIndices" :hit="hit" :source="source" :ready="ready" :locale="locale" @apply="emit('apply',$event)" @preview="emit('preview',$event)" @close="workbenchOpen=false" />
 <MainSketchTools v-if="sketchOpen" :key="sketchAnchor?JSON.stringify(sketchAnchor):JSON.stringify(sketchPlane)" :session-key="sketchAnchor?JSON.stringify(sketchAnchor):undefined" :snap-points="snapPoints" :plane="sketchPlane" :project="project" :ray="ray" :revision="cameraRevision" :locale="locale" @body="applySketch" @close="sketchOpen=false" />
 <MainModelingOverlay v-if="!sketchOpen" :meshes="meshes" :selected="selected" :selection="selectedIndices" :hit="hit" :operation="op" :parameters="p" :project="project" :revision="cameraRevision" :box="boxSelect" @parameters="p=$event" @preview="preview" @apply="apply" @cancel="cancel" @select="emit('selectMany',$event)" @box-done="boxSelect=false" />
 <div class="main-model-tools" @keydown.stop>
  <div class="primitives"><button :aria-pressed="boxSelect" @click="cancel();sketchOpen=false;boxSelect=!boxSelect">{{ l('Рамка','Box select') }}</button><button @click="cancel();workbenchOpen=false;sketchAnchor=null;sketchPlane=xyPlane();sketchOpen=true">{{ l('Эскиз XY','XY sketch') }}</button><button @click="cancel();sketchOpen=false;workbenchAction='union';workbenchOpen=!workbenchOpen">{{ l('Операции CAD','CAD operations') }}</button><button @click="cancel();sketchOpen=false;workbenchAction='texture';workbenchOpen=true">{{ l('Текстура','Texture') }}</button><button @click="cancel();sketchOpen=false;workbenchAction='lighten';workbenchOpen=true">{{ l('Облегчение','Lighten') }}</button><button :aria-pressed="dimensionOpen" @click="dimensionOpen=!dimensionOpen">{{ l('Размеры','Dimensions') }}</button><strong>{{ l('Примитивы','Primitives') }}</strong><button v-for="k in kinds" :key="k[0]" @click="run(()=>emit('append',primitiveSource(k[0],size)))">{{ l(k[1],k[2]) }}</button><label>{{ l('мм','mm') }} <input v-model.number="size" aria-label="Primitive size" type="number" min="0.1" max="10000"></label><button :title="l('Отменить изменение','Undo edit')" :disabled="!canUndo" @click="cancel();emit('undo')">↶</button><button :title="l('Повторить изменение','Redo edit')" :disabled="!canRedo" @click="cancel();emit('redo')">↷</button></div>
  <div v-if="selected!==null" class="operations"><span>{{ l('Тело','Body') }} {{ selected+1 }} · {{ selectedIndices.length }} {{ l('выбрано','selected') }}</span><button v-for="o in ops" :key="o[0]" :disabled="!ready" :aria-pressed="op===o[0]" @click="choose(o[0])">{{ l(o[1],o[2]) }}</button></div>
  <div v-if="op" class="parameters">
   <label v-if="!['move','duplicate','delete'].includes(op)">{{ l('Размер / угол','Size / angle') }} <input v-model.number="p.amount" type="number" step="0.5"></label>
   <template v-if="['move','duplicate','profile'].includes(op)"><label v-for="axis in (op==='profile'?['x','y']:['x','y','z']) as ('x'|'y'|'z')[]" :key="axis">{{ axis.toUpperCase() }} <input v-model.number="p[axis]" type="number"></label></template>
   <label v-if="['rotate','split'].includes(op)">{{ l('Ось','Axis') }} <select v-model="p.axis" @change="p.normal=undefined"><option>x</option><option>y</option><option>z</option></select></label>
   <template v-if="op==='split'"><label v-for="(axis,i) in ['Nx','Ny','Nz']" :key="axis">{{ axis }} <input type="number" step="0.1" :value="(p.normal??[0,0,1])[i]" @input="p.normal=[...(p.normal??[0,0,1])] as [number,number,number];p.normal[i]=Number(($event.target as HTMLInputElement).value)"></label></template>
   <label v-if="['fillet','chamfer'].includes(op)">{{ l('Ребро','Edge') }} <select v-model.number="p.edge"><option v-for="(_e,i) in topology?.topology.edges" :key="i" :value="i">{{ edgeLabel(i) }}</option></select></label>
   <template v-if="op==='profile'"><select v-model="p.shape"><option value="rectangle">{{ l('Прямоугольник','Rectangle') }}</option><option value="circle">{{ l('Круг','Circle') }}</option></select><label>{{ l('Ширина / диаметр','Width / diameter') }} <input v-model.number="p.width" type="number" min="0.1"></label><label v-if="p.shape==='rectangle'">{{ l('Высота','Height') }} <input v-model.number="p.height" type="number" min="0.1"></label><label><input v-model="p.cut" type="checkbox">{{ l('Вырезать','Cut') }}</label></template>
   <button @click="preview">{{ l('Предпросмотр','Preview') }}</button><button class="primary" :disabled="working" @click="continueInSolid">{{ l('Продолжить в Solid','Continue in Solid') }}</button><button :disabled="working" @click="apply">{{ l('Bake mesh в Code','Bake mesh to Code') }}</button><button @click="cancel">{{ l('Отмена','Cancel') }}</button>
   <label v-if="op==='fillet'">{{ l('Конечный радиус','End radius') }} <input v-model.number="p.endRadius" type="number" min="0.01" :placeholder="String(p.amount)"></label><small v-if="op==='fillet'||op==='chamfer'">Shift + {{ l('клик — цепочка рёбер','click — edge chain') }}</small>
   <label v-if="op==='move'"><input type="checkbox" :checked="p.snap!==false" @change="p.snap=($event.target as HTMLInputElement).checked">Snap 3D · Alt {{ l('временно отключает','temporarily disables') }}</label><label v-if="op==='shell'">{{ l('Шаг сетки, мм (0 — авто)','Grid step, mm (0 — auto)') }} <input v-model.number="p.step" type="number" min="0" step="0.1"></label>
   <label v-if="op==='shell'"><input v-model="p.adaptive" type="checkbox">{{ l('Адаптивные блоки (до 256 ячеек/ось)','Adaptive tiles (up to 256 cells/axis)') }}</label><small v-if="op==='shell'||op==='fillet'||op==='chamfer'">{{ l('Сеточная аппроксимация. Для общего Shell шаг ≤ толщины/3; 64 ячейки/ось, адаптивный режим — 256.','Mesh approximation. General Shell: step ≤ thickness/3; 64 cells/axis, adaptive mode — 256.') }}</small>
   <span v-if="working" role="status">{{ l('Расчёт геометрии… Можно отменить.','Computing geometry… Cancel is available.') }}</span>
   <small>{{ l('Solid сохраняет документ редактируемым. Bake mesh в Code явно заменяет связанные выражения полигональной геометрией; остальной код сохраняется.','Solid keeps the document editable. Bake mesh to Code explicitly replaces owning expressions with polygon geometry; other source is preserved.') }}</small>
  </div>
  <p v-if="error" role="alert">{{ error }}</p>
 </div>
</template>
<style scoped>
.main-model-tools{position:absolute;left:8px;right:8px;bottom:35px;z-index:5;background:var(--surface);color:var(--text);border:1px solid var(--border);border-radius:7px;padding:7px;max-height:40%;overflow:auto;font-size:12px}.primitives,.operations,.parameters{display:flex;align-items:center;flex-wrap:wrap;gap:5px}.operations,.parameters{margin-top:6px}button,input,select{font:inherit;color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:4px;padding:5px}button{cursor:pointer}button:disabled{opacity:.4}button[aria-pressed=true]{border-color:var(--accent)}button.primary{background:var(--accent);color:var(--bg);font-weight:600}input[type=number]{width:64px}label{display:flex;align-items:center;gap:3px}small{width:100%;color:var(--text-dim)}p{color:var(--danger);margin:6px 0}
</style>
