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
  if (ndc.z < 0.0 || ndc.z > 1.0 || uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) { return 1.0; }
  let texel = sc.shadowParams.y;
  let depth = ndc.z - sc.shadowParams.z;
  var sum = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      sum += textureSampleCompare(shadowMap, shadowSampler, uv + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
    }
  }
  return mix(1.0, sum / 9.0, sc.shadowParams.w);
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
  // Shadows darken the ambient (indirect) term only; the key light stays.
  let shadow = shadowFactor(v.w);
  // Defaults (baseColor white, metallic 0, emissive black) reproduce the
  // legacy Blinn-Phong look exactly; roughness/materialId are reserved.
  let c = sc.ambient.rgb * base * shadow + d * base + s * vec3f(0.25) * (1.0 - ob.metallic) + bd * base * 0.5 + ob.emissive;
  return vec4f(c, ob.style.x);
}
