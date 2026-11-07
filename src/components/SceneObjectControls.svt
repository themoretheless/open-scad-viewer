<script setup lang="ts">
defineProps<{name:string;hidden:boolean;locked:boolean;locale:string}>()
defineEmits<{visibility:[];lock:[]}>()
</script>
<template>
  <span class="object-controls">
    <button type="button" :aria-label="(locale==='ru'?(hidden?'Показать: ':'Скрыть: '):(hidden?'Show: ':'Hide: '))+name" :aria-pressed="hidden" @click="$emit('visibility')">{{ hidden?'○':'●' }}</button>
    <button type="button" :aria-label="(locale==='ru'?(locked?'Разблокировать: ':'Заблокировать: '):(locked?'Unlock: ':'Lock: '))+name" :aria-pressed="locked" @click="$emit('lock')">{{ locked?'🔒':'◇' }}</button>
  </span>
</template>
<style scoped>
.object-controls{display:flex;flex-shrink:0;gap:2px}.object-controls button{width:26px;padding:3px;border:0;border-radius:4px;background:transparent;color:var(--text-dim);cursor:pointer}.object-controls button[aria-pressed=true]{color:var(--accent);background:var(--surface-raised)}
</style>
