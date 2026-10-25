import { computed, nextTick, ref, watch, type ComputedRef, type Ref, type ShallowRef } from 'vue'
import type { SectionAxis } from '../components/cadPanels.types'
import type { MeshData } from '../core/mesh'
import { clamp } from '../services/math3d'
import { inspectMesh } from '../services/meshInspection'
import type { PickHit } from '../services/rendererContracts'
import type { SceneController, SceneState } from '../services/sceneController'
import type { WebGPURenderer } from '../services/webgpuRenderer'

export interface UseSectionPlaneControlsOptions {
  gpuOk: Ref<boolean>
  sceneMeshes: ComputedRef<MeshData[]>
  meshVisibility: ComputedRef<boolean[]>
  isolated: ComputedRef<boolean>
  selectedMesh: ComputedRef<number | null>
  sceneState: ShallowRef<SceneState<PickHit>>
  sceneController: SceneController<PickHit>
  getRenderer: () => WebGPURenderer | null
}

export function useSectionPlaneControls(options: UseSectionPlaneControlsOptions) {
  const {
    gpuOk,
    sceneMeshes,
    meshVisibility,
    isolated,
    selectedMesh,
    sceneState,
    sceneController,
    getRenderer,
  } = options

  const sectionEnabled = computed(() => sceneState.value.section.enabled)
  const sectionAxis = computed(() => sceneState.value.section.axis)
  const sectionOffset = computed(() => sceneState.value.section.offset)
  const sectionFlip = computed(() => sceneState.value.section.flip)
  const scanPanelOpen = ref(false)
  const scanToggleRef = ref<HTMLButtonElement | null>(null)

  function isSectionMeshVisible(index: number) {
    return meshVisibility.value[index] !== false && (!isolated.value || index === selectedMesh.value)
  }

  const sectionRange = computed(() => {
    const axis = sectionAxis.value === 'x' ? 0 : sectionAxis.value === 'y' ? 1 : 2
    let min = Infinity
    let max = -Infinity
    sceneMeshes.value.forEach((mesh, index) => {
      if (!isSectionMeshVisible(index)) return
      const bounds = inspectMesh(mesh, index).bounds
      if (!bounds) return
      min = Math.min(min, bounds.min[axis])
      max = Math.max(max, bounds.max[axis])
    })
    if (!Number.isFinite(min) || !Number.isFinite(max)) return { min: -100, max: 100, step: 0.1 }
    const span = Math.max(0.001, max - min)
    return { min, max, step: Math.max(0.001, span / 500) }
  })

  const sectionAvailable = computed(() =>
    gpuOk.value && sceneMeshes.value.some((_, index) => isSectionMeshVisible(index)),
  )

  function applySection() {
    const base: [number, number, number] =
      sectionAxis.value === 'x' ? [1, 0, 0] : sectionAxis.value === 'y' ? [0, 1, 0] : [0, 0, 1]
    const sign = sectionFlip.value ? -1 : 1
    getRenderer()?.setSection(
      sectionEnabled.value,
      [base[0] * sign, base[1] * sign, base[2] * sign],
      sectionOffset.value * sign,
    )
  }

  function syncSectionRange() {
    const { min, max } = sectionRange.value
    if (sectionOffset.value < min || sectionOffset.value > max) {
      sceneController.setSection({ offset: (min + max) / 2 })
    }
    applySection()
  }

  watch(sectionRange, syncSectionRange)

  function setSectionEnabled(enabled: boolean) {
    if (enabled && !sectionAvailable.value) return
    const patch =
      enabled && !sceneState.value.section.initialized
        ? { enabled, offset: (sectionRange.value.min + sectionRange.value.max) / 2, initialized: true }
        : { enabled }
    sceneController.setSection(patch)
    applySection()
  }

  function setSectionAxis(axis: SectionAxis) {
    sceneController.setSection({
      axis,
      offset: (sectionRange.value.min + sectionRange.value.max) / 2,
      initialized: true,
    })
    applySection()
  }

  function setSectionOffset(offset: number) {
    if (!Number.isFinite(offset)) return
    sceneController.setSection({
      offset: clamp(offset, sectionRange.value.min, sectionRange.value.max),
      initialized: true,
    })
    applySection()
  }

  function setSectionFlip(flip: boolean) {
    sceneController.setSection({ flip })
    applySection()
  }

  function resetSection() {
    sceneController.setSection({
      offset: (sectionRange.value.min + sectionRange.value.max) / 2,
      flip: false,
      initialized: true,
    })
    applySection()
  }

  function closeScanPanel() {
    scanPanelOpen.value = false
    void nextTick(() => scanToggleRef.value?.focus())
  }

  return {
    sectionEnabled,
    sectionAxis,
    sectionOffset,
    sectionFlip,
    scanPanelOpen,
    scanToggleRef,
    sectionRange,
    sectionAvailable,
    applySection,
    syncSectionRange,
    setSectionEnabled,
    setSectionAxis,
    setSectionOffset,
    setSectionFlip,
    resetSection,
    closeScanPanel,
  }
}
