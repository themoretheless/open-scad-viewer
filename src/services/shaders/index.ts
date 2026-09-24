export {
  OBJECT_UNIFORM_LAYOUT,
  MESH_VERTEX_STRIDE,
  MORPH_VERTEX_STRIDE,
  OBJ_BINDING,
  OBJ_STRUCT,
  SCENE_BINDING,
  SCENE_STRUCT,
  SECTION_CLIP_WGSL,
  sceneStruct,
} from './chunks'
export { MESH_WGSL } from './mesh'
export { DEEP_MESH_WGSL } from './deepMesh'
export { EDGE_WGSL } from './edge'
export { LINE_WGSL } from './line'
export { GRID_WGSL } from './grid'
export { SELECTION_OVERLAY_WGSL } from './selectionOverlay'
export { immediateObjectShader, instancedObjectShader, supportsImmediateAddressSpace } from './variants'
