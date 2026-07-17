<script setup lang="ts">
import type { StandardView } from '../services/webgpuRenderer'

const props = defineProps<{
  activeView: StandardView | 'custom'
}>()

const emit = defineEmits<{
  view: [view: StandardView]
}>()

const labels: Record<StandardView, string> = {
  iso: 'Isometric / Изометрия',
  front: 'Front / Спереди',
  back: 'Back / Сзади',
  left: 'Left / Слева',
  right: 'Right / Справа',
  top: 'Top / Сверху',
  bottom: 'Bottom / Снизу',
}

function choose(view: StandardView) {
  emit('view', view)
}
</script>

<template>
  <div class="view-cube" role="group" aria-label="View orientation / Ориентация вида">
    <svg class="cube-svg" viewBox="0 0 128 132" aria-hidden="false">
      <g class="axes" aria-hidden="true">
        <path class="axis axis-z" d="M64 62V5" />
        <path class="axis axis-y" d="M64 62 11 94" />
        <path class="axis axis-x" d="m64 62 53 32" />
        <text class="axis-label axis-z-label" x="64" y="8">Z</text>
        <text class="axis-label axis-y-label" x="7" y="101">Y</text>
        <text class="axis-label axis-x-label" x="121" y="101">X</text>
      </g>

      <g
        class="cube-action face-action"
        :class="{ active: props.activeView === 'top' }"
        role="button"
        tabindex="0"
        :aria-label="labels.top"
        :aria-pressed="props.activeView === 'top'"
        @click="choose('top')"
        @keydown.enter.prevent="choose('top')"
        @keydown.space.prevent="choose('top')"
      >
        <title>{{ labels.top }}</title>
        <polygon class="face face-top" points="64,18 104,40 64,62 24,40" />
        <text class="face-label" x="64" y="39">TOP</text>
      </g>

      <g
        class="cube-action face-action"
        :class="{ active: props.activeView === 'front' }"
        role="button"
        tabindex="0"
        :aria-label="labels.front"
        :aria-pressed="props.activeView === 'front'"
        @click="choose('front')"
        @keydown.enter.prevent="choose('front')"
        @keydown.space.prevent="choose('front')"
      >
        <title>{{ labels.front }}</title>
        <polygon class="face face-front" points="24,40 64,62 64,108 24,86" />
        <text class="face-label" x="44" y="77">FRONT</text>
      </g>

      <g
        class="cube-action face-action"
        :class="{ active: props.activeView === 'right' }"
        role="button"
        tabindex="0"
        :aria-label="labels.right"
        :aria-pressed="props.activeView === 'right'"
        @click="choose('right')"
        @keydown.enter.prevent="choose('right')"
        @keydown.space.prevent="choose('right')"
      >
        <title>{{ labels.right }}</title>
        <polygon class="face face-right" points="64,62 104,40 104,86 64,108" />
        <text class="face-label" x="84" y="77">RIGHT</text>
      </g>

      <g
        class="cube-action badge-action badge-back"
        :class="{ active: props.activeView === 'back' }"
        role="button"
        tabindex="0"
        :aria-label="labels.back"
        :aria-pressed="props.activeView === 'back'"
        @click="choose('back')"
        @keydown.enter.prevent="choose('back')"
        @keydown.space.prevent="choose('back')"
      >
        <title>{{ labels.back }}</title>
        <rect class="badge" x="96" y="10" width="29" height="20" rx="7" />
        <text class="badge-label" x="110.5" y="23.5">BK</text>
      </g>

      <g
        class="cube-action badge-action badge-left"
        :class="{ active: props.activeView === 'left' }"
        role="button"
        tabindex="0"
        :aria-label="labels.left"
        :aria-pressed="props.activeView === 'left'"
        @click="choose('left')"
        @keydown.enter.prevent="choose('left')"
        @keydown.space.prevent="choose('left')"
      >
        <title>{{ labels.left }}</title>
        <rect class="badge" x="3" y="52" width="25" height="22" rx="7" />
        <text class="badge-label" x="15.5" y="67">L</text>
      </g>

      <g
        class="cube-action badge-action badge-bottom"
        :class="{ active: props.activeView === 'bottom' }"
        role="button"
        tabindex="0"
        :aria-label="labels.bottom"
        :aria-pressed="props.activeView === 'bottom'"
        @click="choose('bottom')"
        @keydown.enter.prevent="choose('bottom')"
        @keydown.space.prevent="choose('bottom')"
      >
        <title>{{ labels.bottom }}</title>
        <rect class="badge" x="51" y="108" width="26" height="21" rx="7" />
        <text class="badge-label" x="64" y="122.5">B</text>
      </g>

      <g
        class="cube-action iso-action"
        :class="{ active: props.activeView === 'iso' }"
        role="button"
        tabindex="0"
        :aria-label="labels.iso"
        :aria-pressed="props.activeView === 'iso'"
        @click="choose('iso')"
        @keydown.enter.prevent="choose('iso')"
        @keydown.space.prevent="choose('iso')"
      >
        <title>{{ labels.iso }}</title>
        <circle class="iso-button" cx="64" cy="62" r="15" />
        <text class="iso-label" x="64" y="65.5">ISO</text>
      </g>
    </svg>
  </div>
