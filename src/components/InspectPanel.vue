<script setup lang="ts">
import { computed, useId } from 'vue'
import type {
  DistanceMeasurement,
  InspectSelection,
  PanelLocale,
  SectionAxis,
  SourceProvenanceRow,
  Vector3,
} from './cadPanels.types'

const props = withDefaults(defineProps<{
  selection?: InspectSelection | null
  measurement?: DistanceMeasurement | null
  measureActive?: boolean
  sectionEnabled?: boolean
  sectionAxis?: SectionAxis
  sectionOffset?: number
  sectionMin?: number
  sectionMax?: number
  sectionStep?: number
  sectionFlip?: boolean
  sourceRevealEnabled?: boolean
  locale?: PanelLocale
}>(), {
  selection: null,
  measurement: null,
  measureActive: false,
  sectionEnabled: false,
  sectionAxis: 'z',
  sectionOffset: 0,
  sectionMin: -100,
  sectionMax: 100,
  sectionStep: 0.1,
  sectionFlip: false,
  sourceRevealEnabled: true,
  locale: 'en',
})

const emit = defineEmits<{
  'start-measure': []
  'cancel-measure': []
  'clear-measurement': []
  'copy-measurement': [measurement: DistanceMeasurement]
  'reveal-source': [source: SourceProvenanceRow]
  'update:section-enabled': [enabled: boolean]
  'update:section-axis': [axis: SectionAxis]
  'update:section-offset': [offset: number]
  'update:section-flip': [flip: boolean]
  'reset-section': []
}>()

const copy = {
  en: {
    title: 'Inspect', selection: 'Selection', noSelection: 'Select a surface to inspect it',
    object: 'Object', position: 'Hit point', normal: 'Surface normal', bounds: 'Bounds',
    size: 'Size', triangles: 'Triangles', area: 'Surface', volume: 'Volume', source: 'Source feature',
    reveal: 'Reveal in source', measurement: 'Measure', measureDistance: 'Measure distance',
    measuring: 'Pick two points in the viewport', pointA: 'Point A', pointB: 'Point B',
    waiting: 'Waiting for point', distance: 'Distance', delta: 'Axis delta', clear: 'Clear', copy: 'Copy',
    section: 'Section analysis', enableSection: 'Enable virtual section', plane: 'Plane normal',
    offset: 'Distance', flip: 'Flip', reset: 'Reset', clippedSide: 'Clipped side',
    positive: 'Positive', negative: 'Negative', keyboard: 'Ctrl+= measure · Shift+F flip',
    line: 'line', characters: 'chars', units: 'units',
  },
  ru: {
    title: 'Инспектор', selection: 'Выбор', noSelection: 'Выберите поверхность для анализа',
    object: 'Объект', position: 'Точка попадания', normal: 'Нормаль поверхности', bounds: 'Границы',
    size: 'Размер', triangles: 'Треугольники', area: 'Площадь', volume: 'Объём', source: 'Операция исходника',
    reveal: 'Показать в исходнике', measurement: 'Измерение', measureDistance: 'Измерить расстояние',
    measuring: 'Укажите две точки во вьюпорте', pointA: 'Точка A', pointB: 'Точка B',
    waiting: 'Ожидание точки', distance: 'Расстояние', delta: 'По осям', clear: 'Очистить', copy: 'Копировать',
    section: 'Анализ сечения', enableSection: 'Включить виртуальное сечение', plane: 'Нормаль плоскости',
    offset: 'Расстояние', flip: 'Развернуть', reset: 'Сбросить', clippedSide: 'Скрытая сторона',
    positive: 'Положительная', negative: 'Отрицательная', keyboard: 'Ctrl+= измерить · Shift+F разворот',
    line: 'строка', characters: 'симв.', units: 'ед.',
  },
} as const

const text = computed(() => copy[props.locale])
const instanceId = useId()
const sectionToggleId = `${instanceId}-section-toggle`
const sectionOffsetId = `${instanceId}-section-offset`
const sectionNumberId = `${instanceId}-section-number`

