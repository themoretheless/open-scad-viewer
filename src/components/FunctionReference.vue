<script setup lang="ts">
import { computed, nextTick, ref, useId, watch } from 'vue'
import {
  REFERENCE_CATEGORIES,
  searchFunctionReference,
  type ReferenceLanguage,
} from '../data/functionReference'
import { highlightCode } from '../services/codeHighlight'

const props = defineProps<{
  open: boolean
  locale: 'ru' | 'en'
  language: ReferenceLanguage
  initialQuery?: string
}>()
const emit = defineEmits<{ close: [] }>()

const labels = {
  ru: {
    title: 'Справочник функций', description: 'Синтаксис, параметры и готовые примеры.',
    close: 'Закрыть справочник', search: 'Найти функцию', placeholder: 'translate, поворот, список…',
    category: 'Категория', all: 'Все категории', language: 'Язык примеров', found: 'Найдено',
    functions: 'Функции', syntax: 'Синтаксис', parameters: 'Параметры', note: 'Особенности',
    example: 'Готовый пример', copy: 'Копировать', copying: 'Копирование…', copied: 'Скопировано',
    copyError: 'Не удалось скопировать. Выделите и скопируйте код примера вручную.',
    exampleHint: 'Скопируйте пример в редактор и запустите сборку.',
    empty: 'Функции не найдены', emptyHint: 'Измените запрос или выберите другую категорию.',
    reset: 'Сбросить фильтры', detail: 'Описание функции',
  },
  en: {
    title: 'Function reference', description: 'Syntax, parameters and ready-to-run examples.',
    close: 'Close reference', search: 'Find a function', placeholder: 'translate, rotation, list…',
    category: 'Category', all: 'All categories', language: 'Example language', found: 'Results',
    functions: 'Functions', syntax: 'Syntax', parameters: 'Parameters', note: 'Notes',
    example: 'Ready-to-run example', copy: 'Copy', copying: 'Copying…', copied: 'Copied',
    copyError: 'Could not copy. Select the example code and copy it manually.',
    exampleHint: 'Copy the example into the editor and build it.',
    empty: 'No functions found', emptyHint: 'Try another search or choose a different category.',
    reset: 'Reset filters', detail: 'Function details',
  },
} as const

const instanceId = useId()
const titleId = `${instanceId}-title`
const descriptionId = `${instanceId}-description`
const categoryId = `${instanceId}-category`
const detailId = `${instanceId}-detail`
const query = ref('')
const category = ref<'all' | typeof REFERENCE_CATEGORIES[number]['id']>('all')
const exampleLanguage = ref<ReferenceLanguage>(props.language)
const selectedId = ref('translate')
const copyState = ref<'idle' | 'copying' | 'copied' | 'error'>('idle')
const searchRef = ref<HTMLInputElement | null>(null)
const dialogRef = ref<HTMLElement | null>(null)
const detailRef = ref<HTMLElement | null>(null)
let previousFocus: HTMLElement | null = null
let copyRequest = 0

const copy = computed(() => labels[props.locale])
const filtered = computed(() => searchFunctionReference(query.value, exampleLanguage.value, category.value))
const selected = computed(() => filtered.value.find(entry => entry.id === selectedId.value))
const variant = computed(() => selected.value?.variants[exampleLanguage.value])
const selectedCategory = computed(() => REFERENCE_CATEGORIES.find(item => item.id === selected.value?.category))
const highlightedExample = computed(() => highlightCode(variant.value?.example ?? ''))

watch(filtered, entries => {
  if (!entries.some(entry => entry.id === selectedId.value)) selectedId.value = entries[0]?.id ?? ''
})

watch([selectedId, exampleLanguage], async () => {
  resetCopyState()
  await nextTick()
  detailRef.value?.scrollTo({ top: 0 })
})

watch(() => props.open, async open => {
  if (open) {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
    query.value = props.initialQuery?.slice(0, 200) ?? ''
    category.value = 'all'
    exampleLanguage.value = props.language
    selectedId.value = 'translate'
    resetCopyState()
    await nextTick()
    if (props.open) searchRef.value?.focus()
  } else {
    resetCopyState()
    const target = previousFocus
    previousFocus = null
    await nextTick()
    if (!props.open && target?.isConnected) target.focus({ preventScroll: true })
  }
}, { immediate: true })

