// GENERATED FILE — do not edit.
// Golden outputs of crates/raster-core/src/variants.rs (immediate/instanced
// shader transforms), used by vitest to pin the hand-written TypeScript
// variants.ts to the Rust implementations. Regenerate with `wgsl_export`.

export const VARIANTS_GOLDEN: Record<string, string> = {
  'immediate:mesh': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
// Contact shadow map (key-light depth), bound per renderer at group(2). The
// renderer always binds something: with shadowParams.x == 0 the factor below
// is 1, so the default look is pixel-identical to the pre-shadow one.
@group(2) @binding(0) var shadowMap: texture_depth_2d;
@group(2) @binding(1) var shadowSampler: sampler_comparison;

struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

// PCF 3x3 visibility of the key light at a world position; 1 when shadows
// are disabled or the point lies outside the light frustum.
fn shadowFactor(wp: vec3f) -> f32 {
  if (sc.shadowParams.x < 0.5) { return 1.0; }
  let lp = sc.lightVP * vec4f(wp, 1.0);
  let ndc = lp.xyz / lp.w;
  let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
  let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
  let texel = sc.shadowParams.y;
  let depth = ndc.z - sc.shadowParams.z;
  var sum = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
    }
  }
  return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(sc.vp * vec4f(wp,1), wn, wp);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation: fragments just inside the clip plane
  // (within a fixed world-space epsilon) shade flat/unlit to suggest the cut.
  // Shadows darken the ambient (indirect) term only; the key light stays.
  // Hoisted above the section-cap early return: textureSampleCompare must run
  // in uniform control flow (WGSL uniformity analysis).
  let shadow = shadowFactor(v.w);
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, objectStyle().x); }
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  let d = max(dot(N, L), 0.0);
  let s = pow(max(dot(N, H), 0.0), 40.0);
  let bd = max(dot(-N, L), 0.0) * 0.25;
  let selected = mix(ob.color.rgb, sc.selectionColor, objectStyle().y * 0.48);
  let base = mix(selected, sc.hoverColor, objectStyle().w * 0.38) * ob.baseColor;
  // Defaults (baseColor white, metallic 0, emissive black) reproduce the
  // legacy Blinn-Phong look exactly; roughness/materialId are reserved.
  let c = sc.ambient.rgb * base * shadow + d * base + s * vec3f(0.25) * (1.0 - ob.metallic) + bd * base * 0.5 + ob.emissive;
  return vec4f(c, objectStyle().x);
}`,
  'instanced:mesh': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<storage, read> objects: array<Obj>;
// Contact shadow map (key-light depth), bound per renderer at group(2). The
// renderer always binds something: with shadowParams.x == 0 the factor below
// is 1, so the default look is pixel-identical to the pre-shadow one.
@group(2) @binding(0) var shadowMap: texture_depth_2d;
@group(2) @binding(1) var shadowSampler: sampler_comparison;

struct V { @location(2) @interpolate(flat) instance: u32, @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

// PCF 3x3 visibility of the key light at a world position; 1 when shadows
// are disabled or the point lies outside the light frustum.
fn shadowFactor(wp: vec3f) -> f32 {
  if (sc.shadowParams.x < 0.5) { return 1.0; }
  let lp = sc.lightVP * vec4f(wp, 1.0);
  let ndc = lp.xyz / lp.w;
  let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
  let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
  let texel = sc.shadowParams.y;
  let depth = ndc.z - sc.shadowParams.z;
  var sum = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
    }
  }
  return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}

@vertex fn vs(@builtin(instance_index) instance: u32, @location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let ob = objects[instance];
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(instance, sc.vp * vec4f(wp,1), wn, wp);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
  let ob = objects[v.instance];
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation: fragments just inside the clip plane
  // (within a fixed world-space epsilon) shade flat/unlit to suggest the cut.
  // Shadows darken the ambient (indirect) term only; the key light stays.
  // Hoisted above the section-cap early return: textureSampleCompare must run
  // in uniform control flow (WGSL uniformity analysis).
  let shadow = shadowFactor(v.w);
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  let d = max(dot(N, L), 0.0);
  let s = pow(max(dot(N, H), 0.0), 40.0);
  let bd = max(dot(-N, L), 0.0) * 0.25;
  let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
  let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
  // Defaults (baseColor white, metallic 0, emissive black) reproduce the
  // legacy Blinn-Phong look exactly; roughness/materialId are reserved.
  let c = sc.ambient.rgb * base * shadow + d * base + s * vec3f(0.25) * (1.0 - ob.metallic) + bd * base * 0.5 + ob.emissive;
  return vec4f(c, ob.style.x);
}`,
  'immediate:mesh_pbr': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
