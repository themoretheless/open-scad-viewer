import { computed, nextTick, ref, shallowRef, type ComputedRef, type Ref, type ShallowRef, type WritableComputedRef } from 'svet'
import type { GeometryQuality } from '../../core/build'
import { meshTransferables, type MeshData } from '../../core/mesh'
import { AutoBuildScheduler } from '../../app/autoBuildScheduler'
import {
  BuildCoordinator,
  type BuildCoordinatorState,
  type PublishedGeometryBuild,
} from '../../app/buildCoordinator'
import { BuildPerformanceHistory, type BuildPerformanceSample } from '../../app/buildPerformance'
import { planGeometryPublication } from '../../app/buildPromotionPolicy'
import type { CameraState } from '../../render/cameraHistory'
import { diagnosticFromBuildError, type EditorDiagnostic } from '../../language/editorDiagnostics'
import { withSelectionSurfaces } from '../../kernel/meshSurfaceGroups'
import type {
  DisplayMode,
  PickHit,
  RendererLifecycleEvent,
  SelectionMode,
  ShadingModel,
} from '../../render/rendererContracts'
import { RendererRecoveryGate } from '../../render/rendererRecoveryGate'
import type { SceneController } from '../../app/sceneController'
import { planScenePublication } from '../../app/scenePublication'
import type { ViewportController } from '../../render/viewportController'
import { WebGPURenderer } from '../../render/webgpuRenderer'
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
