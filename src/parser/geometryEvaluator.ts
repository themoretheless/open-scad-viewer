// Public entry point; implementations retain their original contracts.


export {
  compatibilityDxfProfile,
  makeImport,
  makeSurface,
  makeText,
} from './assetEvaluator'
export { requireStableProfile, stableModuleSemanticsContext, stableGeometrySemanticsContext, stablePrimitiveSemanticsContext, stableTransformSemanticsContext, stableFragmentInput, stableFragmentInputFromEvaluated, stableSegmentsFromEvaluated, stableEmptyShape, warnIgnoredPrimitiveChildren, makeStableCube, makeStableSquare, makeStableSphere, makeStableCircle, makeStableCylinder, makeStablePolyhedron, makeStablePolygon } from './geometryEvaluator/EvalNodesFn'
export type { EvalNodesFn, EvalPreparedNodesFn } from './geometryEvaluator/EvalNodesFn'
export { segments, makeCylinder, makePolyhedron, makePolygon, transformStableChildren, rotateShapes, transformByMatrix, homogeneousShapes, booleanShapes } from './geometryEvaluator/segments'
export { differenceChildren, hullShapes, resizeChildren, minkowskiShapes, linearExtrude, dxfLinearExtrude } from './geometryEvaluator/differenceChildren'
export { dxfRotateExtrude, rotateExtrude, parseLegacyColor } from './geometryEvaluator/dxfRotateExtrude'
