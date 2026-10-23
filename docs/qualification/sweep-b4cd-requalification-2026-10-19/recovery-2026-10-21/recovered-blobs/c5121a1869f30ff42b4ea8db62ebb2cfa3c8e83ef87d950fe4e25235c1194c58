<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { advanceScanPlane, type ScanDirection } from './scanPlaneAnimation'

const props = defineProps<{
  enabled: boolean
  axis: 'x' | 'y' | 'z'
  offset: number
  min: number
  max: number
  step: number
  flip: boolean
  locale: 'ru' | 'en'
  available: boolean
}>()
const emit = defineEmits<{
  'update:enabled': [value: boolean]
  'update:axis': [value: 'x' | 'y' | 'z']
  'update:offset': [value: number]
  'update:flip': [value: boolean]
  reset: []
  close: []
}>()

const ru = computed(() => props.locale === 'ru')
const playing = ref(false)
const closeButton = ref<HTMLButtonElement | null>(null)
const disabled = computed(() => !props.available || !props.enabled)
const hasRange = computed(() => Number.isFinite(props.min) && Number.isFinite(props.max) && props.max > props.min)
const axes = ['x', 'y', 'z'] as const
const format = (value: number) => Number(value.toFixed(3)).toString()
const sideDescription = computed(() => `${ru.value ? 'Скрыта сторона' : 'Hidden side'}: ${props.axis.toUpperCase()} ${props.flip ? '>' : '<'} ${format(props.offset)} ${ru.value ? 'мм' : 'mm'}`)
let frame: number | null = null
let previousTime: number | null = null
let direction: ScanDirection = 1
let animatedOffset = 0

function stop() {
  playing.value = false
  if (frame !== null) cancelAnimationFrame(frame)
  frame = null
  previousTime = null
}

function tick(time: number) {
  if (!playing.value) return
  if (previousTime !== null) {
    const next = advanceScanPlane(animatedOffset, direction, props.min, props.max, time - previousTime)
    animatedOffset = next.offset
    direction = next.direction
    emit('update:offset', next.offset)
  }
  previousTime = time
  frame = requestAnimationFrame(tick)
}

function togglePlayback() {
  if (playing.value) { stop(); return }
  if (disabled.value || !hasRange.value || document.hidden) return
  animatedOffset = Math.min(props.max, Math.max(props.min, props.offset))
  direction = animatedOffset >= props.max ? -1 : 1
  playing.value = true
  frame = requestAnimationFrame(tick)
}

function setOffset(event: Event, commit = false) {
  stop()
  const input = event.target as HTMLInputElement
  const value = input.valueAsNumber
  if (!Number.isFinite(value)) {
    if (commit) input.value = format(props.offset)
    return
  }
  if (!commit && (value < props.min || value > props.max)) return
  const next = Math.min(props.max, Math.max(props.min, value))
  if (commit) input.value = format(next)
  emit('update:offset', next)
}

function setAxis(axis: 'x' | 'y' | 'z') { stop(); emit('update:axis', axis) }
function setEnabled(event: Event) { stop(); emit('update:enabled', (event.target as HTMLInputElement).checked) }
function flipSide() { stop(); emit('update:flip', !props.flip) }
function reset() { stop(); emit('reset') }
function close() { stop(); emit('close') }
function visibilityChanged() { if (document.hidden) stop() }

watch(() => [props.enabled, props.available, props.axis, props.min, props.max, props.flip], stop)
watch(() => props.offset, offset => {
  // An inspector edit must win over playback; our own emitted frame is acknowledged unchanged.
  if (playing.value && offset !== animatedOffset) stop()
})
onMounted(() => {
  closeButton.value?.focus({ preventScroll: true })
  document.addEventListener('visibilitychange', visibilityChanged)
})
onBeforeUnmount(() => {
  stop()
  document.removeEventListener('visibilitychange', visibilityChanged)
})
</script>