// Equirectangular environment map, bound per renderer (not per object). The
// renderer always binds something: a 1x1 dummy selects the analytic-lighting
// fallback below (textureDimensions == 1), a real map switches to the IBL
// path. Same binding contract as mesh_matcap.wgsl's capture.
@group(2) @binding(0) var envTex: texture_2d<f32>;
@group(2) @binding(1) var envSampler: sampler;
// Contact shadow map (key-light depth), sharing the environment group. With
// shadowParams.x == 0 the factor below is 1: the default look is unchanged.
@group(2) @binding(2) var shadowMap: texture_depth_2d;
@group(2) @binding(3) var shadowSampler: sampler_comparison;

struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

// PCF 3x3 visibility of the key light at a world position; 1 when shadows
// are disabled or the point lies outside the light frustum.
fn shadowFactor(wp: vec3f) -> f32 {
  if (sc.shadowParams.x < 0.5) { return 1.0; }
  let lp = sc.lightVP * vec4f(wp, 1.0);
  let ndc = lp.xyz / lp.w;
  let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
  let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
  let texel = sc.shadowParams.y;
  let depth = ndc.z - sc.shadowParams.z;
  var sum = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
    }
  }
  return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(sc.vp * vec4f(wp,1), wn, wp);
}

fn fresnelSchlick(cosTheta: f32, F0: vec3f) -> vec3f {
  return F0 + (vec3f(1.0) - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}
fn distributionGGX(N: vec3f, H: vec3f, rough: f32) -> f32 {
  let a = max(rough * rough, 0.001);
  let a2 = a * a;
  let ndh = max(dot(N, H), 0.0);
  let denom = ndh * ndh * (a2 - 1.0) + 1.0;
  return a2 / (3.14159265 * denom * denom);
}
fn geometrySchlickGGX(ndv: f32, rough: f32) -> f32 {
  let r = rough + 1.0;
  let k = (r * r) / 8.0;
  return ndv / (ndv * (1.0 - k) + k);
}
fn geometrySmith(N: vec3f, V2: vec3f, L: vec3f, rough: f32) -> f32 {
  return geometrySchlickGGX(max(dot(N, V2), 0.0), rough)
    * geometrySchlickGGX(max(dot(N, L), 0.0), rough);
}

// Equirectangular lookup in the viewer's Z-up world: u wraps the azimuth
// around +Z, v = 0 is the +Z zenith, which is the PNG's top row (uploaded
// without flipY). On the pole axis the azimuth is undefined (atan2(0, 0));
// any u is correct there, so it is pinned instead of left to the backend.
fn envUv(dir: vec3f) -> vec2f {
  let d = normalize(dir);
  let azimuth = select(atan2(d.y, d.x), 0.0, abs(d.x) + abs(d.y) < 1e-6);
  let u = azimuth / 6.2831853 + 0.5;
  let v = acos(clamp(d.z, -1.0, 1.0)) / 3.14159265;
  return vec2f(u, v);
}
fn envSample(dir: vec3f, lod: f32) -> vec3f {
  return textureSampleLevel(envTex, envSampler, envUv(dir), lod).rgb;
}
// Diffuse irradiance approximation: a 5-tap cone average around N. Equirect
// textures carry no prefiltered irradiance map, and mips are not generated,
// so the taps integrate a small hemisphere cap cheaply (offset ~0.55 rad in
// tangent space around the normal; a wide soft cap matches lambert well for
// the smooth studio/sky maps shipped in public/env/).
fn envIrradiance(N: vec3f) -> vec3f {
  let upRef = select(vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0), abs(N.y) > 0.99);
  let t = normalize(cross(upRef, N));
  let b = cross(N, t);
  let w = 0.55;
  return (envSample(N, 0.0)
    + envSample(normalize(N + t * w), 0.0)
    + envSample(normalize(N - t * w), 0.0)
    + envSample(normalize(N + b * w), 0.0)
    + envSample(normalize(N - b * w), 0.0)) * 0.2;
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation: fragments just inside the clip plane
  // (within a fixed world-space epsilon) shade flat/unlit to suggest the cut.
  // Shadows darken the indirect terms (irradiance/ambient) only; the direct
  // key light and specular stay. Hoisted above the section-cap early return:
  // textureSampleCompare must run in uniform control flow (WGSL uniformity).
  let shadow = shadowFactor(v.w);
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, objectStyle().x); }
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  let selected = mix(ob.color.rgb, sc.selectionColor, objectStyle().y * 0.48);
  let albedo = mix(selected, sc.hoverColor, objectStyle().w * 0.38) * ob.baseColor;
  let metallic = clamp(ob.metallic, 0.0, 1.0);
  let rough = clamp(ob.roughness, 0.05, 1.0);
  let F0 = mix(vec3f(0.04), albedo, metallic);
  let ndf = distributionGGX(N, H, rough);
  let g = geometrySmith(N, V2, L, rough);
  let f = fresnelSchlick(max(dot(H, V2), 0.0), F0);
  let specular = (ndf * g * f) / max(4.0 * max(dot(N, V2), 0.0) * max(dot(N, L), 0.0), 0.0001);
  let kd = (vec3f(1.0) - f) * (1.0 - metallic);
  let ndl = max(dot(N, L), 0.0);
  // Fill term mirroring the Phong shader's back-light contribution.
  let bd = max(dot(-N, L), 0.0) * 0.25;
  var c: vec3f;
  if (textureDimensions(envTex).x > 1u) {
    // IBL path: the environment replaces the analytic key light entirely —
    // it already supplies both the diffuse irradiance and the specular
    // highlight, so adding the key light on top would double-light the model.
    // Diffuse: cone-averaged irradiance at the surface normal.
    // Specular: reflect view about the normal. The shipped env textures have
    // no mip chain (textureNumLevels == 1), so roughness is approximated by
    // bending the reflection toward the normal (softens the lobe) and scaling
    // by a (1-rough)^2 gloss factor; when a mipmapped env map is bound the
    // roughness-driven LOD is used instead. Fresnel(F0) weights the lobe.
    let irradiance = envIrradiance(N);
    var R = reflect(-V2, N);
    R = normalize(mix(R, N, rough * rough));
    var envSpec: vec3f;
    if (textureNumLevels(envTex) > 1u) {
      envSpec = envSample(R, rough * f32(textureNumLevels(envTex) - 1u));
    } else {
      envSpec = envSample(R, 0.0) * pow(1.0 - rough, 2.0);
    }
    let fEnv = fresnelSchlick(max(dot(N, V2), 0.0), F0);
    c = irradiance * albedo * (vec3f(1.0) - fEnv) * (1.0 - metallic) * shadow
      + envSpec * fEnv
      + sc.ambient.rgb * albedo * 0.25 * shadow + ob.emissive;
  } else {
    c = (kd * albedo / 3.14159265 + specular) * ndl
      + sc.ambient.rgb * albedo * shadow + bd * albedo * 0.5 + ob.emissive;
  }
  // Reinhard tone map + gamma so metals do not blow out under the key light.
  c = c / (c + vec3f(1.0));
  c = pow(c, vec3f(1.0 / 2.2));
  return vec4f(c, objectStyle().x);
}`,
  'instanced:mesh_pbr': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<storage, read> objects: array<Obj>;
// Equirectangular environment map, bound per renderer (not per object). The
// renderer always binds something: a 1x1 dummy selects the analytic-lighting
// fallback below (textureDimensions == 1), a real map switches to the IBL
// path. Same binding contract as mesh_matcap.wgsl's capture.
@group(2) @binding(0) var envTex: texture_2d<f32>;
@group(2) @binding(1) var envSampler: sampler;
// Contact shadow map (key-light depth), sharing the environment group. With
// shadowParams.x == 0 the factor below is 1: the default look is unchanged.
@group(2) @binding(2) var shadowMap: texture_depth_2d;
@group(2) @binding(3) var shadowSampler: sampler_comparison;

struct V { @location(2) @interpolate(flat) instance: u32, @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

// PCF 3x3 visibility of the key light at a world position; 1 when shadows
// are disabled or the point lies outside the light frustum.
fn shadowFactor(wp: vec3f) -> f32 {
  if (sc.shadowParams.x < 0.5) { return 1.0; }
  let lp = sc.lightVP * vec4f(wp, 1.0);
  let ndc = lp.xyz / lp.w;
  let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
  let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
  let texel = sc.shadowParams.y;
  let depth = ndc.z - sc.shadowParams.z;
  var sum = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
    }
  }
  return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}

@vertex fn vs(@builtin(instance_index) instance: u32, @location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let ob = objects[instance];
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(instance, sc.vp * vec4f(wp,1), wn, wp);
}

fn fresnelSchlick(cosTheta: f32, F0: vec3f) -> vec3f {
  return F0 + (vec3f(1.0) - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}
fn distributionGGX(N: vec3f, H: vec3f, rough: f32) -> f32 {
  let a = max(rough * rough, 0.001);
  let a2 = a * a;
  let ndh = max(dot(N, H), 0.0);
  let denom = ndh * ndh * (a2 - 1.0) + 1.0;
  return a2 / (3.14159265 * denom * denom);
}
fn geometrySchlickGGX(ndv: f32, rough: f32) -> f32 {
  let r = rough + 1.0;
  let k = (r * r) / 8.0;
  return ndv / (ndv * (1.0 - k) + k);
}
fn geometrySmith(N: vec3f, V2: vec3f, L: vec3f, rough: f32) -> f32 {
  return geometrySchlickGGX(max(dot(N, V2), 0.0), rough)
    * geometrySchlickGGX(max(dot(N, L), 0.0), rough);
}

// Equirectangular lookup in the viewer's Z-up world: u wraps the azimuth
// around +Z, v = 0 is the +Z zenith, which is the PNG's top row (uploaded
// without flipY). On the pole axis the azimuth is undefined (atan2(0, 0));
// any u is correct there, so it is pinned instead of left to the backend.
fn envUv(dir: vec3f) -> vec2f {
  let d = normalize(dir);
  let azimuth = select(atan2(d.y, d.x), 0.0, abs(d.x) + abs(d.y) < 1e-6);
  let u = azimuth / 6.2831853 + 0.5;
  let v = acos(clamp(d.z, -1.0, 1.0)) / 3.14159265;
  return vec2f(u, v);
}
fn envSample(dir: vec3f, lod: f32) -> vec3f {
  return textureSampleLevel(envTex, envSampler, envUv(dir), lod).rgb;
}
// Diffuse irradiance approximation: a 5-tap cone average around N. Equirect
// textures carry no prefiltered irradiance map, and mips are not generated,
// so the taps integrate a small hemisphere cap cheaply (offset ~0.55 rad in
// tangent space around the normal; a wide soft cap matches lambert well for
// the smooth studio/sky maps shipped in public/env/).
fn envIrradiance(N: vec3f) -> vec3f {
  let upRef = select(vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0), abs(N.y) > 0.99);
  let t = normalize(cross(upRef, N));
  let b = cross(N, t);
  let w = 0.55;
  return (envSample(N, 0.0)
    + envSample(normalize(N + t * w), 0.0)
    + envSample(normalize(N - t * w), 0.0)
    + envSample(normalize(N + b * w), 0.0)
    + envSample(normalize(N - b * w), 0.0)) * 0.2;
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  let ob = objects[v.instance];
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation: fragments just inside the clip plane
  // (within a fixed world-space epsilon) shade flat/unlit to suggest the cut.
  // Shadows darken the indirect terms (irradiance/ambient) only; the direct
  // key light and specular stay. Hoisted above the section-cap early return:
  // textureSampleCompare must run in uniform control flow (WGSL uniformity).
  let shadow = shadowFactor(v.w);
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
  let albedo = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
  let metallic = clamp(ob.metallic, 0.0, 1.0);
  let rough = clamp(ob.roughness, 0.05, 1.0);
  let F0 = mix(vec3f(0.04), albedo, metallic);
  let ndf = distributionGGX(N, H, rough);
  let g = geometrySmith(N, V2, L, rough);
  let f = fresnelSchlick(max(dot(H, V2), 0.0), F0);
  let specular = (ndf * g * f) / max(4.0 * max(dot(N, V2), 0.0) * max(dot(N, L), 0.0), 0.0001);
  let kd = (vec3f(1.0) - f) * (1.0 - metallic);
  let ndl = max(dot(N, L), 0.0);
  // Fill term mirroring the Phong shader's back-light contribution.
  let bd = max(dot(-N, L), 0.0) * 0.25;
  var c: vec3f;
  if (textureDimensions(envTex).x > 1u) {
    // IBL path: the environment replaces the analytic key light entirely —
    // it already supplies both the diffuse irradiance and the specular
    // highlight, so adding the key light on top would double-light the model.
    // Diffuse: cone-averaged irradiance at the surface normal.
    // Specular: reflect view about the normal. The shipped env textures have
    // no mip chain (textureNumLevels == 1), so roughness is approximated by
    // bending the reflection toward the normal (softens the lobe) and scaling
    // by a (1-rough)^2 gloss factor; when a mipmapped env map is bound the
    // roughness-driven LOD is used instead. Fresnel(F0) weights the lobe.
    let irradiance = envIrradiance(N);
    var R = reflect(-V2, N);
    R = normalize(mix(R, N, rough * rough));
    var envSpec: vec3f;
    if (textureNumLevels(envTex) > 1u) {
      envSpec = envSample(R, rough * f32(textureNumLevels(envTex) - 1u));
    } else {
      envSpec = envSample(R, 0.0) * pow(1.0 - rough, 2.0);
    }
    let fEnv = fresnelSchlick(max(dot(N, V2), 0.0), F0);
    c = irradiance * albedo * (vec3f(1.0) - fEnv) * (1.0 - metallic) * shadow
      + envSpec * fEnv
      + sc.ambient.rgb * albedo * 0.25 * shadow + ob.emissive;
  } else {
    c = (kd * albedo / 3.14159265 + specular) * ndl
      + sc.ambient.rgb * albedo * shadow + bd * albedo * 0.5 + ob.emissive;
  }
  // Reinhard tone map + gamma so metals do not blow out under the key light.
  c = c / (c + vec3f(1.0));
  c = pow(c, vec3f(1.0 / 2.2));
  return vec4f(c, ob.style.x);
}`,
  'immediate:mesh_matcap': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