</template>

<style scoped>
.view-cube {
  width: 118px;
  aspect-ratio: 128 / 132;
  color: #edf1f7;
  user-select: none;
  filter: drop-shadow(0 5px 12px rgba(0, 0, 0, 0.32));
}

.cube-svg {
  width: 100%;
  height: 100%;
  display: block;
  overflow: visible;
}

.axes { pointer-events: none; }
.axis {
  fill: none;
  stroke-width: 1.4;
  stroke-linecap: round;
  opacity: 0.82;
}
.axis-x { stroke: #ef5b59; }
.axis-y { stroke: #59c978; }
.axis-z { stroke: #5898ff; }
.axis-label {
  font: 800 8px/1 ui-sans-serif, system-ui, sans-serif;
  text-anchor: middle;
  paint-order: stroke;
  stroke: rgba(12, 15, 20, 0.9);
  stroke-width: 2.5px;
  stroke-linejoin: round;
}
.axis-x-label { fill: #ff7774; }
.axis-y-label { fill: #6ce18b; }
.axis-z-label { fill: #72a8ff; }

.cube-action {
  cursor: pointer;
  outline: none;
}

.face,
.badge,
.iso-button {
  stroke: rgba(235, 241, 250, 0.5);
  stroke-width: 1.2;
  transition: fill 100ms ease, stroke 100ms ease, filter 100ms ease;
}

.face-top { fill: rgba(93, 105, 125, 0.93); }
.face-front { fill: rgba(55, 63, 77, 0.96); }
.face-right { fill: rgba(69, 79, 96, 0.96); }

.face-label,
.badge-label,
.iso-label {
  fill: #f5f7fb;
  font: 750 7px/1 ui-sans-serif, system-ui, sans-serif;
  text-anchor: middle;
  pointer-events: none;
  paint-order: stroke;
  stroke: rgba(16, 19, 25, 0.72);
  stroke-width: 1.8px;
  stroke-linejoin: round;
}

.badge {
  fill: rgba(29, 34, 43, 0.92);
  stroke: rgba(235, 241, 250, 0.34);
}
.badge-label { font-size: 8px; }

.iso-button {
  fill: rgba(30, 35, 44, 0.94);
  stroke: rgba(235, 241, 250, 0.62);
}
.iso-label { font-size: 7px; }

.cube-action:hover .face,
.cube-action:hover .badge,
.cube-action:hover .iso-button {
  fill: color-mix(in srgb, var(--accent, #559dff) 55%, #2e3643);
  stroke: #cfe4ff;
}

.cube-action.active .face,
.cube-action.active .badge,
.cube-action.active .iso-button {
  fill: color-mix(in srgb, var(--accent, #559dff) 78%, #26384e);
  stroke: #ffffff;
  filter: drop-shadow(0 0 4px color-mix(in srgb, var(--accent, #559dff) 70%, transparent));
}

.cube-action:focus-visible .face,
.cube-action:focus-visible .badge,
.cube-action:focus-visible .iso-button {
  stroke: var(--focus, #8ec1ff);
  stroke-width: 2.8;
}

@media (max-width: 800px) {
  .view-cube { width: 102px; }
  .face-label { font-size: 6.5px; }
}

@media (max-width: 420px) {
  .view-cube { width: 92px; }
}

@media (prefers-reduced-motion: reduce) {
  .face,
  .badge,
  .iso-button { transition-duration: 0.01ms; }
}
</style>
