struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f }
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
}
