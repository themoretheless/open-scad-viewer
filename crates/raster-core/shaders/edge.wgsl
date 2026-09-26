struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f }
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
  let selected = mix(sc.edgeColor, vec3f(1.0, 0.55, 0.08), ob.style.y);
  let color = mix(selected, vec3f(0.1, 0.82, 1.0), ob.style.w);
  return vec4f(color, ob.style.z);
}