// Matcap capture, bound per renderer (not per object). The renderer always
// binds something: a 1x1 white dummy selects the procedural fallback below
// (textureDimensions == 1), a real capture switches to the textured path.
@group(2) @binding(0) var matcapTex: texture_2d<f32>;
@group(2) @binding(1) var matcapSampler: sampler;
// Contact shadow map (key-light depth), sharing the matcap group. With
// shadowParams.x == 0 the factor below is 1: the default look is unchanged.
@group(2) @binding(2) var shadowMap: texture_depth_2d;
@group(2) @binding(3) var shadowSampler: sampler_comparison;

struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

// PCF 3x3 visibility of the key light at a world position; 1 when shadows
// are disabled or the point lies outside the light frustum.
fn shadowFactor(wp: vec3f) -> f32 {
  if (sc.shadowParams.x < 0.5) { return 1.0; }
  let lp = sc.lightVP * vec4f(wp, 1.0);
  let ndc = lp.xyz / lp.w;
  let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
  let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
  let texel = sc.shadowParams.y;
  let depth = ndc.z - sc.shadowParams.z;
  var sum = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
    }
  }
  return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(sc.vp * vec4f(wp,1), wn, wp);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation (world-space epsilon highlight).
  // Shadows darken the ambient floor term only; key/rim/specular stay.
  // Hoisted above the section-cap early return: textureSampleCompare must run
  // in uniform control flow (WGSL uniformity analysis).
  let shadow = shadowFactor(v.w);
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, objectStyle().x); }
  let N = normalize(v.n);
  let V2 = normalize(sc.eye.xyz - v.w);
  // Project the normal onto an orthonormal view basis: the 2D "material
  // capture" space, shared by the textured and procedural paths.
  let upRef = select(vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0), abs(V2.y) > 0.99);
  let right = normalize(cross(upRef, V2));
  let camUp = cross(V2, right);
  let m = vec2f(dot(N, right), dot(N, camUp));
  let selected = mix(ob.color.rgb, sc.selectionColor, objectStyle().y * 0.48);
  let base = mix(selected, sc.hoverColor, objectStyle().w * 0.38) * ob.baseColor;
  if (textureDimensions(matcapTex).x > 1u) {
    // Textured matcap: view-space normal projected into capture UV space,
    // tinted by the (selection/hover-aware) base color.
    let uv = m * 0.5 + vec2f(0.5, 0.5);
    let captured = textureSampleLevel(matcapTex, matcapSampler, uv, 0.0).rgb;
    return vec4f(captured * base + ob.emissive, objectStyle().x);
  }
  // Soft top-left key light.
  let key = clamp(dot(m, vec2f(-0.35, 0.55)) * 0.5 + 0.55, 0.0, 1.0);
  // Fresnel rim, tinted slightly cool like a studio bounce.
  let rim = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 2.5);
  // Specular blob: Gaussian around the key-light reflection spot.
  let blob = m - vec2f(-0.3, 0.45);
  let spec = exp(-dot(blob, blob) * 18.0) * mix(1.0, 0.4, clamp(ob.metallic, 0.0, 1.0));
  let c = base * (0.25 * shadow + 0.75 * key)
    + vec3f(0.9, 0.95, 1.0) * rim * 0.35
    + vec3f(spec) * 0.6 + ob.emissive;
  return vec4f(c, objectStyle().x);
}`,
  'instanced:mesh_matcap': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<storage, read> objects: array<Obj>;
// Matcap capture, bound per renderer (not per object). The renderer always
// binds something: a 1x1 white dummy selects the procedural fallback below
// (textureDimensions == 1), a real capture switches to the textured path.
@group(2) @binding(0) var matcapTex: texture_2d<f32>;
@group(2) @binding(1) var matcapSampler: sampler;
// Contact shadow map (key-light depth), sharing the matcap group. With
// shadowParams.x == 0 the factor below is 1: the default look is unchanged.
@group(2) @binding(2) var shadowMap: texture_depth_2d;
@group(2) @binding(3) var shadowSampler: sampler_comparison;

struct V { @location(2) @interpolate(flat) instance: u32, @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

// PCF 3x3 visibility of the key light at a world position; 1 when shadows
// are disabled or the point lies outside the light frustum.
fn shadowFactor(wp: vec3f) -> f32 {
  if (sc.shadowParams.x < 0.5) { return 1.0; }
  let lp = sc.lightVP * vec4f(wp, 1.0);
  let ndc = lp.xyz / lp.w;
  let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
  let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
  let texel = sc.shadowParams.y;
  let depth = ndc.z - sc.shadowParams.z;
  var sum = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
    }
  }
  return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}

@vertex fn vs(@builtin(instance_index) instance: u32, @location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let ob = objects[instance];
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(instance, sc.vp * vec4f(wp,1), wn, wp);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  let ob = objects[v.instance];
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation (world-space epsilon highlight).
  // Shadows darken the ambient floor term only; key/rim/specular stay.
  // Hoisted above the section-cap early return: textureSampleCompare must run
  // in uniform control flow (WGSL uniformity analysis).
  let shadow = shadowFactor(v.w);
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
  let N = normalize(v.n);
  let V2 = normalize(sc.eye.xyz - v.w);
  // Project the normal onto an orthonormal view basis: the 2D "material
  // capture" space, shared by the textured and procedural paths.
  let upRef = select(vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0), abs(V2.y) > 0.99);
  let right = normalize(cross(upRef, V2));
  let camUp = cross(V2, right);
  let m = vec2f(dot(N, right), dot(N, camUp));
  let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
  let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
  if (textureDimensions(matcapTex).x > 1u) {
    // Textured matcap: view-space normal projected into capture UV space,
    // tinted by the (selection/hover-aware) base color.
    let uv = m * 0.5 + vec2f(0.5, 0.5);
    let captured = textureSampleLevel(matcapTex, matcapSampler, uv, 0.0).rgb;
    return vec4f(captured * base + ob.emissive, ob.style.x);
  }
  // Soft top-left key light.
  let key = clamp(dot(m, vec2f(-0.35, 0.55)) * 0.5 + 0.55, 0.0, 1.0);
  // Fresnel rim, tinted slightly cool like a studio bounce.
  let rim = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 2.5);
  // Specular blob: Gaussian around the key-light reflection spot.
  let blob = m - vec2f(-0.3, 0.45);
  let spec = exp(-dot(blob, blob) * 18.0) * mix(1.0, 0.4, clamp(ob.metallic, 0.0, 1.0));
  let c = base * (0.25 * shadow + 0.75 * key)
    + vec3f(0.9, 0.95, 1.0) * rim * 0.35
    + vec3f(spec) * 0.6 + ob.emissive;
  return vec4f(c, ob.style.x);
}`,
  'immediate:mesh_toon': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
