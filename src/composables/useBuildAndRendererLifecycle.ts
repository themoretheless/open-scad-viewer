import {createRendererOperations} from './useBuildAndRendererLifecycle/rendererOperations'
import { computed, nextTick, ref, shallowRef, type ComputedRef, type Ref, type ShallowRef, type WritableComputedRef } from 'vue'
import type { GeometryQuality } from '../core/build'
import { meshTransferables, type MeshData } from '../core/mesh'
import { AutoBuildScheduler } from '../services/autoBuildScheduler'
import {
  BuildCoordinator,
  type BuildCoordinatorState,
  type PublishedGeometryBuild,
} from '../services/buildCoordinator'
import { BuildPerformanceHistory, type BuildPerformanceSample } from '../services/buildPerformance'
import { planGeometryPublication } from '../services/buildPromotionPolicy'
import type { CameraState } from '../services/cameraHistory'
import { diagnosticFromBuildError, type EditorDiagnostic } from '../services/editorDiagnostics'
import { withSelectionSurfaces } from '../services/meshSurfaceGroups'
import type {
  DisplayMode,
  PickHit,
  RendererLifecycleEvent,
  SelectionMode,
  ShadingModel,
} from '../services/rendererContracts'
import { RendererRecoveryGate } from '../services/rendererRecoveryGate'
import type { SceneController } from '../services/sceneController'
import { planScenePublication } from '../services/scenePublication'
import type { ViewportController } from '../services/viewportController'
import { WebGPURenderer } from '../services/webgpuRenderer'
import { buildSources } from '../appHelpers'
import type { Language } from '../i18n/appStrings'

export interface SweepViewportProgressSnapshot {
  sections: number
  deviation: number
  budget: number
  profileRegularityCertified?: boolean
  wallRegularityCertified?: boolean | null
  certifiedErrorUpper?: number | null
  continuousErrorUpper?: number
  phaseResolved?: boolean
  frameTransportCertified?: boolean
}

export interface UseBuildAndRendererLifecycleOptions {
  canvasRef: Ref<HTMLCanvasElement | null>
  code: Ref<string>
  lang: Ref<Language>
  autoRender: Ref<boolean>
  sourceViewportVisible: ComputedRef<boolean>
  viewportController: ViewportController
  sceneController: SceneController<PickHit>
  sceneMeshes: WritableComputedRef<MeshData[]>
  meshVisibility: WritableComputedRef<boolean[]>
  selectedMesh: WritableComputedRef<number | null>
  selectedHit: WritableComputedRef<PickHit | null>
  isolated: WritableComputedRef<boolean>
  measurement: ComputedRef<PickHit['point'] extends infer _ ? any : any>
  measureActive: ComputedRef<boolean>
  displayMode: Ref<DisplayMode>
  shadingModel: Ref<ShadingModel>
  matcapId: Ref<string>
  envId: Ref<string>
  shadowsEnabled: Ref<boolean>
  renderThemeId: Ref<string>
  selectionMode: Ref<SelectionMode>
  gridVisible: Ref<boolean>
  gridStep: Ref<number>
  error: Ref<string>
  editorDiagnostic: Ref<EditorDiagnostic | null>
  warnings: Ref<string[]>
  meshCount: Ref<number>
  triangleCount: Ref<number>
  volume: Ref<number>
  surfaceArea: Ref<number>
  renderDuration: Ref<number>
  rendering: Ref<boolean>
  renderingQuality: Ref<GeometryQuality>
  renderedQuality: Ref<GeometryQuality>
  renderedSource: Ref<string>
  gpuOk: Ref<boolean>
  rendererInitializing: Ref<boolean>
  rendererUnavailableMessage: Ref<string>
  mainSelectedIndices: Ref<number[]>
  mainShiftSelection: Ref<boolean>
  sweepViewportProgress: Ref<SweepViewportProgressSnapshot | null>
  defaultMaterialState: () => Parameters<WebGPURenderer['setDefaultMaterial']>[0]
  effectiveCanvasColor: () => [number, number, number]
  applySection: () => void
  syncSectionRange: () => void
  syncSourceHighlightFromEditor: () => void
  syncCameraState: (camera: CameraState) => void
  isComponentActive: () => boolean
  showNotice: (message: string) => void
  translate: (key: string) => string
}

export interface RendererRecoveryNavigationSnapshot {
  camera: CameraState
  history: ReturnType<WebGPURenderer['getCameraHistorySnapshot']>
}

