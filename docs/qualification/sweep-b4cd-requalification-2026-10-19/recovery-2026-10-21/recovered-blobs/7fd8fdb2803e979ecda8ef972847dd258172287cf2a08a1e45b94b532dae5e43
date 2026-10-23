<script setup lang="ts">
import { computed, nextTick, ref, useId, watch } from 'vue'
import type { ExampleCatalogEntry } from '../data/examples'
import { filterExampleCatalog, MAX_EXAMPLE_QUERY_LENGTH } from '../services/exampleCatalog'

const props = defineProps<{
  open: boolean
  examples: readonly ExampleCatalogEntry[]
  locale: 'ru' | 'en'
}>()
const emit = defineEmits<{ close: []; select: [id: string]; downloadCurrent: [] }>()

const labels = {
  ru: { title: 'Галерея примеров', search: 'Найти пример', close: 'Закрыть', load: 'Выбрать', warning: 'Пример заменит текущий документ.', empty: 'Ничего не найдено', confirm: 'Заменить текст в редакторе?', cancel: 'Отмена', replace: 'Заменить', download: 'Скачать текущий и заменить' },
  en: { title: 'Example gallery', search: 'Find an example', close: 'Close', load: 'Choose', warning: 'An example replaces the current document.', empty: 'No examples found', confirm: 'Replace the editor text?', cancel: 'Cancel', replace: 'Replace', download: 'Download current and replace' },
} as const

const instanceId = useId()
const titleId = `${instanceId}-title`
const descriptionId = `${instanceId}-description`
const query = ref('')
const pendingId = ref<string | null>(null)
const searchRef = ref<HTMLInputElement | null>(null)
const dialogRef = ref<HTMLElement | null>(null)
let previousFocus: HTMLElement | null = null

const copy = computed(() => labels[props.locale])
const filtered = computed(() => filterExampleCatalog(props.examples, query.value, props.locale))

watch(() => props.open, async open => {
  if (open) {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
    query.value = ''
    pendingId.value = null
    await nextTick()
    searchRef.value?.focus()
  } else if (previousFocus?.isConnected) {
    await nextTick()
    previousFocus.focus({ preventScroll: true })
    previousFocus = null
  }
})

