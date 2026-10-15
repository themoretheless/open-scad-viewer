/** Render shaders for the GPU CSG depth/stencil preview. */
export const CSG_COMMON_WGSL = /* wgsl */`
struct CsgScene {
  vp: mat4x4f,
  eye: vec4f,
  light: vec4f,
  ambient: vec4f,
  cutTint: vec4f,
};

struct CsgObj {
  model: mat4x4f,
  nmat: mat4x4f,
  color: vec4f,
  params: vec4f, // x: isCutSurface (0 or 1), y: opacity
};

@group(0) @binding(0) var<uniform> sc: CsgScene;
@group(1) @binding(0) var<uniform> ob: CsgObj;
`

export const CSG_DEPTH_WGSL = /* wgsl */`
${CSG_COMMON_WGSL}

struct VDepth {
  @builtin(position) p: vec4f,
};

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f) -> VDepth {
  let wp = (ob.model * vec4f(pos, 1.0)).xyz;
  return VDepth(sc.vp * vec4f(wp, 1.0));
}

@fragment fn fs() {
  // Empty fragment stage: updates depth and/or stencil buffer only
}
`

export const CSG_SHADE_WGSL = /* wgsl */`
${CSG_COMMON_WGSL}

struct VShade {
  @builtin(position) p: vec4f,
  @location(0) n: vec3f,
  @location(1) w: vec3f,
};

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f) -> VShade {
  let wp = (ob.model * vec4f(pos, 1.0)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm, 0.0)).xyz);
  return VShade(sc.vp * vec4f(wp, 1.0), wn, wp);
}

@fragment fn fs(v: VShade) -> @location(0) vec4f {
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V);
  let diff = max(dot(N, L), 0.0);
  let spec = pow(max(dot(N, H), 0.0), 32.0) * 0.25;

  let base = select(ob.color.rgb, mix(ob.color.rgb, sc.cutTint.rgb, 0.6), ob.params.x > 0.5);
  let lit = base * (sc.ambient.rgb + diff * 0.75) + vec3f(spec);
  return vec4f(lit, ob.color.a);
}
`

/* ── WebGPU Pipeline Manager ──────────────────────────── */
