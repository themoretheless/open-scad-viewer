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

function formatNumber(value: number, digits = 3) {
  return value.toLocaleString(props.locale, { maximumFractionDigits: digits })
}

function formatVector(vector: Vector3 | undefined | null) {
  if (!vector) return '—'
  return vector.map(value => formatNumber(value)).join('  ·  ')
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
              <dd>{{ selection.triangleCount.toLocaleString(locale) }}</dd>
            </div>
            <div v-if="selection.surfaceArea !== undefined">
              <dt>{{ text.area }}</dt>
              <dd>{{ formatNumber(selection.surfaceArea) }}</dd>
            </div>
            <div v-if="selection.volume !== undefined">
              <dt>{{ text.volume }}</dt>
              <dd>{{ formatNumber(selection.volume) }}</dd>
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

<style scoped>
.inspect-panel {
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
}

.panel-header {
  min-height: 38px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 5px 9px 5px 10px;
  color: var(--text, #eef0f5);
  background: color-mix(in srgb, var(--surface-raised, #22252d) 66%, var(--surface, #1a1c22));
  border-bottom: 1px solid var(--border, #30343e);
}
.panel-header h2 { margin: 0; font-size: 0.78rem; font-weight: 720; letter-spacing: 0.01em; }
.panel-header svg,
.primary-action svg,
.source-card svg,
.section-actions svg { fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.5; }
.panel-header svg { color: var(--text-dim, #a4a9b5); }

.panel-body { min-height: 0; flex: 1; overflow: auto; scrollbar-width: thin; }
.inspect-section { border-bottom: 1px solid var(--border, #30343e); }
.inspect-section > summary {
  min-height: 31px;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 5px 9px;
  color: var(--text-dim, #a4a9b5);
  background: color-mix(in srgb, var(--surface-raised, #22252d) 36%, transparent);
  cursor: pointer;
  font-size: 0.64rem;
  font-weight: 720;
  letter-spacing: 0.045em;
  list-style: none;
  text-transform: uppercase;
}
.inspect-section > summary::-webkit-details-marker { display: none; }
.inspect-section > summary::before {
  width: 9px;
  content: '›';
  color: var(--text, #eef0f5);
  font-size: 0.82rem;
  line-height: 1;
  transform: rotate(0deg);
  transition: transform 100ms ease;
}
.inspect-section[open] > summary::before { transform: rotate(90deg); }
.inspect-section > summary:focus-visible,
button:focus-visible,
input:focus-visible {
  outline: 2px solid var(--focus, #8ec1ff);
  outline-offset: -2px;
}
.summary-value {
  min-width: 0;
  margin-left: auto;
  overflow: hidden;
  max-width: 130px;
  color: var(--text, #eef0f5);
  font-size: 0.6rem;
  font-weight: 620;
  letter-spacing: 0;
  text-overflow: ellipsis;
  text-transform: none;
  white-space: nowrap;
}
.accent-value { color: var(--accent, #559dff); font-family: ui-monospace, monospace; }
.status-dot { width: 7px; height: 7px; margin-left: auto; background: var(--text-dim, #a4a9b5); border-radius: 50%; opacity: 0.55; }
.status-dot.enabled { background: #5fd785; box-shadow: 0 0 7px rgba(95, 215, 133, 0.5); opacity: 1; }

.section-content { padding: 9px; }
.property-grid { margin: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 1px; overflow: hidden; border: 1px solid var(--border, #30343e); border-radius: 7px; }
.property-grid > div { min-width: 0; padding: 7px 8px; background: var(--surface-raised, #22252d); }
.property-grid dt,
.bounds-card dt { color: var(--text-dim, #a4a9b5); font-size: 0.58rem; }
.property-grid dd { margin: 3px 0 0; overflow: hidden; font: 0.67rem/1.2 ui-monospace, monospace; text-overflow: ellipsis; white-space: nowrap; }
.property-grid .strong-value { font-family: inherit; font-weight: 700; }

.vector-property { display: grid; gap: 3px; margin-top: 7px; padding: 7px 8px; background: color-mix(in srgb, var(--surface-raised, #22252d) 72%, transparent); border: 1px solid var(--border, #30343e); border-radius: 7px; }
.vector-property > span,
.subheading { color: var(--text-dim, #a4a9b5); font-size: 0.58rem; }
.vector-property output { overflow: hidden; font: 0.66rem/1.35 ui-monospace, monospace; text-overflow: ellipsis; white-space: nowrap; }
.vector-property small { color: color-mix(in srgb, var(--text-dim, #a4a9b5) 68%, transparent); font: 0.5rem/1 ui-monospace, monospace; }

.bounds-card { margin-top: 7px; padding: 7px 8px; border: 1px solid var(--border, #30343e); border-radius: 7px; }
.bounds-card dl { margin: 5px 0 0; display: grid; gap: 4px; }
.bounds-card dl > div { display: grid; grid-template-columns: 36px minmax(0, 1fr); gap: 5px; }
.bounds-card dd { margin: 0; overflow: hidden; font: 0.6rem/1.3 ui-monospace, monospace; text-overflow: ellipsis; white-space: nowrap; }

button { color: inherit; font: inherit; }
.source-card {
  width: 100%;
  min-height: 43px;
  display: flex;
  align-items: center;
  gap: 7px;
  margin-top: 7px;
  padding: 6px 7px;
  text-align: left;
  background: color-mix(in srgb, var(--accent, #559dff) 8%, var(--surface-raised, #22252d));
  border: 1px solid color-mix(in srgb, var(--accent, #559dff) 28%, var(--border, #30343e));
  border-radius: 7px;
  cursor: pointer;
}
.source-card > span:nth-child(2) { min-width: 0; flex: 1; display: grid; gap: 2px; }
.source-card strong { overflow: hidden; font-size: 0.65rem; text-overflow: ellipsis; white-space: nowrap; }
.source-card small { color: var(--text-dim, #a4a9b5); font: 0.55rem/1 ui-monospace, monospace; }
.source-card svg { flex: 0 0 auto; color: var(--accent, #559dff); }
.source-card:disabled { opacity: 0.52; cursor: not-allowed; }
.reveal-label { flex: 0 0 auto !important; color: var(--accent, #559dff); font-size: 0.56rem; }
.empty-selection { margin: 0; padding: 18px 12px; color: var(--text-dim, #a4a9b5); font-size: 0.67rem; text-align: center; }

.measure-content { display: grid; gap: 7px; }
.primary-action {
  width: 100%;
  min-height: 34px;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 5px 7px;
  color: white;
  background: var(--accent-strong, #287eea);
  border: 1px solid var(--accent-strong, #287eea);
  border-radius: 7px;
  cursor: pointer;
  font-size: 0.67rem;
  font-weight: 680;
}
.primary-action.active { background: color-mix(in srgb, var(--accent, #559dff) 24%, var(--surface-raised, #22252d)); border-color: var(--accent, #559dff); color: var(--text, #eef0f5); }
.primary-action span { flex: 1; text-align: left; }
kbd {
  padding: 2px 4px;
  color: inherit;
  background: rgba(0, 0, 0, 0.18);
  border: 1px solid rgba(255, 255, 255, 0.24);
  border-radius: 4px;
  font: 0.55rem/1 ui-monospace, monospace;
}

.point-list { display: grid; gap: 1px; overflow: hidden; border: 1px solid var(--border, #30343e); border-radius: 7px; }
.point-list > div { min-height: 36px; display: flex; align-items: center; gap: 7px; padding: 5px 7px; background: var(--surface-raised, #22252d); }
.point-list > div.pending { opacity: 0.58; }
.point-marker { width: 20px; height: 20px; display: grid; place-items: center; flex: 0 0 20px; color: #0d1117; border-radius: 50%; font: 800 0.58rem/1 ui-monospace, monospace; }
.point-a { background: #61c7ff; }
.point-b { background: #ffbc5e; }
.point-list > div > span:last-child { min-width: 0; flex: 1; display: grid; gap: 2px; }
.point-list strong { font-size: 0.6rem; }
.point-list small { overflow: hidden; color: var(--text-dim, #a4a9b5); font: 0.56rem/1.2 ui-monospace, monospace; text-overflow: ellipsis; white-space: nowrap; }
.measure-result { display: grid; gap: 5px; padding: 7px 8px; background: color-mix(in srgb, var(--accent, #559dff) 8%, var(--surface-raised, #22252d)); border: 1px solid color-mix(in srgb, var(--accent, #559dff) 26%, var(--border, #30343e)); border-radius: 7px; }
.measure-result > div { display: flex; align-items: baseline; gap: 8px; }
.measure-result span { color: var(--text-dim, #a4a9b5); font-size: 0.58rem; }
.measure-result output { margin-left: auto; font: 700 0.67rem/1.2 ui-monospace, monospace; }
.inline-actions,
.section-actions { display: flex; justify-content: flex-end; gap: 5px; }
.inline-actions button,
.section-actions button {
  min-height: 27px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 8px;
  background: var(--surface-raised, #22252d);
  border: 1px solid var(--border, #30343e);
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.61rem;
}
.inline-actions button:hover,
.section-actions button:hover { background: var(--hover, #2a2e38); border-color: color-mix(in srgb, var(--accent, #559dff) 45%, var(--border, #30343e)); }
button:disabled { opacity: 0.42; cursor: not-allowed; }

.section-controls { display: grid; gap: 9px; }
.section-controls.disabled > :not(.toggle-row) { opacity: 0.48; }
.toggle-row { display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 0.66rem; }
.toggle-row > span:first-child { flex: 1; }
.toggle-row input { position: absolute; opacity: 0; pointer-events: none; }
.switch { position: relative; width: 28px; height: 16px; background: var(--border, #30343e); border-radius: 999px; transition: background 100ms ease; }
.switch::after { position: absolute; top: 2px; left: 2px; width: 12px; height: 12px; content: ''; background: var(--text-dim, #a4a9b5); border-radius: 50%; transition: transform 100ms ease, background 100ms ease; }
.toggle-row input:checked + .switch { background: var(--accent-strong, #287eea); }
.toggle-row input:checked + .switch::after { background: white; transform: translateX(12px); }
.toggle-row input:focus-visible + .switch { outline: 2px solid var(--focus, #8ec1ff); outline-offset: 2px; }

fieldset { min-width: 0; margin: 0; padding: 0; border: 0; }
legend { margin-bottom: 5px; color: var(--text-dim, #a4a9b5); font-size: 0.58rem; }
.axis-picker { display: grid; grid-template-columns: repeat(3, 1fr); gap: 3px; padding: 3px; background: var(--surface-raised, #22252d); border: 1px solid var(--border, #30343e); border-radius: 7px; }
.axis-picker button { min-height: 28px; background: transparent; border: 1px solid transparent; border-radius: 5px; cursor: pointer; font: 750 0.65rem/1 ui-monospace, monospace; }
.axis-picker button:hover { background: var(--hover, #2a2e38); }
.axis-picker button.active { background: color-mix(in srgb, var(--axis-color) 18%, var(--hover, #2a2e38)); border-color: color-mix(in srgb, var(--axis-color) 58%, var(--border, #30343e)); color: var(--axis-color); }
.axis-x { --axis-color: #ff716d; }
.axis-y { --axis-color: #66d889; }
.axis-z { --axis-color: #65a5ff; }

.offset-controls { display: grid; grid-template-columns: minmax(80px, 1fr) 88px; align-items: center; gap: 7px; }
.offset-slider { width: 100%; accent-color: var(--accent, #559dff); }
.number-wrap { min-width: 0; display: flex; align-items: center; overflow: hidden; background: var(--surface-raised, #22252d); border: 1px solid var(--border, #30343e); border-radius: 6px; }
.number-wrap:focus-within { border-color: var(--focus, #8ec1ff); }
.number-wrap input { min-width: 0; width: 100%; height: 28px; padding: 3px 3px 3px 7px; color: var(--text, #eef0f5); background: transparent; border: 0; outline: 0; font: 0.62rem/1 ui-monospace, monospace; }
.number-wrap span { padding-right: 6px; color: var(--text-dim, #a4a9b5); font-size: 0.54rem; }
.section-actions { justify-content: space-between; }
.section-actions button.active { color: var(--accent, #559dff); border-color: color-mix(in srgb, var(--accent, #559dff) 50%, var(--border, #30343e)); }
.section-actions kbd { margin-left: 2px; color: var(--text-dim, #a4a9b5); border-color: var(--border, #30343e); }
.clip-direction { display: flex; align-items: center; gap: 8px; padding-top: 7px; color: var(--text-dim, #a4a9b5); border-top: 1px solid var(--border, #30343e); font-size: 0.56rem; }
.clip-direction strong { margin-left: auto; color: var(--text, #eef0f5); font: 0.58rem/1 ui-monospace, monospace; }

.panel-footer { min-height: 25px; padding: 6px 9px; overflow: hidden; color: var(--text-dim, #a4a9b5); background: color-mix(in srgb, var(--surface-raised, #22252d) 45%, transparent); border-top: 1px solid var(--border, #30343e); font-size: 0.56rem; text-overflow: ellipsis; white-space: nowrap; }

@media (max-width: 680px) {
  .inspect-panel { width: 100%; max-height: 58dvh; border-radius: 8px; }
  .panel-footer { display: none; }
}

@media (prefers-reduced-motion: reduce) {
  .inspect-section > summary::before,
  .switch,
  .switch::after { transition-duration: 0.01ms; }
}
</style>
