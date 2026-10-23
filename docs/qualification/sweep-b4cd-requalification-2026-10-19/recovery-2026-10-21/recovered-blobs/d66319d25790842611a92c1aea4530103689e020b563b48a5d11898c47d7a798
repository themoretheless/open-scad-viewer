<script setup lang="ts">
import { computed, useId } from 'vue'
import { projectDirectPoint, type OrbitCamera } from '../services/directModelingTools'
import { useModelingGrid } from '../services/modelingGrid'

const props = defineProps<{ camera: OrbitCamera; size: number; center: readonly number[]; flipY?: boolean }>()
const id = useId()
const { step } = useModelingGrid()
const grid = computed(() => {
  const level = Math.max(0, Math.log10(props.size / (8 * step.value)))
  const fine = step.value * 10 ** Math.floor(level)
  const a = projectDirectPoint([1, 0, 0], props.camera)
  const b = projectDirectPoint([0, 1, 0], props.camera)
  const y = props.flipY ? -1 : 1
  return { fine, opacity: 1 - (level - Math.floor(level)), transform: `matrix(${a[0]} ${a[1] * y} ${b[0]} ${b[1] * y} 0 0)` }
})
</script>

<template>
  <g v-if="Math.abs(Math.sin(camera.pitch)) > .001" pointer-events="none">
    <defs>
      <pattern v-for="(spacing, i) in [grid.fine, grid.fine * 10]" :id="`${id}-floor-${i}`" :key="i" :width="spacing" :height="spacing" patternUnits="userSpaceOnUse" :patternTransform="grid.transform">
        <path :d="`M ${spacing} 0 H 0 V ${spacing}`" fill="none" stroke="var(--border)" :stroke-opacity=".45 * (i ? 1 : grid.opacity)" stroke-width=".5" vector-effect="non-scaling-stroke" />
      </pattern>
    </defs>
    <rect v-for="i in [0, 1]" :key="i" :x="center[0]! - size * 100" :y="center[1]! - size * 100" :width="size * 200" :height="size * 200" :fill="`url(#${id}-floor-${i})`" />
  </g>
</template>
