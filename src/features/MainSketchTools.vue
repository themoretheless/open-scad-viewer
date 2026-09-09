<script setup lang="ts">
import {computed,ref,shallowRef,watch} from 'vue'
import {storageGet,storageSet} from '../services/safeStorage'
import type {Ray3} from '../services/math3d'
import {DirectHistory,emptyDirectDocument,parseDirectDocument,extrudeDirectSketch,type Point2,type DirectSketch} from '../services/directModeling'
import {sampleCurve,worldPoint,xyPlane,cross3,dot3,offsetSketch,trimSketch,extendSketch,type SketchPlane} from '../services/directSketchGeometry'
const props=defineProps<{plane:SketchPlane|null;project:(p:readonly number[])=>[number,number]|null;ray:(x:number,y:number)=>Ray3|null;revision:number;locale:string;sessionKey?:string;snapPoints?:number[][]}>()
const emit=defineEmits<{body:[body:ReturnType<typeof extrudeDirectSketch>,cut:boolean];close:[]}>()
const storageKey='scad-main-sketches-v1:'+(props.sessionKey??JSON.stringify(props.plane??xyPlane()))
function load(){try{return parseDirectDocument(storageGet(storageKey)??'')}catch{return emptyDirectDocument()}}
const history=new DirectHistory(load()),document=shallowRef(history.document),tool=ref('select'),selected=ref(''),draft=ref<Point2[]>([]),error=ref(''),depth=ref(10),offset=ref(1),cut=ref(false),svg=ref<SVGSVGElement>()
watch(document,d=>{if(!storageSet(storageKey,JSON.stringify(d)))error.value='Could not save sketches in the browser.'})
const snapping=ref(true),snapMarker=ref<Point2|null>(null)
const endpoint=ref<'start'|'end'>('end')
const plane=computed(()=>props.plane??xyPlane()),current=computed(()=>document.value.sketches.find(s=>s.id===selected.value))
const label=(a:string,b:string)=>props.locale==='ru'?a:b
const clone=<T,>(v:T):T=>JSON.parse(JSON.stringify(v))
watch(()=>props.plane,p=>{if(p)document.value={...document.value,sketches:document.value.sketches.map(s=>({...s,plane:clone(p)}))}},{deep:true})
function commit(){history.commit(document.value);document.value=history.document}
function run(fn:()=>void){error.value='';try{fn()}catch(e){error.value=e instanceof Error?e.message:String(e)}}
function at(e:PointerEvent):Point2|null{const ray=props.ray(e.clientX,e.clientY);if(!ray)return null;const n=cross3(plane.value.u,plane.value.v),den=dot3(n,ray.direction);if(Math.abs(den)<1e-7)return null;const t=dot3(n,plane.value.origin.map((v,i)=>v-ray.origin[i]))/den;if(t<0)return null;let q=ray.origin.map((v,i)=>v+t*ray.direction[i]-plane.value.origin[i]);snapMarker.value=null
 if(snapping.value&&svg.value){const rect=svg.value.getBoundingClientRect();let best=10;for(const point of props.snapPoints??[]){const rel=point.map((v,k)=>v-plane.value.origin[k]);if(Math.abs(dot3(rel,n))>1e-5)continue;const projected=props.project(point);if(!projected)continue;const distance=Math.hypot(projected[0]+rect.left-e.clientX,projected[1]+rect.top-e.clientY);if(distance<best){best=distance;q=rel;snapMarker.value=[dot3(q,plane.value.u),dot3(q,plane.value.v)]}}}
 return [dot3(q,plane.value.u),dot3(q,plane.value.v)]}
