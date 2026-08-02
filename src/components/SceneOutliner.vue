<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onBeforeUpdate, ref, watch } from 'vue'
import type {
  MeshKey,
  PanelLocale,
  SceneMeshRow,
  SourceProvenanceRow,
} from './cadPanels.types'

const props = withDefaults(defineProps<{
  meshes: readonly SceneMeshRow[]
  selectedMeshId?: MeshKey | null
  hoveredMeshId?: MeshKey | null
  locale?: PanelLocale
  busy?: boolean
}>(), {
  selectedMeshId: null,
  hoveredMeshId: null,
  locale: 'en',
  busy: false,
})

const emit = defineEmits<{
  select: [meshId: MeshKey]
  'clear-selection': []
  'toggle-visibility': [meshId: MeshKey, visible: boolean]
  preselect: [meshId: MeshKey | null]
  focus: [meshId: MeshKey]
  isolate: [meshId: MeshKey]
  'reveal-source': [source: SourceProvenanceRow, meshId: MeshKey]
  'highlight-source': [sourceId: number | null]
}>()

const copy = {
  en: {
    title: 'Scene', meshes: 'Meshes', search: 'Search scene', noObjects: 'No meshes yet',
    noResults: 'No matching objects', show: 'Show', hide: 'Hide', focus: 'Focus selection',
    isolate: 'Isolate selection', sources: 'Source features', source: 'Source', triangles: 'tri',
    collapse: 'Collapse all source features', expand: 'Expand all source features',
    clearSearch: 'Clear search', selectionHelp: 'Enter select · / focus · . isolate · H visibility',
    locked: 'Locked', disabled: 'Disabled', line: 'line',
  },
  ru: {
    title: 'Сцена', meshes: 'Меши', search: 'Поиск по сцене', noObjects: 'Мешей пока нет',
    noResults: 'Ничего не найдено', show: 'Показать', hide: 'Скрыть', focus: 'Фокус на выбранном',
    isolate: 'Изолировать выбранное', sources: 'Исходные операции', source: 'Исходник', triangles: 'тр.',
    collapse: 'Свернуть исходные операции', expand: 'Развернуть исходные операции',
    clearSearch: 'Очистить поиск', selectionHelp: 'Enter выбрать · / фокус · . изолировать · H видимость',
    locked: 'Заблокирован', disabled: 'Отключён', line: 'строка',
  },
} as const

const text = computed(() => copy[props.locale])
const query = ref('')
const searchOpen = ref(false)
const expandedIds = ref<Set<MeshKey>>(new Set())
const panelRef = ref<HTMLElement | null>(null)
const searchRef = ref<HTMLInputElement | null>(null)
const rowRefs = ref<HTMLButtonElement[]>([])
const activeMeshId = ref<MeshKey | null>(null)
const hoveredSourceId = ref<number | null>(null)
const focusedSourceId = ref<number | null>(null)

onBeforeUnmount(clearActiveSourceHighlight)
watch(query, clearActiveSourceHighlight)
watch(() => props.meshes, meshes => {
  const availableSourceIds = new Set(
    meshes.flatMap(mesh => (mesh.sources ?? []).map(source => source.sourceId)),
  )
  if ((hoveredSourceId.value !== null && !availableSourceIds.has(hoveredSourceId.value))
      || (focusedSourceId.value !== null && !availableSourceIds.has(focusedSourceId.value))) {
    clearActiveSourceHighlight()
  } else if (hoveredSourceId.value !== null || focusedSourceId.value !== null) {
    // setMeshes() replaces GPU buffers, so a still-active DOM row must restore
    // its overlay even though pointerenter/focus does not fire again.
    emitActiveSourceHighlight()
  }
})

const filteredMeshes = computed(() => {
  const needle = normalize(query.value.trim())
  if (!needle) return props.meshes
  return props.meshes.filter(mesh => normalize([
    mesh.name,
    ...(mesh.sources ?? []).map(source => source.label ?? ''),
  ].join(' ')).includes(needle))
})

const filteredHasSelection = computed(() => filteredMeshes.value.some(mesh => isSelected(mesh)))

watch(filteredMeshes, meshes => {
  if (meshes.some(mesh => mesh.id === activeMeshId.value)) return
  activeMeshId.value = meshes.find(mesh => isSelected(mesh))?.id ?? meshes[0]?.id ?? null
}, { immediate: true })

const allExpanded = computed(() => {
  const expandable = filteredMeshes.value.filter(mesh => mesh.sources?.length)
  return expandable.length > 0 && expandable.every(mesh => expandedIds.value.has(mesh.id))
})