// Contact shadow map (key-light depth), bound per renderer at group(2). With
// shadowParams.x == 0 the factor below is 1: the default look is unchanged.
@group(2) @binding(0) var shadowMap: texture_depth_2d;
@group(2) @binding(1) var shadowSampler: sampler_comparison;

struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

// PCF 3x3 visibility of the key light at a world position; 1 when shadows
// are disabled or the point lies outside the light frustum.
fn shadowFactor(wp: vec3f) -> f32 {
  if (sc.shadowParams.x < 0.5) { return 1.0; }
  let lp = sc.lightVP * vec4f(wp, 1.0);
  let ndc = lp.xyz / lp.w;
  let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
  let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
  let texel = sc.shadowParams.y;
  let depth = ndc.z - sc.shadowParams.z;
  var sum = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
    }
  }
  return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(sc.vp * vec4f(wp,1), wn, wp);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation (world-space epsilon highlight).
  // Shadows darken the ambient (indirect) term only; the toon steps stay.
  // Hoisted above the section-cap early return: textureSampleCompare must run
  // in uniform control flow (WGSL uniformity analysis).
  let shadow = shadowFactor(v.w);
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, objectStyle().x); }
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let selected = mix(ob.color.rgb, sc.selectionColor, objectStyle().y * 0.48);
  let base = mix(selected, sc.hoverColor, objectStyle().w * 0.38) * ob.baseColor;
  // Quantized (4-step) diffuse for the technical-illustration look.
  let d = max(dot(N, L), 0.0);
  let step = clamp(floor(d * 4.0) / 4.0 + 0.12, 0.0, 1.0);
  // Fresnel rim darkening suggests an outline without a second pass.
  let rim = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 3.0);
  let outline = 1.0 - smoothstep(0.55, 0.95, rim) * 0.85;
  let c = base * (sc.ambient.rgb * 0.6 * shadow + step) * outline + ob.emissive;
  return vec4f(c, objectStyle().x);
}`,
  'instanced:mesh_toon': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<storage, read> objects: array<Obj>;
// Contact shadow map (key-light depth), bound per renderer at group(2). With
// shadowParams.x == 0 the factor below is 1: the default look is unchanged.
@group(2) @binding(0) var shadowMap: texture_depth_2d;
@group(2) @binding(1) var shadowSampler: sampler_comparison;

struct V { @location(2) @interpolate(flat) instance: u32, @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

// PCF 3x3 visibility of the key light at a world position; 1 when shadows
// are disabled or the point lies outside the light frustum.
fn shadowFactor(wp: vec3f) -> f32 {
  if (sc.shadowParams.x < 0.5) { return 1.0; }
  let lp = sc.lightVP * vec4f(wp, 1.0);
  let ndc = lp.xyz / lp.w;
  let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
  let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
  let texel = sc.shadowParams.y;
  let depth = ndc.z - sc.shadowParams.z;
  var sum = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
    }
  }
  return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}

@vertex fn vs(@builtin(instance_index) instance: u32, @location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let ob = objects[instance];
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(instance, sc.vp * vec4f(wp,1), wn, wp);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  let ob = objects[v.instance];
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation (world-space epsilon highlight).
  // Shadows darken the ambient (indirect) term only; the toon steps stay.
  // Hoisted above the section-cap early return: textureSampleCompare must run
  // in uniform control flow (WGSL uniformity analysis).
  let shadow = shadowFactor(v.w);
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
  let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
  // Quantized (4-step) diffuse for the technical-illustration look.
  let d = max(dot(N, L), 0.0);
  let step = clamp(floor(d * 4.0) / 4.0 + 0.12, 0.0, 1.0);
  // Fresnel rim darkening suggests an outline without a second pass.
  let rim = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 3.0);
  let outline = 1.0 - smoothstep(0.55, 0.95, rim) * 0.85;
  let c = base * (sc.ambient.rgb * 0.6 * shadow + step) * outline + ob.emissive;
  return vec4f(c, ob.style.x);
}`,
  'immediate:mesh_unlit': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(sc.vp * vec4f(wp,1), wn, wp);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation (world-space epsilon highlight).
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, objectStyle().x); }
  // Unlit: flat base color (with the standard selection/hover tints) plus emission.
  let selected = mix(ob.color.rgb, sc.selectionColor, objectStyle().y * 0.48);
  let base = mix(selected, sc.hoverColor, objectStyle().w * 0.38) * ob.baseColor;
  return vec4f(base + ob.emissive, objectStyle().x);
}`,
  'instanced:mesh_unlit': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<storage, read> objects: array<Obj>;

struct V { @location(2) @interpolate(flat) instance: u32, @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

@vertex fn vs(@builtin(instance_index) instance: u32, @location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let ob = objects[instance];
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(instance, sc.vp * vec4f(wp,1), wn, wp);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  let ob = objects[v.instance];
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation (world-space epsilon highlight).
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
  // Unlit: flat base color (with the standard selection/hover tints) plus emission.
  let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
  let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
  return vec4f(base + ob.emissive, ob.style.x);
}`,
  'immediate:mesh_section_cap': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

