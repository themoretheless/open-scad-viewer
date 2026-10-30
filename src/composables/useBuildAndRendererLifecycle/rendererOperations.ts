import { computed, nextTick, ref, shallowRef, type ComputedRef, type Ref, type ShallowRef, type WritableComputedRef } from 'vue'
import type { GeometryQuality } from '../../core/build'
import { meshTransferables, type MeshData } from '../../core/mesh'
import { AutoBuildScheduler } from '../../services/autoBuildScheduler'
import {
  BuildCoordinator,
  type BuildCoordinatorState,
  type PublishedGeometryBuild,
} from '../../services/buildCoordinator'
import { BuildPerformanceHistory, type BuildPerformanceSample } from '../../services/buildPerformance'
import { planGeometryPublication } from '../../services/buildPromotionPolicy'
import type { CameraState } from '../../services/cameraHistory'
import { diagnosticFromBuildError, type EditorDiagnostic } from '../../services/editorDiagnostics'
import { withSelectionSurfaces } from '../../services/meshSurfaceGroups'
import type {
  DisplayMode,
  PickHit,
  RendererLifecycleEvent,
  SelectionMode,
  ShadingModel,
} from '../../services/rendererContracts'
import { RendererRecoveryGate } from '../../services/rendererRecoveryGate'
import type { SceneController } from '../../services/sceneController'
import { planScenePublication } from '../../services/scenePublication'
import type { ViewportController } from '../../services/viewportController'
import { WebGPURenderer } from '../../services/webgpuRenderer'
import { buildSources } from '../../appHelpers'
import type { Language } from '../../i18n/appStrings'
import type {SweepViewportProgressSnapshot,UseBuildAndRendererLifecycleOptions,RendererRecoveryNavigationSnapshot} from '../useBuildAndRendererLifecycle'
import type {RendererOperationsContext} from './rendererOperationsContext'
export function createRendererOperations(context: RendererOperationsContext) {
async function initializeViewportRenderer() {
    const canvas = context.options.canvasRef.value
    if (!canvas || !context.options.isComponentActive() || !context.options.sourceViewportVisible.value || context.options.rendererInitializing.value) return
    context.options.rendererInitializing.value = true
    context.options.rendererUnavailableMessage.value = ''
    const nextRenderer = new WebGPURenderer()
    const ok = await nextRenderer.init(canvas)
    if (!context.options.isComponentActive() || !context.options.sourceViewportVisible.value) {
      nextRenderer.destroy()
      context.options.rendererInitializing.value = false
      return
    }
    if (!ok) {
      const status = nextRenderer.currentStatus
      context.options.rendererUnavailableMessage.value = status.status === 'unavailable'
        ? status.message
        : status.status === 'error' ? status.error.message : context.options.translate('noGpu')
      nextRenderer.destroy()
      context.renderer = null
      context.options.gpuOk.value = false
      context.options.rendererInitializing.value = false
      return
    }

    context.renderer?.destroy()
    context.renderer = nextRenderer
    bindRendererCallbacks(nextRenderer)
    nextRenderer.setDisplayMode(context.options.displayMode.value)
    nextRenderer.setDefaultShadingModel(context.options.shadingModel.value)
    nextRenderer.setDefaultMaterial(context.options.defaultMaterialState())
    void nextRenderer.setMatcapTexture(context.options.matcapId.value).catch(() => {})
    void nextRenderer.setEnvMap(context.options.envId.value).catch(() => {})
    nextRenderer.setShadowsEnabled(context.options.shadowsEnabled.value)
    nextRenderer.setBackgroundColor(context.options.effectiveCanvasColor())
    // Applied after the app-theme background so a theme backgroundColor wins.
    nextRenderer.setTheme(context.options.renderThemeId.value)
    nextRenderer.setSelectionMode(context.options.selectionMode.value)
    nextRenderer.setGridVisible(context.options.gridVisible.value)
    nextRenderer.setGridStep(context.options.gridStep.value)
    if (context.options.sceneMeshes.value.length) {
      nextRenderer.setMeshes(context.options.sceneMeshes.value)
      nextRenderer.setMeshVisibilityBatch(context.options.meshVisibility.value)
      if (context.options.selectedMesh.value !== null) nextRenderer.selectMesh(context.options.selectedMesh.value)
      context.options.applySection()
      context.options.syncSourceHighlightFromEditor()
      nextRenderer.fitView()
    }
    context.options.gpuOk.value = true
    context.options.rendererInitializing.value = false
  }
function bindRendererCallbacks(instance: WebGPURenderer) {
    instance.onSelectionChange = (index, isIsolated, hit) => {
      if (context.options.viewportController.isRecovering || context.mainPreviewActive) return
      if (context.options.mainShiftSelection.value && index !== null) {
        const ids = new Set(context.options.mainSelectedIndices.value)
        ids.has(index) ? ids.delete(index) : ids.add(index)
        context.options.mainSelectedIndices.value = [...ids]
        if (!ids.has(index)) {
          index = context.options.mainSelectedIndices.value.at(-1) ?? null
          hit = null
        }
      } else if (!context.mainPreserveGroup) {
        context.options.mainSelectedIndices.value = index === null ? [] : [index]
      }
      context.options.mainShiftSelection.value = false
      context.options.sceneController.applyRendererSelection(index, isIsolated, hit)
    }
    instance.onHoverChange = hit => {
      if (context.options.viewportController.isRecovering || context.mainPreviewActive) return
      context.options.sceneController.applyRendererHover(hit)
    }
    instance.onMeasurementChange = (value, active) => {
      if (context.options.viewportController.isRecovering || context.mainPreviewActive) return
      context.options.sceneController.applyRendererMeasurement(value, active)
    }
    instance.onCameraHistoryChange = available => {
      context.options.viewportController.applyHistoryAvailability(available)
    }
    instance.onFrameSubmitted = (token, at) => {
      if (instance !== context.renderer) return
      if (!context.pendingPerformanceFrame || context.pendingPerformanceFrame.frameToken !== token) return
      if (context.performanceHistory.submitted(context.pendingPerformanceFrame.historyToken, at)) {
        context.performanceSamples.value = context.performanceHistory.snapshot()
      }
    }
    instance.onCameraChange = context.options.syncCameraState
    instance.onStatusChange = event => context.handleRendererStatus(instance, event)
    context.options.viewportController.applyHistoryAvailability(instance.canGoToPreviousView)
    context.options.syncCameraState(instance.getCameraState())
  }
async function recoverRenderer(
    instance: WebGPURenderer,
    navigation: RendererRecoveryNavigationSnapshot = {
      camera: instance.getCameraState(),
      history: instance.getCameraHistorySnapshot(),
    },
  ) {
    const canvas = context.options.canvasRef.value
    if (!canvas || context.renderer !== instance) {
      context.rendererRecoveryGate.reset()
      return
    }
    const token = context.options.viewportController.beginRecovery()
    // Triangle identities and hover ownership are renderer-local. Object
    // selection can be restored by scene identity, but stale surface hits cannot.
    context.options.selectedHit.value = null
    context.options.sceneController.applyRendererHover(null)
    let ready = false
    let failureMessage = context.options.translate('gpuRecoverFailed')

    try {
      const recovered = await instance.init(canvas)
      if (!context.options.viewportController.isCurrentRecovery(token) || context.renderer !== instance) return
      if (!recovered) return
      if (instance.currentStatus.status === 'device-lost' && !context.rendererRecoveryGate.mustDeferReady) {
        context.rendererRecoveryGate.registerDeviceLoss()
      }
      if (context.rendererRecoveryGate.mustDeferReady) return

      instance.setDisplayMode(context.options.displayMode.value)
      instance.setDefaultShadingModel(context.options.shadingModel.value)
      instance.setDefaultMaterial(context.options.defaultMaterialState())
      void instance.setMatcapTexture(context.options.matcapId.value).catch(() => {})
      void instance.setEnvMap(context.options.envId.value).catch(() => {})
      instance.setShadowsEnabled(context.options.shadowsEnabled.value)
      instance.setBackgroundColor(context.options.effectiveCanvasColor())
      instance.setTheme(context.options.renderThemeId.value)
      instance.setSelectionMode(context.options.selectionMode.value)
      instance.setGridVisible(context.options.gridVisible.value)
      instance.setGridStep(context.options.gridStep.value)
      // Builds can complete while adapter/device acquisition is pending. The CPU
      // scene and Vue state are authoritative, so re-read them after the await.
      const currentVisibility = [...context.options.meshVisibility.value]
      const selectionCandidate = context.options.selectedMesh.value
      const currentSelection = selectionCandidate !== null
        && Number.isInteger(selectionCandidate)
        && selectionCandidate >= 0
        && selectionCandidate < context.options.sceneMeshes.value.length
        && currentVisibility[selectionCandidate] !== false
        ? selectionCandidate
        : null
      const currentIsolation = currentSelection !== null && context.options.isolated.value
      const currentMeasurement = context.options.measurement.value
      const currentMeasureActive = context.options.measureActive.value
      context.options.selectedMesh.value = currentSelection
      context.options.isolated.value = currentIsolation
      if (context.options.sceneMeshes.value.length) {
        instance.setMeshes(context.options.sceneMeshes.value)
        instance.setMeshVisibilityBatch(currentVisibility)
        if (currentSelection !== null) {
          instance.selectMesh(currentSelection)
          if (currentIsolation) instance.toggleIsolateSelection()
        }
      }
      instance.restoreCameraState(navigation.camera)
      if (!instance.restoreCameraHistory(navigation.history)) throw new Error(context.options.translate('gpuRecoverFailed'))
      instance.restoreMeasurement(currentMeasurement, currentMeasureActive)
      context.options.applySection()
      context.options.syncSourceHighlightFromEditor()
      if (instance.currentStatus.status === 'device-lost' && !context.rendererRecoveryGate.mustDeferReady) {
        context.rendererRecoveryGate.registerDeviceLoss()
      }
      if (!context.rendererRecoveryGate.mustDeferReady) ready = true
    } catch (caught) {
      failureMessage = caught instanceof Error ? caught.message : context.options.translate('gpuRecoverFailed')
    } finally {
      context.options.viewportController.completeRecovery(token)
      if (!context.options.viewportController.isCurrentRecovery(token) || context.renderer !== instance) return

      const completion = context.rendererRecoveryGate.completeAttempt()
      if (completion === 'retry') {
        void recoverRenderer(instance, navigation)
        return
      }

      if (!ready || completion === 'exhausted' || instance.currentStatus.status === 'device-lost') {
        context.options.gpuOk.value = false
        context.options.error.value = failureMessage
        return
      }

      context.options.selectedHit.value = null
      context.options.sceneController.applyRendererHover(null)
      context.options.gpuOk.value = true
      if (context.options.error.value === context.options.translate('gpuRecoverFailed')) context.options.error.value = ''
      context.options.showNotice(context.options.translate('gpuRecovered'))
    }
  }
function handleGeometryResponse(response: PublishedGeometryBuild) {
    if (response.status === 'succeeded') context.autoBuildScheduler.observe(response.quality, response.durationMs)
    // The editor revision advances before its debounced preview is submitted.
    // Never publish an older build during that window, even if the coordinator
    // has not seen the replacement job yet.
    if (response.documentRevision !== context.buildGeneration) return
    context.clearSweepViewportPreview()
    context.options.renderDuration.value = response.durationMs
    const source = buildSources.get(response.documentRevision) ?? context.options.code.value
    for (const revision of buildSources.keys()) {
      if (revision < response.documentRevision) buildSources.delete(revision)
    }

    if (response.status === 'failed') {
      context.options.error.value = response.error.code === 'WORKER_TIMEOUT' ? context.options.translate('buildTimeout') : response.error.message
      context.options.editorDiagnostic.value = diagnosticFromBuildError(source, response.error)
      return
    }

    const publicationPlan = planGeometryPublication({
      buildGeneration: response.documentRevision,
      source,
      quality: response.quality,
      reduced: response.reduced,
    }, { buildGeneration: context.buildGeneration, source: context.options.code.value })
    if (!publicationPlan.publish) return
    const effectiveQuality = publicationPlan.effectiveQuality

    try {
      const publicationStartedAt = performance.now()
      const displayMeshes = response.meshes.map(withSelectionSurfaces)
      context.options.mainSelectedIndices.value = []
      const sameSourceSnapshot = context.options.renderedSource.value !== '' && context.options.renderedSource.value === source
      const previousFaceHit = context.renderer?.currentHit
      const previousMeshes = context.options.sceneMeshes.value
      const previousVisibility = context.options.meshVisibility.value
      const previousMeasurement = context.options.measurement.value
      const previousMeasureActive = context.options.measureActive.value
      const publication = planScenePublication({
        previousMeshes,
        previousVisibility,
        previousSelectedIndex: context.options.selectedMesh.value,
        previousIsolated: context.options.isolated.value,
        nextMeshes: displayMeshes,
        sameSourceSnapshot,
      })

      const frameToken = context.nextPerformanceFrameToken++
      context.renderer?.setMeshes(displayMeshes, {
        frameToken,
        animate: context.options.renderedSource.value !== '' && !sameSourceSnapshot,
        preserveMeasurement: publication.measurementMayBePreserved,
      })
      context.options.sceneController.publish({
        meshes: displayMeshes,
        visibility: publication.nextVisibility,
        selectedIndex: publication.nextSelectedIndex,
        isolated: publication.nextIsolated,
      })
      context.renderer?.setMeshVisibilityBatch(context.options.meshVisibility.value)
      context.options.sceneController.applyRendererHover(null)
      if (publication.nextSelectedIndex !== null) {
        const faceRestored = previousFaceHit && previousMeshes[previousFaceHit.meshIndex]
          && context.renderer?.restoreNativeFaceSelection(previousMeshes[previousFaceHit.meshIndex], previousFaceHit, publication.nextSelectedIndex)
        if (!faceRestored) context.renderer?.selectMesh(publication.nextSelectedIndex)
        if (publication.nextIsolated) context.renderer?.toggleIsolateSelection()
      } else {
        context.options.sceneController.applyRendererSelection(null, false, null)
      }
      if (context.mainEditSelection?.source === source) {
        const index = context.mainEditSelection.index < 0
          ? displayMeshes.length - 1
          : Math.min(context.mainEditSelection.index, displayMeshes.length - 1)
        context.mainEditSelection = null
        if (index >= 0) context.renderer?.selectMesh(index)
      }
      if (publication.measurementMayBePreserved) {
        // The renderer may rebuild this overlay; retaining the UI value avoids a
        // preview → full flash and lets it restore the exact world-space points.
        context.options.sceneController.setMeasurement(previousMeasurement, previousMeasureActive)
      } else {
        context.options.sceneController.setMeasurement(null, false)
      }
      if (context.fitNextRender) {
        context.renderer?.fitView()
        context.fitNextRender = false
      }
      context.options.meshCount.value = response.meshes.length
      context.options.triangleCount.value = response.meshes.reduce((sum, mesh) => sum + mesh.indices.length / 3, 0)
      context.options.volume.value = response.volume
      context.options.surfaceArea.value = response.surfaceArea
      context.options.warnings.value = response.warnings
      context.options.renderedSource.value = source
      context.options.renderedQuality.value = effectiveQuality
      context.options.syncSourceHighlightFromEditor()
      context.options.syncSectionRange()
      const publishedAt = performance.now()
      const historyToken = context.performanceHistory.record({
        revision: response.documentRevision,
        quality: response.quality,
        phases: response.timings,
        workerMs: response.durationMs,
        hostMs: response.hostElapsedMs ?? null,
        publicationMs: publishedAt - publicationStartedAt,
        transferBytes: meshTransferables(response.meshes).reduce((sum, buffer) => sum + buffer.byteLength, 0),
        upload: context.renderer?.currentStatus.status === 'ready' ? context.renderer.sceneUploadMetrics : null,
      }, publishedAt)
      context.pendingPerformanceFrame = { frameToken, historyToken }
      context.performanceSamples.value = context.performanceHistory.snapshot()
      // Auto builds are sequential: request full only after preview proves it
      // used a reduced quality decision. Equivalent models therefore compile
      // exactly once and reduced models retain the warm Worker for their full
      // continuation.
      if (publicationPlan.requestFull && context.options.autoRender.value) context.autoBuildScheduler.promote()
    } catch (caught) {
      context.options.error.value = caught instanceof Error ? caught.message : String(caught)
    }
  }
return {initializeViewportRenderer, bindRendererCallbacks, recoverRenderer, handleGeometryResponse}
}
