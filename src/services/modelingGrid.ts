import { computed, ref } from 'vue'
import { storageGet, storageSet } from './safeStorage'

export const GRID_UNITS = { mm: 1, cm: 10, m: 1000, in: 25.4 } as const
export type GridUnit = keyof typeof GRID_UNITS
export function validGridStep(value: number) { return Number.isFinite(value) && value >= 0.000001 && value <= 1e6 }
export function convertGridSize(value: number, from: GridUnit, to: GridUnit) { return value * GRID_UNITS[from] / GRID_UNITS[to] }
const savedUnit = storageGet('scad-grid-unit')
const unit = ref<GridUnit>(savedUnit && Object.hasOwn(GRID_UNITS, savedUnit) ? savedUnit as GridUnit : 'mm')
const savedStep = Number(storageGet('scad-grid-step'))
const step = ref(validGridStep(savedStep) ? savedStep : 10)
const enabled = ref(storageGet('scad-snap-enabled') !== 'false')
const grid = ref(storageGet('scad-snap-grid') !== 'false')
const geometry = ref(storageGet('scad-snap-geometry') !== 'false')
const guides = ref(storageGet('scad-snap-guides') !== 'false')
const size = computed(() => Number((step.value / GRID_UNITS[unit.value]).toPrecision(12)))
export function useModelingGrid() {
  return { unit, step, size, enabled, grid, geometry, guides,
    setSize(value: number) { const mm = value * GRID_UNITS[unit.value]; if (!validGridStep(mm)) return false; step.value = mm; storageSet('scad-grid-step', String(mm)); return true },
    setUnit(value: GridUnit) { if (!Object.hasOwn(GRID_UNITS, value)) return; unit.value = value; storageSet('scad-grid-unit', value) },
    saveSnaps() { for (const [key, value] of Object.entries({ enabled, grid, geometry, guides })) storageSet(`scad-snap-${key}`, String(value.value)) },
  }
}
