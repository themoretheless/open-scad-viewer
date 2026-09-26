/**
 * Shader registry: every shipped WGSL module with the pipeline descriptors
 * the renderer needs (blend, depth, topology, vertex layout, bind layout).
 * The registry is the single place that knows these defaults; the renderer's
 * cached pipeline factory reads them instead of hand-wiring descriptors.
 */

import { MESH_WGSL } from './mesh'
import { MESH_PBR_WGSL } from './meshPbr'
import { MESH_MATCAP_WGSL } from './meshMatcap'
import { MESH_TOON_WGSL } from './meshToon'
import { MESH_UNLIT_WGSL } from './meshUnlit'
import { MESH_SECTION_CAP_WGSL } from './meshSectionCap'
import { DEEP_MESH_WGSL } from './deepMesh'
import { EDGE_WGSL } from './edge'
import { LINE_WGSL } from './line'
import { GRID_WGSL } from './grid'
import { SELECTION_OVERLAY_WGSL } from './selectionOverlay'
import { MESH_SHADOW_WGSL } from './meshShadow'

export {
  OBJECT_UNIFORM_LAYOUT,
  MESH_VERTEX_STRIDE,
  MORPH_VERTEX_STRIDE,
  OBJ_BINDING,
  OBJ_STRUCT,
  SCENE_BINDING,
  SCENE_STRUCT,
  SCENE_UNIFORM_LAYOUT,
  SECTION_CAP_WGSL,
  SECTION_CLIP_WGSL,
  SHADOW_MAP_WGSL,
  SHADOW_PCF_SAMPLE_WGSL,
  SHADOW_PCF_WGSL,
  SHADOW_SAMPLER_WGSL,
  sceneStruct,
} from './chunks'
export { MESH_WGSL } from './mesh'
export { MESH_PBR_WGSL } from './meshPbr'
export { MESH_MATCAP_WGSL } from './meshMatcap'
export { MESH_TOON_WGSL } from './meshToon'
export { MESH_UNLIT_WGSL } from './meshUnlit'
export { MESH_SECTION_CAP_WGSL } from './meshSectionCap'
export { DEEP_MESH_WGSL } from './deepMesh'
export { EDGE_WGSL } from './edge'
export { LINE_WGSL } from './line'
export { GRID_WGSL } from './grid'
export { SELECTION_OVERLAY_WGSL } from './selectionOverlay'
export { MESH_SHADOW_WGSL } from './meshShadow'
export { immediateObjectShader, instancedObjectShader, supportsImmediateAddressSpace } from './variants'

/* ── Registry ─────────────────────────────────────── */

/** Shader family: object shaders declare the Obj uniform and morph blend. */
export type ShaderKind = 'object' | 'overlay' | 'line' | 'grid'

/** Which Obj source a pipeline variant reads. */
export type ShaderVariant = 'uniform' | 'immediate' | 'instanced'

/** Named interleaved vertex-buffer layouts shared by pipeline creation. */
export type ShaderVertexLayout = 'mesh' | 'edge' | 'line' | 'grid'

export interface ShaderDepthSpec {
  writeEnabled: boolean
  compare: 'less' | 'less-equal' | 'always'
}

export interface ShaderSpec {
  /** Stable registry id, e.g. 'mesh'. */
  readonly id: string
  /** Canonical WGSL source (uniform variant). */
  readonly source: string
  readonly kind: ShaderKind
  /** Color/alpha blend: 'none' = opaque target, 'alpha' = premultiplied alpha blend. */
  readonly blend: 'none' | 'alpha'
  /** Default depth state; individual pipelines may override per flavor. */
  readonly depth: ShaderDepthSpec
  readonly topology: 'triangle-list' | 'line-list'
  readonly cullMode: 'none' | 'front' | 'back'
  readonly vertexLayout: ShaderVertexLayout
  /** True when the immediate/instanced textual variants apply. */
  readonly supportsVariants: boolean
  /**
   * True when the pipeline needs the renderer's matcap bind group at
   * group(2) (texture + sampler); only meshMatcap sets this today.
   */
  readonly usesMatcapBinding?: boolean
  /**
   * True when the pipeline needs the renderer's environment-map bind group at
   * group(2) (texture + sampler); only meshPbr sets this today.
   */
  readonly usesEnvBinding?: boolean
  /**
   * True when the pipeline needs the renderer's shadow-map bind group:
   * mesh-surface shaders and the grid sample the key-light depth map with a
   * comparison sampler (mesh/meshToon at group(2), meshPbr/meshMatcap share
   * their group(2) at bindings 2/3, the grid at group(1)).
   */
  readonly usesShadowBinding?: boolean
  /**
   * Depth-only pipeline (meshShadow): no fragment stage, renders into the
   * depth32float shadow map instead of the canvas depth24plus target.
   */
  readonly depthOnly?: boolean
}

