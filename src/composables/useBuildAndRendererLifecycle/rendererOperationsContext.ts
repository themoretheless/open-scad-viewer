import { type ShallowRef } from 'svet'
import { AutoBuildScheduler } from '../../app/autoBuildScheduler'
import { BuildPerformanceHistory, type BuildPerformanceSample } from '../../app/buildPerformance'
import type {
  RendererLifecycleEvent,
} from '../../render/rendererContracts'
import { RendererRecoveryGate } from '../../render/rendererRecoveryGate'
import { WebGPURenderer } from '../../render/webgpuRenderer'
import type {UseBuildAndRendererLifecycleOptions} from '../useBuildAndRendererLifecycle'
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
