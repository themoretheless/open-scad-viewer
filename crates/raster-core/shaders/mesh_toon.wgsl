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
}
