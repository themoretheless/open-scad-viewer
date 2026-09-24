import { OBJ_BINDING, OBJ_STRUCT, SCENE_BINDING, SCENE_STRUCT } from './chunks'

/** Depth-peeled "deep" highlight shell for the selected object. */
export const DEEP_MESH_WGSL = /* wgsl */`
${SCENE_STRUCT}
${OBJ_STRUCT}
${SCENE_BINDING}
${OBJ_BINDING}

struct V { @builtin(position) p: vec4f, @location(0) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(2) fromPos: vec3f) -> V {
  let local = mix(fromPos, pos, ob.morph.x);
  let world = (ob.model * vec4f(local, 1)).xyz;
  return V(sc.vp * vec4f(world, 1), world);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  return vec4f(1.0, 0.42, 0.06, 0.14);
}
`
