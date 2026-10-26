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

interface RendererRecoveryNavigationSnapshot {
  camera: CameraState
  history: ReturnType<WebGPURenderer['getCameraHistorySnapshot']>
}

export function useBuildAndRendererLifecycle(options: UseBuildAndRendererLifecycleOptions) {
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

  async function initializeViewportRenderer() {
    const canvas = options.canvasRef.value
    if (!canvas || !options.isComponentActive() || !options.sourceViewportVisible.value || options.rendererInitializing.value) return
    options.rendererInitializing.value = true
    options.rendererUnavailableMessage.value = ''
    const nextRenderer = new WebGPURenderer()
    const ok = await nextRenderer.init(canvas)
    if (!options.isComponentActive() || !options.sourceViewportVisible.value) {
      nextRenderer.destroy()
      options.rendererInitializing.value = false
      return
    }
    if (!ok) {
      const status = nextRenderer.currentStatus
      options.rendererUnavailableMessage.value = status.status === 'unavailable'
        ? status.message
        : status.status === 'error' ? status.error.message : options.translate('noGpu')
      nextRenderer.destroy()
      renderer = null
      options.gpuOk.value = false
      options.rendererInitializing.value = false
      return
    }

    renderer?.destroy()
    renderer = nextRenderer
    bindRendererCallbacks(nextRenderer)
    nextRenderer.setDisplayMode(options.displayMode.value)
    nextRenderer.setDefaultShadingModel(options.shadingModel.value)
    nextRenderer.setDefaultMaterial(options.defaultMaterialState())
    void nextRenderer.setMatcapTexture(options.matcapId.value).catch(() => {})
    void nextRenderer.setEnvMap(options.envId.value).catch(() => {})
    nextRenderer.setShadowsEnabled(options.shadowsEnabled.value)
    nextRenderer.setBackgroundColor(options.effectiveCanvasColor())
    // Applied after the app-theme background so a theme backgroundColor wins.
    nextRenderer.setTheme(options.renderThemeId.value)
    nextRenderer.setSelectionMode(options.selectionMode.value)
    nextRenderer.setGridVisible(options.gridVisible.value)
    nextRenderer.setGridStep(options.gridStep.value)
    if (options.sceneMeshes.value.length) {
      nextRenderer.setMeshes(options.sceneMeshes.value)
      nextRenderer.setMeshVisibilityBatch(options.meshVisibility.value)
      if (options.selectedMesh.value !== null) nextRenderer.selectMesh(options.selectedMesh.value)
      options.applySection()
      options.syncSourceHighlightFromEditor()
      nextRenderer.fitView()
    }
    options.gpuOk.value = true
    options.rendererInitializing.value = false
  }

  function bindRendererCallbacks(instance: WebGPURenderer) {
    instance.onSelectionChange = (index, isIsolated, hit) => {
      if (options.viewportController.isRecovering || mainPreviewActive) return
      if (options.mainShiftSelection.value && index !== null) {
        const ids = new Set(options.mainSelectedIndices.value)
        ids.has(index) ? ids.delete(index) : ids.add(index)
        options.mainSelectedIndices.value = [...ids]
        if (!ids.has(index)) {
          index = options.mainSelectedIndices.value.at(-1) ?? null
          hit = null
        }
      } else if (!mainPreserveGroup) {
        options.mainSelectedIndices.value = index === null ? [] : [index]
      }
      options.mainShiftSelection.value = false
      options.sceneController.applyRendererSelection(index, isIsolated, hit)
    }
    instance.onHoverChange = hit => {
      if (options.viewportController.isRecovering || mainPreviewActive) return
      options.sceneController.applyRendererHover(hit)
    }
    instance.onMeasurementChange = (value, active) => {
      if (options.viewportController.isRecovering || mainPreviewActive) return
      options.sceneController.applyRendererMeasurement(value, active)
    }
    instance.onCameraHistoryChange = available => {
      options.viewportController.applyHistoryAvailability(available)
    }
    instance.onFrameSubmitted = (token, at) => {
      if (instance !== renderer) return
      if (!pendingPerformanceFrame || pendingPerformanceFrame.frameToken !== token) return
      if (performanceHistory.submitted(pendingPerformanceFrame.historyToken, at)) {
        performanceSamples.value = performanceHistory.snapshot()
      }
    }
    instance.onCameraChange = options.syncCameraState
    instance.onStatusChange = event => handleRendererStatus(instance, event)
    options.viewportController.applyHistoryAvailability(instance.canGoToPreviousView)
    options.syncCameraState(instance.getCameraState())
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

  async function recoverRenderer(
    instance: WebGPURenderer,
    navigation: RendererRecoveryNavigationSnapshot = {
      camera: instance.getCameraState(),
      history: instance.getCameraHistorySnapshot(),
    },
  ) {
    const canvas = options.canvasRef.value
    if (!canvas || renderer !== instance) {
      rendererRecoveryGate.reset()
      return
    }
    const token = options.viewportController.beginRecovery()
    // Triangle identities and hover ownership are renderer-local. Object
    // selection can be restored by scene identity, but stale surface hits cannot.
    options.selectedHit.value = null
    options.sceneController.applyRendererHover(null)
    let ready = false
    let failureMessage = options.translate('gpuRecoverFailed')

    try {
      const recovered = await instance.init(canvas)
      if (!options.viewportController.isCurrentRecovery(token) || renderer !== instance) return
      if (!recovered) return
      if (instance.currentStatus.status === 'device-lost' && !rendererRecoveryGate.mustDeferReady) {
        rendererRecoveryGate.registerDeviceLoss()
      }
      if (rendererRecoveryGate.mustDeferReady) return

      instance.setDisplayMode(options.displayMode.value)
      instance.setDefaultShadingModel(options.shadingModel.value)
      instance.setDefaultMaterial(options.defaultMaterialState())
      void instance.setMatcapTexture(options.matcapId.value).catch(() => {})
      void instance.setEnvMap(options.envId.value).catch(() => {})
      instance.setShadowsEnabled(options.shadowsEnabled.value)
      instance.setBackgroundColor(options.effectiveCanvasColor())
      instance.setTheme(options.renderThemeId.value)
      instance.setSelectionMode(options.selectionMode.value)
      instance.setGridVisible(options.gridVisible.value)
      instance.setGridStep(options.gridStep.value)
      // Builds can complete while adapter/device acquisition is pending. The CPU
      // scene and Vue state are authoritative, so re-read them after the await.
      const currentVisibility = [...options.meshVisibility.value]
      const selectionCandidate = options.selectedMesh.value
      const currentSelection = selectionCandidate !== null
        && Number.isInteger(selectionCandidate)
        && selectionCandidate >= 0
        && selectionCandidate < options.sceneMeshes.value.length
        && currentVisibility[selectionCandidate] !== false
        ? selectionCandidate
        : null
      const currentIsolation = currentSelection !== null && options.isolated.value
      const currentMeasurement = options.measurement.value
      const currentMeasureActive = options.measureActive.value
      options.selectedMesh.value = currentSelection
      options.isolated.value = currentIsolation
      if (options.sceneMeshes.value.length) {
        instance.setMeshes(options.sceneMeshes.value)
        instance.setMeshVisibilityBatch(currentVisibility)
        if (currentSelection !== null) {
          instance.selectMesh(currentSelection)
          if (currentIsolation) instance.toggleIsolateSelection()
        }
      }
      instance.restoreCameraState(navigation.camera)
      if (!instance.restoreCameraHistory(navigation.history)) throw new Error(options.translate('gpuRecoverFailed'))
      instance.restoreMeasurement(currentMeasurement, currentMeasureActive)
      options.applySection()
      options.syncSourceHighlightFromEditor()
      if (instance.currentStatus.status === 'device-lost' && !rendererRecoveryGate.mustDeferReady) {
        rendererRecoveryGate.registerDeviceLoss()
      }
      if (!rendererRecoveryGate.mustDeferReady) ready = true
    } catch (caught) {
      failureMessage = caught instanceof Error ? caught.message : options.translate('gpuRecoverFailed')
    } finally {
      options.viewportController.completeRecovery(token)
      if (!options.viewportController.isCurrentRecovery(token) || renderer !== instance) return

      const completion = rendererRecoveryGate.completeAttempt()
      if (completion === 'retry') {
        void recoverRenderer(instance, navigation)
        return
      }

      if (!ready || completion === 'exhausted' || instance.currentStatus.status === 'device-lost') {
        options.gpuOk.value = false
        options.error.value = failureMessage
        return
      }

      options.selectedHit.value = null
      options.sceneController.applyRendererHover(null)
      options.gpuOk.value = true
      if (options.error.value === options.translate('gpuRecoverFailed')) options.error.value = ''
      options.showNotice(options.translate('gpuRecovered'))
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

  function handleGeometryResponse(response: PublishedGeometryBuild) {
    if (response.status === 'succeeded') autoBuildScheduler.observe(response.quality, response.durationMs)
    // The editor revision advances before its debounced preview is submitted.
    // Never publish an older build during that window, even if the coordinator
    // has not seen the replacement job yet.
    if (response.documentRevision !== buildGeneration) return
    clearSweepViewportPreview()
    options.renderDuration.value = response.durationMs
    const source = buildSources.get(response.documentRevision) ?? options.code.value
    for (const revision of buildSources.keys()) {
      if (revision < response.documentRevision) buildSources.delete(revision)
    }

    if (response.status === 'failed') {
      options.error.value = response.error.code === 'WORKER_TIMEOUT' ? options.translate('buildTimeout') : response.error.message
      options.editorDiagnostic.value = diagnosticFromBuildError(source, response.error)
      return
    }

    const publicationPlan = planGeometryPublication({
      buildGeneration: response.documentRevision,
      source,
      quality: response.quality,
      reduced: response.reduced,
    }, { buildGeneration, source: options.code.value })
    if (!publicationPlan.publish) return
    const effectiveQuality = publicationPlan.effectiveQuality

    try {
      const publicationStartedAt = performance.now()
      const displayMeshes = response.meshes.map(withSelectionSurfaces)
      options.mainSelectedIndices.value = []
      const sameSourceSnapshot = options.renderedSource.value !== '' && options.renderedSource.value === source
      const previousFaceHit = renderer?.currentHit
      const previousMeshes = options.sceneMeshes.value
      const previousVisibility = options.meshVisibility.value
      const previousMeasurement = options.measurement.value
      const previousMeasureActive = options.measureActive.value
      const publication = planScenePublication({
        previousMeshes,
        previousVisibility,
        previousSelectedIndex: options.selectedMesh.value,
        previousIsolated: options.isolated.value,
        nextMeshes: displayMeshes,
        sameSourceSnapshot,
      })

      const frameToken = nextPerformanceFrameToken++
      renderer?.setMeshes(displayMeshes, {
        frameToken,
        animate: options.renderedSource.value !== '' && !sameSourceSnapshot,
        preserveMeasurement: publication.measurementMayBePreserved,
      })
      options.sceneController.publish({
        meshes: displayMeshes,
        visibility: publication.nextVisibility,
        selectedIndex: publication.nextSelectedIndex,
        isolated: publication.nextIsolated,
      })
      renderer?.setMeshVisibilityBatch(options.meshVisibility.value)
      options.sceneController.applyRendererHover(null)
      if (publication.nextSelectedIndex !== null) {
        const faceRestored = previousFaceHit && previousMeshes[previousFaceHit.meshIndex]
          && renderer?.restoreNativeFaceSelection(previousMeshes[previousFaceHit.meshIndex], previousFaceHit, publication.nextSelectedIndex)
        if (!faceRestored) renderer?.selectMesh(publication.nextSelectedIndex)
        if (publication.nextIsolated) renderer?.toggleIsolateSelection()
      } else {
        options.sceneController.applyRendererSelection(null, false, null)
      }
      if (mainEditSelection?.source === source) {
        const index = mainEditSelection.index < 0
          ? displayMeshes.length - 1
          : Math.min(mainEditSelection.index, displayMeshes.length - 1)
        mainEditSelection = null
        if (index >= 0) renderer?.selectMesh(index)
      }
      if (publication.measurementMayBePreserved) {
        // The renderer may rebuild this overlay; retaining the UI value avoids a
        // preview → full flash and lets it restore the exact world-space points.
        options.sceneController.setMeasurement(previousMeasurement, previousMeasureActive)
      } else {
        options.sceneController.setMeasurement(null, false)
      }
      if (fitNextRender) {
        renderer?.fitView()
        fitNextRender = false
      }
      options.meshCount.value = response.meshes.length
      options.triangleCount.value = response.meshes.reduce((sum, mesh) => sum + mesh.indices.length / 3, 0)
      options.volume.value = response.volume
      options.surfaceArea.value = response.surfaceArea
      options.warnings.value = response.warnings
      options.renderedSource.value = source
      options.renderedQuality.value = effectiveQuality
      options.syncSourceHighlightFromEditor()
      options.syncSectionRange()
      const publishedAt = performance.now()
      const historyToken = performanceHistory.record({
        revision: response.documentRevision,
        quality: response.quality,
        phases: response.timings,
        workerMs: response.durationMs,
        hostMs: response.hostElapsedMs ?? null,
        publicationMs: publishedAt - publicationStartedAt,
        transferBytes: meshTransferables(response.meshes).reduce((sum, buffer) => sum + buffer.byteLength, 0),
        upload: renderer?.currentStatus.status === 'ready' ? renderer.sceneUploadMetrics : null,
      }, publishedAt)
      pendingPerformanceFrame = { frameToken, historyToken }
      performanceSamples.value = performanceHistory.snapshot()
      // Auto builds are sequential: request full only after preview proves it
      // used a reduced quality decision. Equivalent models therefore compile
      // exactly once and reduced models retain the warm Worker for their full
      // continuation.
      if (publicationPlan.requestFull && options.autoRender.value) autoBuildScheduler.promote()
    } catch (caught) {
      options.error.value = caught instanceof Error ? caught.message : String(caught)
    }
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
