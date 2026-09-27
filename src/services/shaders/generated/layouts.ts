// GENERATED FILE — do not edit.
// Generated from crates/raster-core/src/layout.rs (the single source of truth
// for the Scene/Obj uniform layouts) by the raster-core `wgsl_export` codegen.
// Regenerate:
//   cargo run --manifest-path crates/Cargo.toml -p raster-core --bin wgsl_export
// Verify:
//   cargo run --manifest-path crates/Cargo.toml -p raster-core --bin wgsl_export -- --check

/**
 * CPU-side mirror of the Scene uniform struct: vp (16 floats) + eye (4)
 * + light (4) + ambient (4) + section (4) + options (4) + inverseVP (16)
 * + theme block (6 colors × vec3+pad = 24) + shadow block (lightVP 16
 * + shadowParams 4). shadowParams = (enabled, 1/mapSize, depth bias,
 * strength); enabled 0 keeps every sampling shader on its unshadowed path.
 */
export const SCENE_UNIFORM_LAYOUT = {
  floats: 96,
  bytes: 384,
  /** View-projection (column-major mat4) at +0..15. */
  vpFloatOffset: 0,
  vpByteOffset: 0,
  /** Eye position (w = 1). */
  eyeFloatOffset: 16,
  eyeByteOffset: 64,
  /** Light direction. */
  lightFloatOffset: 20,
  lightByteOffset: 80,
  /** Ambient color. */
  ambientFloatOffset: 24,
  ambientByteOffset: 96,
  /** Section plane (normal xyz, offset w). */
  sectionFloatOffset: 28,
  sectionByteOffset: 112,
  /** Options (section enabled, grid step, extent, fade). */
  optionsFloatOffset: 32,
  optionsByteOffset: 128,
  /** Inverse view-projection (column-major mat4) at +0..15. */
  inverseVPFloatOffset: 36,
  inverseVPByteOffset: 144,
  /** Theme block: selectionColor at +0..2, then hover/edge/xray/grid/cap colors every 4 floats (vec3 + pad, 16-byte aligned). */
  themeFloatOffset: 52,
  themeByteOffset: 208,
  /** hoverColor rgb at +0..2. */
  hoverFloatOffset: 56,
  /** edgeColor rgb at +0..2. */
  edgeFloatOffset: 60,
  /** xrayColor rgb at +0..2. */
  xrayFloatOffset: 64,
  /** gridColor rgb at +0..2. */
  gridFloatOffset: 68,
  /** capColor rgb at +0..2. */
  capFloatOffset: 72,
  capByteOffset: 288,
  /** Light view-projection (column-major mat4) at +0..15. */
  lightVPFloatOffset: 76,
  lightVPByteOffset: 304,
  /** Shadow params (enabled, texel, bias, strength) at +0..3. */
  shadowFloatOffset: 92,
  shadowByteOffset: 368,
} as const

/**
 * CPU-side mirror of the Obj uniform struct: model (16 floats) + nmat (16)
 * + color (4) + style (4) + morph (4) + baseColor (3) + metallic (1)
 * + emissive (3) + roughness (1) + materialId (1) + pad (3).
 */
export const OBJECT_UNIFORM_LAYOUT = {
  floats: 56,
  bytes: 224,
  /** style = (alpha, selected, edge, hovered). */
  styleFloatOffset: 36,
  styleByteOffset: 144,
  /** morph.x drives the GPU vertex blend. */
  morphFloatOffset: 40,
  morphByteOffset: 160,
  /** baseColor rgb at +0..2, metallic at +3. */
  materialFloatOffset: 44,
  materialByteOffset: 176,
  /** metallic scalar. */
  metallicFloatOffset: 47,
  /** emissive rgb at +0..2, roughness at +3. */
  emissiveFloatOffset: 48,
  emissiveByteOffset: 192,
  /** roughness scalar. */
  roughnessFloatOffset: 51,
  /** materialId scalar. */
  materialIdFloatOffset: 52,
  materialIdByteOffset: 208,
} as const
