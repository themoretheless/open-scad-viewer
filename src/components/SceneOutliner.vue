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
  if (hoveredSourceId.value === null && focusedSourceId.value === null) return
  const availableSourceIds = new Set<number>()
  for (const mesh of meshes) {
    if (!mesh.sources) continue
    for (const source of mesh.sources) availableSourceIds.add(source.sourceId)
  }
  if ((hoveredSourceId.value !== null && !availableSourceIds.has(hoveredSourceId.value))
      || (focusedSourceId.value !== null && !availableSourceIds.has(focusedSourceId.value))) {
    clearActiveSourceHighlight()
  } else {
    // setMeshes() replaces GPU buffers, so a still-active DOM row must restore
    // its overlay even though pointerenter/focus does not fire again.
    emitActiveSourceHighlight()
  }
})

const filteredMeshes = computed(() => {
  const needle = normalize(query.value.trim())
  if (!needle) return props.meshes
  return props.meshes.filter(mesh => {
    if (normalize(mesh.name).includes(needle)) return true
    if (!mesh.sources?.length) return false
    const parts = [mesh.name]
    for (const source of mesh.sources) if (source.label) parts.push(source.label)
    return normalize(parts.join(' ')).includes(needle)
  })
})

const filteredHasSelection = computed(() => filteredMeshes.value.some(mesh => isSelected(mesh)))

watch(filteredMeshes, meshes => {
  if (meshes.some(mesh => mesh.id === activeMeshId.value)) return
  activeMeshId.value = meshes.find(mesh => isSelected(mesh))?.id ?? meshes[0]?.id ?? null
}, { immediate: true })

const allExpanded = computed(() => {
  let hasExpandable = false
  const expanded = expandedIds.value
  for (const mesh of filteredMeshes.value) {
    if (!mesh.sources?.length) continue
    hasExpandable = true
    if (!expanded.has(mesh.id)) return false
  }
  return hasExpandable
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

const countFormatterCache = new Map<string, Intl.NumberFormat>()
function formatCount(value: number) {
  let fmt = countFormatterCache.get(props.locale)
  if (!fmt) {
    fmt = new Intl.NumberFormat(props.locale)
    countFormatterCache.set(props.locale, fmt)
  }
  return fmt.format(value)
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

<style scoped src="./SceneOutliner.css"></style>
