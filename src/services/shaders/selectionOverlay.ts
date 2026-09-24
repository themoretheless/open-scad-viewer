import { SCENE_BINDING, SCENE_STRUCT } from './chunks'

/** Translucent face highlight and boundary overlay drawn above the surface. */
export const SELECTION_OVERLAY_WGSL = /* wgsl */`
${SCENE_STRUCT}
${SCENE_BINDING}

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
`