function projected(p:Point2){void props.revision;return props.project(worldPoint(p,plane.value))}
function path(points:Point2[]){return points.map(projected).filter(Boolean).map(p=>p!.join(',')).join(' ')}
let start:Point2|null=null,before:typeof document.value|null=null,handle:{kind:string;vertex:number}|null=null
function add(points:Point2[],closed:boolean,analytic?:DirectSketch['analytic']){const id=crypto.randomUUID();document.value={...document.value,sketches:[...document.value.sketches,{id,name:'Sketch',points,closed,plane:clone(plane.value),...(analytic?{analytic}: {})}]};selected.value=id;commit()}
function down(e:PointerEvent){if(e.button!==0)return;const p=at(e);if(!p)return
 if(tool.value==='polyline'){draft.value.push(p);return}
 if(['rectangle','circle','arc'].includes(tool.value)){start=p;draft.value=[p,p];svg.value?.setPointerCapture(e.pointerId)}
}
function pick(e:PointerEvent,s:DirectSketch){e.stopPropagation();selected.value=s.id
 if(tool.value==='trim'){run(()=>{const p=at(e);if(!p)return;let edge=0,best=Infinity,bestT=0.5;for(let i=0;i<s.points.length-(s.closed?0:1);i++){const a=s.points[i],b=s.points[(i+1)%s.points.length],v=[b[0]-a[0],b[1]-a[1]],t=Math.max(0,Math.min(1,((p[0]-a[0])*v[0]+(p[1]-a[1])*v[1])/(v[0]*v[0]+v[1]*v[1]))),dist=Math.hypot(p[0]-a[0]-v[0]*t,p[1]-a[1]-v[1]*t);if(dist<best){best=dist;edge=i;bestT=t}}
 const result=trimSketch(clone(s),edge,bestT,clone(document.value.sketches));document.value={...document.value,sketches:document.value.sketches.filter(o=>o.id!==s.id).concat(result.map((o,i)=>({...o,id:i?crypto.randomUUID():s.id})))};commit()})}
}
function beginHandle(e:PointerEvent,kind:string,vertex=0){e.stopPropagation();before=clone(document.value);handle={kind,vertex};svg.value?.setPointerCapture(e.pointerId)}
function move(e:PointerEvent){const p=at(e);if(!p)return
 if(handle&&current.value){const s=clone(current.value),a=s.analytic
 if(a){if(handle.kind==='center')a.center=p;else if(handle.kind==='radius')a.radius=Math.max(.01,Math.hypot(p[0]-a.center[0],p[1]-a.center[1]));else {const degrees=Math.atan2(p[1]-a.center[1],p[0]-a.center[0])*180/Math.PI;if(handle.kind==='start'){const end=a.start+a.sweep;a.start=degrees;a.sweep=((end-degrees+360)%360)||360}else a.sweep=((degrees-a.start+360)%360)||360} s.points=sampleCurve(a)}else s.points[handle.vertex]=p
 document.value={...document.value,sketches:document.value.sketches.map(o=>o.id===s.id?s:o)}
 }else if(start)draft.value=[start,p]
}
function up(){if(handle){handle=null;before=null;commit();return}if(!start)return
 const [a,b]=draft.value;start=null;draft.value=[];if(!a||!b)return
 run(()=>{if(tool.value==='rectangle'){if(Math.abs(a[0]-b[0])<.01||Math.abs(a[1]-b[1])<.01)return;add([a,[b[0],a[1]],b,[a[0],b[1]]],true)}else{const analytic={kind:tool.value as 'circle'|'arc',center:a,radius:Math.hypot(b[0]-a[0],b[1]-a[1]),start:0,sweep:tool.value==='circle'?360:180};add(sampleCurve(analytic),analytic.kind==='circle',analytic)}})
}
function cancel(){if(before)document.value=before;before=null;handle=null;start=null;draft.value=[]}
function modify(action:'offset'|'extend'){run(()=>{if(!current.value)return;const s=clone(current.value),next=action==='offset'?offsetSketch(s,offset.value):extendSketch(s,endpoint.value,clone(document.value.sketches));document.value={...document.value,sketches:document.value.sketches.map(o=>o.id===s.id?next:o)};commit()})}
function curveField(key:'radius'|'start'|'sweep',value:number){run(()=>{const s=clone(current.value!);s.analytic![key]=value;s.points=sampleCurve(s.analytic!);document.value={...document.value,sketches:document.value.sketches.map(o=>o.id===s.id?s:o)};commit()})}
function extrude(){run(()=>{if(!current.value)return;const s=clone(current.value);s.plane=clone(plane.value);if(cut.value){s.plane!.v=s.plane!.v.map(v=>-v) as [number,number,number];s.points=s.points.map(p=>[p[0],-p[1]])}emit('body',extrudeDirectSketch(s,depth.value,crypto.randomUUID()),cut.value)})}
</script>
<template>
 <svg ref="svg" class="sketch-overlay" :class="{drawing:tool!=='select'&&tool!=='trim'}" aria-label="Эскиз на основной сцене" @pointerdown="down" @pointermove="move" @pointerup="up" @pointercancel="cancel" @keydown.esc="cancel">
  <polyline v-for="s in document.sketches" :key="s.id" :points="path(s.closed?[...s.points,s.points[0]]:s.points)" fill="none" :stroke="s.id===selected?'#ffc43d':'#45c9b6'" stroke-width="3" class="pick" @pointerdown="pick($event,s)"/>
  <circle v-if="snapMarker" :cx="projected(snapMarker)?.[0]" :cy="projected(snapMarker)?.[1]" r="8" fill="none" stroke="#ff6c3d"/><polyline :points="path(draft)" fill="none" stroke="#45c9b6" stroke-width="2"/>
  <template v-if="current?.analytic"><circle v-for="(p,i) in [current.analytic.center,current.points[0],...(current.analytic.kind==='arc'?[current.points.at(-1)!]:[])]" :key="i" :cx="projected(p)?.[0]" :cy="projected(p)?.[1]" r="6" fill="#ffc43d" class="pick" @pointerdown="beginHandle($event,i===0?'center':i===1?(current!.analytic!.kind==='arc'?'start':'radius'):'end')"/><circle :cx="projected([current.analytic.center[0],current.analytic.center[1]+current.analytic.radius])?.[0]" :cy="projected([current.analytic.center[0],current.analytic.center[1]+current.analytic.radius])?.[1]" r="5" fill="#45c9b6" class="pick" @pointerdown="beginHandle($event,'radius')"/></template>
  <template v-else-if="current"><circle v-for="(p,i) in current.points" :key="i" :cx="projected(p)?.[0]" :cy="projected(p)?.[1]" r="5" fill="#ffc43d" class="pick" @pointerdown="beginHandle($event,'vertex',i)"/></template>
 </svg>
 <div class="sketch-tools" @keydown.stop><strong>{{ label('Эскиз на грани','Face sketch') }}</strong><label><input v-model="snapping" type="checkbox">Snap 3D</label><button v-for="(name,key) in {select:'Выбор',rectangle:'Прямоугольник',circle:'Круг',arc:'Дуга',polyline:'Ломаная',trim:'Обрезать'}" :key="key" :aria-pressed="tool===key" @click="cancel();tool=key">{{ locale==='ru'?name:key }}</button><button v-if="draft.length>1&&tool==='polyline'" @click="add([...draft],false);draft=[]">{{ label('Завершить','Finish') }}</button><button v-if="draft.length>2&&tool==='polyline'" @click="add([...draft],true);draft=[]">{{ label('Замкнуть','Close contour') }}</button><button :disabled="!history.canUndo" @click="document=history.undo()">↶</button><button :disabled="!history.canRedo" @click="document=history.redo()">↷</button><button @click="emit('close')">×</button>
 <template v-if="current"><label>Offset <input v-model.number="offset" type="number"></label><button @click="modify('offset')">Offset</button><select v-model="endpoint"><option value="start">{{ label('Начало','Start') }}</option><option value="end">{{ label('Конец','End') }}</option></select><button @click="modify('extend')">Extend</button><template v-if="current.analytic"><label v-for="key in ['radius','start','sweep'] as const" :key="key">{{ key }} <input type="number" :value="current.analytic[key]" @change="curveField(key,Number(($event.target as HTMLInputElement).value))"></label></template><label>{{ label('Глубина','Depth') }} <input v-model.number="depth" type="number" min="0.01"></label><label><input v-model="cut" type="checkbox">{{ label('Вырезать','Cut') }}</label><button :disabled="!current.closed" @click="extrude">{{ label('Выдавить','Extrude') }}</button></template><p v-if="error" role="alert">{{ error }}</p></div>
</template>
<style scoped>.sketch-overlay{position:absolute;inset:0;width:100%;height:100%;pointer-events:none;z-index:5}.sketch-overlay.drawing{pointer-events:all;touch-action:none;cursor:crosshair}.pick{pointer-events:all;cursor:pointer}.sketch-tools{position:absolute;top:95px;left:8px;right:320px;z-index:6;padding:8px;background:var(--surface);border:1px solid var(--border);display:flex;flex-wrap:wrap;gap:5px;color:var(--text);font-size:12px}.sketch-tools input[type=number]{width:55px}.sketch-tools button,.sketch-tools input{font:inherit;color:var(--text);background:var(--surface-raised);border:1px solid var(--border);padding:4px}.sketch-tools p{color:var(--danger)}@media(max-width:900px){.sketch-tools{right:8px}}</style>