onBeforeUpdate(() => { rowRefs.value = [] })

function normalize(value: string) {
  return value.normalize('NFKD').toLocaleLowerCase(props.locale)
}

function registerRow(element: unknown) {
  if (element instanceof HTMLButtonElement) rowRefs.value.push(element)
}

function isSelected(mesh: SceneMeshRow) {
  return props.selectedMeshId === mesh.id
}

function rowTabIndex(mesh: SceneMeshRow, index: number) {
  if (activeMeshId.value !== null) return activeMeshId.value === mesh.id ? 0 : -1
  return isSelected(mesh) || (!filteredHasSelection.value && index === 0) ? 0 : -1
}

function toggleExpanded(meshId: MeshKey) {
  const next = new Set(expandedIds.value)
  if (next.has(meshId)) {
    next.delete(meshId)
    clearActiveSourceHighlight()
  }
  else next.add(meshId)
  expandedIds.value = next
}

function toggleAllExpanded() {
  if (allExpanded.value) {
    expandedIds.value = new Set()
    clearActiveSourceHighlight()
    return
  }
  expandedIds.value = new Set(
    filteredMeshes.value.filter(mesh => mesh.sources?.length).map(mesh => mesh.id),
  )
}

// Single-select only: multi-select needs a renderer selection-model change first (recommendation.md P2).
function selectMesh(mesh: SceneMeshRow) {
  if (mesh.locked || mesh.disabled) return
  emit('select', mesh.id)
}

function focusRow(index: number) {
  const rows = rowRefs.value
  if (!rows.length) return
  const nextIndex = Math.max(0, Math.min(rows.length - 1, index))
  activeMeshId.value = filteredMeshes.value[nextIndex]?.id ?? null
  rows[nextIndex]?.focus()
}

function handleRowKeydown(event: KeyboardEvent, mesh: SceneMeshRow, index: number) {
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    focusRow(index + 1)
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    focusRow(index - 1)
  } else if (event.key === 'Home') {
    event.preventDefault()
    focusRow(0)
  } else if (event.key === 'End') {
    event.preventDefault()
    focusRow(rowRefs.value.length - 1)
  } else if (event.key === 'ArrowRight' && mesh.sources?.length) {
    event.preventDefault()
    if (!expandedIds.value.has(mesh.id)) toggleExpanded(mesh.id)
  } else if (event.key === 'ArrowLeft' && expandedIds.value.has(mesh.id)) {
    event.preventDefault()
    toggleExpanded(mesh.id)
  } else if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    selectMesh(mesh)
  }
}

function handlePanelKeydown(event: KeyboardEvent) {
  const target = event.target
  const typing = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement
  if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase() === 'f') {
    event.preventDefault()
    openSearch()
  } else if (event.key === 'Escape' && searchOpen.value) {
    event.preventDefault()
    if (query.value) query.value = ''
    else closeSearch()
  } else if (event.key === 'Escape' && !typing) {
    event.preventDefault()
    emit('clear-selection')
  } else if (!typing && props.selectedMeshId !== null) {
    if (event.key === '/') {
      event.preventDefault()
      emit('focus', props.selectedMeshId)
    } else if (event.key === '.') {
      event.preventDefault()
      emit('isolate', props.selectedMeshId)
    } else if (event.key.toLocaleLowerCase() === 'h') {
      const mesh = props.meshes.find(candidate => candidate.id === props.selectedMeshId)
      if (mesh) {
        event.preventDefault()
        emit('toggle-visibility', mesh.id, !mesh.visible)
      }
    }
  }
}

async function openSearch() {
  searchOpen.value = true
  await nextTick()
  searchRef.value?.focus()
  searchRef.value?.select()
}

function closeSearch() {
  query.value = ''
  searchOpen.value = false
  panelRef.value?.focus({ preventScroll: true })
}

function formatCount(value: number) {
  return value.toLocaleString(props.locale)
}

function colorStyle(color: SceneMeshRow['color'] | SourceProvenanceRow['color']) {
  if (!color) return undefined
  if (typeof color === 'string') return { backgroundColor: color }
  const maximum = Math.max(color[0], color[1], color[2])
  const scale = maximum <= 1 ? 255 : 1
  const red = Math.round(color[0] * scale)
  const green = Math.round(color[1] * scale)
  const blue = Math.round(color[2] * scale)
  const alpha = color[3] === undefined ? 1 : Math.min(1, color[3] > 1 ? color[3] / 255 : color[3])
  return { backgroundColor: `rgba(${red}, ${green}, ${blue}, ${alpha})` }
}

