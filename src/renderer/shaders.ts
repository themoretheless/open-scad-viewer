/**
 * WGSL shader sources for the WebGPU renderer.
 *
 * The Scene uniform struct is defined ONCE and interpolated into every
 * shader that binds it — previously it was copy-pasted verbatim into three
 * shaders, so editing one copy silently desynced the others (a binding-
 * layout mismatch WebGPU reports as garbage values, not an error).
 *
 * NOTE: flags are currently smuggled in _pad0/_pad1 (flat/SSAO/Gooch/toon);
 * replacing them with a typed FeatureFlags field is tracked in
 * RECOMMENDATIONS.md Phase 4.2.
 */

export const SCENE_STRUCT_WGSL =
  'struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, clip: vec4f, fogParams: vec4f, fogColor: vec4f, _pad0: vec4f, _pad1: vec4f, sectionBox: vec4f }'

export const MESH_WGSL = /* wgsl */`
${SCENE_STRUCT_WGSL}
struct Obj   { model: mat4x4f, nmat: mat4x4f, color: vec4f }

@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f) -> V {
  let wp = (ob.model * vec4f(pos,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(sc.vp * vec4f(wp,1), wn, wp);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  // Multi-axis clipping plane
  if (sc.clip.y > 0.5) {
    let clipAxis = i32(sc.clip.z);
    var clipCoord = v.w.y;
    if (clipAxis == 0) { clipCoord = v.w.x; }
    else if (clipAxis == 2) { clipCoord = v.w.z; }
    if (clipCoord < sc.clip.x) { discard; }
  }

  // Section box: clip on all 3 axes simultaneously
  if (sc.sectionBox.w > 0.5) {
    if (v.w.x < sc.sectionBox.x) { discard; }
    if (v.w.y < sc.sectionBox.y) { discard; }
    if (v.w.z < sc.sectionBox.z) { discard; }
  }

  // Flat or smooth shading
  var N = normalize(v.n);
  if (sc._pad0.x > 0.5) {
    N = normalize(cross(dpdx(v.w), dpdy(v.w)));
  }

  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  var d = max(dot(N, L), 0.0);

  // Toon / cel shading: quantize diffuse into discrete steps
  if (sc._pad1.x > 0.5) {
    d = floor(d * 4.0) / 4.0;
  }

  let s = pow(max(dot(N, H), 0.0), 40.0);
  let bd = max(dot(-N, L), 0.0) * 0.25;
  var c = sc.ambient.rgb * ob.color.rgb + d * ob.color.rgb + s * vec3f(0.25) + bd * ob.color.rgb * 0.5;

  // Fog
  if (sc.fogParams.z > 0.5) {
    let dist = length(sc.eye.xyz - v.w);
    let fogFactor = clamp((dist - sc.fogParams.x) / (sc.fogParams.y - sc.fogParams.x), 0.0, 1.0);
    c = mix(c, sc.fogColor.rgb, fogFactor);
  }

  // SSAO approximation
  if (sc._pad0.y > 0.5) {
    let ao = 0.5 + 0.5 * max(dot(N, V2), 0.0);
    c = c * ao;
  }

  // Gooch shading
  if (sc._pad0.z > 0.5) {
    let gooch_cool = vec3f(0.0, 0.0, 0.55) + 0.25 * ob.color.rgb;
    let gooch_warm = vec3f(0.3, 0.3, 0.0) + 0.25 * ob.color.rgb;
    let gooch_t = (1.0 + dot(N, L)) * 0.5;
    let gooch_c = mix(gooch_cool, gooch_warm, gooch_t);
    c = gooch_c + s * vec3f(0.25);
  }

  return vec4f(c, ob.color.a);
}
`

export const LINE_WGSL = /* wgsl */`
${SCENE_STRUCT_WGSL}
@group(0) @binding(0) var<uniform> sc: Scene;

struct V { @builtin(position) p: vec4f, @location(0) c: vec4f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) col: vec4f) -> V {
  return V(sc.vp * vec4f(pos,1), col);
}
@fragment fn fs(v: V) -> @location(0) vec4f { return v.c; }
`

export const OUTLINE_WGSL = /* wgsl */`
${SCENE_STRUCT_WGSL}
struct Obj   { model: mat4x4f, nmat: mat4x4f, color: vec4f }

@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f) -> @builtin(position) vec4f {
  let wp = (ob.model * vec4f(pos,1)).xyz + normalize((ob.nmat * vec4f(norm,0)).xyz) * 0.3;
  return sc.vp * vec4f(wp,1);
}

@fragment fn fs() -> @location(0) vec4f {
  return vec4f(0.0, 0.0, 0.0, 1.0);
}
`

export const SKY_WGSL = /* wgsl */`
struct SkyParams { topColor: vec4f, bottomColor: vec4f }
@group(0) @binding(0) var<uniform> sky: SkyParams;

struct V { @builtin(position) p: vec4f, @location(0) uv: f32 }

@vertex fn vs(@builtin(vertex_index) vi: u32) -> V {
  let x = f32(i32(vi) / 2) * 4.0 - 1.0;
  let y = f32(i32(vi) % 2) * 4.0 - 1.0;
  return V(vec4f(x, y, 0.999, 1.0), (y + 1.0) * 0.5);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  return mix(sky.bottomColor, sky.topColor, v.uv);
}
`
