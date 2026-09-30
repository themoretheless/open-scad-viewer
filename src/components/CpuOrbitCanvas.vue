<script setup lang="ts">
import {ref,watch,onMounted} from 'vue'
import {paintCpuOrbit,type CpuPaintTriangle} from '../services/cpuOrbitPaint'
const props=defineProps<{active:boolean;triangles:readonly CpuPaintTriangle[];size:number;left:number;top:number}>()
const emit=defineEmits<{ready:[value:boolean]}>(),canvas=ref<HTMLCanvasElement|null>(null)
function draw(){
 if(!props.active){emit('ready',false);return}
 const element=canvas.value
 if(!element?.getContext){emit('ready',false);return}
 element.style.visibility='hidden'
 try{
  const ctx=element.getContext('2d'),bounds=element.getBoundingClientRect()
  if(!ctx||!bounds.width||!bounds.height){emit('ready',false);return}
  const ratio=globalThis.devicePixelRatio||1,width=Math.ceil(bounds.width*ratio),height=Math.ceil(bounds.height*ratio)
  if(element.width!==width)element.width=width;if(element.height!==height)element.height=height
  paintCpuOrbit(ctx,props.triangles,width,height,props.size,props.left,props.top,bounds.width)
  element.style.visibility='visible';emit('ready',true)
 }catch{emit('ready',false)}
}
watch(()=>[props.active,props.triangles,props.size,props.left,props.top],draw,{flush:'post'})
onMounted(draw)
</script>
<template><foreignObject v-show="active" :x="left" :y="top" :width="size" :height="size" pointer-events="none" aria-hidden="true"><canvas ref="canvas" data-cpu-orbit style="display:block;width:100%;height:100%;pointer-events:none" /></foreignObject></template>
