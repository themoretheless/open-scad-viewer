// GENERATED FILE — do not edit.
// Golden outputs of crates/raster-core/src/variants.rs (immediate/instanced
// shader transforms), used by vitest to pin the hand-written TypeScript
// variants.ts to the Rust implementations. Regenerate with `wgsl_export`.

export const VARIANTS_GOLDEN: Record<string, string> = {
  'immediate:mesh': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f }
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
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  let d = max(dot(N, L), 0.0);
  let s = pow(max(dot(N, H), 0.0), 40.0);
  let bd = max(dot(-N, L), 0.0) * 0.25;
  let selected = mix(ob.color.rgb, vec3f(1.0, 0.52, 0.06), objectStyle().y * 0.48);
  let base = mix(selected, vec3f(0.12, 0.78, 1.0), objectStyle().w * 0.38);
  let c = sc.ambient.rgb * base + d * base + s * vec3f(0.25) + bd * base * 0.5;
  return vec4f(c, objectStyle().x);
}`,
  'instanced:mesh': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f }
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
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V2 = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V2);
  let d = max(dot(N, L), 0.0);
  let s = pow(max(dot(N, H), 0.0), 40.0);
  let bd = max(dot(-N, L), 0.0) * 0.25;
  let selected = mix(ob.color.rgb, vec3f(1.0, 0.52, 0.06), ob.style.y * 0.48);
  let base = mix(selected, vec3f(0.12, 0.78, 1.0), ob.style.w * 0.38);
  let c = sc.ambient.rgb * base + d * base + s * vec3f(0.25) + bd * base * 0.5;
  return vec4f(c, ob.style.x);
}`,
  'immediate:deep_mesh': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;

struct V { @builtin(position) p: vec4f, @location(0) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(2) fromPos: vec3f) -> V {
  let local = mix(fromPos, pos, ob.morph.x);
  let world = (ob.model * vec4f(local, 1)).xyz;
  return V(sc.vp * vec4f(world, 1), world);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  return vec4f(1.0, 0.42, 0.06, 0.14);
}`,
  'instanced:deep_mesh': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<storage, read> objects: array<Obj>;

struct V { @location(2) @interpolate(flat) instance: u32, @builtin(position) p: vec4f, @location(0) w: vec3f }

@vertex fn vs(@builtin(instance_index) instance: u32, @location(0) pos: vec3f, @location(2) fromPos: vec3f) -> V {
  let ob = objects[instance];
  let local = mix(fromPos, pos, ob.morph.x);
  let world = (ob.model * vec4f(local, 1)).xyz;
  return V(instance, sc.vp * vec4f(world, 1), world);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  let ob = objects[v.instance];
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  return vec4f(1.0, 0.42, 0.06, 0.14);
}`,
  'immediate:edge': /* wgsl */`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }

struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f }
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
  let selected = mix(vec3f(0.025, 0.03, 0.04), vec3f(1.0, 0.55, 0.08), objectStyle().y);
  let color = mix(selected, vec3f(0.1, 0.82, 1.0), objectStyle().w);
  return vec4f(color, objectStyle().z);
}`,
  'instanced:edge': /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f }
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
  let selected = mix(vec3f(0.025, 0.03, 0.04), vec3f(1.0, 0.55, 0.08), ob.style.y);
  let color = mix(selected, vec3f(0.1, 0.82, 1.0), ob.style.w);
  return vec4f(color, ob.style.z);
}`,
}