const registry = new Map<string, ShaderSpec>()

/** Registers a shader; throws on duplicate ids so drift fails loudly. */
export function registerShader(spec: ShaderSpec): ShaderSpec {
  if (registry.has(spec.id)) throw new Error(`Shader registry: duplicate id '${spec.id}'`)
  registry.set(spec.id, spec)
  return spec
}

/** Looks up a registered shader; throws on unknown ids. */
export function getShader(id: string): ShaderSpec {
  const spec = registry.get(id)
  if (!spec) throw new Error(`Shader registry: unknown shader '${id}'`)
  return spec
}

export function hasShader(id: string): boolean {
  return registry.has(id)
}

export function listShaders(): readonly ShaderSpec[] {
  return [...registry.values()]
}

registerShader({
  id: 'mesh',
  source: MESH_WGSL,
  kind: 'object',
  blend: 'none',
  depth: { writeEnabled: true, compare: 'less' },
  topology: 'triangle-list',
  cullMode: 'none',
  vertexLayout: 'mesh',
  supportsVariants: true,
  usesShadowBinding: true,
})
// Alternate mesh shading models share the mesh pipeline defaults (opaque,
// depth write+less, triangle-list, mesh vertex layout); only the WGSL differs.
// meshMatcap additionally binds the renderer-wide capture texture at group(2);
// meshPbr binds the renderer-wide environment map at group(2). All four surface
// shaders sample the shadow map (meshUnlit stays unlit, shadow-free).
for (const [id, source, usesMatcapBinding, usesEnvBinding] of [
  ['meshPbr', MESH_PBR_WGSL, false, true],
  ['meshMatcap', MESH_MATCAP_WGSL, true, false],
  ['meshToon', MESH_TOON_WGSL, false, false],
  ['meshUnlit', MESH_UNLIT_WGSL, false, false],
] as const) {
  registerShader({
    id,
    source,
    kind: 'object',
    blend: 'none',
    depth: { writeEnabled: true, compare: 'less' },
    topology: 'triangle-list',
    cullMode: 'none',
    vertexLayout: 'mesh',
    supportsVariants: true,
    usesMatcapBinding,
    usesEnvBinding,
    usesShadowBinding: id !== 'meshUnlit',
  })
}
registerShader({
  id: 'meshSectionCap',
  source: MESH_SECTION_CAP_WGSL,
  kind: 'object',
  blend: 'none',
  depth: { writeEnabled: true, compare: 'less' },
  topology: 'triangle-list',
  // Front-face culling draws only back faces: with the inverted clip test,
  // the interior of closed solids reads as a filled cap surface.
  cullMode: 'front',
  vertexLayout: 'mesh',
  supportsVariants: true,
})
registerShader({
  id: 'deepMesh',
  source: DEEP_MESH_WGSL,
  kind: 'object',
  blend: 'alpha',
  depth: { writeEnabled: false, compare: 'always' },
  topology: 'triangle-list',
  cullMode: 'none',
  vertexLayout: 'mesh',
  supportsVariants: true,
})
registerShader({
  id: 'edge',
  source: EDGE_WGSL,
  kind: 'object',
  blend: 'alpha',
  depth: { writeEnabled: false, compare: 'less-equal' },
  topology: 'line-list',
  cullMode: 'none',
  vertexLayout: 'edge',
  supportsVariants: true,
})
registerShader({
  id: 'line',
  source: LINE_WGSL,
  kind: 'line',
  blend: 'alpha',
  depth: { writeEnabled: true, compare: 'less' },
  topology: 'line-list',
  cullMode: 'none',
  vertexLayout: 'line',
  supportsVariants: false,
})
registerShader({
  id: 'grid',
  source: GRID_WGSL,
  kind: 'grid',
  blend: 'alpha',
  depth: { writeEnabled: true, compare: 'less' },
  topology: 'triangle-list',
  cullMode: 'none',
  vertexLayout: 'grid',
  supportsVariants: false,
  usesShadowBinding: true,
})
registerShader({
  id: 'meshShadow',
  source: MESH_SHADOW_WGSL,
  kind: 'object',
  blend: 'none',
  depth: { writeEnabled: true, compare: 'less' },
  topology: 'triangle-list',
  cullMode: 'none',
  vertexLayout: 'mesh',
  supportsVariants: false,
  depthOnly: true,
})
registerShader({
  id: 'selectionOverlay',
  source: SELECTION_OVERLAY_WGSL,
  kind: 'overlay',
  blend: 'alpha',
  depth: { writeEnabled: false, compare: 'less-equal' },
  topology: 'triangle-list',
  cullMode: 'none',
  vertexLayout: 'line',
  supportsVariants: false,
})
