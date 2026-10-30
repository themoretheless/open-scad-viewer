// Public entry point; implementations retain their original contracts.
export { IDENTITY_2, multiply2, transform2, dxfPairs, dxfSections, dxfValues, dxfValue, dxfNumber, dxfInteger, stitchDxfSegments, dxfPolyline, dxfLwPolyline, entityLayer, dxfDimensionMetadata, dxfQueryDiagnostic, dxfQueryResult } from './dxfImporter/Matrix2'
export type { Matrix2, DxfPair, DxfEntity, DxfSegment, DxfBlock, OpenScadDxfIntermediate, ResolvedDxfQueryMetadata } from './dxfImporter/Matrix2'
export { resolvePreparedDxfQueryMetadata, resolveDxfQueryTransform, transformDxfDimensionCoordinates, buildDxfQueryPaths } from './dxfImporter/resolvePreparedDxfQueryMetadata'
export type { ResolvedDxfQueryTransform } from './dxfImporter/resolvePreparedDxfQueryMetadata'
