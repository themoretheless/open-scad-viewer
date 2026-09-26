// Depth-only shadow pass: renders mesh depth from the key light's
// orthographic view into the shadow map. Same vertex contract as the mesh
// surface shaders (interleaved pos+normal slot 0, morph source slot 1,
// Scene + Obj uniforms); no fragment stage — the pipeline has no color
// targets and writes depth32float only. sc.lightVP is the light-space
// view-projection built on the CPU per frame.
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> @builtin(position) vec4f {
  let local = mix(fromPos, pos, ob.morph.x);
  return sc.lightVP * (ob.model * vec4f(local, 1.0));
}