function sourceLocation(source: SourceProvenanceRow) {
  if (source.line !== undefined) {
    return `${text.value.line} ${source.line}${source.column === undefined ? '' : `:${source.column}`}`
  }
  return `${source.sourceStart}–${source.sourceEnd}`
}

function emitActiveSourceHighlight() {
  emit('highlight-source', hoveredSourceId.value ?? focusedSourceId.value)
}

function clearActiveSourceHighlight() {
  hoveredSourceId.value = null
  focusedSourceId.value = null
  emit('highlight-source', null)
}

function hoverSource(source: SourceProvenanceRow) {
  hoveredSourceId.value = source.sourceId
  emit('preselect', null)
  emitActiveSourceHighlight()
}

function leaveSource(meshId: MeshKey) {
  hoveredSourceId.value = null
  emitActiveSourceHighlight()
  if (focusedSourceId.value === null) emit('preselect', meshId)
}

function focusSource(source: SourceProvenanceRow) {
  focusedSourceId.value = source.sourceId
  emit('preselect', null)
  emitActiveSourceHighlight()
}

function blurSource(meshId: MeshKey, source: SourceProvenanceRow) {
  if (focusedSourceId.value === source.sourceId) focusedSourceId.value = null
  emitActiveSourceHighlight()
  if (hoveredSourceId.value === null) emit('preselect', meshId)
}
</script>

