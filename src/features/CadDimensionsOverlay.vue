<script setup lang="ts">
import {computed} from 'vue'
import type {MeshData} from '../core/mesh'
import {sceneBody} from '../services/mainModeling'
import {bounds} from '../services/cadWorkbench'
const props=defineProps<{meshes:MeshData[];selection:number[];project:(p:number[])=>number[]|null;revision:number}>()
const emit=defineEmits<{resize:[dimensions:number[]]}>()
const box=computed(()=>{try{return bounds(props.selection.map(i=>sceneBody(props.meshes[i],i)))}catch{return null}})
const lines=computed(()=>{void props.revision;const b=box.value;if(!b)return [];return [0,1,2].map(k=>{const end=[...b.min];end[k]=b.max[k];return {a:props.project(b.min),b:props.project(end),size:b.max[k]-b.min[k]}})})
function change(k:number,value:number){if(!box.value||!Number.isFinite(value)||value<=0)return;const sizes=box.value.max.map((v,i)=>v-box.value!.min[i]);sizes[k]=value;emit('resize',sizes)}
</script>
<template><svg class="dimensions"><g v-for="(line,i) in lines" :key="i" v-show="line.a&&line.b"><line :x1="line.a?.[0]" :y1="line.a?.[1]" :x2="line.b?.[0]" :y2="line.b?.[1]" stroke="#ecb53e" stroke-width="2"/><foreignObject :x="((line.a?.[0]??0)+(line.b?.[0]??0))/2" :y="((line.a?.[1]??0)+(line.b?.[1]??0))/2" width="105" height="32"><label>{{ ['X','Y','Z'][i] }}<input type="number" :aria-label="'Размер '+['X','Y','Z'][i]" :value="Number(line.size.toFixed(3))" min="0.01" @pointerdown.stop @keydown.stop @change="change(i,Number(($event.target as HTMLInputElement).value))"></label></foreignObject></g></svg></template>
<style scoped>.dimensions{position:absolute;inset:0;width:100%;height:100%;z-index:5;pointer-events:none}.dimensions label{display:flex;gap:3px;align-items:center;background:var(--surface);color:var(--text);font-size:12px;padding:3px;pointer-events:auto}.dimensions input{width:70px;background:var(--surface);color:var(--text);border:1px solid var(--border)}</style>