function requestClose() { emit('close') }

function resetCopyState() {
  copyRequest++
  copyState.value = 'idle'
}

function resetFilters() {
  query.value = ''
  category.value = 'all'
  selectedId.value = 'translate'
  searchRef.value?.focus()
}

async function copyExample() {
  const source = variant.value?.example
  if (!source || copyState.value === 'copying') return
  const request = ++copyRequest
  copyState.value = 'copying'
  try {
    await navigator.clipboard.writeText(source)
    if (request === copyRequest) copyState.value = 'copied'
  } catch {
    if (request === copyRequest) copyState.value = 'error'
  }
}

function focusSelected() {
  const button = dialogRef.value?.querySelector<HTMLElement>('[data-reference-function][aria-pressed="true"]')
  button?.focus({ preventScroll: true })
  button?.scrollIntoView({ block: 'nearest' })
}

async function handleListKeydown(event: KeyboardEvent, index: number) {
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  event.stopPropagation()
  const count = filtered.value.length
  const next = event.key === 'Home' ? 0
    : event.key === 'End' ? count - 1
    : (index + (event.key === 'ArrowDown' ? 1 : -1) + count) % count
  selectedId.value = filtered.value[next].id
  await nextTick()
  focusSelected()
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault()
    event.stopPropagation()
    requestClose()
  } else if (event.key === 'Tab') {
    const controls = [...(dialogRef.value?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex="0"]',
    ) ?? [])].filter(control => control.tabIndex >= 0 && control.getClientRects().length > 0)
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
    <div v-if="open" class="reference-backdrop" @pointerdown.self="requestClose">
      <section
        ref="dialogRef"
        class="reference"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        :aria-describedby="descriptionId"
        @keydown="handleKeydown"
      >
        <header class="reference-header">
          <div class="reference-heading">
            <svg class="reference-book" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true">
              <path d="M12 5.5C9 3.5 5.5 3.5 3 4.5v15c2.5-1 6-1 9 1 3-2 6.5-2 9-1v-15c-2.5-1-6-1-9 1Zm0 0v15" />
              <path d="M6 8h3M6 11h3m6-3h3m-3 3h3" />
            </svg>
            <div>
              <h2 :id="titleId">{{ copy.title }}</h2>
              <p :id="descriptionId">{{ copy.description }}</p>
            </div>
          </div>
          <button type="button" class="reference-close" :aria-label="copy.close" @click="requestClose">×</button>
        </header>

        <div class="reference-toolbar">
          <div class="reference-search">
            <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
              <circle cx="10.5" cy="10.5" r="6.5" /><path d="m16 16 4.5 4.5" />
            </svg>
            <input
              ref="searchRef"
              v-model="query"
              type="search"
              autocomplete="off"
              spellcheck="false"
              maxlength="200"
              :aria-label="copy.search"
              :placeholder="copy.placeholder"
              @keydown.down.prevent="focusSelected"
            >
          </div>
          <div class="reference-languages" role="group" :aria-label="copy.language">
            <button type="button" :aria-pressed="exampleLanguage === 'openscad'" @click="exampleLanguage = 'openscad'">OpenSCAD</button>
            <button type="button" :aria-pressed="exampleLanguage === 'modelgraph'" @click="exampleLanguage = 'modelgraph'">ModelGraph</button>
          </div>
        </div>

        <div class="reference-body">
          <aside class="reference-sidebar" :aria-label="copy.functions">
            <div class="reference-filter">
              <label :for="categoryId">{{ copy.category }}</label>
              <select :id="categoryId" v-model="category">
                <option value="all">{{ copy.all }}</option>
                <option v-for="item in REFERENCE_CATEGORIES" :key="item.id" :value="item.id">{{ item.label[locale] }}</option>
              </select>
              <span class="reference-count" role="status">{{ copy.found }}: {{ filtered.length }}</span>
            </div>
            <nav class="reference-list" :aria-label="copy.functions">
              <button
                v-for="(entry, index) in filtered"
                :key="entry.id"
                type="button"
                data-reference-function
                :aria-pressed="entry.id === selectedId"
                :aria-controls="detailId"
                :tabindex="entry.id === selectedId ? 0 : -1"
                @click="selectedId = entry.id"
                @keydown="handleListKeydown($event, index)"
              >
                <span class="reference-function-name">{{ entry.name }}</span>
                <span class="reference-function-summary">{{ entry.summary[locale] }}</span>
              </button>
            </nav>
          </aside>

          <article
            v-if="selected && variant"
            :id="detailId"
            ref="detailRef"
            class="reference-detail"
            tabindex="0"
            :aria-label="`${copy.detail}: ${selected.name}`"
          >
            <span class="reference-category-tag">{{ selectedCategory?.label[locale] }}</span>
            <h3>{{ selected.name }}</h3>
            <p class="reference-summary">{{ selected.summary[locale] }}</p>

            <h4>{{ copy.syntax }}</h4>
            <pre class="reference-signature"><code>{{ variant.signature }}</code></pre>

            <template v-if="variant.parameters.length">
              <h4>{{ copy.parameters }}</h4>
              <dl class="reference-parameters">
                <div v-for="parameter in variant.parameters" :key="parameter.name">
                  <dt><code>{{ parameter.name }}</code></dt>
                  <dd>{{ parameter.description[locale] }}</dd>
                </div>
              </dl>
            </template>

            <aside v-if="variant.notes" class="reference-note">
              <h4>{{ copy.note }}</h4>
              <p>{{ variant.notes[locale] }}</p>
            </aside>

            <div class="reference-example-heading">
              <h4>{{ copy.example }}</h4>
              <span>{{ exampleLanguage === 'openscad' ? 'OpenSCAD' : 'ModelGraph' }}</span>
              <button type="button" class="reference-copy" :disabled="copyState === 'copying'" @click="copyExample">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
                  <path v-if="copyState === 'copied'" d="m5 12 4 4L19 6" />
                  <template v-else><rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V4H4v12h4" /></template>
                </svg>
                {{ copyState === 'copied' ? copy.copied : copyState === 'copying' ? copy.copying : copy.copy }}
              </button>
            </div>
            <pre class="reference-example" tabindex="0" :aria-label="copy.example"><code v-html="highlightedExample"></code></pre>
            <p class="reference-example-hint">{{ copy.exampleHint }}</p>
            <p class="reference-copy-status" :class="{ 'is-error': copyState === 'error' }" role="status" aria-live="polite" aria-atomic="true">
              {{ copyState === 'copied' ? copy.copied : copyState === 'error' ? copy.copyError : '' }}
            </p>
          </article>

          <div v-else :id="detailId" class="reference-empty">
            <h3>{{ copy.empty }}</h3>
            <p>{{ copy.emptyHint }}</p>
            <button type="button" @click="resetFilters">{{ copy.reset }}</button>
          </div>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.reference-backdrop { position: fixed; z-index: 100; inset: 0; display: grid; place-items: center; padding: 20px; background: rgba(4, 6, 10, .64); backdrop-filter: blur(3px); }