<template>
  <section
    class="scan-panel" role="dialog" aria-labelledby="scan-plane-title"
    @pointerdown.stop @pointerup.stop @click.stop @dblclick.stop @wheel.stop
    @keydown.stop @keyup.stop @keydown.esc.prevent.stop="close"
  >
    <header>
      <h2 id="scan-plane-title">{{ ru ? 'Плоскость сканирования' : 'Scanning plane' }}</h2>
      <button ref="closeButton" class="close" type="button" :aria-label="ru ? 'Закрыть панель' : 'Close panel'" @click="close">×</button>
    </header>

    <p class="hint">{{ ru ? 'Перемещайте срез, чтобы заглянуть внутрь модели. Срез влияет только на просмотр: при экспорте модель остаётся целой.' : 'Move the cut to look inside the model. The cut only affects the view; exports keep the complete model.' }}</p>
    <p v-if="!available" class="hint unavailable" role="status">{{ ru ? 'Сначала постройте модель.' : 'Build a model first.' }}</p>
    <label class="enable">
      <input type="checkbox" :checked="enabled" :disabled="!available && !enabled" @change="setEnabled">
      {{ ru ? 'Включить срез' : 'Enable cut' }}
    </label>

    <fieldset :disabled="disabled">
      <legend>{{ ru ? 'Ось' : 'Axis' }}</legend>
      <div class="axes">
        <label v-for="value in axes" :key="value" :class="{ selected: axis === value }">
          <input type="radio" name="scan-plane-axis" :value="value" :checked="axis === value" @change="setAxis(value)">
          {{ value.toUpperCase() }}
        </label>
      </div>
    </fieldset>

    <div class="position-heading">
      <label for="scan-plane-position">{{ ru ? 'Положение' : 'Position' }}</label>
      <div class="number-field">
        <input id="scan-plane-position" type="number" :value="format(offset)" :min="min" :max="max" :step="step" :disabled="disabled || !hasRange" @input="setOffset($event)" @change="setOffset($event, true)" @blur="setOffset($event, true)" @pointerdown="stop" @keydown="stop">
        <span>{{ ru ? 'мм' : 'mm' }}</span>
      </div>
    </div>
    <input class="position-slider" type="range" :aria-label="ru ? 'Положение плоскости сканирования' : 'Scanning plane position'" :value="offset" :min="min" :max="max" :step="step" :disabled="disabled || !hasRange" @input="setOffset($event)" @pointerdown="stop" @keydown="stop">
    <div class="range"><span>{{ format(min) }}</span><span>{{ format(max) }} {{ ru ? 'мм' : 'mm' }}</span></div>
    <p class="side" :class="{ muted: !enabled }">{{ sideDescription }}</p>

    <div class="actions">
      <button class="play" type="button" :disabled="disabled || !hasRange" :aria-pressed="playing" @click="togglePlayback">{{ playing ? (ru ? 'Ⅱ Пауза' : 'Ⅱ Pause') : (ru ? '▶ Сканировать' : '▶ Scan') }}</button>
      <button type="button" :disabled="disabled" @click="reset">{{ ru ? 'В центр' : 'Center' }}</button>
    </div>
    <button class="flip" type="button" :disabled="disabled" @click="flipSide">{{ ru ? 'Показать другую сторону' : 'Show the other side' }}</button>
  </section>
</template>

<style scoped>
.scan-panel{position:absolute;top:98px;left:12px;z-index:25;box-sizing:border-box;width:290px;max-width:calc(100% - 24px);max-height:calc(100% - 110px);overflow:auto;padding:16px;border:1px solid var(--border);border-radius:12px;background:var(--surface);color:var(--text);box-shadow:0 10px 34px #0005;font-size:13px;line-height:1.4}
header{display:flex;align-items:flex-start;justify-content:space-between;gap:6px}h2{margin:0;font-size:15px;font-weight:650;line-height:1.35}.hint{margin:10px 0;color:var(--text-muted,var(--text));opacity:.8}.unavailable{color:var(--accent);opacity:1}.enable{display:flex;align-items:center;gap:8px;margin:14px 0;cursor:pointer}.enable input{margin:0;accent-color:var(--accent)}
button,input{font:inherit;color:inherit}button{border:1px solid var(--border);border-radius:6px;background:var(--surface-raised);padding:7px 9px;cursor:pointer}button:hover:not(:disabled){border-color:var(--accent)}button:focus-visible,input:focus-visible{outline:2px solid var(--accent);outline-offset:2px}button:disabled,input:disabled{opacity:.45;cursor:default}.close{border:0;background:transparent;font-size:22px;line-height:18px;padding:0 2px 5px 5px}
fieldset{border:0;padding:0;margin:0 0 14px}legend{padding:0;margin-bottom:6px}.axes{display:flex;gap:6px}.axes label{flex:1;display:flex;align-items:center;justify-content:center;gap:6px;padding:6px 4px;border:1px solid var(--border);border-radius:6px;cursor:pointer;background:var(--surface-raised)}.axes input{margin:0;accent-color:var(--accent)}.axes .selected{border-color:var(--accent)}fieldset:disabled .axes{opacity:.45}fieldset:disabled label{cursor:default}
.position-heading{display:flex;justify-content:space-between;align-items:center;gap:8px}.number-field{display:flex;align-items:center;gap:5px}.number-field input{box-sizing:border-box;width:94px;border:1px solid var(--border);border-radius:5px;background:var(--surface-raised);padding:5px}.position-slider{display:block;box-sizing:border-box;width:100%;margin:12px 0 3px;accent-color:var(--accent)}.range{display:flex;justify-content:space-between;font-size:11px;opacity:.65}.side{margin:10px 0 14px;font-size:12px}.muted{opacity:.45}.actions{display:flex;gap:7px}.play{flex:1;background:var(--accent);color:var(--accent-contrast,#fff)}.flip{width:100%;margin-top:7px}
</style>
