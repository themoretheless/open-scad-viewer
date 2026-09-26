// GENERATED FILE — do not edit.
// Golden outputs of crates/raster-core/src/variants.rs (immediate/instanced
// shader transforms), used by vitest to pin the hand-written TypeScript
// variants.ts to the Rust implementations. Regenerate with `wgsl_export`.

export const VARIANTS_GOLDEN: Record<string, string> = {
  'immediate:mesh': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
  // Cheap section-cap approximation: fragments just inside the clip plane
  // (within a fixed world-space epsilon) shade flat/unlit to suggest the cut.
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, objectStyle().x); }
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
  let c = sc.ambient.rgb * base + d * base + s * vec3f(0.25) * (1.0 - ob.metallic) + bd * base * 0.5 + ob.emissive;
  return vec4f(c, objectStyle().x);
}`,
  'instanced:mesh': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
  // Cheap section-cap approximation: fragments just inside the clip plane
  // (within a fixed world-space epsilon) shade flat/unlit to suggest the cut.
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, ob.style.x); }
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
  let c = sc.ambient.rgb * base + d * base + s * vec3f(0.25) * (1.0 - ob.metallic) + bd * base * 0.5 + ob.emissive;
  return vec4f(c, ob.style.x);
}`,
  'immediate:mesh_pbr': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation: fragments just inside the clip plane
  // (within a fixed world-space epsilon) shade flat/unlit to suggest the cut.
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, objectStyle().x); }
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
  var c = (kd * albedo / 3.14159265 + specular) * ndl
    + sc.ambient.rgb * albedo + bd * albedo * 0.5 + ob.emissive;
  // Reinhard tone map + gamma so metals do not blow out under the key light.
  c = c / (c + vec3f(1.0));
  c = pow(c, vec3f(1.0 / 2.2));
  return vec4f(c, objectStyle().x);
}`,
  'instanced:mesh_pbr': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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

@fragment fn fs(v: V) -> @location(0) vec4f {
  let ob = objects[v.instance];
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation: fragments just inside the clip plane
  // (within a fixed world-space epsilon) shade flat/unlit to suggest the cut.
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, ob.style.x); }
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
  var c = (kd * albedo / 3.14159265 + specular) * ndl
    + sc.ambient.rgb * albedo + bd * albedo * 0.5 + ob.emissive;
  // Reinhard tone map + gamma so metals do not blow out under the key light.
  c = c / (c + vec3f(1.0));
  c = pow(c, vec3f(1.0 / 2.2));
  return vec4f(c, ob.style.x);
}`,
  'immediate:mesh_matcap': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, objectStyle().x); }
  let N = normalize(v.n);
  let V2 = normalize(sc.eye.xyz - v.w);
  // Procedural matcap: project the normal onto an orthonormal view basis and
  // shade in that 2D "material capture" space — no texture involved.
  let upRef = select(vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0), abs(V2.y) > 0.99);
  let right = normalize(cross(upRef, V2));
  let camUp = cross(V2, right);
  let m = vec2f(dot(N, right), dot(N, camUp));
  let selected = mix(ob.color.rgb, sc.selectionColor, objectStyle().y * 0.48);
  let base = mix(selected, sc.hoverColor, objectStyle().w * 0.38) * ob.baseColor;
  // Soft top-left key light.
  let key = clamp(dot(m, vec2f(-0.35, 0.55)) * 0.5 + 0.55, 0.0, 1.0);
  // Fresnel rim, tinted slightly cool like a studio bounce.
  let rim = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 2.5);
  // Specular blob: Gaussian around the key-light reflection spot.
  let blob = m - vec2f(-0.3, 0.45);
  let spec = exp(-dot(blob, blob) * 18.0) * mix(1.0, 0.4, clamp(ob.metallic, 0.0, 1.0));
  let c = base * (0.25 + 0.75 * key)
    + vec3f(0.9, 0.95, 1.0) * rim * 0.35
    + vec3f(spec) * 0.6 + ob.emissive;
  return vec4f(c, objectStyle().x);
}`,
  'instanced:mesh_matcap': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, ob.style.x); }
  let N = normalize(v.n);
  let V2 = normalize(sc.eye.xyz - v.w);
  // Procedural matcap: project the normal onto an orthonormal view basis and
  // shade in that 2D "material capture" space — no texture involved.
  let upRef = select(vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0), abs(V2.y) > 0.99);
  let right = normalize(cross(upRef, V2));
  let camUp = cross(V2, right);
  let m = vec2f(dot(N, right), dot(N, camUp));
  let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
  let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
  // Soft top-left key light.
  let key = clamp(dot(m, vec2f(-0.35, 0.55)) * 0.5 + 0.55, 0.0, 1.0);
  // Fresnel rim, tinted slightly cool like a studio bounce.
  let rim = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 2.5);
  // Specular blob: Gaussian around the key-light reflection spot.
  let blob = m - vec2f(-0.3, 0.45);
  let spec = exp(-dot(blob, blob) * 18.0) * mix(1.0, 0.4, clamp(ob.metallic, 0.0, 1.0));
  let c = base * (0.25 + 0.75 * key)
    + vec3f(0.9, 0.95, 1.0) * rim * 0.35
    + vec3f(spec) * 0.6 + ob.emissive;
  return vec4f(c, ob.style.x);
}`,
  'immediate:mesh_toon': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, objectStyle().x); }
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
  let c = base * (sc.ambient.rgb * 0.6 + step) * outline + ob.emissive;
  return vec4f(c, objectStyle().x);
}`,
  'instanced:mesh_toon': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, ob.style.x); }
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
  let c = base * (sc.ambient.rgb * 0.6 + step) * outline + ob.emissive;
  return vec4f(c, ob.style.x);
}`,
  'immediate:mesh_unlit': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, objectStyle().x); }
  // Unlit: flat base color (with the standard selection/hover tints) plus emission.
  let selected = mix(ob.color.rgb, sc.selectionColor, objectStyle().y * 0.48);
  let base = mix(selected, sc.hoverColor, objectStyle().w * 0.38) * ob.baseColor;
  return vec4f(base + ob.emissive, objectStyle().x);
}`,
  'instanced:mesh_unlit': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, vec3f(0.85, 0.87, 0.9), 0.6) + ob.emissive * 0.2, ob.style.x); }
  // Unlit: flat base color (with the standard selection/hover tints) plus emission.
  let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
  let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
  return vec4f(base + ob.emissive, ob.style.x);
}`,
  'immediate:deep_mesh': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f }
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
