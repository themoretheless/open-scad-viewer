<script setup lang="ts">
import type {Vec3} from '../services/directSketchGeometry'
defineProps<{points:Vec3[];project:(p:Vec3,pane:'3d')=>number[];size:number;ru:boolean}>()
</script>
<template>
 <g pointer-events="none" :data-measurement="points.length===1?'volume-contact':'volume-separation'">
  <line v-if="points.length===2" :x1="project(points[0],'3d')[0]" :y1="project(points[0],'3d')[1]" :x2="project(points[1],'3d')[0]" :y2="project(points[1],'3d')[1]" stroke="#f4afee" stroke-width="2" vector-effect="non-scaling-stroke"/>
  <circle v-for="(p,i) in points" :key="i" :cx="project(p,'3d')[0]" :cy="project(p,'3d')[1]" :r="size/70" fill="none" stroke="#f4afee" stroke-width="3" vector-effect="non-scaling-stroke">
   <title>{{points.length===1?(ru?'Область подтверждённого контакта':'Certified contact region'):(ru?'Точка на исходной поверхности':'Point on authored surface')}} {{points.length===2?(i===0?'A':'B'):''}}</title>
  </circle>
 </g>
</template>
