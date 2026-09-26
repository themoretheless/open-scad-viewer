/**
 * Shared WGSL chunks and the CPU-side mirror of the uniform layouts.
 *
 * The Obj struct and OBJECT_UNIFORM_LAYOUT are one contract, as are the Scene
 * struct and SCENE_UNIFORM_LAYOUT: keep them in sync, and read upload offsets
 * from the layout instead of magic numbers.
 */

const SCENE_HEAD = 'struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f'
/** Theme tail: sits after inverseVP at float 52, each vec3f 16-byte aligned. */
const SCENE_THEME_TAIL = 'selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f'

/** Canonical themed Scene struct (object shaders and the grid share it). */
export const SCENE_STRUCT = `${SCENE_HEAD}, inverseVP: mat4x4f, ${SCENE_THEME_TAIL} }`

/**
 * Scene struct composer. `extraMembers` insert between the shared head and
 * the inverseVP + theme tail, so offsets of the themed fields stay fixed.
 */
export function sceneStruct(extraMembers = ''): string {
  return `${SCENE_HEAD}${extraMembers}, inverseVP: mat4x4f, ${SCENE_THEME_TAIL} }`
}

/**
 * CPU-side mirror of Scene: vp (16 floats) + eye (4) + light (4) + ambient (4)
 * + section (4) + options (4) + inverseVP (16) + theme block (5 colors ×
 * vec3+pad = 20) = 72 floats = 288 bytes. The theme tail starts at float 52,
 * so legacy offsets are unchanged.
 */
export const SCENE_UNIFORM_LAYOUT = {
  floats: 72,
  bytes: 288,
  /** selectionColor at themeFloatOffset..+2, then hover/edge/xray/grid every 4 floats. */
  themeFloatOffset: 52,
  themeByteOffset: 208,
  hoverFloatOffset: 56,
  edgeFloatOffset: 60,
  xrayFloatOffset: 64,
  gridFloatOffset: 68,
} as const

export const SCENE_BINDING = `@group(0) @binding(0) var<uniform> sc: Scene;`

export const OBJ_STRUCT = `struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }`

export const OBJ_BINDING = `@group(1) @binding(0) var<uniform> ob: Obj;`

/**
 * CPU-side mirror of Obj: model (16 floats) + nmat (16) + color (4)
 * + style (4) + morph (4) + baseColor (3) + metallic (1) + emissive (3)
 * + roughness (1) + materialId (1) + pad (3). style = (alpha, selected,
 * edge, hovered); morph.x drives the GPU vertex blend. The material tail
 * starts at float 44, so legacy offsets are unchanged.
 */
export const OBJECT_UNIFORM_LAYOUT = {
  floats: 56,
  bytes: 224,
  styleFloatOffset: 36,
  styleByteOffset: 144,
  morphFloatOffset: 40,
  morphByteOffset: 160,
  /** baseColor rgb at materialFloatOffset..+2, metallic at +3. */
  materialFloatOffset: 44,
  materialByteOffset: 176,
  metallicFloatOffset: 47,
  /** emissive rgb at emissiveFloatOffset..+2, roughness at +3. */
  emissiveFloatOffset: 48,
  emissiveByteOffset: 192,
  roughnessFloatOffset: 51,
  materialIdFloatOffset: 52,
  materialIdByteOffset: 208,
} as const

/** Interleaved position+normal mesh vertex stride in bytes. */
export const MESH_VERTEX_STRIDE = 24
/** Morph source positions-only stride in bytes (vertex slot 1). */
export const MORPH_VERTEX_STRIDE = 12

/** Section-plane discard shared by object fragment shaders. */
export const SECTION_CLIP_WGSL = `if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }`

/**
 * Cheap section-cap approximation shared by the mesh-surface shaders:
 * fragments just inside the clip plane (within a fixed world-space epsilon)
 * shade flat and unlit with a distinct cap tint to suggest the cut surface.
 * Exact capping (filling the clipped solid) is intentionally not attempted.
 */
export const SECTION_CAP_WGSL = `if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, ob.style.x); }`
