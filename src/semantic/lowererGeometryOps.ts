// Public entry point; implementations retain their original contracts.
export { IDENTITY_MATRIX, multiplyMatrix, translationMatrix, scaleMatrix, matrixValue, parseColor, semanticSlot, viewportSemanticSlots, viewportRootActivates, invoke, nodeValueType, requestedSegments, semanticSegments, semanticSegmentsFromRequested, primitiveColor, transformShapes, combineShapes, unionReduceInternal, makeCylinder } from './lowererGeometryOps/IDENTITY_MATRIX'
export type { SemanticReducedShape } from './lowererGeometryOps/IDENTITY_MATRIX'
export { makePolyhedron, makePolygon, compatibilityRequestedSegments, compatibilityPlanarChildren, evalDxfLinearExtrude, evalDxfRotateExtrude } from './lowererGeometryOps/makePolyhedron'
