import { SILENT_CONTEXT } from '../openScadStablePrimitiveSemantics'
import type { OpenScadPolygonPrimitiveInput, OpenScadPolygonPrimitivePlan, OpenScadStablePrimitiveContext, OpenScadVec2 } from '../openScadStablePrimitiveSemantics'
import { asVector, convexity, frozenOutlines2, normalizeLimits, point2, reduction, unsignedIndex, warn } from './resolveOpenScadCube'

export function resolveOpenScadPolygon(
  input: OpenScadPolygonPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadPolygonPrimitivePlan {
  const limits = normalizeLimits(input.limits)
  const authoredPoints = asVector(input.points)
  if (authoredPoints.length > limits.maximumPoints) {
    const limited = reduction('polygon', 'points', authoredPoints.length, limits.maximumPoints, context)
    return Object.freeze({
      kind: 'polygon', points: Object.freeze([]), outlines: Object.freeze([]), pathSource: 'implicit',
      convexity: convexity(input.convexity), aborted: true, empty: true, reduced: true, reduction: limited,
    })
  }

  const points: OpenScadVec2[] = []
  for (let pointIndex = 0; pointIndex < authoredPoints.length; pointIndex++) {
    const converted = point2(authoredPoints[pointIndex])
    if (converted === null) {
      warn(context, {
        code: 'OPENSCAD_PRIMITIVE_POINT_INVALID',
        message: 'polygon produced no outlines because a point failed exact vec2 conversion',
        primitive: 'polygon',
        field: `points[${pointIndex}]`,
        index: pointIndex,
        value: authoredPoints[pointIndex],
      })
      return Object.freeze({
        kind: 'polygon', points: Object.freeze(points), outlines: Object.freeze([]), pathSource: 'implicit',
        convexity: convexity(input.convexity), aborted: true, empty: true, reduced: false, reduction: null,
      })
    }
    points.push(converted)
  }

  const authoredPaths = asVector(input.paths)
  const pathSource: OpenScadPolygonPrimitivePlan['pathSource'] = authoredPaths.length === 0 && points.length > 2
    ? 'implicit'
    : 'explicit'
  if (authoredPaths.length > limits.maximumFacesOrPaths) {
    const limited = reduction('polygon', 'paths', authoredPaths.length, limits.maximumFacesOrPaths, context)
    return Object.freeze({
      kind: 'polygon', points: Object.freeze(points), outlines: Object.freeze([]), pathSource,
      convexity: convexity(input.convexity), aborted: true, empty: true, reduced: true, reduction: limited,
    })
  }

  if (pathSource === 'implicit') {
    const outline = Object.freeze([...points])
    return Object.freeze({
      kind: 'polygon', points: Object.freeze(points), outlines: Object.freeze([outline]), pathSource,
      convexity: convexity(input.convexity), aborted: false, empty: false, reduced: false, reduction: null,
    })
  }

  const outlines: OpenScadVec2[][] = []
  let indexCount = 0
  for (let pathIndex = 0; pathIndex < authoredPaths.length; pathIndex++) {
    const authoredPath = asVector(authoredPaths[pathIndex])
    const outline: OpenScadVec2[] = []
    outlines.push(outline)
    for (let pathPointIndex = 0; pathPointIndex < authoredPath.length; pathPointIndex++) {
      indexCount++
      if (indexCount > limits.maximumIndices) {
        const limited = reduction('polygon', 'indices', indexCount, limits.maximumIndices, context)
        return Object.freeze({
          kind: 'polygon', points: Object.freeze(points), outlines: frozenOutlines2(outlines), pathSource,
          convexity: convexity(input.convexity), aborted: true,
          empty: outlines.every(candidate => candidate.length < 3), reduced: true, reduction: limited,
        })
      }
      const index = unsignedIndex(authoredPath[pathPointIndex])
      if (index >= points.length) {
        warn(context, {
          code: 'OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS',
          message: 'polygon skipped a path entry whose point index is outside the point vector',
          primitive: 'polygon',
          field: `paths[${pathIndex}]`,
          index,
          value: authoredPath[pathPointIndex],
        })
        continue
      }
      outline.push(points[index])
    }
  }

  return Object.freeze({
    kind: 'polygon',
    points: Object.freeze(points),
    outlines: frozenOutlines2(outlines),
    pathSource,
    convexity: convexity(input.convexity),
    aborted: false,
    empty: outlines.every(outline => outline.length < 3),
    reduced: false,
    reduction: null,
  })
}
