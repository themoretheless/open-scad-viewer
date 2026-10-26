import { computed, ref, type ComputedRef, type Ref, type ShallowRef, type WritableComputedRef } from 'vue'
import { hexToRgb, lineAndColumn, type DockTab } from '../appHelpers'
import type {
  DistanceMeasurement as PanelMeasurement,
  InspectSelection,
  SceneMeshRow,
  SourceProvenanceRow,
} from '../components/cadPanels.types'
import type { MeshData } from '../core/mesh'
import type { Language } from '../i18n/appStrings'
import type { CameraState } from '../services/cameraHistory'
import { inspectMesh } from '../services/meshInspection'
import { useModelingGrid } from '../services/modelingGrid'
import type {
  DisplayMode,
  DistanceMeasurement,
  PickHit,
  ProjectionMode,
  SelectionMode,
  ShadingModel,
  StandardView,
} from '../services/rendererContracts'
import { DEFAULT_ENV, DEFAULT_MATCAP, getMaterialPreset } from '../services/rendererContracts'
import type { SceneController, SceneState } from '../services/sceneController'
import type { ViewportController, ViewportState } from '../services/viewportController'
import type { WebGPURenderer } from '../services/webgpuRenderer'

export interface UseViewportDisplayControlsOptions {
  viewportController: ViewportController
  viewportState: ShallowRef<ViewportState>
  sceneController: SceneController<PickHit>
  sceneState: ShallowRef<SceneState<PickHit>>
  sceneMeshes: ComputedRef<MeshData[]>
  meshVisibility: ComputedRef<boolean[]>
  selectedMesh: ComputedRef<number | null>
  selectedHit: ComputedRef<PickHit | null>
  hoveredHit: ComputedRef<PickHit | null>
  isolated: WritableComputedRef<boolean>
  measurement: ComputedRef<DistanceMeasurement | null>
  measureActive: ComputedRef<boolean>
  renderedSource: Ref<string>
  code: Ref<string>
  surfaceArea: Ref<number>
  volume: Ref<number>
  mainCameraRevision: Ref<number>
  dockOpen: Ref<boolean>
  dockTab: Ref<DockTab>
  getRenderer: () => WebGPURenderer | null
  showNotice: (message: string) => void
  translate: (key: string) => string
}

