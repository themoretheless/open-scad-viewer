struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f }
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
}
