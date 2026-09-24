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
}