// Vertex stage identical to mesh.wgsl (same layout, morph blend, transforms).
@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(sc.vp * vec4f(wp,1), wn, wp);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
  // Inverted clip test: the surface pass keeps dot(w, n) >= w, so this pass —
  // drawn with front-face culling right after it — keeps only the clipped
  // side's back faces. For a closed solid those interior back faces read as a
  // filled, unlit cut surface (stencil-free section cap).
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) >= sc.section.w) { discard; }
  return vec4f(sc.capColor, 1.0);
}`,
  'instanced:mesh_section_cap': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<storage, read> objects: array<Obj>;

struct V { @location(2) @interpolate(flat) instance: u32, @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

// Vertex stage identical to mesh.wgsl (same layout, morph blend, transforms).
@vertex fn vs(@builtin(instance_index) instance: u32, @location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let ob = objects[instance];
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(instance, sc.vp * vec4f(wp,1), wn, wp);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
  let ob = objects[v.instance];
  // Inverted clip test: the surface pass keeps dot(w, n) >= w, so this pass —
  // drawn with front-face culling right after it — keeps only the clipped
  // side's back faces. For a closed solid those interior back faces read as a
  // filled, unlit cut surface (stencil-free section cap).
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) >= sc.section.w) { discard; }
  return vec4f(sc.capColor, 1.0);
}`,
  'immediate:deep_mesh': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