export function useViewportDisplayControls(options: UseViewportDisplayControlsOptions) {
  const {
    viewportController,
    viewportState,
    sceneController,
    sceneMeshes,
    meshVisibility,
    selectedMesh,
    selectedHit,
    hoveredHit,
    isolated,
    measurement,
    renderedSource,
    code,
    surfaceArea,
    volume,
    mainCameraRevision,
    dockOpen,
    dockTab,
    getRenderer,
    showNotice,
    translate: t,
  } = options

  const projection = computed<ProjectionMode>(() => viewportState.value.camera.projection)
  const gridVisible = ref(true)
  const { step: gridStep } = useModelingGrid()
  const standardView = computed({
    get: () => viewportState.value.standardView,
    set: (view: StandardView) => {
      viewportController.setStandardView(view)
    },
  })
  const activeView = computed(() => viewportState.value.activeView)
  const cameraYaw = computed(() => viewportState.value.camera.yaw)
  const cameraPitch = computed(() => viewportState.value.camera.pitch)
  const displayMode = ref<DisplayMode>('shaded')
  /** Scene-level default shading model for entities without their own material. */
  const shadingModel = ref<ShadingModel>('phong')
  /** Active matcap capture preset for the matcap shading model. */
  const matcapId = ref<string>(DEFAULT_MATCAP.id)
  /** Active environment-map preset for the PBR shading model (IBL). */
  const envId = ref<string>(DEFAULT_ENV.id)
  /** Key-light contact shadows (mesh surfaces + grid floor); default off. */
  const shadowsEnabled = ref(false)
  /** Active render theme preset id (Scene uniform theme tail). */
  const renderThemeId = ref<string>('default')
  /** Scene-level default material tail for entities without their own material. */
  const defaultBaseColor = ref('#ffffff')
  const defaultMetallic = ref(0)
  const defaultRoughness = ref(0.7)
  /** Preset-only material channels (no slider): kept so re-init and recovery restore them. */
  const defaultEmissive = ref<[number, number, number]>([0, 0, 0])
  const defaultAlpha = ref(1)
  const selectionMode = ref<SelectionMode>('face')

  const sceneRows = computed<SceneMeshRow[]>(() =>
    sceneMeshes.value.map((mesh, index) => {
      const meshId = mesh.entityId ?? index
      const grouped = new Map<string | number, SourceProvenanceRow>()
      for (const run of mesh.provenance) {
        if (!run.source) continue
        const sourceKey = run.source.instanceId ?? run.source.operationId ?? run.source.id
        const existing = grouped.get(sourceKey)
        const triangleCount = run.triangleEnd - run.triangleStart
        if (existing) existing.triangleCount = (existing.triangleCount ?? 0) + triangleCount
        else {
          const location = lineAndColumn(renderedSource.value || code.value, run.source.start)
          grouped.set(sourceKey, {
            id: `${meshId}:${sourceKey}`,
            sourceId: run.source.id,
            label: run.source.label,
            sourceStart: run.source.start,
            sourceEnd: run.source.end,
            line: location.line,
            column: location.column,
            originalId: run.source.originalId,
            triangleCount,
            color: mesh.color,
          })
        }
      }
      return {
        id: meshId,
        name: `${t('object')} ${index + 1}`,
        visible: meshVisibility.value[index] !== false,
        triangleCount: mesh.indices.length / 3,
        color: mesh.color,
        sources: [...grouped.values()],
      }
    }),
  )

  const sceneMeshId = (index: number | null) =>
    index === null ? null : sceneMeshes.value[index]?.entityId ?? index
  const selectedSceneMeshId = computed(() => sceneMeshId(selectedMesh.value))
  const hoveredMesh = computed(() => sceneMeshId(hoveredHit.value?.meshIndex ?? null))

  const currentInspection = computed<InspectSelection | null>(() => {
    const index = selectedMesh.value
    const mesh = index === null ? null : sceneMeshes.value[index]
    if (!mesh || index === null) return null
    const facts = inspectMesh(mesh, index)
    const hit = selectedHit.value?.meshIndex === index ? selectedHit.value : null
    let source: SourceProvenanceRow | undefined
    if (hit?.source) {
      const location = lineAndColumn(renderedSource.value || code.value, hit.source.start)
      source = {
        id: `${mesh.entityId ?? index}:${hit.source.instanceId ?? hit.source.operationId ?? hit.source.originalId}`,
        sourceId: hit.source.id,
        label: hit.source.label,
        sourceStart: hit.source.start,
        sourceEnd: hit.source.end,
        line: location.line,
        column: location.column,
        originalId: hit.source.originalId,
        color: mesh.color,
      }
    }
    return {
      meshId: mesh.entityId ?? index,
      meshName: `${t('object')} ${index + 1}`,
      triangleCount: facts.triangles,
      surfaceArea: sceneMeshes.value.length === 1 ? surfaceArea.value : undefined,
      volume: sceneMeshes.value.length === 1 ? volume.value : undefined,
      position: hit?.point,
      normal: hit?.normal,
      bounds: facts.bounds ?? undefined,
      source,
    }
  })

  const panelMeasurement = computed<PanelMeasurement | null>(() =>
    measurement.value
      ? {
          points: measurement.value.points,
          distance: measurement.value.distance ?? undefined,
        }
      : null,
  )

  function fitView() {
    getRenderer()?.fitView()
  }

  function resetView() {
    viewportController.resetView()
    getRenderer()?.resetView()
  }

  function previousView() {
    const state = getRenderer()?.previousView()
    if (!state) return
    viewportController.applyRestoredCamera(state)
  }

  /** Mirrors the renderer camera into the UI so the view cube stays truthful. */
  function syncCameraState(state: CameraState) {
    mainCameraRevision.value++
    viewportController.applyCamera(state)
    if (viewportController.state.camera.projection !== state.projection) {
      getRenderer()?.setProjection(viewportController.state.camera.projection)
    }
  }

  function focusSelection() {
    const r = getRenderer()
    if (!r?.fitSelection()) r?.fitView()
  }

  function clearSelection() {
    getRenderer()?.clearSelection()
  }

  function toggleIsolate() {
    isolated.value = getRenderer()?.toggleIsolateSelection() ?? false
  }

  function toggleProjection() {
    getRenderer()?.setProjection(viewportController.toggleProjection())
  }

  function toggleGrid() {
    gridVisible.value = !gridVisible.value
    getRenderer()?.setGridVisible(gridVisible.value)
  }

  function changeStandardView() {
    const { view, projection: nextProjection } = viewportController.applyStandardView(
      viewportController.state.standardView,
    )
    getRenderer()?.setCameraPreset(view, nextProjection)
  }

  function setStandardView(view: StandardView) {
    viewportController.setStandardView(view)
    changeStandardView()
  }

  function setDisplayMode(mode: DisplayMode) {
    displayMode.value = mode
    getRenderer()?.setDisplayMode(mode)
  }

  function changeDisplayMode() {
    getRenderer()?.setDisplayMode(displayMode.value)
  }

  function setShadingModel(model: ShadingModel) {
    shadingModel.value = model
    getRenderer()?.setDefaultShadingModel(model)
  }

  function setRenderTheme(themeId: string) {
    renderThemeId.value = themeId
    getRenderer()?.setTheme(themeId)
  }

  /** The full scene default material; renderer (re)initialization applies all of it. */
  function defaultMaterialState() {
    return {
      baseColor: hexToRgb(defaultBaseColor.value),
      metallic: defaultMetallic.value,
      roughness: defaultRoughness.value,
      emissive: defaultEmissive.value,
      alpha: defaultAlpha.value,
    }
  }

  function applyDefaultMaterial() {
    getRenderer()?.setDefaultMaterial(defaultMaterialState())
  }

  function setDefaultBaseColor(hex: string) {
    defaultBaseColor.value = hex
    applyDefaultMaterial()
  }

  function setDefaultMetallic(value: number) {
    defaultMetallic.value = value
    applyDefaultMaterial()
  }

  function setDefaultRoughness(value: number) {
    defaultRoughness.value = value
    applyDefaultMaterial()
  }

  function setMatcap(id: string) {
    matcapId.value = id
    void getRenderer()?.setMatcapTexture(id)?.catch(() => {})
  }

  function setEnv(id: string) {
    envId.value = id
    void getRenderer()?.setEnvMap(id)?.catch(() => {})
  }

  function setShadows(enabled: boolean) {
    shadowsEnabled.value = enabled
    getRenderer()?.setShadowsEnabled(enabled)
  }

  /** Applies a MATERIAL_PRESETS entry to the scene default material controls. */
  function applyMaterialPreset(id: string) {
    const preset = getMaterialPreset(id)
    if (!preset) return
    defaultBaseColor.value =
      '#' +
      preset.baseColor
        .map(channel =>
          Math.round(Math.min(1, Math.max(0, channel)) * 255)
            .toString(16)
            .padStart(2, '0'),
        )
        .join('')
    defaultMetallic.value = preset.metallic
    defaultRoughness.value = preset.roughness
    defaultEmissive.value = [...preset.emissive]
    defaultAlpha.value = preset.alpha ?? 1
    setShadingModel(preset.shadingModel)
    applyDefaultMaterial()
  }

  function setSelectionMode(mode: SelectionMode) {
    selectionMode.value = mode
    getRenderer()?.setSelectionMode(mode)
  }

  function meshIndex(id: number | string) {
    const entityIndex =
      typeof id === 'string' ? sceneMeshes.value.findIndex(mesh => mesh.entityId === id) : -1
    const index = entityIndex >= 0 ? entityIndex : typeof id === 'number' ? id : Number(id)
    return Number.isInteger(index) && index >= 0 && index < sceneMeshes.value.length ? index : null
  }

  function selectSceneMesh(id: number | string) {
    const index = meshIndex(id)
    if (index === null) return
    getRenderer()?.selectMesh(index)
  }

  function setSceneMeshVisibility(id: number | string, visible: boolean) {
    const index = meshIndex(id)
    if (index === null) return
    sceneController.setVisibility(index, visible)
    getRenderer()?.setMeshVisibility(index, visible)
  }

  function preselectSceneMesh(id: number | string | null) {
    getRenderer()?.setPreselection(id === null ? null : meshIndex(id))
  }

  function focusSceneMesh(id: number | string) {
    const index = meshIndex(id)
    if (index === null) return
    const r = getRenderer()
    r?.selectMesh(index)
    r?.fitSelection()
  }

  function isolateSceneMesh(id: number | string) {
    const index = meshIndex(id)
    if (index === null) return
    const r = getRenderer()
    if (selectedMesh.value !== index) r?.selectMesh(index)
    isolated.value = r?.toggleIsolateSelection() ?? false
  }

  function startMeasure() {
    sceneController.setMeasurement(measurement.value, true)
    dockOpen.value = true
    dockTab.value = 'inspect'
    getRenderer()?.setMeasureMode(true)
  }

  function cancelMeasure() {
    sceneController.setMeasurement(measurement.value, false)
    getRenderer()?.setMeasureMode(false)
  }

  function clearMeasurement() {
    getRenderer()?.clearMeasurement()
  }

  async function copyMeasurement(value: PanelMeasurement) {
    if (value.points.length < 2) return
    const [a, b] = value.points
    const dx = b[0] - a[0]
    const dy = b[1] - a[1]
    const dz = b[2] - a[2]
    const distance = value.distance ?? (dx * dx + dy * dy + dz * dz) ** 0.5
    try {
      await navigator.clipboard.writeText(
        `A ${a.join(', ')}\nB ${b.join(', ')}\nDistance ${distance}`,
      )
      showNotice(t('copied'))
    } catch {
      /* Clipboard may be unavailable outside a secure context. */
    }
  }

  return {
    projection,
    gridVisible,
    gridStep,
    standardView,
    activeView,
    cameraYaw,
    cameraPitch,
    displayMode,
    shadingModel,
    matcapId,
    envId,
    shadowsEnabled,
    renderThemeId,
    defaultBaseColor,
    defaultMetallic,
    defaultRoughness,
    defaultEmissive,
    defaultAlpha,
    selectionMode,
    sceneRows,
    selectedSceneMeshId,
    hoveredMesh,
    currentInspection,
    panelMeasurement,
    fitView,
    resetView,
    previousView,
    syncCameraState,
    focusSelection,
    clearSelection,
    toggleIsolate,
    toggleProjection,
    toggleGrid,
    changeStandardView,
    setStandardView,
    setDisplayMode,
    changeDisplayMode,
    setShadingModel,
    setRenderTheme,
    defaultMaterialState,
    applyDefaultMaterial,
    setDefaultBaseColor,
    setDefaultMetallic,
    setDefaultRoughness,
    setMatcap,
    setEnv,
    setShadows,
    applyMaterialPreset,
    setSelectionMode,
    selectSceneMesh,
    setSceneMeshVisibility,
    preselectSceneMesh,
    focusSceneMesh,
    isolateSceneMesh,
    startMeasure,
    cancelMeasure,
    clearMeasurement,
    copyMeasurement,
  }
}
