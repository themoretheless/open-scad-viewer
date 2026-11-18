// Project-only build stages: multi-file compilation and the VFS-backed surface() and text()
// assets. Single-source builds never reach them, so the parser loads this module on the first
// project build instead of carrying it in the geometry build engine chunk.
export { compileOpenScadProject } from './openScadProjectCompiler'
export {
  loadOpenScadSurfaceHeightMap,
  prepareOpenScadSurfaceAssets,
  triangulateOpenScadSurface,
} from './openScadSurface/decodeOpenScadSurfacePng'
export { prepareOpenScadTextAssets } from './openScadText/prepareOpenScadTextAssets'
export { renderOpenScadText } from './openScadText/renderOpenScadText'
