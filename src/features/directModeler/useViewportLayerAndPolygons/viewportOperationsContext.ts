import type {DirectMaterial} from '../../../app/directModeling'
import {
  type ComputedRef,
  type Ref,
} from 'svet'
import { SolidDisplayQueue } from '../../../kernel/solidDisplayQueue'
import { SolidGpuLayer } from '../../../kernel/solidGpuView'
import type {UseViewportLayerAndPolygonsOptions} from '../useViewportLayerAndPolygons'
export interface ViewportOperationsContext {
  fpsPrevious: number
  fpsIntervals: number[]
  fpsSince: number
  fps: Ref<number, number>
  frameMs: Ref<number, number>
  drawMs: Ref<number, number>
  gpuLayer: SolidGpuLayer | null
  fpsHandle: number
  dragNodes: SVGElement[]
  dragOffsetIds: string[]
  settling: Ref<boolean, boolean>
  settleHandle: number
  displayVersion: Ref<number, number>
  options: UseViewportLayerAndPolygonsOptions
  smoothDisplay: Ref<boolean, boolean>
  displayQueue: SolidDisplayQueue
  gpuGeneration: number
  gpuResize: ResizeObserver | null
  gpuActive: Ref<boolean, boolean>
  gpuInitializing: Ref<boolean, boolean>
  gpuCanvas: HTMLCanvasElement | null
  gpuRetryVisible: Ref<boolean, boolean>
  gpuFailureNotice: Ref<string, string>
  gpuInitStarted: boolean
  cancelDisplayPreparation: () => void
  displayGeneration: number
  displayPending: Ref<boolean, boolean>
  polygons: ComputedRef<{ preview: number; surface: boolean; normal: number[]; closed: boolean; vertices: [number, number, number][]; id: string; material: DirectMaterial | undefined; triangle: number; key: string; points: string; shade: number; depth: number; }[]>
}
