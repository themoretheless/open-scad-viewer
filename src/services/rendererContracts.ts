import type { MeshSourceReference } from '../core/mesh'
import type { CameraState } from './cameraHistory'
import type { Vec3 } from './math3d'
export type { ProjectionMode, StandardView } from './viewportModel'

export type DisplayMode = 'shaded' | 'edges' | 'xray'
export type SelectionMode = 'object' | 'face' | 'point'

/** Shading model a material asks the object shaders to use. */
export type ShadingModel = 'phong' | 'pbr' | 'matcap' | 'toon' | 'unlit'

/**
 * Maps a shading model to its shader-registry id. 'phong' is the default and
 * resolves to the legacy 'mesh' shader, so scenes without materials render
 * exactly as before.
 */
export function resolveMeshShaderId(model: ShadingModel | undefined): string {
  switch (model ?? 'phong') {
    case 'pbr': return 'meshPbr'
    case 'matcap': return 'meshMatcap'
    case 'toon': return 'meshToon'
    case 'unlit': return 'meshUnlit'
    default: return 'mesh'
  }
}

/**
 * Material definition, mirroring the material tail of the Obj uniform
 * (baseColor, metallic, emissive, roughness; see OBJECT_UNIFORM_LAYOUT).
 */
export interface MaterialDef {
  /** Stable identifier, carried to the GPU as Obj.materialId when numeric. */
  id: string
  name: string
  baseColor: Vec3
  /** 0 = dielectric, 1 = metal. */
  metallic: number
  /** 0 = mirror-smooth, 1 = fully rough. */
  roughness: number
  emissive: Vec3
  shadingModel: ShadingModel
}

/** Built-in preset materials; `default` reproduces the legacy shading look. */
export const MATERIAL_PRESETS: readonly MaterialDef[] = [
  { id: 'default', name: 'Default Plastic', baseColor: [1, 1, 1], metallic: 0, roughness: 0.7, emissive: [0, 0, 0], shadingModel: 'phong' },
  { id: 'brushed-metal', name: 'Brushed Metal', baseColor: [0.92, 0.93, 0.95], metallic: 1, roughness: 0.35, emissive: [0, 0, 0], shadingModel: 'pbr' },
  { id: 'matte', name: 'Matte', baseColor: [1, 1, 1], metallic: 0, roughness: 0.95, emissive: [0, 0, 0], shadingModel: 'pbr' },
  { id: 'emissive', name: 'Emissive', baseColor: [0.1, 0.1, 0.1], metallic: 0, roughness: 0.7, emissive: [1, 0.9, 0.6], shadingModel: 'unlit' },
]
export const DEFAULT_MATERIAL: MaterialDef = MATERIAL_PRESETS[0]

/**
 * Render theme: the Scene-uniform theme tail (selection/hover/edge/xray/grid
 * colors, see SCENE_UNIFORM_LAYOUT) plus an optional background override.
 * `backgroundColor` is optional because the app UI theme already owns the
 * canvas clear color via setBackgroundColor; presets that omit it leave that
 * pathway untouched.
 */
export interface RenderTheme {
  id: string
  name: string
  selectionColor: Vec3
  hoverColor: Vec3
  edgeColor: Vec3
  xrayColor: Vec3
  gridColor: Vec3
  backgroundColor?: Vec3
}

/**
 * Built-in render themes. `default` reproduces the previously hard-coded
 * shader colors exactly, so the out-of-the-box look is unchanged.
 */
export const THEME_PRESETS: readonly RenderTheme[] = [
  { id: 'default', name: 'Default', selectionColor: [1, 0.52, 0.06], hoverColor: [0.12, 0.78, 1], edgeColor: [0.025, 0.03, 0.04], xrayColor: [1, 0.42, 0.06], gridColor: [0.42, 0.42, 0.42] },
  { id: 'dark-contrast', name: 'Dark Contrast', selectionColor: [1, 0.6, 0], hoverColor: [0.3, 0.9, 1], edgeColor: [0, 0, 0.01], xrayColor: [1, 0.5, 0.1], gridColor: [0.55, 0.55, 0.6], backgroundColor: [0.05, 0.05, 0.07] },
  { id: 'light', name: 'Light', selectionColor: [0.9, 0.35, 0], hoverColor: [0, 0.5, 0.9], edgeColor: [0.1, 0.1, 0.12], xrayColor: [0.85, 0.3, 0.05], gridColor: [0.6, 0.6, 0.62], backgroundColor: [0.92, 0.92, 0.94] },
]
export const DEFAULT_THEME: RenderTheme = THEME_PRESETS[0]

/** Looks up a render theme preset by id; undefined when unknown. */
export function getThemePreset(id: string): RenderTheme | undefined {
  return THEME_PRESETS.find(theme => theme.id === id)
}

export interface PickHit {
  meshIndex: number
  triangleIndex: number
  faceId: number | null
  point: Vec3
  normal: Vec3
  barycentric: Vec3
  source: MeshSourceReference | null
  backside: boolean
  cycleIndex?: number
  cycleCount?: number
}

export interface DistanceMeasurement {
  points: Vec3[]
  distance: number | null
}

export type SelectionChangeHandler = (selectedIndex: number | null, isIsolated: boolean, hit: PickHit | null) => void
export type HoverChangeHandler = (hit: PickHit | null) => void
export type MeasurementChangeHandler = (measurement: DistanceMeasurement | null, active: boolean) => void
export type CameraHistoryChangeHandler = (canGoBack: boolean) => void
export type CameraChangeHandler = (state: CameraState) => void

export type RendererLifecycleEvent =
  | { readonly status: 'idle' }
  | { readonly status: 'initializing' }
  | { readonly status: 'ready' }
  | { readonly status: 'unavailable'; readonly reason: 'webgpu' | 'adapter' | 'context'; readonly message: string }
  | { readonly status: 'device-lost'; readonly reason: 'destroyed' | 'unknown'; readonly message: string }
  | { readonly status: 'error'; readonly phase: 'initialization' | 'frame'; readonly error: Error }
  | { readonly status: 'destroyed' }

export type RendererStatusChangeHandler = (event: RendererLifecycleEvent) => void

export interface SetMeshesOptions {
  /** Animate compatible parameter edits in display buffers only. */
  animate?: boolean
  preserveMeasurement?: boolean
  /** Opaque publication token, echoed only after its first GPU submission. */
  frameToken?: number
}

/** CPU-side publication counters, not GPU execution time. */
export interface SceneUploadMetrics {
  geometryUploadBytes: number
  geometryBuffersCreated: number
  reusedEntities: number
}