<template>
  <aside
    ref="panelRef"
    class="outliner"
    tabindex="-1"
    :aria-label="text.title"
    aria-describedby="scene-selection-help"
    @keydown="handlePanelKeydown"
  >
    <header class="panel-header">
      <div class="title-wrap">
        <h2>{{ text.title }}</h2>
        <span class="count" aria-hidden="true">{{ meshes.length }}</span>
      </div>
      <div class="header-actions">
        <button
          class="icon-button"
          type="button"
          :title="allExpanded ? text.collapse : text.expand"
          :aria-label="allExpanded ? text.collapse : text.expand"
          :aria-pressed="allExpanded"
          @click="toggleAllExpanded"
        >
          <svg width="15" height="15" viewBox="0 0 20 20" aria-hidden="true">
            <path d="m5 7 5 5 5-5" />
            <path d="m5 3 5 5 5-5" />
          </svg>
        </button>
        <button
          class="icon-button"
          type="button"
          :title="text.search"
          :aria-label="text.search"
          :aria-expanded="searchOpen"
          @click="searchOpen ? closeSearch() : openSearch()"
        >
          <svg width="15" height="15" viewBox="0 0 20 20" aria-hidden="true">
            <circle cx="8.5" cy="8.5" r="5" />
            <path d="m12.5 12.5 4 4" />
          </svg>
        </button>
      </div>
    </header>

    <div v-if="searchOpen" class="search-wrap">
      <svg width="14" height="14" viewBox="0 0 20 20" aria-hidden="true">
        <circle cx="8.5" cy="8.5" r="5" />
        <path d="m12.5 12.5 4 4" />
      </svg>
      <input
        ref="searchRef"
        v-model="query"
        type="search"
        autocomplete="off"
        :placeholder="text.search"
        :aria-label="text.search"
      >
      <button v-if="query" type="button" :aria-label="text.clearSearch" @click="query = ''">×</button>
      <kbd v-else>⌘F</kbd>
    </div>

    <div class="section-label">
      <span class="chevron" aria-hidden="true">⌄</span>
      <span>{{ text.meshes }}</span>
      <span class="section-count">{{ filteredMeshes.length }}</span>
    </div>

    <div class="mesh-list" role="tree" :aria-label="text.meshes" :aria-busy="busy" aria-multiselectable="false">
      <div
        v-for="(mesh, index) in filteredMeshes"
        :key="mesh.id"
        class="mesh-item"
        :class="{
          selected: isSelected(mesh),
          preselected: hoveredMeshId === mesh.id && !isSelected(mesh),
          muted: !mesh.visible || mesh.disabled,
        }"
        @pointerenter="emit('preselect', mesh.id)"
        @pointerleave="emit('preselect', null)"
      >
        <div class="mesh-row">
          <button
            v-if="mesh.sources?.length"
            class="disclosure"
            type="button"
            :aria-label="expandedIds.has(mesh.id) ? text.collapse : text.expand"
            :aria-expanded="expandedIds.has(mesh.id)"
            @click="toggleExpanded(mesh.id)"
          >
            <svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true">
              <path :d="expandedIds.has(mesh.id) ? 'm2.5 4 3.5 3.5L9.5 4' : 'm4 2.5 3.5 3.5L4 9.5'" />
            </svg>
          </button>
          <span v-else class="disclosure-spacer" />

          <button
            class="visibility-button"
            type="button"
            :title="mesh.visible ? text.hide : text.show"
            :aria-label="`${mesh.visible ? text.hide : text.show}: ${mesh.name}`"
            :aria-pressed="mesh.visible"
            @click="emit('toggle-visibility', mesh.id, !mesh.visible)"
          >
            <svg v-if="mesh.visible" width="14" height="14" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M2.5 10s2.7-4.5 7.5-4.5 7.5 4.5 7.5 4.5-2.7 4.5-7.5 4.5S2.5 10 2.5 10Z" />
              <circle cx="10" cy="10" r="2.2" />
            </svg>
            <svg v-else width="14" height="14" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M3 3 17 17M8.1 5.8A8.8 8.8 0 0 1 10 5.5c4.8 0 7.5 4.5 7.5 4.5a12.6 12.6 0 0 1-2 2.5M11.9 14.2a8.8 8.8 0 0 1-1.9.3C5.2 14.5 2.5 10 2.5 10a12.7 12.7 0 0 1 2-2.5" />
            </svg>
          </button>

          <span class="material-dot" :style="colorStyle(mesh.color)" aria-hidden="true" />

          <button
            :ref="registerRow"
            class="mesh-name"
            type="button"
            :tabindex="rowTabIndex(mesh, index)"
            role="treeitem"
            :aria-level="1"
            :aria-posinset="index + 1"
            :aria-setsize="filteredMeshes.length"
            :aria-selected="isSelected(mesh)"
            :aria-expanded="mesh.sources?.length ? expandedIds.has(mesh.id) : undefined"
            :aria-disabled="mesh.locked || mesh.disabled || undefined"
            @focus="activeMeshId = mesh.id"
            @click="selectMesh(mesh)"
            @dblclick="emit('focus', mesh.id)"
            @keydown="handleRowKeydown($event, mesh, index)"
          >
            <svg class="mesh-icon" width="15" height="15" viewBox="0 0 20 20" aria-hidden="true">
              <path d="m10 2.5 6.5 3.7v7.6L10 17.5l-6.5-3.7V6.2L10 2.5Z" />
              <path d="m3.5 6.2 6.5 3.7 6.5-3.7M10 9.9v7.6" />
            </svg>
            <span class="mesh-label" :title="mesh.name">{{ mesh.name }}</span>
            <span v-if="mesh.locked" class="state-glyph" :title="text.locked" aria-hidden="true">⌑</span>
            <span v-if="mesh.disabled" class="state-glyph" :title="text.disabled" aria-hidden="true">⊘</span>
            <span class="triangle-count">{{ formatCount(mesh.triangleCount) }} {{ text.triangles }}</span>
          </button>
        </div>

        <ul v-if="mesh.sources?.length && expandedIds.has(mesh.id)" class="source-list" role="group" :aria-label="text.sources">
          <li v-for="(source, sourceIndex) in mesh.sources" :key="source.id" role="none">
            <button
              type="button"
              role="treeitem"
              aria-level="2"
              :aria-posinset="sourceIndex + 1"
              :aria-setsize="mesh.sources.length"
              @pointerenter="hoverSource(source)"
              @pointerleave="leaveSource(mesh.id)"
              @focus="focusSource(source)"
              @blur="blurSource(mesh.id, source)"
              @click="emit('reveal-source', source, mesh.id)"
            >
              <span class="source-rail" aria-hidden="true" />
              <span class="material-dot source-dot" :style="colorStyle(source.color ?? mesh.color)" aria-hidden="true" />
              <svg width="13" height="13" viewBox="0 0 20 20" aria-hidden="true">
                <path d="m7 5-4 5 4 5M13 5l4 5-4 5M11.5 3l-3 14" />
              </svg>
              <span class="source-copy">
                <span>{{ source.label || `${text.source} ${source.originalId ?? source.id}` }}</span>
                <small>{{ sourceLocation(source) }}</small>
              </span>
              <span v-if="source.triangleCount !== undefined" class="source-triangles">
                {{ formatCount(source.triangleCount) }}
              </span>
            </button>
          </li>
        </ul>
      </div>

      <p v-if="!meshes.length" class="empty-state">{{ text.noObjects }}</p>
      <p v-else-if="!filteredMeshes.length" class="empty-state">{{ text.noResults }}</p>
    </div>

    <footer id="scene-selection-help" class="panel-footer">{{ text.selectionHelp }}</footer>
  </aside>