const pointA = computed(() => props.measurement?.points[0])
const pointB = computed(() => props.measurement?.points[1])
const measurementDelta = computed<Vector3 | null>(() => {
  if (!pointA.value || !pointB.value) return null
  return [
    pointB.value[0] - pointA.value[0],
    pointB.value[1] - pointA.value[1],
    pointB.value[2] - pointA.value[2],
  ]
})
const measurementDistance = computed(() => {
  if (props.measurement?.distance !== undefined) return props.measurement.distance
  const delta = measurementDelta.value
  return delta ? Math.hypot(delta[0], delta[1], delta[2]) : null
})
const selectionSize = computed<Vector3 | null>(() => {
  const bounds = props.selection?.bounds
  if (!bounds) return null
  return [
    bounds.max[0] - bounds.min[0],
    bounds.max[1] - bounds.min[1],
    bounds.max[2] - bounds.min[2],
  ]
})
const safeSectionMin = computed(() => Math.min(props.sectionMin, props.sectionMax))
const safeSectionMax = computed(() => Math.max(props.sectionMin, props.sectionMax))

function toggleMeasure() {
  if (props.measureActive) emit('cancel-measure')
  else emit('start-measure')
}

function updateOffset(rawValue: string | number) {
  const numeric = typeof rawValue === 'number' ? rawValue : Number(rawValue)
  if (!Number.isFinite(numeric)) return
  emit('update:section-offset', Math.max(safeSectionMin.value, Math.min(safeSectionMax.value, numeric)))
}

const MAX_NUMBER_FORMAT_CACHE_ENTRIES = 32
const numberFormatCache = new Map<string, Intl.NumberFormat>()
function getNumberFormatter(locale: string, digits: number): Intl.NumberFormat {
  const key = `${locale}:${digits}`
  let formatter = numberFormatCache.get(key)
  if (!formatter) {
    if (numberFormatCache.size >= MAX_NUMBER_FORMAT_CACHE_ENTRIES) {
      const oldestKey = numberFormatCache.keys().next().value
      if (oldestKey !== undefined) numberFormatCache.delete(oldestKey)
    }
    formatter = new Intl.NumberFormat(locale, { maximumFractionDigits: digits })
    numberFormatCache.set(key, formatter)
  }
  return formatter
}

function formatNumber(value: number, digits = 3) {
  return getNumberFormatter(props.locale, digits).format(value)
}

function formatSiUnit(value: number, kind: 'length' | 'area' | 'volume', digits = 3) {
  const fmt = getNumberFormatter(props.locale, digits)
  if (!Number.isFinite(value)) return '—'
  const abs = Math.abs(value)
  if (kind === 'length') {
    if (abs >= 1000) return `${fmt.format(value / 1000)} m`
    if (abs > 0 && abs < 0.1) return `${fmt.format(value * 1000)} µm`
    return `${fmt.format(value)} mm`
  }
  if (kind === 'area') {
    if (abs >= 1_000_000) return `${fmt.format(value / 1_000_000)} m²`
    if (abs >= 1_000) return `${fmt.format(value / 100)} cm²`
    return `${fmt.format(value)} mm²`
  }
  if (abs >= 1_000_000_000) return `${fmt.format(value / 1_000_000_000)} m³`
  if (abs >= 10_000) return `${fmt.format(value / 1000)} cm³`
  return `${fmt.format(value)} mm³`
}

function formatVector(vector: Vector3 | undefined | null) {
  if (!vector) return '—'
  const fmt = getNumberFormatter(props.locale, 3)
  return `${fmt.format(vector[0])}  ·  ${fmt.format(vector[1])}  ·  ${fmt.format(vector[2])}`
}

function sourceLocation(source: SourceProvenanceRow) {
  if (source.line !== undefined) {
    return `${text.value.line} ${source.line}${source.column === undefined ? '' : `:${source.column}`}`
  }
  return `${source.sourceEnd - source.sourceStart} ${text.value.characters}`
}
</script>

