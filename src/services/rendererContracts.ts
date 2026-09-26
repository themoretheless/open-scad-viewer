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
  /**
   * Optional opacity multiplier routed through Obj.style.x; values below 1
   * move the mesh into the object-sorted transparent draw pass.
   */
  alpha?: number
}

/**
 * Built-in preset materials; `default` reproduces the legacy shading look.
 * Presets with a numeric id keep Obj.materialId at 0 — the field is a spare.
 */
export const MATERIAL_PRESETS: readonly MaterialDef[] = [
  { id: 'default', name: 'Default Plastic', baseColor: [1, 1, 1], metallic: 0, roughness: 0.7, emissive: [0, 0, 0], shadingModel: 'phong' },
  { id: 'brushed-metal', name: 'Brushed Metal', baseColor: [0.92, 0.93, 0.95], metallic: 1, roughness: 0.35, emissive: [0, 0, 0], shadingModel: 'pbr' },
  { id: 'matte', name: 'Matte', baseColor: [1, 1, 1], metallic: 0, roughness: 0.95, emissive: [0, 0, 0], shadingModel: 'pbr' },
  { id: 'emissive', name: 'Emissive', baseColor: [0.1, 0.1, 0.1], metallic: 0, roughness: 0.7, emissive: [1, 0.9, 0.6], shadingModel: 'unlit' },
  { id: 'plastic', name: 'Plastic', baseColor: [0.85, 0.2, 0.15], metallic: 0, roughness: 0.45, emissive: [0, 0, 0], shadingModel: 'pbr' },
  { id: 'rubber', name: 'Rubber', baseColor: [0.09, 0.09, 0.1], metallic: 0, roughness: 0.98, emissive: [0, 0, 0], shadingModel: 'pbr' },
  { id: 'glass', name: 'Glass', baseColor: [0.75, 0.88, 0.95], metallic: 0, roughness: 0.08, emissive: [0, 0, 0], shadingModel: 'pbr', alpha: 0.35 },
  { id: 'anodized-aluminum', name: 'Anodized Aluminum', baseColor: [0.25, 0.45, 0.85], metallic: 1, roughness: 0.45, emissive: [0, 0, 0], shadingModel: 'pbr' },
]
export const DEFAULT_MATERIAL: MaterialDef = MATERIAL_PRESETS[0]

/** Looks up a material preset by id; undefined when unknown. */
export function getMaterialPreset(id: string): MaterialDef | undefined {
  return MATERIAL_PRESETS.find(material => material.id === id)
}

/**
 * Matcap capture preset for the meshMatcap shader. 'procedural' binds a 1×1
 * dummy texture, which selects the shader's procedural fallback path exactly
 * (the pre-texture look); every other preset fetches a PNG capture.
 */
export interface MatcapPreset {
  id: string
  name: string
  /** Fetch URL of the capture PNG; undefined for the procedural fallback. */
  url?: string
}

export const MATCAP_PRESETS: readonly MatcapPreset[] = [
  { id: 'procedural', name: 'Procedural' },
  { id: 'studio', name: 'Studio', url: 'matcaps/studio.png' },
  { id: 'clay', name: 'Clay', url: 'matcaps/clay.png' },
  { id: 'chrome', name: 'Chrome', url: 'matcaps/chrome.png' },
  { id: 'pearl', name: 'Pearl', url: 'matcaps/pearl.png' },
]
export const DEFAULT_MATCAP: MatcapPreset = MATCAP_PRESETS[0]

/** Looks up a matcap preset by id; undefined when unknown. */
export function getMatcapPreset(id: string): MatcapPreset | undefined {
  return MATCAP_PRESETS.find(matcap => matcap.id === id)
}

/**
 * Render theme: the Scene-uniform theme tail (selection/hover/edge/xray/grid/
 * cap colors, see SCENE_UNIFORM_LAYOUT) plus an optional background override.
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
  /** Section-cap fill color (true cap pass + epsilon accent band). */
  capColor: Vec3
  backgroundColor?: Vec3
}

/**
 * Built-in render themes. `default` reproduces the previously hard-coded
 * shader colors exactly, so the out-of-the-box look is unchanged.
 */
export const THEME_PRESETS: readonly RenderTheme[] = [
  { id: 'default', name: 'Default', selectionColor: [1, 0.52, 0.06], hoverColor: [0.12, 0.78, 1], edgeColor: [0.025, 0.03, 0.04], xrayColor: [1, 0.42, 0.06], gridColor: [0.42, 0.42, 0.42], capColor: [0.85, 0.87, 0.9] },
  { id: 'dark-contrast', name: 'Dark Contrast', selectionColor: [1, 0.6, 0], hoverColor: [0.3, 0.9, 1], edgeColor: [0, 0, 0.01], xrayColor: [1, 0.5, 0.1], gridColor: [0.55, 0.55, 0.6], capColor: [0.8, 0.82, 0.88], backgroundColor: [0.05, 0.05, 0.07] },
  { id: 'light', name: 'Light', selectionColor: [0.9, 0.35, 0], hoverColor: [0, 0.5, 0.9], edgeColor: [0.1, 0.1, 0.12], xrayColor: [0.85, 0.3, 0.05], gridColor: [0.6, 0.6, 0.62], capColor: [0.58, 0.6, 0.66], backgroundColor: [0.92, 0.92, 0.94] },
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
