struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f }
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
  var c = (kd * albedo / 3.14159265 + specular) * ndl
    + sc.ambient.rgb * albedo + bd * albedo * 0.5 + ob.emissive;
  // Reinhard tone map + gamma so metals do not blow out under the key light.
  c = c / (c + vec3f(1.0));
  c = pow(c, vec3f(1.0 / 2.2));
  return vec4f(c, ob.style.x);
}
