<script setup lang="ts">
import {computed} from 'vue'
import {sourceBodyDisplayMessage,sourceBodyDisplayLocation} from '../services/sourceBodyDisplay'
const props=defineProps<{name:string;id:string;error:Error;locale:string;pending?:boolean}>()
defineEmits<{retry:[]}>()
const message=computed(()=>sourceBodyDisplayMessage(props.error,props.locale))
const location=computed(()=>sourceBodyDisplayLocation(props.error,props.locale))
</script>
<template>
 <span role="alert" class="source-failure">
  <strong>{{ name }}:</strong> <span>{{ message }}</span>
  <span v-if="location" class="source-location">{{ location }}</span>
  <button type="button" :disabled="pending" @click="$emit('retry')">{{ locale==='ru'?'Повторить':'Retry' }}</button>
  <details><summary>{{ locale==='ru'?'Подробности':'Details' }}</summary>{{ id }}: {{ error.message }}</details>
 </span>
</template>
<style scoped>
.source-failure{display:flex;align-items:center;flex-wrap:wrap;gap:4px 8px;min-width:0;font-size:12px;line-height:1.45}
.source-failure>span{min-width:0}.source-location{font-weight:600;white-space:nowrap}
.source-failure button{font:inherit;padding:3px 7px;cursor:pointer}.source-failure details{overflow-wrap:anywhere}
.source-failure summary{cursor:pointer}
</style>