struct V { @builtin(position) p: vec4f, @location(0) w: vec3f, @location(1) n: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let local = mix(fromPos, pos, ob.morph.x);
  let world = (ob.model * vec4f(local, 1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm, 0)).xyz);
  return V(sc.vp * vec4f(world, 1), world, wn);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Fresnel-weighted translucency: grazing angles (silhouette edges) glow,
  // face-on fragments stay clear so the geometry behind reads through.
  let N = normalize(v.n);
  let V2 = normalize(sc.eye.xyz - v.w);
  let fresnel = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 2.0);
  let glow = sc.xrayColor * (0.85 + 0.55 * fresnel);
  return vec4f(glow, mix(0.1, 0.55, fresnel));
}`,
  'instanced:deep_mesh': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<storage, read> objects: array<Obj>;

struct V { @location(2) @interpolate(flat) instance: u32, @builtin(position) p: vec4f, @location(0) w: vec3f, @location(1) n: vec3f }

@vertex fn vs(@builtin(instance_index) instance: u32, @location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let ob = objects[instance];
  let local = mix(fromPos, pos, ob.morph.x);
  let world = (ob.model * vec4f(local, 1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm, 0)).xyz);
  return V(instance, sc.vp * vec4f(world, 1), world, wn);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  let ob = objects[v.instance];
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Fresnel-weighted translucency: grazing angles (silhouette edges) glow,
  // face-on fragments stay clear so the geometry behind reads through.
  let N = normalize(v.n);
  let V2 = normalize(sc.eye.xyz - v.w);
  let fresnel = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 2.0);
  let glow = sc.xrayColor * (0.85 + 0.55 * fresnel);
  return vec4f(glow, mix(0.1, 0.55, fresnel));
}`,
  'immediate:edge': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

