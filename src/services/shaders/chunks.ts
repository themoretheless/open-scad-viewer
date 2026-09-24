/**
 * Shared WGSL chunks and the CPU-side mirror of the object uniform layout.
 *
 * The Obj struct and OBJECT_UNIFORM_LAYOUT are one contract: keep them in
 * sync, and read upload offsets from the layout instead of magic numbers.
 */

export const SCENE_STRUCT = `struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }`

/** Scene struct with additional members (the grid reconstructs rays via inverseVP). */
export function sceneStruct(extraMembers = ''): string {
  return `struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f${extraMembers} }`
}

export const SCENE_BINDING = `@group(0) @binding(0) var<uniform> sc: Scene;`

export const OBJ_STRUCT = `struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f }`

export const OBJ_BINDING = `@group(1) @binding(0) var<uniform> ob: Obj;`

/**
 * CPU-side mirror of Obj: model (16 floats) + nmat (16) + color (4)
 * + style (4) + morph (4). style = (alpha, selected, edge, hovered);
 * morph.x drives the GPU vertex blend.
 */
export const OBJECT_UNIFORM_LAYOUT = {
  floats: 44,
  bytes: 176,
  styleFloatOffset: 36,
  styleByteOffset: 144,
  morphFloatOffset: 40,
  morphByteOffset: 160,
} as const

/** Interleaved position+normal mesh vertex stride in bytes. */
export const MESH_VERTEX_STRIDE = 24
/** Morph source positions-only stride in bytes (vertex slot 1). */
export const MORPH_VERTEX_STRIDE = 12

/** Section-plane discard shared by object fragment shaders. */
export const SECTION_CLIP_WGSL = `if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }`