<template>
  <aside class="inspect-panel" :aria-label="text.title">
    <header class="panel-header">
      <h2>{{ text.title }}</h2>
      <svg width="16" height="16" viewBox="0 0 20 20" aria-hidden="true">
        <circle cx="10" cy="10" r="7" />
        <path d="M10 9v5M10 6.3v.2" />
      </svg>
    </header>

    <div class="panel-body">
      <details class="inspect-section" open>
        <summary>
          <span>{{ text.selection }}</span>
          <span v-if="selection" class="summary-value">{{ selection.meshName }}</span>
        </summary>
        <div v-if="selection" class="section-content selection-content">
          <dl class="property-grid">
            <div>
              <dt>{{ text.object }}</dt>
              <dd class="strong-value">{{ selection.meshName }}</dd>
            </div>
            <div v-if="selection.triangleCount !== undefined">
              <dt>{{ text.triangles }}</dt>
              <dd>{{ formatNumber(selection.triangleCount, 0) }}</dd>
            </div>
            <div v-if="selection.surfaceArea !== undefined">
              <dt>{{ text.area }}</dt>
              <dd :title="formatSiUnit(selection.surfaceArea, 'area')">{{ formatNumber(selection.surfaceArea) }}</dd>
            </div>
            <div v-if="selection.volume !== undefined">
              <dt>{{ text.volume }}</dt>
              <dd :title="formatSiUnit(selection.volume, 'volume')">{{ formatNumber(selection.volume) }}</dd>
            </div>
          </dl>

          <div v-if="selection.position" class="vector-property">
            <span>{{ text.position }}</span>
            <output>{{ formatVector(selection.position) }}</output>
            <small>X&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;Y&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;Z</small>
          </div>
          <div v-if="selection.normal" class="vector-property">
            <span>{{ text.normal }}</span>
            <output>{{ formatVector(selection.normal) }}</output>
          </div>

          <div v-if="selection.bounds" class="bounds-card">
            <div class="subheading">{{ text.bounds }}</div>
            <dl>
              <div><dt>Min</dt><dd>{{ formatVector(selection.bounds.min) }}</dd></div>
              <div><dt>Max</dt><dd>{{ formatVector(selection.bounds.max) }}</dd></div>
              <div v-if="selectionSize"><dt>{{ text.size }}</dt><dd>{{ formatVector(selectionSize) }}</dd></div>
            </dl>
          </div>

          <button
            v-if="selection.source"
            class="source-card"
            type="button"
            :disabled="!sourceRevealEnabled"
            @click="emit('reveal-source', selection.source)"
          >
            <svg width="16" height="16" viewBox="0 0 20 20" aria-hidden="true">
              <path d="m7 5-4 5 4 5M13 5l4 5-4 5M11.5 3l-3 14" />
            </svg>
            <span>
              <strong>{{ selection.source.label || text.source }}</strong>
              <small>{{ sourceLocation(selection.source) }}</small>
            </span>
            <span class="reveal-label">{{ text.reveal }} →</span>
          </button>
        </div>
        <p v-else class="empty-selection">{{ text.noSelection }}</p>
      </details>

      <details class="inspect-section" open>
        <summary>
          <span>{{ text.measurement }}</span>
          <span v-if="measurementDistance !== null" class="summary-value accent-value">
            {{ formatNumber(measurementDistance) }}
          </span>
        </summary>
        <div class="section-content measure-content">
          <button
            class="primary-action"
            :class="{ active: measureActive }"
            type="button"
            :aria-pressed="measureActive"
            aria-keyshortcuts="Control+= Meta+="
            @click="toggleMeasure"
          >
            <svg width="16" height="16" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M3 14.5 14.5 3M5.5 12l2.5 2.5M9 8.5l2.5 2.5M12.5 5 15 7.5" />
            </svg>
            <span>{{ measureActive ? text.measuring : text.measureDistance }}</span>
            <kbd>Ctrl =</kbd>
          </button>

          <div v-if="measureActive || measurement" class="point-list" aria-live="polite">
            <div :class="{ pending: !pointA }">
              <span class="point-marker point-a" aria-hidden="true">A</span>
              <span><strong>{{ text.pointA }}</strong><small>{{ pointA ? formatVector(pointA) : text.waiting }}</small></span>
            </div>
            <div :class="{ pending: !pointB }">
              <span class="point-marker point-b" aria-hidden="true">B</span>
              <span><strong>{{ text.pointB }}</strong><small>{{ pointB ? formatVector(pointB) : text.waiting }}</small></span>
            </div>
          </div>

          <div v-if="measurementDistance !== null" class="measure-result">
            <div>
              <span>{{ text.distance }}</span>
              <output>{{ formatNumber(measurementDistance, 5) }}</output>
            </div>
            <div v-if="measurementDelta">
              <span>{{ text.delta }}</span>
              <output>{{ formatVector(measurementDelta) }}</output>
            </div>
          </div>

          <div v-if="measurement" class="inline-actions">
            <button type="button" @click="emit('clear-measurement')">{{ text.clear }}</button>
            <button type="button" :disabled="measurementDistance === null" @click="emit('copy-measurement', measurement)">
              {{ text.copy }}
            </button>
          </div>
        </div>
      </details>

      <details class="inspect-section" open>
        <summary>
          <span>{{ text.section }}</span>
          <span class="status-dot" :class="{ enabled: sectionEnabled }" aria-hidden="true" />
        </summary>
        <div class="section-content section-controls" :class="{ disabled: !sectionEnabled }">
          <label class="toggle-row" :for="sectionToggleId">
            <span>{{ text.enableSection }}</span>
            <input
              :id="sectionToggleId"
              type="checkbox"
              role="switch"
              :checked="sectionEnabled"
              @change="emit('update:section-enabled', ($event.target as HTMLInputElement).checked)"
            >
            <span class="switch" aria-hidden="true" />
          </label>

          <fieldset :disabled="!sectionEnabled">
            <legend>{{ text.plane }}</legend>
            <div class="axis-picker">
              <button
                v-for="axis in (['x', 'y', 'z'] as const)"
                :key="axis"
                type="button"
                :class="[`axis-${axis}`, { active: sectionAxis === axis }]"
                :aria-pressed="sectionAxis === axis"
                @click="emit('update:section-axis', axis)"
              >{{ axis.toUpperCase() }}</button>
            </div>
          </fieldset>

          <fieldset :disabled="!sectionEnabled">
            <legend>{{ text.offset }}</legend>
            <div class="offset-controls">
              <input
                :id="sectionOffsetId"
                class="offset-slider"
                type="range"
                :min="safeSectionMin"
                :max="safeSectionMax"
                :step="sectionStep"
                :value="sectionOffset"
                :aria-label="text.offset"
                @input="updateOffset(($event.target as HTMLInputElement).valueAsNumber)"
              >
              <label :for="sectionNumberId" class="number-wrap">
                <input
                  :id="sectionNumberId"
                  type="number"
                  :min="safeSectionMin"
                  :max="safeSectionMax"
                  :step="sectionStep"
                  :value="sectionOffset"
                  :aria-label="text.offset"
                  @change="updateOffset(($event.target as HTMLInputElement).valueAsNumber)"
                >
                <span>{{ text.units }}</span>
              </label>
            </div>
          </fieldset>

          <div class="section-actions">
            <button
              type="button"
              :disabled="!sectionEnabled"
              :class="{ active: sectionFlip }"
              :aria-pressed="sectionFlip"
              @click="emit('update:section-flip', !sectionFlip)"
            >
              <svg width="15" height="15" viewBox="0 0 20 20" aria-hidden="true">
                <path d="M4 6h10l-2.5-2.5M16 14H6l2.5 2.5" />
              </svg>
              {{ text.flip }} <kbd>Shift F</kbd>
            </button>
            <button type="button" :disabled="!sectionEnabled" @click="emit('reset-section')">
              {{ text.reset }}
            </button>
          </div>

          <div class="clip-direction" :aria-label="text.clippedSide">
            <span>{{ text.clippedSide }}</span>
            <strong>{{ sectionFlip ? text.negative : text.positive }} {{ sectionAxis.toUpperCase() }}</strong>
          </div>
        </div>
      </details>
    </div>

    <footer class="panel-footer" aria-hidden="true">{{ text.keyboard }}</footer>
  </aside>
</template>

<style scoped src="./InspectPanel.css"></style>