struct EdgeV { @builtin(position) p: vec4f, @location(0) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) fromPos: vec3f) -> EdgeV {
  let local = mix(fromPos, pos, ob.morph.x);
  let world = (ob.model * vec4f(local, 1)).xyz;
  var p = sc.vp * ob.model * vec4f(local, 1);
  p.z -= 0.00008 * p.w;
  return EdgeV(p, world);
}

@fragment fn fs(v: EdgeV) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Edge accents stay brighter than the theme selection/hover surface tints
  // (they apply at full strength); only the base edge color is themed.
  let selected = mix(sc.edgeColor, vec3f(1.0, 0.55, 0.08), objectStyle().y);
  let color = mix(selected, vec3f(0.1, 0.82, 1.0), objectStyle().w);
  return vec4f(color, objectStyle().z);
}`,
  'instanced:edge': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<storage, read> objects: array<Obj>;

struct EdgeV { @location(2) @interpolate(flat) instance: u32, @builtin(position) p: vec4f, @location(0) w: vec3f }

@vertex fn vs(@builtin(instance_index) instance: u32, @location(0) pos: vec3f, @location(1) fromPos: vec3f) -> EdgeV {
  let ob = objects[instance];
  let local = mix(fromPos, pos, ob.morph.x);
  let world = (ob.model * vec4f(local, 1)).xyz;
  var p = sc.vp * ob.model * vec4f(local, 1);
  p.z -= 0.00008 * p.w;
  return EdgeV(instance, p, world);
}

@fragment fn fs(v: EdgeV) -> @location(0) vec4f {
  let ob = objects[v.instance];
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Edge accents stay brighter than the theme selection/hover surface tints
  // (they apply at full strength); only the base edge color is themed.
  let selected = mix(sc.edgeColor, vec3f(1.0, 0.55, 0.08), ob.style.y);
  let color = mix(selected, vec3f(0.1, 0.82, 1.0), ob.style.w);
  return vec4f(color, ob.style.z);
}`,
}
