import type {DirectMaterial} from '../../../services/directModeling'
import {
  computed,
  nextTick,
  onUnmounted,
  ref,
  watch,
  watchEffect,
  type ComponentPublicInstance,
  type ComputedRef,
  type Ref,
  type ShallowRef,
} from 'vue'
import type { DirectBody, DirectDocument, Point2 } from '../../../services/directModeling'
import { createDirectProjector, directExtrusionTool, directFaceShade, projectDirectPoint } from '../../../services/directModelingTools'
import type { Vec3 } from '../../../services/directSketchGeometry'
import type { PolygonMesh } from '../../../services/geometry/polygon'
import { SolidDisplayCache, type DisplayMesh } from '../../../services/solidDisplayCache'
import { SolidDisplayQueue } from '../../../services/solidDisplayQueue'
import { isSolidGpuSupported, smoothTriangleList, SolidGpuLayer, type SolidGpuBody } from '../../../services/solidGpuView'
import { solidMaterialRgb } from '../../../services/solidMaterial'
import type { SolidNurbsSurface } from '../../../services/solidNurbs'
import { createSolidPreviewWorker } from '../../../services/solidPreviewWorker'
import type { SolidSurfaceDisplayQueue } from '../../../services/solidSurfaceDisplayQueue'
import { TransparentBsp } from '../../../services/transparentBsp'
import { prepareVrPolygons } from '../../../services/vrScene'
import { COARSE_DISPLAY, type Pane } from '.././helpers'
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
