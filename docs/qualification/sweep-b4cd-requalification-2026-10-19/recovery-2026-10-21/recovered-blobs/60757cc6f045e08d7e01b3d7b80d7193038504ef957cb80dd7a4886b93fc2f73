/**
 * Scene- and object-uniform scratch fills for the WebGPU renderer: pure
 * functions over Float32Array scratch space, addressed by the named offsets
 * in SCENE_UNIFORM_LAYOUT / OBJECT_UNIFORM_LAYOUT. No GPU objects here — the
 * caller uploads the scratch to its uniform buffer.
 */
import { lookAt, orthographic, type Mat4, type Vec3 } from './math3d'
import { OBJECT_UNIFORM_LAYOUT, SCENE_UNIFORM_LAYOUT } from './shaders'
import type { MaterialDef, RenderTheme } from './rendererContracts'
import { SHADOW_MAP_SIZE } from './textureResources'

/**
 * Row-major mat4 product (a · b), pure JS: the shadow light VP is rebuilt per
 * frame only when shadows are on, and must not depend on the geometry kernel
 * (math3d.multiply routes through WASM, which is unavailable in unit tests).
 */
function multiplyRowMajor(a: Mat4, b: Mat4): Mat4 {
  const out = new Float32Array(16)
  for (let row = 0; row < 4; row++) {
    for (let column = 0; column < 4; column++) {
      out[row * 4 + column] = a[row * 4] * b[column] + a[row * 4 + 1] * b[4 + column]
        + a[row * 4 + 2] * b[8 + column] + a[row * 4 + 3] * b[12 + column]
    }
  }
  return out
}

/** Minimal bounds shape the shadow light VP needs (center + radius). */
export interface ShadowBounds {
  center: readonly [number, number, number]
  radius: number
}

/**
 * Orthographic key-light view-projection covering the scene bounds, written
 * column-major into `out` at `offset`. Used by both the shadow-map depth
 * pass and the surface/grid shaders sampling it.
 */
export function shadowLightVP(out: Float32Array, offset: number, bounds: ShadowBounds | null, light: readonly [number, number, number]) {
  if (!bounds) {
    out.fill(0, offset, offset + 16)
    out[offset] = out[offset + 5] = out[offset + 10] = out[offset + 15] = 1
    return
  }
  const len = Math.hypot(light[0], light[1], light[2]) || 1
  const L: Vec3 = [light[0] / len, light[1] / len, light[2] / len]
  const radius = Math.max(bounds.radius, 1e-3)
  const extent = radius * 1.25
  const center = bounds.center
  const eye: Vec3 = [center[0] + L[0] * extent * 2, center[1] + L[1] * extent * 2, center[2] + L[2] * extent * 2]
  const view = lookAt(eye, [...center] as Vec3, [0, 0, 1])
  const proj = orthographic(-extent, extent, -extent, extent, 0, extent * 4)
  const vp = multiplyRowMajor(proj, view)
  for (let row = 0; row < 4; row++) {
    for (let column = 0; column < 4; column++) out[offset + column * 4 + row] = vp[row * 4 + column]
  }
}

/** Per-frame inputs for the Scene uniform fill. */
export interface SceneUniformParams {
  viewProjection: Mat4
  inverseVP: Mat4
  eye: readonly [number, number, number] | Vec3
  sectionNormal: Vec3
  sectionOffset: number
  sectionEnabled: boolean
  gridStep: number
  gridExtent: number
  gridFadeDistance: number
  theme: RenderTheme
  /** Shadows sample the map only when enabled, a map exists, and bounds exist. */
  shadowActive: boolean
  bounds: ShadowBounds | null
}

/**
 * Fills the whole Scene uniform scratch (camera VP + inverse, eye, light,
 * ambient, section plane, grid options, theme tail, shadow tail). The shadow
 * tail carries the light VP plus (enabled, texel, bias, strength); enabled 0
 * keeps every sampling shader on the pixel-identical unshadowed path.
 */
