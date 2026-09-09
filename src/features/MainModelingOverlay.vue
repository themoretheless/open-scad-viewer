<script setup lang="ts">
import {sceneSnapPoints} from '../services/mainSnapping'
import {computed,ref} from 'vue'
import type {MeshData} from '../core/mesh'
import type {PickHit} from '../services/rendererContracts'
import {sceneBody,sceneFace,type MainOperation,type MainParameters} from '../services/mainModeling'
import {cross3,unit3,type Vec3} from '../services/directSketchGeometry'
const props=defineProps<{meshes:MeshData[];selected:number|null;selection:number[];hit:PickHit|null;operation:MainOperation|null;parameters:MainParameters;project:(p:readonly number[])=>[number,number]|null;revision:number;box:boolean}>()
const emit=defineEmits<{parameters:[p:MainParameters];preview:[];apply:[];cancel:[];select:[indices:number[]];boxDone:[]}>()
const snapCandidates=computed(()=>props.operation==='move'?sceneSnapPoints(props.meshes,props.selection):[])
const snapTarget=ref<number[]|null>(null)
const svg=ref<SVGSVGElement>(),rectangle=ref<{a:number[];b:number[]}|null>(null)
const bodies=computed(()=>props.meshes.map(sceneBody))
const points=(i:number)=>{const b=bodies.value[i];return b?Array.from({length:b.mesh.positions.length/3},(_,j)=>b.mesh.positions.slice(j*3,j*3+3)):[]}
const selectedPoints=computed(()=>props.selection.flatMap(points))
const center=computed(()=>[0,1,2].map(k=>{const a=selectedPoints.value.map(p=>p[k]);return a.length?(Math.min(...a)+Math.max(...a))/2:0}) as Vec3)
const size=computed(()=>Math.max(5,...selectedPoints.value.map(p=>Math.hypot(...p.map((v,k)=>v-center.value[k]))))*.7)
const project=(p:readonly number[])=>{void props.revision;return props.project(p)}
const path=(points:number[][])=>points.map(project).filter((p):p is [number,number]=>p!==null).map(p=>p.join(',')).join(' ')
const topology=computed(()=>{try{return props.selected===null?null:sceneFace(props.meshes[props.selected],props.hit)}catch{return null}})
const axes:Vec3[]=[[1,0,0],[0,1,0],[0,0,1]],colors=['#ef665c','#60bd72','#599af0']
const add=(a:number[],b:number[],scale=1)=>a.map((v,k)=>v+b[k]*scale) as Vec3
const normal=computed(()=>{try{return unit3(props.parameters.normal??axes[['x','y','z'].indexOf(props.parameters.axis)])}catch{return [0,0,1] as Vec3}})
const planeCenter=computed(()=>add(center.value,normal.value,props.parameters.amount-center.value.reduce((s,v,k)=>s+v*normal.value[k],0)))
const planePoints=computed(()=>{const n=normal.value,u=unit3(cross3(n,Math.abs(n[0])<.8?[1,0,0]:[0,1,0])),v=cross3(n,u);return [[-1,-1],[1,-1],[1,1],[-1,1],[-1,-1]].map(([a,b])=>add(add(planeCenter.value,u,a*size.value),v,b*size.value))})
const rings=computed(()=>axes.map((axis,k)=>{const u=axes[(k+1)%3],v=axes[(k+2)%3];return Array.from({length:65},(_,i)=>add(add(center.value,u,Math.cos(i*Math.PI/32)*size.value),v,Math.sin(i*Math.PI/32)*size.value))}))
const faceCenter=computed(()=>topology.value?.topology.faces[topology.value.face]?.center)
function facePath(triangles:number[]){const t=topology.value;if(!t)return '';return triangles.map(i=>{const p=t.body.mesh.indices.slice(i*3,i*3+3).map(j=>project(t.body.mesh.positions.slice(j*3,j*3+3)));return p.every(Boolean)?'M '+p.map(v=>v!.join(',')).join(' L ')+' Z':''}).join(' ')}
const edges=computed(()=>{const t=topology.value;if(!t)return [];return t.topology.edges.map(e=>[t.body.mesh.positions.slice(e.a*3,e.a*3+3),t.body.mesh.positions.slice(e.b*3,e.b*3+3)])})
let drag:{start:number[];direction:number[];factor:number;before:MainParameters;kind:string;axis:number;origin:number[];pointer:number;rotationSign:number;moved:boolean}|null=null
function begin(e:PointerEvent,kind:string,axis:number,origin:Vec3,direction:Vec3){
 const a=project(origin),b=project(add(origin,direction,size.value));if(!a||!b)return
 e.preventDefault();e.stopPropagation();svg.value?.focus();svg.value?.setPointerCapture(e.pointerId)
 const u=project(add(origin,axes[(axis+1)%3],size.value)),v=project(add(origin,axes[(axis+2)%3],size.value)),rotationSign=u&&v?Math.sign((u[0]-a[0])*(v[1]-a[1])-(u[1]-a[1])*(v[0]-a[0]))||1:1
 drag={rotationSign,moved:false,start:[e.clientX,e.clientY],direction:[b[0]-a[0],b[1]-a[1]],factor:size.value,before:JSON.parse(JSON.stringify(props.parameters)),kind,axis,origin:a,pointer:e.pointerId}
}
function move(e:PointerEvent){
 if(rectangle.value){const r=svg.value!.getBoundingClientRect();rectangle.value.b=[e.clientX-r.left,e.clientY-r.top];return}
 if(!drag)return
 const d=drag,dx=e.clientX-d.start[0],dy=e.clientY-d.start[1],length2=d.direction.reduce((s,v)=>s+v*v,0)
 const distance=length2>4?(dx*d.direction[0]+dy*d.direction[1])/length2*d.factor:-dy*d.factor/80
 d.moved ||= Math.hypot(dx,dy)>2
 const p={...d.before}
 if(d.kind==='move'){
  const key=(['x','y','z'] as const)[d.axis];p[key]+=distance;snapTarget.value=null
  if(props.parameters.snap!==false&&!e.altKey){const moved=center.value.map((v,k)=>v+p[(['x','y','z'] as const)[k]]),at=project(moved);let best=10;if(at)for(const point of snapCandidates.value){if(point.some((v,k)=>k!==d.axis&&Math.abs(v-moved[k])>1e-5))continue;const xy=project(point);if(!xy)continue;const pixels=Math.hypot(xy[0]-at[0],xy[1]-at[1]);if(pixels<best){best=pixels;p[key]=point[d.axis]-center.value[d.axis];snapTarget.value=point}}}
 }
 else if(d.kind==='rotate'){
  const rect=svg.value!.getBoundingClientRect(),cx=rect.left+d.origin[0],cy=rect.top+d.origin[1]
  p.axis=(['x','y','z'] as const)[d.axis];p.amount=d.before.amount+(Math.atan2(e.clientY-cy,e.clientX-cx)-Math.atan2(d.start[1]-cy,d.start[0]-cx))*180/Math.PI*d.rotationSign
 }else if(d.kind==='scale')p.amount=Math.max(.01,d.before.amount*Math.exp(distance/d.factor))
 else p.amount=d.before.amount+distance
 emit('parameters',p);emit('preview')
}
function end(){
 snapTarget.value=null
 if(rectangle.value){const {a,b}=rectangle.value;const ids=bodies.value.map((_,i)=>i).filter(i=>{const ps=points(i).map(project);return ps.length>0&&ps.every(p=>p&&p[0]>=Math.min(a[0],b[0])&&p[0]<=Math.max(a[0],b[0])&&p[1]>=Math.min(a[1],b[1])&&p[1]<=Math.max(a[1],b[1]))});rectangle.value=null;emit('select',ids);emit('boxDone');return}
 if(drag){const moved=drag.moved;drag=null;if(moved)emit('apply');else emit('cancel')}
}
function cancel(){snapTarget.value=null;if(drag)emit('parameters',drag.before);drag=null;rectangle.value=null;emit('cancel')}
function boxStart(e:PointerEvent){if(!props.box)return;const r=svg.value!.getBoundingClientRect();const a=[e.clientX-r.left,e.clientY-r.top];rectangle.value={a,b:a};svg.value!.setPointerCapture(e.pointerId)}
function toggleFace(i:number){const ids=new Set(props.parameters.openings??[]);ids.has(i)?ids.delete(i):ids.add(i);emit('parameters',{...props.parameters,openings:[...ids]})}
</script>
<template>
 <svg ref="svg" tabindex="0" class="model-overlay" :class="{box}" aria-label="Инструменты на модели" @pointerdown="boxStart" @pointermove="move" @pointerup="end" @pointercancel="cancel" @keydown.esc="cancel">
  <template v-if="operation==='fillet'||operation==='chamfer'"><polyline v-for="(e,i) in edges" :key="i" :points="path(e)" fill="none" :stroke="(parameters.edges?.includes(i)||parameters.edge===i)?'#ffd04a':'#58a3ed'" :stroke-width="(parameters.edges?.includes(i)||parameters.edge===i)?5:3" class="handle" @pointerdown.stop="emit('parameters',{...parameters,edge:i,edges:$event.shiftKey?(parameters.edges?.includes(i)?parameters.edges.filter(e=>e!==i):[...(parameters.edges??[parameters.edge]),i]):[i]})"/></template>
  <template v-if="operation==='shell'&&topology"><path v-for="(f,i) in topology.topology.faces" :key="i" :d="facePath(f.triangles)" :fill="parameters.openings?.includes(i)?'#ffb938':'#78b8ee'" fill-opacity=".25" stroke="#78b8ee" class="handle" @pointerdown.stop="toggleFace(i)"/></template>
  <template v-if="operation==='move'||operation==='scale'||operation==='rotate'"><g v-for="(axis,i) in axes" :key="i" :stroke="colors[i]" stroke-width="3" fill="none" class="handle" @pointerdown="begin($event,operation!,i,center,axis)"><polyline v-if="operation==='rotate'" :points="path(rings[i])"/><template v-else><polyline :points="path([center,add(center,axis,size)])"/><circle v-if="project(add(center,axis,size))" :cx="project(add(center,axis,size))![0]" :cy="project(add(center,axis,size))![1]" r="7" :fill="colors[i]"/></template></g></template>
  <circle v-if="operation==='push'&&faceCenter&&project(faceCenter)" class="handle" :cx="project(faceCenter)![0]" :cy="project(faceCenter)![1]" r="9" fill="#ffc340" @pointerdown="begin($event,'push',0,faceCenter,topology!.topology.faces[topology!.face].normal)"/>
  <g v-if="operation==='split'" class="handle" @pointerdown="begin($event,'split',0,planeCenter,normal)"><polygon :points="path(planePoints)" fill="#edbd40" fill-opacity=".2" stroke="#edbd40"/><circle v-if="project(planeCenter)" :cx="project(planeCenter)![0]" :cy="project(planeCenter)![1]" r="9" fill="#edbd40"/></g>
  <circle v-if="snapTarget&&project(snapTarget)" :cx="project(snapTarget)![0]" :cy="project(snapTarget)![1]" r="9" stroke="#ffb938" fill="none"/>
  <rect v-if="rectangle" :x="Math.min(rectangle.a[0],rectangle.b[0])" :y="Math.min(rectangle.a[1],rectangle.b[1])" :width="Math.abs(rectangle.a[0]-rectangle.b[0])" :height="Math.abs(rectangle.a[1]-rectangle.b[1])" fill="#58a3ed33" stroke="#58a3ed"/>
 </svg>
</template>
<style scoped>.model-overlay{position:absolute;inset:0;width:100%;height:100%;pointer-events:none;z-index:4;overflow:hidden}.handle{pointer-events:all;cursor:grab;touch-action:none}.box{pointer-events:all;cursor:crosshair;touch-action:none}</style>