</template>

<style scoped>
.outliner {
  width: min(304px, 100%);
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: var(--text, #eef0f5);
  background: color-mix(in srgb, var(--surface, #1a1c22) 96%, transparent);
  border: 1px solid var(--border, #30343e);
  border-radius: 9px;
  box-shadow: 0 12px 34px rgba(0, 0, 0, 0.24);
  outline: none;
}

.panel-header,
.section-label,
.mesh-row,
.search-wrap,
.panel-footer { flex: 0 0 auto; }

.panel-header {
  min-height: 38px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 5px 7px 5px 10px;
  background: color-mix(in srgb, var(--surface-raised, #22252d) 66%, var(--surface, #1a1c22));
  border-bottom: 1px solid var(--border, #30343e);
}

.title-wrap,
.header-actions,
.search-wrap,
.section-label,
.mesh-row,
.mesh-name { display: flex; align-items: center; }

.title-wrap { min-width: 0; gap: 7px; }
.title-wrap h2 { margin: 0; font-size: 0.78rem; font-weight: 720; letter-spacing: 0.01em; }
.count,
.section-count {
  color: var(--text-dim, #a4a9b5);
  font: 650 0.62rem/1 ui-monospace, monospace;
}

.count {
  min-width: 18px;
  padding: 3px 5px;
  text-align: center;
  background: var(--surface, #1a1c22);
  border: 1px solid var(--border, #30343e);
  border-radius: 999px;
}

.header-actions { gap: 2px; }
button { color: inherit; font: inherit; }
.icon-button,
.disclosure,
.visibility-button {
  display: inline-grid;
  place-items: center;
  padding: 0;
  color: var(--text-dim, #a4a9b5);
  background: transparent;
  border: 0;
  border-radius: 5px;
  cursor: pointer;
}

.icon-button { width: 27px; height: 27px; }
.icon-button svg,
.search-wrap svg,
.disclosure svg,
.visibility-button svg,
.mesh-icon,
.source-list svg { fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.45; }
.icon-button:hover,
.disclosure:hover,
.visibility-button:hover { color: var(--text, #eef0f5); background: var(--hover, #2a2e38); }

.search-wrap {
  gap: 6px;
  min-height: 36px;
  padding: 5px 7px 5px 10px;
  color: var(--text-dim, #a4a9b5);
  border-bottom: 1px solid var(--border, #30343e);
}

.search-wrap > svg { flex: 0 0 auto; }
.search-wrap input {
  min-width: 0;
  flex: 1;
  padding: 3px 0;
  color: var(--text, #eef0f5);
  background: transparent;
  border: 0;
  outline: 0;
  font-size: 0.72rem;
}
.search-wrap input::placeholder { color: var(--text-dim, #a4a9b5); }
.search-wrap input::-webkit-search-cancel-button { display: none; }
.search-wrap button {
  width: 22px;
  height: 22px;
  padding: 0;
  color: var(--text-dim, #a4a9b5);
  background: transparent;
  border: 0;
  border-radius: 4px;
  cursor: pointer;
}
.search-wrap kbd {
  padding: 2px 4px;
  color: var(--text-dim, #a4a9b5);
  background: var(--surface-raised, #22252d);
  border: 1px solid var(--border, #30343e);
  border-radius: 4px;
  font: 0.56rem/1 ui-monospace, monospace;
}

.section-label {
  gap: 5px;
  min-height: 27px;
  padding: 4px 9px;
  color: var(--text-dim, #a4a9b5);
  background: color-mix(in srgb, var(--surface-raised, #22252d) 35%, transparent);
  border-bottom: 1px solid color-mix(in srgb, var(--border, #30343e) 72%, transparent);
  font-size: 0.64rem;
  font-weight: 700;
  letter-spacing: 0.055em;
  text-transform: uppercase;
}
.chevron { color: var(--text, #eef0f5); font-size: 0.76rem; }
.section-count { margin-left: auto; }

.mesh-list {
  min-height: 76px;
  flex: 1;
  overflow: auto;
  padding-block: 3px;
  scrollbar-width: thin;
}

.mesh-item { border-inline: 2px solid transparent; }
.mesh-item.selected {
  background: color-mix(in srgb, var(--accent, #559dff) 17%, transparent);
  border-left-color: var(--accent, #559dff);
}
.mesh-item.preselected { background: color-mix(in srgb, var(--accent, #559dff) 8%, var(--hover, #2a2e38)); }
.mesh-item.muted { opacity: 0.52; }
.mesh-item.muted.selected { opacity: 0.78; }

.mesh-row { min-height: 33px; padding: 2px 5px 2px 3px; }
.disclosure { width: 22px; height: 27px; flex: 0 0 22px; }
.disclosure-spacer { width: 22px; flex: 0 0 22px; }
.visibility-button { width: 25px; height: 27px; flex: 0 0 25px; }

.material-dot {
  width: 9px;
  height: 9px;
  flex: 0 0 9px;
  margin-inline: 3px 5px;
  background: var(--accent, #559dff);
  border: 1px solid color-mix(in srgb, white 55%, transparent);
  border-radius: 50%;
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.28);
}

.mesh-name {
  min-width: 0;
  min-height: 27px;
  flex: 1;
  gap: 5px;
  padding: 2px 4px;
  text-align: left;
  background: transparent;
  border: 0;
  border-radius: 5px;
  cursor: default;
}
.mesh-name:hover { background: color-mix(in srgb, var(--hover, #2a2e38) 75%, transparent); }
.mesh-name:focus-visible,
.icon-button:focus-visible,
.disclosure:focus-visible,
.visibility-button:focus-visible,
.search-wrap button:focus-visible,
.source-list button:focus-visible {
  outline: 2px solid var(--focus, #8ec1ff);
  outline-offset: -1px;
}
.mesh-icon { flex: 0 0 auto; color: color-mix(in srgb, var(--accent, #559dff) 72%, var(--text, #eef0f5)); }
.mesh-label {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  color: var(--text, #eef0f5);
  font-size: 0.72rem;
  font-weight: 570;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.selected .mesh-label { font-weight: 700; }
.state-glyph { color: var(--text-dim, #a4a9b5); font-size: 0.68rem; }
.triangle-count,
.source-triangles {
  flex: 0 0 auto;
  color: var(--text-dim, #a4a9b5);
  font: 0.57rem/1 ui-monospace, monospace;
}

.source-list { margin: 0; padding: 0 4px 3px 33px; list-style: none; }
.source-list li { position: relative; }
.source-list button {
  position: relative;
  width: 100%;
  min-height: 31px;
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 3px 5px 3px 8px;
  text-align: left;
  background: transparent;
  border: 0;
  border-radius: 5px;
  cursor: pointer;
}
.source-list button:hover { background: var(--hover, #2a2e38); }
.source-list svg { flex: 0 0 auto; color: var(--text-dim, #a4a9b5); }
.source-rail {
  position: absolute;
  left: -12px;
  top: -7px;
  width: 12px;
  height: 23px;
  border-left: 1px solid color-mix(in srgb, var(--border, #30343e) 85%, transparent);
  border-bottom: 1px solid color-mix(in srgb, var(--border, #30343e) 85%, transparent);
  border-bottom-left-radius: 5px;
}
.source-dot { width: 7px; height: 7px; flex-basis: 7px; margin: 0 1px 0 0; opacity: 0.8; }
.source-copy { min-width: 0; flex: 1; display: grid; gap: 1px; }
.source-copy > span {
  overflow: hidden;
  color: var(--text, #eef0f5);
  font-size: 0.65rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.source-copy small { color: var(--text-dim, #a4a9b5); font: 0.55rem/1.2 ui-monospace, monospace; }

.empty-state {
  margin: 0;
  padding: 26px 14px;
  color: var(--text-dim, #a4a9b5);
  font-size: 0.7rem;
  text-align: center;
}
.panel-footer {
  min-height: 25px;
  padding: 6px 9px;
  overflow: hidden;
  color: var(--text-dim, #a4a9b5);
  background: color-mix(in srgb, var(--surface-raised, #22252d) 45%, transparent);
  border-top: 1px solid var(--border, #30343e);
  font-size: 0.56rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 680px) {
  .outliner { width: 100%; max-height: 42dvh; border-radius: 8px; }
  .panel-footer { display: none; }
}

@media (pointer: coarse) {
  .disclosure, .visibility-button, .icon-button, .search-wrap button { min-width: 44px; min-height: 44px; }
  .mesh-name, .source-list button { min-height: 44px; }
}

@media (prefers-reduced-motion: reduce) {
  * { scroll-behavior: auto !important; }
}
</style>