export function useBuildAndRendererLifecycle(options: UseBuildAndRendererLifecycleOptions) {
  const {initializeViewportRenderer, bindRendererCallbacks, recoverRenderer, handleGeometryResponse} = createRendererOperations({
    get options() { return options },
    get renderer() { return renderer },
    set renderer(value: typeof renderer) { renderer = value },
    get mainPreviewActive() { return mainPreviewActive },
    set mainPreviewActive(value: typeof mainPreviewActive) { mainPreviewActive = value },
    get mainPreserveGroup() { return mainPreserveGroup },
    set mainPreserveGroup(value: typeof mainPreserveGroup) { mainPreserveGroup = value },
    get pendingPerformanceFrame() { return pendingPerformanceFrame },
    set pendingPerformanceFrame(value: typeof pendingPerformanceFrame) { pendingPerformanceFrame = value },
    get performanceHistory() { return performanceHistory },
    get performanceSamples() { return performanceSamples },
    get handleRendererStatus() { return handleRendererStatus },
    get rendererRecoveryGate() { return rendererRecoveryGate },
    get autoBuildScheduler() { return autoBuildScheduler },
    get buildGeneration() { return buildGeneration },
    set buildGeneration(value: typeof buildGeneration) { buildGeneration = value },
    get clearSweepViewportPreview() { return clearSweepViewportPreview },
    get nextPerformanceFrameToken() { return nextPerformanceFrameToken },
    set nextPerformanceFrameToken(value: typeof nextPerformanceFrameToken) { nextPerformanceFrameToken = value },
    get mainEditSelection() { return mainEditSelection },
    set mainEditSelection(value: typeof mainEditSelection) { mainEditSelection = value },
    get fitNextRender() { return fitNextRender },
    set fitNextRender(value: typeof fitNextRender) { fitNextRender = value },
  })

  let renderer: WebGPURenderer | null = null
  let buildCoordinator: BuildCoordinator | null = null
  let buildGeneration = 0
  let fitNextRender = false
  let mainPreserveGroup = false
  let mainPreviewActive = false
  let sweepViewportActive = false
  let mainEditSelection: { source: string; index: number } | null = null
  let nextPerformanceFrameToken = 1
  let previousBuildCounters = { builds: 0, superseded: 0, workerStarts: 0, hardRestarts: 0 }
  let pendingPerformanceFrame: { frameToken: number; historyToken: number } | null = null
  let rendererErrorMessage = ''

  const rendererRecoveryGate = new RendererRecoveryGate()
  const performanceHistory = new BuildPerformanceHistory()
  const performanceSamples: ShallowRef<BuildPerformanceSample[]> = shallowRef([])
  const buildCounters = shallowRef({ builds: 0, superseded: 0, workerStarts: 0, hardRestarts: 0 })

  const performanceLast = computed(() => performanceSamples.value.at(-1))
  const performanceMs = (value: number | null | undefined) => value == null ? '—' : `${value.toFixed(1)} ms`
  const performanceRows = computed(() => {
    const sample = performanceSamples.value.at(-1)
    if (!sample) return []
    return [
      [options.lang.value === 'ru' ? 'Разбор исходника' : 'Parse', performanceMs(sample.phases.parseMs)],
      [options.lang.value === 'ru' ? 'Инициализация ядра' : 'Kernel initialization', performanceMs(sample.phases.initializeMs)],
      [options.lang.value === 'ru' ? 'Вычисление геометрии' : 'Geometry evaluation', performanceMs(sample.phases.evaluateMs)],
      [options.lang.value === 'ru' ? 'Анализ геометрии' : 'Geometry analysis', performanceMs(sample.phases.analyzeMs)],
      [options.lang.value === 'ru' ? 'Вся работа в Worker' : 'Worker total', performanceMs(sample.workerMs)],
      [options.lang.value === 'ru' ? 'Запрос → проверенный результат' : 'Request → validated result', performanceMs(sample.hostMs)],
      [options.lang.value === 'ru' ? 'Публикация сцены (CPU)' : 'Scene publication (CPU)', performanceMs(sample.publicationMs)],
      [options.lang.value === 'ru' ? 'Публикация → отправка кадра' : 'Publication → frame submission', performanceMs(sample.publicationToSubmitMs)],
      [options.lang.value === 'ru' ? 'Правка → отправка кадра' : 'Edit → frame submission', performanceMs(sample.editToSubmitMs)],
      [options.lang.value === 'ru' ? 'Переданные буферы' : 'Transferred buffers', `${(sample.transferBytes / 1024).toFixed(1)} KiB`],
      [options.lang.value === 'ru' ? 'Загрузка вершин и индексов на GPU' : 'Vertex/index GPU upload', sample.upload ? `${(sample.upload.geometryUploadBytes / 1024).toFixed(1)} KiB` : '—'],
      [options.lang.value === 'ru' ? 'Новые буферы вершин и индексов' : 'New vertex/index buffers', sample.upload?.geometryBuffersCreated ?? '—'],
      [options.lang.value === 'ru' ? 'Объекты с повторно использованной геометрией' : 'Entities reusing geometry', sample.upload?.reusedEntities ?? '—'],
    ]
  })

  const autoBuildScheduler = new AutoBuildScheduler({
    build: quality => {
      if (options.isComponentActive() && options.autoRender.value) doRender(quality)
    },
    edit: () => renderer?.previewCsgSource(options.code.value),
  })

  function getRenderer(): WebGPURenderer | null {
    return renderer
  }

  function markFitNextRender() {
    fitNextRender = true
  }

  function advanceBuildGeneration(recordEditAt?: number): number {
    buildGeneration++
    if (recordEditAt !== undefined) {
      performanceHistory.edited(buildGeneration, recordEditAt)
    }
    return buildGeneration
  }

  function setMainEditSelection(selection: { source: string; index: number } | null) {
    mainEditSelection = selection
  }

  function mainSelectMany(indices: number[]) {
    options.mainSelectedIndices.value = indices
    mainPreserveGroup = true
    try {
      renderer?.selectMesh(indices[0] ?? null)
    } finally {
      mainPreserveGroup = false
    }
  }

  function previewMainGeometry(meshes: MeshData[] | null) {
    if (meshes) {
      mainPreviewActive = true
      renderer?.setMeshes(meshes)
    } else if (mainPreviewActive) {
      try {
        renderer?.setMeshes(options.sceneMeshes.value)
        renderer?.setMeshVisibilityBatch(options.meshVisibility.value)
        renderer?.selectMesh(options.selectedMesh.value)
      } finally {
        mainPreviewActive = false
      }
    }
  }

  function clearSweepViewportPreview() {
    options.sweepViewportProgress.value = null
    if (sweepViewportActive) {
      previewMainGeometry(null)
      sweepViewportActive = false
    }
  }

  function scheduleRender(delay?: number) {
    autoBuildScheduler.edited(delay === 0)
  }

  function cancelAutoBuild() {
    autoBuildScheduler.cancel()
  }

  function beginParameterGesture(event: PointerEvent) {
    if (options.autoRender.value && event.target instanceof HTMLInputElement && event.target.type === 'range') {
      autoBuildScheduler.beginGesture()
    }
  }

  async function endParameterGesture() {
    // Vue source watchers must advance the revision before submitting the final value.
    await nextTick()
    if (options.isComponentActive() && options.autoRender.value) autoBuildScheduler.endGesture()
  }

  async function commitParameterControl(event: Event) {
    if (!(event.target instanceof HTMLInputElement) || event.target.type !== 'range') return
    await nextTick()
    if (options.isComponentActive() && options.autoRender.value) autoBuildScheduler.flush()
  }

  function handleRendererStatus(instance: WebGPURenderer, event: RendererLifecycleEvent) {
    if (renderer !== instance) return
    if (event.status === 'device-lost') {
      options.showNotice(options.translate('gpuLost'))
      if (rendererRecoveryGate.registerDeviceLoss() === 'start') void recoverRenderer(instance)
    } else if (event.status === 'error') {
      rendererErrorMessage = `${options.translate('rendererError')}: ${event.error.message}`
      if (!options.rendering.value) options.error.value = rendererErrorMessage
    } else if (event.status === 'ready' && rendererErrorMessage && options.error.value === rendererErrorMessage) {
      options.error.value = ''
      rendererErrorMessage = ''
    }
  }

  function destroySourceViewportRenderer() {
    renderer?.destroy()
    renderer = null
    options.gpuOk.value = false
  }

  function disposeRendererAndBuildCoordinator() {
    options.viewportController.invalidateRecovery()
    rendererRecoveryGate.reset()
    autoBuildScheduler.cancel()
    buildCoordinator?.dispose()
    buildCoordinator = null
    if (renderer) {
      renderer.onSelectionChange = null
      renderer.onHoverChange = null
      renderer.onMeasurementChange = null
      renderer.onCameraHistoryChange = null
      renderer.onCameraChange = null
      renderer.onStatusChange = null
      renderer.onFrameSubmitted = null
    }
    renderer?.destroy()
    renderer = null
  }

  function startBuildCoordinator() {
    if (buildCoordinator) return
    buildCoordinator = new BuildCoordinator({
      selectionSurfaces: true,
      workerFactory: () => new Worker(new URL('../workers/geometry.worker.ts', import.meta.url), { type: 'module' }),
      // Real Worker events already own their buffers; skip the same-realm snapshot.
      snapshotEvents: false,
      // The parser cancels cooperatively at its yield points (every ~50ms), so a
      // superseded build normally reports `cancelled` well inside this grace
      // window and the warm worker keeps its initialized WASM.
      // The grace timer stays as the hard boundary for statements that never
      // reach a yield point.
      supersedeGraceMs: 300,
      onPublish: handleGeometryResponse,
      onSweepPreview: preview => {
        if (preview.documentRevision !== buildGeneration || !renderer) return
        if (mainPreviewActive && !sweepViewportActive) return
        previewMainGeometry(preview.meshes)
        sweepViewportActive = true
        options.sweepViewportProgress.value = {
          sections: preview.sections,
          deviation: preview.sampledControlDeviation,
          budget: preview.budget,
          profileRegularityCertified: preview.profileRegularityCertified,
          wallRegularityCertified: preview.wallRegularityCertified,
          certifiedErrorUpper: preview.certifiedErrorUpper,
          continuousErrorUpper: preview.continuousErrorUpper,
          phaseResolved: preview.phaseResolved,
          frameTransportCertified: preview.frameTransportCertified,
        }
      },
      workerSilenceTimeoutMs: 30_000,
      onWorkerRestart: () => options.showNotice(options.translate('workerRestarted')),
      onStateChange: handleBuildState,
    })
  }

  function doRender(quality: GeometryQuality = 'full') {
    autoBuildScheduler.cancelPending()
    try {
      startBuildCoordinator()
      const documentRevision = buildGeneration
      const source = options.code.value
      buildSources.clear()
      buildSources.set(documentRevision, source)
      buildCoordinator!.requestBuild({ documentRevision, source, quality })
    } catch (caught) {
      handleWorkerError(caught)
      return
    }
    options.error.value = ''
    options.editorDiagnostic.value = null
    options.warnings.value = []
  }

  function handleBuildState(state: BuildCoordinatorState) {
    if (state.status !== 'building') clearSweepViewportPreview()
    if (buildCoordinator) {
      const current = buildCoordinator.diagnostics
      buildCounters.value = {
        builds: previousBuildCounters.builds + current.builds,
        superseded: previousBuildCounters.superseded + current.superseded,
        workerStarts: previousBuildCounters.workerStarts + current.workerStarts,
        hardRestarts: previousBuildCounters.hardRestarts + current.hardRestarts,
      }
    }
    options.rendering.value = state.status === 'building'
    autoBuildScheduler.setBuilding(options.rendering.value)
    if (state.requestedQuality) options.renderingQuality.value = state.requestedQuality
    if (!options.rendering.value && rendererErrorMessage && !options.error.value) options.error.value = rendererErrorMessage
  }

  function cancelGeometryBuild() {
    autoBuildScheduler.cancel()
    buildCoordinator?.cancel('user')
    options.showNotice(options.translate('buildCancelled'))
  }

  function handleWorkerError(caught?: unknown) {
    options.rendering.value = false
    buildCoordinator?.dispose()
    previousBuildCounters = { ...buildCounters.value }
    buildCoordinator = null
    options.error.value = caught instanceof Error ? `${options.translate('workerError')}: ${caught.message}` : options.translate('workerError')
  }

  return {
    getRenderer,
    performanceSamples,
    performanceLast,
    performanceMs,
    performanceRows,
    buildCounters,
    markFitNextRender,
    advanceBuildGeneration,
    setMainEditSelection,
    mainSelectMany,
    previewMainGeometry,
    clearSweepViewportPreview,
    scheduleRender,
    cancelAutoBuild,
    beginParameterGesture,
    endParameterGesture,
    commitParameterControl,
    initializeViewportRenderer,
    destroySourceViewportRenderer,
    disposeRendererAndBuildCoordinator,
    startBuildCoordinator,
    doRender,
    cancelGeometryBuild,
  }
}
