<script setup lang="ts">
import { nextTick, ref, useId, watch } from 'vue'
import type { ShortcutHelpGroup } from '../services/commandRegistry'

const props = defineProps<{ open: boolean; groups: readonly ShortcutHelpGroup[]; locale: 'ru' | 'en' }>()
const emit = defineEmits<{ close: [] }>()
const labels = {
  ru: { title: 'Горячие клавиши', close: 'Закрыть', workspace: 'Файл и приложение', editor: 'Редактор', navigation: 'Навигация', selection: 'Выбор', inspection: 'Измерения и анализ', display: 'Отображение', note: 'Сочетания в разделе сцены работают, когда фокус не находится в поле ввода.' },
  en: { title: 'Keyboard shortcuts', close: 'Close', workspace: 'File and app', editor: 'Editor', navigation: 'Navigation', selection: 'Selection', inspection: 'Measure and inspect', display: 'Display', note: 'Viewport shortcuts work while focus is outside an editable field.' },
} as const
const titleId = `${useId()}-title`
const dialogRef = ref<HTMLElement | null>(null)
const closeRef = ref<HTMLButtonElement | null>(null)
let previousFocus: HTMLElement | null = null

watch(() => props.open, async open => {
  if (open) {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
    await nextTick()
    closeRef.value?.focus()
  } else if (previousFocus?.isConnected) {
    await nextTick()
    previousFocus.focus({ preventScroll: true })
    previousFocus = null
  }
})

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape' || event.key === '?') {
    event.preventDefault()
    event.stopPropagation()
    emit('close')
  } else if (event.key === 'Tab') {
    event.preventDefault()
    closeRef.value?.focus()
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="shortcut-backdrop" @pointerdown.self="emit('close')">
      <section ref="dialogRef" class="shortcut-dialog" role="dialog" aria-modal="true" :aria-labelledby="titleId" @keydown="handleKeydown">
        <header>
          <div>
            <h2 :id="titleId">{{ labels[locale].title }}</h2>
            <p>{{ labels[locale].note }}</p>
          </div>
          <button ref="closeRef" type="button" :aria-label="labels[locale].close" @click="emit('close')">×</button>
        </header>
        <div class="shortcut-groups">
          <section v-for="group in groups" :key="group.id" class="shortcut-group">
            <h3>{{ labels[locale][group.id] }}</h3>
            <dl>
              <div v-for="row in group.rows" :key="row.id" class="shortcut-row">
                <dt>{{ row.label }}</dt>
                <dd>
                  <template v-for="(shortcut, index) in row.shortcuts" :key="shortcut">
                    <span v-if="index" aria-hidden="true"> / </span><kbd>{{ shortcut }}</kbd>
                  </template>
                </dd>
              </div>
            </dl>
          </section>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.shortcut-backdrop { position: fixed; z-index: 110; inset: 0; display: grid; place-items: center; padding: 18px; background: rgba(4,6,10,.62); backdrop-filter: blur(3px); }
.shortcut-dialog { width: min(760px, 100%); max-height: min(760px, 92dvh); overflow: hidden; display: flex; flex-direction: column; padding: 18px; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: 14px; box-shadow: 0 24px 80px rgba(0,0,0,.46); }
header { display: flex; justify-content: space-between; gap: 18px; padding-bottom: 13px; border-bottom: 1px solid var(--border); }
h2 { margin: 0 0 4px; font-size: 1.2rem; } header p { margin: 0; color: var(--text-dim); font-size: .76rem; }
header button { flex: 0 0 auto; width: 40px; height: 40px; color: inherit; background: transparent; border: 1px solid var(--border); border-radius: 8px; cursor: pointer; }
header button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
.shortcut-groups { overflow-y: auto; display: grid; grid-template-columns: repeat(2, minmax(0,1fr)); gap: 18px 26px; padding: 16px 3px 3px; }
.shortcut-group h3 { margin: 0 0 7px; color: var(--text-dim); font-size: .7rem; letter-spacing: .05em; text-transform: uppercase; }
.shortcut-group dl { margin: 0; }
.shortcut-row { display: flex; align-items: center; justify-content: space-between; gap: 14px; min-height: 32px; border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent); }
.shortcut-row dt { min-width: 0; font-size: .78rem; }
.shortcut-row dd { margin: 0; }
kbd { display: inline-flex; min-height: 23px; align-items: center; padding: 2px 7px; color: var(--text-dim); background: var(--surface-raised); border: 1px solid var(--border); border-radius: 5px; font: 600 .68rem/1 ui-monospace, monospace; white-space: nowrap; }
@media (max-width: 600px) { .shortcut-backdrop { padding: 8px; } .shortcut-dialog { max-height: 96dvh; padding: 12px; } .shortcut-groups { grid-template-columns: 1fr; } }
</style>