function requestClose() { emit('close') }
async function choose(id: string) {
  pendingId.value = id
  await nextTick()
  dialogRef.value?.querySelector<HTMLElement>('[data-confirm-cancel]')?.focus()
}
function cancelChoice() { pendingId.value = null }
function replace(download: boolean) {
  const id = pendingId.value
  if (!id) return
  if (download) emit('downloadCurrent')
  emit('select', id)
  emit('close')
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault()
    event.stopPropagation()
    requestClose()
  } else if (event.key === 'Tab') {
    const controls = [...(dialogRef.value?.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled])') ?? [])]
    if (!controls.length) return
    const current = controls.indexOf(document.activeElement as HTMLElement)
    const next = event.shiftKey
      ? (current <= 0 ? controls.length - 1 : current - 1)
      : (current === controls.length - 1 ? 0 : current + 1)
    event.preventDefault()
    controls[next].focus()
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="gallery-backdrop" @pointerdown.self="requestClose">
      <section
        ref="dialogRef"
        class="gallery"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        :aria-describedby="descriptionId"
        @keydown="handleKeydown"
      >
        <header class="gallery-header">
          <div>
            <h2 :id="titleId">{{ copy.title }}</h2>
            <p :id="descriptionId">{{ copy.warning }}</p>
          </div>
          <button type="button" class="gallery-close" :aria-label="copy.close" @click="requestClose">×</button>
        </header>
        <template v-if="!pendingId">
        <input
          ref="searchRef"
          v-model="query"
          class="gallery-search"
          type="search"
          autocomplete="off"
          :maxlength="MAX_EXAMPLE_QUERY_LENGTH"
          :aria-label="copy.search"
          :placeholder="copy.search"
        >
        <div class="gallery-grid">
          <article v-for="entry in filtered" :key="entry.id" class="example-card">
            <div class="example-preview" aria-hidden="true">
              <code>{{ entry.source.split('\n').filter(Boolean).slice(0, 3).join('\n') }}</code>
            </div>
            <h3>{{ entry.title[locale] }}</h3>
            <p>{{ entry.description[locale] }}</p>
            <div class="example-tags" aria-hidden="true">
              <span v-for="tag in entry.tags.slice(0, 3)" :key="tag">{{ tag }}</span>
            </div>
            <button type="button" @click="choose(entry.id)">{{ copy.load }}</button>
          </article>
          <p v-if="!filtered.length" class="gallery-empty" role="status">{{ copy.empty }}</p>
        </div>
        </template>
        <div v-else class="gallery-confirm" role="alert">
          <h3>{{ copy.confirm }}</h3>
          <p>{{ examples.find(entry => entry.id === pendingId)?.title[locale] }}</p>
          <div class="gallery-confirm-actions">
            <button data-confirm-cancel type="button" @click="cancelChoice">{{ copy.cancel }}</button>
            <button type="button" @click="replace(true)">{{ copy.download }}</button>
            <button class="danger-action" type="button" @click="replace(false)">{{ copy.replace }}</button>
          </div>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.gallery-backdrop { position: fixed; z-index: 100; inset: 0; display: grid; place-items: center; padding: 20px; background: rgba(4, 6, 10, .64); backdrop-filter: blur(3px); }
.gallery { width: min(900px, 100%); max-height: min(760px, 92vh); overflow: hidden; display: flex; flex-direction: column; gap: 14px; padding: 18px; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: 14px; box-shadow: 0 24px 80px rgba(0,0,0,.45); }
.gallery-header { display: flex; justify-content: space-between; gap: 16px; }
.gallery-header h2 { margin: 0 0 4px; font-size: 1.25rem; }
.gallery-header p { margin: 0; color: var(--text-dim); font-size: .8rem; }
.gallery-close { align-self: flex-start; width: 36px; height: 36px; border: 1px solid var(--border); border-radius: 8px; background: transparent; cursor: pointer; }
.gallery-search { width: 100%; min-height: 42px; padding: 8px 11px; color: inherit; background: var(--surface-raised); border: 1px solid var(--border); border-radius: 8px; outline: none; }
.gallery-search:focus-visible, button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
.gallery-grid { overflow-y: auto; display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; padding: 2px; }
.example-card { min-width: 0; display: flex; flex-direction: column; padding: 12px; border: 1px solid var(--border); border-radius: 10px; background: var(--surface-raised); }
.example-preview { height: 80px; overflow: hidden; padding: 9px; color: var(--text-dim); background: var(--canvas-bg); border-radius: 7px; white-space: pre; }
.example-preview code { font-size: .65rem; }
.example-card h3 { margin: 11px 0 4px; font-size: .95rem; }
.example-card p { min-height: 2.5em; margin: 0 0 10px; color: var(--text-dim); font-size: .78rem; }
.example-tags { display: flex; flex-wrap: wrap; gap: 5px; margin-bottom: 11px; }
.example-tags span { padding: 2px 6px; color: var(--text-dim); border: 1px solid var(--border); border-radius: 999px; font-size: .62rem; }
.example-card button { min-height: 36px; margin-top: auto; color: #fff; background: var(--accent-strong); border: 0; border-radius: 7px; cursor: pointer; }
.gallery-empty { grid-column: 1 / -1; padding: 40px; text-align: center; color: var(--text-dim); }
.gallery-confirm { padding: 28px 4px 8px; text-align: center; }
.gallery-confirm h3 { margin: 0 0 8px; }
.gallery-confirm p { color: var(--text-dim); }
.gallery-confirm-actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 8px; margin-top: 22px; }
.gallery-confirm-actions button { min-height: 40px; padding: 7px 12px; color: inherit; background: var(--surface-raised); border: 1px solid var(--border); border-radius: 7px; cursor: pointer; }
.gallery-confirm-actions .danger-action { color: #fff; background: var(--accent-strong); border-color: transparent; }
@media (max-width: 620px) { .gallery-backdrop { padding: 8px; } .gallery { max-height: 96vh; padding: 12px; } .gallery-grid { grid-template-columns: 1fr; } }
@media (pointer: coarse) { .gallery-close, .example-card button { min-height: 44px; } .gallery-close { width: 44px; } }
</style>