.reference { display: flex; flex-direction: column; width: min(940px, 100%); height: min(760px, 92dvh); min-height: 0; overflow: hidden; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: 14px; box-shadow: 0 24px 80px rgba(0, 0, 0, .45); }
.reference button, .reference input, .reference select { font: inherit; }
.reference button { color: inherit; cursor: pointer; }
.reference button:focus-visible, .reference input:focus-visible, .reference select:focus-visible, .reference [tabindex="0"]:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
.reference-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; padding: 20px 20px 16px; }
.reference-heading { display: flex; align-items: center; gap: 12px; min-width: 0; }
.reference-book { flex-shrink: 0; color: var(--accent); }
.reference-header h2 { margin: 0 0 5px; font-size: 1.16rem; letter-spacing: -.02em; }
.reference-header p { margin: 0; color: var(--text-dim); font-size: .77rem; line-height: 1.4; }
.reference-close { flex-shrink: 0; width: 34px; height: 34px; font-size: 1.4rem !important; line-height: 1; border: 1px solid var(--border); border-radius: 7px; background: transparent; }
.reference-close:hover, .reference-copy:hover, .reference-empty button:hover { background: var(--hover); }
.reference-toolbar { display: flex; align-items: center; gap: 12px; padding: 0 20px 16px; border-bottom: 1px solid var(--border); }
.reference-search { position: relative; display: flex; align-items: center; flex: 1; min-width: 0; }
.reference-search svg { position: absolute; left: 12px; color: var(--text-dim); pointer-events: none; }
.reference-search input { width: 100%; min-width: 0; height: 40px; padding: 8px 10px 8px 37px; color: var(--text); background: var(--surface-raised); border: 1px solid var(--border); border-radius: 8px; font-size: .8rem; }
.reference-search input::placeholder { color: var(--text-dim); }
.reference-languages { display: flex; flex-shrink: 0; padding: 3px; gap: 3px; background: var(--surface-raised); border: 1px solid var(--border); border-radius: 8px; }
.reference-languages button { min-height: 32px; padding: 5px 11px; border: 0; border-radius: 5px; color: var(--text-dim); background: transparent; font-size: .75rem; }
.reference-languages button[aria-pressed="true"] { color: var(--accent); background: color-mix(in srgb, var(--accent) 13%, var(--surface)); font-weight: 650; }
.reference-body { display: grid; grid-template-columns: 236px minmax(0, 1fr); flex: 1; min-height: 0; }
.reference-sidebar { display: flex; flex-direction: column; min-height: 0; border-right: 1px solid var(--border); background: var(--surface-raised); }
.reference-filter { padding: 15px 14px 10px; }
.reference-filter label { display: block; margin-bottom: 7px; color: var(--text-dim); font-size: .69rem; font-weight: 650; }
.reference-filter select { width: 100%; height: 35px; padding: 6px 8px; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: 6px; font-size: .76rem; }
.reference-count { display: block; margin-top: 11px; color: var(--text-dim); font-size: .65rem; }
.reference-list { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; padding: 0 7px 10px; }
.reference-list button { display: flex; flex-direction: column; gap: 4px; width: 100%; padding: 10px; border: 1px solid transparent; border-radius: 7px; background: transparent; text-align: left; }
.reference-list button:hover { background: var(--hover); }
.reference-list button[aria-pressed="true"] { background: color-mix(in srgb, var(--accent) 12%, var(--surface)); border-color: color-mix(in srgb, var(--accent) 28%, var(--border)); }
.reference-function-name { color: var(--text); font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: .79rem; font-weight: 650; overflow-wrap: anywhere; }
.reference-list button[aria-pressed="true"] .reference-function-name { color: var(--accent); }
.reference-function-summary { overflow: hidden; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; color: var(--text-dim); font-size: .67rem; line-height: 1.45; }
.reference-detail { min-width: 0; overflow-y: auto; overscroll-behavior: contain; padding: 23px 26px 16px; }
.reference-category-tag { color: var(--accent); font-size: .68rem; font-weight: 650; }
.reference-detail h3 { margin: 7px 0 9px; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 1.65rem; letter-spacing: -.045em; overflow-wrap: anywhere; }
.reference-summary { margin: 0 0 22px; font-size: .84rem; line-height: 1.6; color: var(--text-dim); }
.reference-detail h4 { margin: 20px 0 9px; font-size: .76rem; font-weight: 650; }
.reference-signature { margin: 0; padding: 12px 14px; border: 1px solid var(--border); border-radius: 7px; color: var(--accent); background: var(--surface-raised); font-size: .76rem; line-height: 1.6; white-space: pre-wrap; overflow-wrap: anywhere; }
.reference code { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
.reference-parameters { margin: 0; }
.reference-parameters > div { display: grid; grid-template-columns: minmax(80px, 26%) minmax(0, 1fr); gap: 14px; padding: 10px 0; border-top: 1px solid var(--border); font-size: .76rem; line-height: 1.5; }
.reference-parameters dt { color: var(--text); overflow-wrap: anywhere; }
.reference-parameters dd { margin: 0; color: var(--text-dim); overflow-wrap: anywhere; }
.reference-note { margin: 16px 0 20px; padding: 11px 13px; border-left: 2px solid var(--accent); border-radius: 0 6px 6px 0; background: color-mix(in srgb, var(--accent) 6%, var(--surface)); }
.reference-note h4 { margin: 0 0 5px; font-size: .72rem; }
.reference-note p { margin: 0; color: var(--text-dim); font-size: .75rem; line-height: 1.55; }
.reference-example-heading { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin: 20px 0 9px; }
.reference-example-heading h4 { margin: 0; }
.reference-example-heading > span { color: var(--text-dim); font-size: .63rem; }
.reference-copy { display: inline-flex; align-items: center; justify-content: center; gap: 6px; min-height: 32px; margin-left: auto; padding: 5px 9px; border: 1px solid var(--border); border-radius: 6px; background: var(--surface-raised); font-size: .69rem !important; }
.reference-copy:disabled { opacity: .6; cursor: wait; }
.reference-example { margin: 0; padding: 15px; overflow-x: auto; color: #eef0f5; background: var(--canvas-bg); border: 1px solid var(--border); border-radius: 8px; font-size: .75rem; line-height: 1.7; tab-size: 2; }
.reference-example-hint { margin: 9px 0 0; color: var(--text-dim); font-size: .68rem; line-height: 1.5; }
.reference-copy-status { min-height: 1.5em; margin: 5px 0 0; color: var(--accent); font-size: .71rem; line-height: 1.5; }
.reference-copy-status.is-error { color: var(--danger); }
.reference-example :deep(.syntax-comment) { color: #94a3b0; }
.reference-example :deep(.syntax-keyword) { color: #c792ea; }
.reference-example :deep(.syntax-string) { color: #9acb88; }
.reference-example :deep(.syntax-number) { color: #e8ac76; }
.reference-example :deep(.syntax-function) { color: #76c7df; }
.reference-example :deep(.syntax-property) { color: #d5c288; }
.reference-example :deep(.syntax-operator) { color: #b7bfea; }
.reference-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; min-width: 0; padding: 26px; text-align: center; }
.reference-empty h3 { margin: 0; font-size: 1rem; }
.reference-empty p { margin: 0; color: var(--text-dim); font-size: .8rem; line-height: 1.5; }
.reference-empty button { min-height: 36px; margin-top: 6px; padding: 7px 12px; border: 1px solid var(--border); border-radius: 7px; background: var(--surface-raised); font-size: .75rem; }
@media (max-width: 620px) {
  .reference-backdrop { padding: 8px; }
  .reference { height: 96dvh; border-radius: 11px; }
  .reference-header { padding: 14px 13px 12px; gap: 8px; }
  .reference-heading { gap: 9px; }
  .reference-book { width: 20px; }
  .reference-header h2 { font-size: 1rem; }
  .reference-header p { font-size: .68rem; }
  .reference-toolbar { flex-wrap: wrap; gap: 8px; padding: 0 13px 12px; }
  .reference-search { flex-basis: 100%; }
  .reference-languages { width: 100%; }
  .reference-languages button { flex: 1; }
  .reference-body { grid-template-columns: minmax(0, 1fr); grid-template-rows: 178px minmax(0, 1fr); }
  .reference-sidebar { border-right: 0; border-bottom: 1px solid var(--border); }
  .reference-filter { display: flex; align-items: center; gap: 10px; padding: 9px 12px 7px; }
  .reference-filter label { display: none; }
  .reference-filter select { flex: 1; min-width: 0; }
  .reference-count { flex-shrink: 0; margin: 0; }
  .reference-list { padding: 0 7px 6px; }
  .reference-list button { flex-direction: row; align-items: baseline; gap: 10px; padding: 7px 9px; }
  .reference-function-name { flex: 0 0 38%; font-size: .73rem; }
  .reference-function-summary { -webkit-line-clamp: 1; font-size: .66rem; }
  .reference-detail { padding: 17px 15px 12px; }
  .reference-detail h3 { font-size: 1.4rem; }
  .reference-summary { margin-bottom: 16px; font-size: .78rem; }
  .reference-parameters > div { gap: 10px; font-size: .72rem; }
  .reference-example { padding: 12px; font-size: .69rem; }
  .reference-empty { padding: 18px; }
}
@media (max-height: 620px) and (max-width: 620px) {
  .reference-header p { display: none; }
  .reference-body { grid-template-rows: 110px minmax(0, 1fr); }
}
@media (pointer: coarse) {
  .reference-close { width: 44px; height: 44px; }
  .reference-languages button, .reference-copy, .reference-list button, .reference-empty button { min-height: 44px; }
}
</style>
