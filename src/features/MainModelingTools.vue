<script setup lang="ts">
import {computed,ref,watch} from 'vue'
import type {MeshData} from '../core/mesh'
import type {PickHit} from '../services/rendererContracts'
import {mainOperation,mainSource,previewMeshes,primitiveSource,sceneFace,type MainOperation,type MainParameters} from '../services/mainModeling'
const props=defineProps<{meshes:MeshData[];selected:number|null;hit:PickHit|null;source:string;ready:boolean;locale:string;canUndo:boolean;canRedo:boolean}>()
const emit=defineEmits<{append:[source:string];apply:[source:string];preview:[meshes:MeshData[]|null];undo:[];redo:[]}>()
const ru=computed(()=>props.locale==='ru'),l=(a:string,b:string)=>ru.value?a:b
const size=ref(20),op=ref<MainOperation|null>(null),error=ref(''),previewing=ref(false)
const p=ref<MainParameters>({amount:2,x:0,y:0,z:0,axis:'z',edge:0,shape:'rectangle',width:10,height:10,cut:false})
const kinds=[['box','Куб','Box'],['cylinder','Цилиндр','Cylinder'],['cone','Конус','Cone'],['sphere','Сфера','Sphere']]
const ops:[MainOperation,string,string][]=[['push','Push/Pull','Push/Pull'],['fillet','Скругление','Fillet'],['chamfer','Фаска','Chamfer'],['shell','Оболочка','Shell'],['split','Разрез','Split'],['profile','Профиль на грани','Face profile'],['move','Двигать','Move'],['rotate','Вращать','Rotate'],['scale','Масштаб','Scale'],['duplicate','Копия','Duplicate'],['delete','Удалить','Delete']]
const topology=computed(()=>{try{return props.selected===null?null:sceneFace(props.meshes[props.selected],props.hit)}catch{return null}})
function cancel(){emit('preview',null);previewing.value=false;op.value=null;error.value=''}
watch(()=>[props.source,props.selected,props.hit],()=>cancel())
watch(p,()=>{if(previewing.value){emit('preview',null);previewing.value=false}},{deep:true})
function run(fn:()=>void){error.value='';try{fn()}catch(e){error.value=e instanceof Error?e.message:String(e)}}
function choose(kind:MainOperation){
 cancel();op.value=kind;p.value.amount=kind==='scale'?1:2
 const t=topology.value,hit=props.hit
 p.value.edge=0
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

function result(){if(!props.ready||props.selected===null||!op.value)throw Error('Build the current source and select a body.');return mainOperation(props.meshes,props.selected,props.hit,op.value,p.value)}
function apply(){run(()=>{const source=mainSource(result());emit('preview',null);emit('apply',source);op.value=null;previewing.value=false})}
</script>
<template>
 <div class="main-model-tools" @keydown.stop>
  <div class="primitives"><strong>{{ l('Примитивы','Primitives') }}</strong><button v-for="k in kinds" :key="k[0]" @click="run(()=>emit('append',primitiveSource(k[0],size)))">{{ l(k[1],k[2]) }}</button><label>{{ l('мм','mm') }} <input v-model.number="size" aria-label="Primitive size" type="number" min="0.1" max="10000"></label><button :title="l('Отменить изменение','Undo edit')" :disabled="!canUndo" @click="cancel();emit('undo')">↶</button><button :title="l('Повторить изменение','Redo edit')" :disabled="!canRedo" @click="cancel();emit('redo')">↷</button></div>
  <div v-if="selected!==null" class="operations"><span>{{ l('Тело','Body') }} {{ selected+1 }}</span><button v-for="o in ops" :key="o[0]" :disabled="!ready" :aria-pressed="op===o[0]" @click="choose(o[0])">{{ l(o[1],o[2]) }}</button></div>
  <div v-if="op" class="parameters">
   <label v-if="!['move','duplicate','delete'].includes(op)">{{ l('Размер / угол','Size / angle') }} <input v-model.number="p.amount" type="number" step="0.5"></label>
   <template v-if="['move','duplicate','profile'].includes(op)"><label v-for="axis in (op==='profile'?['x','y']:['x','y','z']) as ('x'|'y'|'z')[]" :key="axis">{{ axis.toUpperCase() }} <input v-model.number="p[axis]" type="number"></label></template>
   <label v-if="['rotate','split'].includes(op)">{{ l('Ось','Axis') }} <select v-model="p.axis"><option>x</option><option>y</option><option>z</option></select></label>
   <label v-if="['fillet','chamfer'].includes(op)">{{ l('Ребро','Edge') }} <select v-model.number="p.edge"><option v-for="(_e,i) in topology?.topology.edges" :key="i" :value="i">{{ edgeLabel(i) }}</option></select></label>
   <template v-if="op==='profile'"><select v-model="p.shape"><option value="rectangle">{{ l('Прямоугольник','Rectangle') }}</option><option value="circle">{{ l('Круг','Circle') }}</option></select><label>{{ l('Ширина / диаметр','Width / diameter') }} <input v-model.number="p.width" type="number" min="0.1"></label><label v-if="p.shape==='rectangle'">{{ l('Высота','Height') }} <input v-model.number="p.height" type="number" min="0.1"></label><label><input v-model="p.cut" type="checkbox">{{ l('Вырезать','Cut') }}</label></template>
   <button @click="run(()=>{emit('preview',previewMeshes(result()));previewing=true})">{{ l('Предпросмотр','Preview') }}</button><button @click="apply">{{ l('Применить','Apply') }}</button><button @click="cancel">{{ l('Отмена','Cancel') }}</button>
   <small>{{ l('Изменённая сцена записывается в код как polyhedron; ↶ возвращает исходный код.','Edited scene is written as polyhedron; ↶ restores the original source.') }}</small>
  </div>
  <p v-if="error" role="alert">{{ error }}</p>
 </div>
</template>
<style scoped>
.main-model-tools{position:absolute;left:8px;right:8px;bottom:35px;z-index:5;background:var(--surface);color:var(--text);border:1px solid var(--border);border-radius:7px;padding:7px;max-height:40%;overflow:auto;font-size:12px}.primitives,.operations,.parameters{display:flex;align-items:center;flex-wrap:wrap;gap:5px}.operations,.parameters{margin-top:6px}button,input,select{font:inherit;color:var(--text);background:var(--surface-raised);border:1px solid var(--border);border-radius:4px;padding:5px}button{cursor:pointer}button:disabled{opacity:.4}button[aria-pressed=true]{border-color:var(--accent)}input[type=number]{width:64px}label{display:flex;align-items:center;gap:3px}small{width:100%;color:var(--text-dim)}p{color:var(--danger);margin:6px 0}
</style>
