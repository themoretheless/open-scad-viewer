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
export interface RendererOperationsContext {
  options: UseBuildAndRendererLifecycleOptions
  renderer: WebGPURenderer | null
  mainPreviewActive: boolean
  mainPreserveGroup: boolean
  pendingPerformanceFrame: { frameToken: number; historyToken: number; } | null
  performanceHistory: BuildPerformanceHistory
  performanceSamples: ShallowRef<BuildPerformanceSample[]>
  handleRendererStatus: (instance: WebGPURenderer, event: RendererLifecycleEvent) => void
  rendererRecoveryGate: RendererRecoveryGate
  autoBuildScheduler: AutoBuildScheduler
  buildGeneration: number
  clearSweepViewportPreview: () => void
  nextPerformanceFrameToken: number
  mainEditSelection: { source: string; index: number; } | null
  fitNextRender: boolean
}
