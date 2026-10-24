<script setup lang="ts">
import {computed,onMounted,onUnmounted,ref} from 'vue'
import CadQuantityInput from './CadQuantityInput.vue'
const props=defineProps<{anchor:number[];center:number[];size:number;modelValue:number;locale:string;failed:boolean;label:string}>()
const emit=defineEmits<{'update:modelValue':[number];validity:[boolean]}>()
const root=ref<HTMLElement>(),extent=ref([0,0,32])
let observer:ResizeObserver|undefined
onMounted(()=>{
 const host=root.value?.parentElement;if(!host)return
 const resize=()=>{extent.value=[host.clientWidth,host.clientHeight,root.value?.offsetHeight??32]}
 observer=new ResizeObserver(resize);observer.observe(host);if(root.value)observer.observe(root.value);resize()
})
onUnmounted(()=>observer?.disconnect())
const position=computed(()=>{
 const [width,height,fieldHeight]=extent.value,scale=Math.min(width,height)/props.size
 return {left:Math.max(4,Math.min(width-128,width/2+(props.anchor[0]-props.center[0])*scale+16))+'px',
  top:Math.max(4,Math.min(height-fieldHeight-4,height/2+(props.anchor[1]-props.center[1])*scale-16))+'px'}
})
</script>
<template>
 <label ref="root" class="gizmo-dimension" :class="{failed}" :style="position" @pointerdown.stop @wheel.stop>
  <CadQuantityInput :model-value="modelValue" :locale="locale" :aria-label="label" step="0.5" @update:model-value="emit('update:modelValue',$event)" @validity="emit('validity',$event)" /><span class="dimension-unit">mm</span>
 </label>
</template>
<style scoped>
.gizmo-dimension{position:absolute;z-index:4;display:flex;align-items:center;gap:4px;width:124px;box-sizing:border-box;padding:5px;background:var(--bg);color:var(--text);font:13px/1.2 var(--font-ui,system-ui);border:1px solid #77eac5;border-radius:5px}
.gizmo-dimension.failed{border-color:#f87171}
.gizmo-dimension:focus-within{outline:2px solid var(--accent)}
.gizmo-dimension :deep(.quantity-field){flex:1;min-width:0}
.gizmo-dimension :deep(input){font:inherit;min-width:0;width:100%;padding:0;border:0;background:transparent;color:inherit}
.gizmo-dimension :deep(input:focus-visible){outline:0}
.gizmo-dimension :deep(small){font-size:11px}
.dimension-unit{flex:none}
</style>
