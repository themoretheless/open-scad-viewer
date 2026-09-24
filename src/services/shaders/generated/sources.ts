// GENERATED FILE — do not edit.
// Generated from crates/raster-core/shaders/*.wgsl (the single source of truth)
// by the raster-core `wgsl_export` codegen. Regenerate:
//   cargo run --manifest-path crates/Cargo.toml -p raster-core --bin wgsl_export
// Verify:
//   cargo run --manifest-path crates/Cargo.toml -p raster-core --bin wgsl_export -- --check

/** Lit opaque/transparent mesh surface with per-object style and GPU morph blend. */
export const MESH_WGSL = /* wgsl */`
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
  let selected = mix(ob.color.rgb, vec3f(1.0, 0.52, 0.06), ob.style.y * 0.48);
  let base = mix(selected, vec3f(0.12, 0.78, 1.0), ob.style.w * 0.38);
  let c = sc.ambient.rgb * base + d * base + s * vec3f(0.25) + bd * base * 0.5;
  return vec4f(c, ob.style.x);
}`

/** X-ray deep-selection mesh (depth Always, translucent orange). */
export const DEEP_MESH_WGSL = /* wgsl */`
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
}`

/** Per-mesh wireframe edges with selection/hover tinting. */
export const EDGE_WGSL = /* wgsl */`
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
  let selected = mix(vec3f(0.025, 0.03, 0.04), vec3f(1.0, 0.55, 0.08), ob.style.y);
  let color = mix(selected, vec3f(0.1, 0.82, 1.0), ob.style.w);
  return vec4f(color, ob.style.z);
}`

/** Plain colored line list (measurements, grid axes). */
export const LINE_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;

struct V { @builtin(position) p: vec4f, @location(0) c: vec4f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) col: vec4f) -> V {
  return V(sc.vp * vec4f(pos,1), col);
}
@fragment fn fs(v: V) -> @location(0) vec4f { return v.c; }`

/** Full-viewport XY grid, reconstructed from camera rays with adaptive spacing. */
export const GRID_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f }
@group(0) @binding(0) var<uniform> sc: Scene;

struct V { @builtin(position) p: vec4f, @location(0) near: vec4f, @location(1) far: vec4f }
struct F { @location(0) color: vec4f, @builtin(frag_depth) depth: f32 }

@vertex fn vs(@location(0) corner: vec2f) -> V {
  return V(vec4f(corner, 0.0, 1.0),
    sc.inverseVP * vec4f(corner, 0.0, 1.0),
    sc.inverseVP * vec4f(corner, 1.0, 1.0));
}

fn lineMask(coord: vec2f, width: vec2f) -> f32 {
  let g = abs(fract(coord - 0.5) - 0.5) / max(width, vec2f(0.000001));
  return 1.0 - min(min(g.x, g.y), 1.0);
}

@fragment fn fs(v: V) -> F {
  let near = v.near.xyz / v.near.w;
  let far = v.far.xyz / v.far.w;
  let ray = far - near;
  let dz = select(-1.0, 1.0, ray.z >= 0.0) * max(abs(ray.z), 0.000001);
  let t = -near.z / dz;
  let world = near + t * ray;
  let pixelWidth = max(fwidth(world.xy), vec2f(0.000001));
  // Blend decade levels so distant areas retain a readable grid instead of
  // losing both fixed levels. Derivatives precede all non-uniform discards.
  let level = max(0.0, log2(max(pixelWidth.x, pixelWidth.y) * 8.0 / sc.options.y) / log2(10.0));
  let step = sc.options.y * pow(10.0, floor(level));
  let blend = fract(level);
  let minor = lineMask(world.xy / step, pixelWidth / step);
  let major = lineMask(world.xy / (step * 10.0), pixelWidth / (step * 10.0));
  let coarse = lineMask(world.xy / (step * 100.0), pixelWidth / (step * 100.0));
  var alpha = max(max(minor * 0.28 * (1.0 - blend), major * mix(0.55, 0.28, blend)), coarse * 0.55 * blend);
  var color = vec3f(0.42, 0.42, 0.42);
  let axisW = pixelWidth * 1.2;
  if (abs(world.y) < axisW.y) { color = vec3f(0.95, 0.18, 0.16); alpha = max(alpha, 0.9); }
  if (abs(world.x) < axisW.x) { color = vec3f(0.2, 0.85, 0.25); alpha = max(alpha, 0.9); }
  let clip = sc.vp * vec4f(world.xy, 0.0, 1.0);
  let depth = clip.z / clip.w;
  if (abs(ray.z) < 0.000001 || t < 0.0 || depth < 0.0 || alpha < 0.004) { discard; }
  // Keep the distant grid as background even beyond the model clipping range.
  return F(vec4f(color, alpha), min(depth, 0.999999));
}`

/** Per-vertex colored overlay for source-face highlighting. */
export const SELECTION_OVERLAY_WGSL = /* wgsl */`
struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;

struct V { @builtin(position) p: vec4f, @location(0) c: vec4f, @location(1) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) col: vec4f) -> V {
  var clip = sc.vp * vec4f(pos, 1);
  clip.z -= 0.00018 * clip.w;
  return V(clip, col, pos);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  return v.c;
}`
