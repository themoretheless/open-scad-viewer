import { SCENE_BINDING, SCENE_STRUCT } from './chunks'

/** Plain colored line list (measurements, grid axes). */
export const LINE_WGSL = /* wgsl */`
${SCENE_STRUCT}
${SCENE_BINDING}

struct V { @builtin(position) p: vec4f, @location(0) c: vec4f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) col: vec4f) -> V {
  return V(sc.vp * vec4f(pos,1), col);
}
@fragment fn fs(v: V) -> @location(0) vec4f { return v.c; }
`
