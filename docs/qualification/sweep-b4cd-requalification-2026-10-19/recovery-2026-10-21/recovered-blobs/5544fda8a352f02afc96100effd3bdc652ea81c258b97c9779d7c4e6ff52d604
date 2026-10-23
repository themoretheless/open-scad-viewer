/**
 * Shared WGSL chunks and the CPU-side mirror of the uniform layouts.
 *
 * Since phase 3 of the shader refactor, the canonical WGSL chunk text lives
 * in Rust (`crates/raster-core/src/chunks.rs`) and the layout values in
 * `crates/raster-core/src/layout.rs`; the `wgsl_export` codegen emits them
 * into `generated/sources.ts` and `generated/layouts.ts`. This module only
 * re-exports those generated constants (plus the `sceneStruct` composer), so
 * the WGSL text and the upload offsets each have exactly one source of truth.
 */

import { SCENE_STRUCT } from './generated/sources'

export {
  SCENE_STRUCT,
  OBJ_STRUCT,
  SECTION_CLIP_WGSL,
  SECTION_CAP_WGSL,
} from './generated/sources'
export { SCENE_UNIFORM_LAYOUT, OBJECT_UNIFORM_LAYOUT } from './generated/layouts'

/**
 * Scene struct composer. `extraMembers` insert between the shared head and
 * the inverseVP + theme tail, so offsets of the themed fields stay fixed.
 */
export function sceneStruct(extraMembers = ''): string {
  if (!extraMembers) return SCENE_STRUCT
  return SCENE_STRUCT.replace(', inverseVP: mat4x4f', `${extraMembers}, inverseVP: mat4x4f`)
}

/** Interleaved position+normal mesh vertex stride in bytes. */
export const MESH_VERTEX_STRIDE = 24
/** Morph source positions-only stride in bytes (vertex slot 1). */
export const MORPH_VERTEX_STRIDE = 12
