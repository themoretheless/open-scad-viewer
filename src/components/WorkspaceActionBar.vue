<script setup lang="ts">
defineProps<{ label: string }>()
</script>

<template>
  <div class="ws-actions" role="toolbar" :aria-label="label"><slot /></div>
</template>

<style>
/* Shared by the Solid and Mesh workspaces: actions for the current selection, floating over the scene. */
.ws-actions {
  position: absolute; z-index: 3; left: 50%; bottom: 44px; transform: translateX(-50%); max-width: calc(100% - 24px);
  display: flex; flex-wrap: wrap; align-items: center; gap: 1px; padding: 3px;
  border: 1px solid var(--hairline); border-radius: 9px; background: color-mix(in srgb, var(--surface) 96%, transparent);
  box-shadow: 0 6px 20px rgba(0, 0, 0, .3); color: var(--text); font-size: 12px;
}
.ws-actions > button { height: 26px; padding: 0 9px; border: 0; border-radius: 6px; background: transparent; color: var(--text); font-weight: 500; cursor: pointer; }
.ws-actions > button:hover:not(:disabled) { background: var(--hover); }
.ws-actions > button:disabled { opacity: .4; cursor: default; }
.ws-actions > button[aria-pressed="true"] { background: color-mix(in srgb, var(--accent) 16%, transparent); color: var(--accent); }
.ws-actions > button.danger { color: var(--danger); }
.ws-actions > .ws-name { padding: 0 8px 0 6px; color: var(--text-dim); max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ws-actions > .ws-sep { width: 1px; height: 16px; margin: 0 3px; background: var(--hairline); }
</style>