export function fillSceneUniforms(sd: Float32Array, params: SceneUniformParams) {
  const { viewProjection, inverseVP, eye } = params
  const vpOff = SCENE_UNIFORM_LAYOUT.vpFloatOffset
  const invOff = SCENE_UNIFORM_LAYOUT.inverseVPFloatOffset
  for (let row = 0; row < 4; row++) {
    for (let column = 0; column < 4; column++) {
      sd[vpOff + column * 4 + row] = viewProjection[row * 4 + column]
      sd[invOff + column * 4 + row] = inverseVP[row * 4 + column]
    }
  }
  const eyeOff = SCENE_UNIFORM_LAYOUT.eyeFloatOffset
  sd[eyeOff] = eye[0]; sd[eyeOff + 1] = eye[1]; sd[eyeOff + 2] = eye[2]; sd[eyeOff + 3] = 1
  const lightOff = SCENE_UNIFORM_LAYOUT.lightFloatOffset
  sd[lightOff] = 0.55; sd[lightOff + 1] = 0.75; sd[lightOff + 2] = 0.45; sd[lightOff + 3] = 0
  const ambientOff = SCENE_UNIFORM_LAYOUT.ambientFloatOffset
  sd[ambientOff] = 0.22; sd[ambientOff + 1] = 0.22; sd[ambientOff + 2] = 0.24; sd[ambientOff + 3] = 1
  const sectionOff = SCENE_UNIFORM_LAYOUT.sectionFloatOffset
  sd[sectionOff] = params.sectionNormal[0]; sd[sectionOff + 1] = params.sectionNormal[1]
  sd[sectionOff + 2] = params.sectionNormal[2]; sd[sectionOff + 3] = params.sectionOffset
  const optionsOff = SCENE_UNIFORM_LAYOUT.optionsFloatOffset
  sd[optionsOff] = params.sectionEnabled ? 1 : 0
  sd[optionsOff + 1] = params.gridStep
  sd[optionsOff + 2] = params.gridExtent
  sd[optionsOff + 3] = params.gridFadeDistance
  // Theme tail (see SCENE_UNIFORM_LAYOUT): vec3 + pad per color.
  const theme = params.theme
  sd.set(theme.selectionColor, SCENE_UNIFORM_LAYOUT.themeFloatOffset)
  sd.set(theme.hoverColor, SCENE_UNIFORM_LAYOUT.hoverFloatOffset)
  sd.set(theme.edgeColor, SCENE_UNIFORM_LAYOUT.edgeFloatOffset)
  sd.set(theme.xrayColor, SCENE_UNIFORM_LAYOUT.xrayFloatOffset)
  sd.set(theme.gridColor, SCENE_UNIFORM_LAYOUT.gridFloatOffset)
  sd.set(theme.capColor, SCENE_UNIFORM_LAYOUT.capFloatOffset)
  if (params.shadowActive) {
    shadowLightVP(sd, SCENE_UNIFORM_LAYOUT.lightVPFloatOffset, params.bounds,
      [sd[lightOff], sd[lightOff + 1], sd[lightOff + 2]])
  } else {
    sd.fill(0, SCENE_UNIFORM_LAYOUT.lightVPFloatOffset, SCENE_UNIFORM_LAYOUT.lightVPFloatOffset + 16)
  }
  sd[SCENE_UNIFORM_LAYOUT.shadowFloatOffset] = params.shadowActive ? 1 : 0
  sd[SCENE_UNIFORM_LAYOUT.shadowFloatOffset + 1] = 1 / SHADOW_MAP_SIZE
  sd[SCENE_UNIFORM_LAYOUT.shadowFloatOffset + 2] = 0.0015
  sd[SCENE_UNIFORM_LAYOUT.shadowFloatOffset + 3] = 1
}

/** Scene-level default material applied to meshes without a MaterialDef. */
export interface DefaultMaterial {
  baseColor: readonly [number, number, number]
  metallic: number
  roughness: number
  emissive: readonly [number, number, number]
  alpha: number
}

/**
 * Fills one entity's Obj uniform scratch: model (row-major source, written
 * column-major), inverse-transpose, color, style (alpha/selected/edge/
 * hovered), morph rest state, and the material tail. morph.x = 1 keeps the
 * zero slot-1 dummy inert: mix(dummy, pos, 1) = pos. Identity material
 * defaults (white base color, non-metal, no emission, roughness 0.7)
 * reproduce the legacy shading look.
 */
export function fillObjectUniform(
  uniform: Float32Array,
  transform: Mat4,
  inverseTransform: Mat4,
  color: readonly number[],
  alpha: number,
  edge: number,
  material: MaterialDef | undefined,
  defaultMaterial: DefaultMaterial,
) {
  for (let row = 0; row < 4; row++) {
    for (let column = 0; column < 4; column++) uniform[column * 4 + row] = transform[row * 4 + column]
  }
  // Column-major bytes for inverse-transpose(row-major model).
  uniform.set(inverseTransform, 16)
  uniform.set(color, 32)
  uniform[36] = alpha; uniform[37] = 0
  uniform[38] = edge; uniform[39] = 0
  // morph.x drives the GPU vertex blend; rest is 1 (target vertices),
  // which keeps the zero slot-1 dummy inert: mix(dummy, pos, 1) = pos.
  uniform[40] = 1; uniform[41] = 0; uniform[42] = 0; uniform[43] = 0
  // Material tail: a provided MaterialDef overrides the identity defaults,
  // and the scene-level default material applies to meshes without one.
  if (material) {
    uniform[44] = material.baseColor[0]; uniform[45] = material.baseColor[1]; uniform[46] = material.baseColor[2]
    uniform[47] = material.metallic
    uniform[48] = material.emissive[0]; uniform[49] = material.emissive[1]; uniform[50] = material.emissive[2]
    uniform[51] = material.roughness
    uniform[52] = 0; uniform[53] = 0; uniform[54] = 0; uniform[55] = 0
  } else {
    const dm = defaultMaterial
    uniform[44] = dm.baseColor[0]; uniform[45] = dm.baseColor[1]; uniform[46] = dm.baseColor[2]
    uniform[47] = dm.metallic
    uniform[48] = dm.emissive[0]; uniform[49] = dm.emissive[1]; uniform[50] = dm.emissive[2]; uniform[51] = dm.roughness
    uniform[52] = 0; uniform[53] = 0; uniform[54] = 0; uniform[55] = 0
  }
}
