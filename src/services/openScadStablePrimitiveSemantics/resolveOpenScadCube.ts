import { SILENT_CONTEXT } from '../openScadStablePrimitiveSemantics'
import type { NormalizedLimits, OpenScadBoxPrimitiveInput, OpenScadBoxPrimitivePlan, OpenScadCylinderPrimitiveInput, OpenScadCylinderPrimitivePlan, OpenScadPolyhedronPrimitiveInput, OpenScadPolyhedronPrimitivePlan, OpenScadPrimitiveReduction, OpenScadPrimitiveSafetyLimits, OpenScadRadialPrimitiveInput, OpenScadRadialPrimitivePlan, OpenScadStablePrimitiveContext, OpenScadStablePrimitiveName, OpenScadStablePrimitiveWarning, OpenScadVec2, OpenScadVec3, RadiusPair } from '../openScadStablePrimitiveSemantics'

export function authoredNumber(value: unknown): value is number {
  // OpenSCAD's numeric type includes NaN and infinities. Geometry creation,
  // rather than argument conversion, decides whether they make an empty node.
  return typeof value === 'number'
}

export function exactNumericVector(value: unknown, length: number): value is number[] {
  return Array.isArray(value)
    && value.length === length
    && value.every(authoredNumber)
}

export function freeze2(x: number, y: number): OpenScadVec2 {
  return Object.freeze([x, y])
}

export function freeze3(x: number, y: number, z: number): OpenScadVec3 {
  return Object.freeze([x, y, z])
}

export function warn(
  context: OpenScadStablePrimitiveContext,
  warning: OpenScadStablePrimitiveWarning,
): void {
  context.warn(Object.freeze(warning))
}

export function rangeEmpty(
  primitive: OpenScadStablePrimitiveName,
  value: unknown,
  context: OpenScadStablePrimitiveContext,
): void {
  if (!context.checkParameterRanges) return
  warn(context, {
    code: 'OPENSCAD_PRIMITIVE_RANGE_EMPTY',
    message: `${primitive} parameters describe an empty object`,
    primitive,
    value,
  })
}

export function boxPlan(
  kind: 'cube' | 'square',
  axes: 2 | 3,
  input: OpenScadBoxPrimitiveInput,
  context: OpenScadStablePrimitiveContext,
): OpenScadBoxPrimitivePlan {
  let values: readonly number[] = axes === 3 ? [1, 1, 1] : [1, 1]
  let sizeSource: OpenScadBoxPrimitivePlan['sizeSource'] = 'default'
  if (authoredNumber(input.size)) {
    values = axes === 3
      ? [input.size, input.size, input.size]
      : [input.size, input.size]
    sizeSource = 'scalar'
  } else if (exactNumericVector(input.size, axes)) {
    values = [...input.size]
    sizeSource = 'vector'
  } else if (input.size !== undefined) {
    // cube's three-coordinate conversion writes directly into its initialized
    // dimensions. A later non-number therefore leaves an observable numeric
    // prefix in place even though conversion as a whole reports failure.
    if (kind === 'cube' && Array.isArray(input.size) && input.size.length === 3) {
      const partial = [1, 1, 1]
      let prefix = 0
      while (prefix < 3 && authoredNumber(input.size[prefix])) {
        partial[prefix] = input.size[prefix]
        prefix++
      }
      if (prefix > 0) {
        values = partial
        sizeSource = 'partial-vector'
      }
    }
    warn(context, {
      code: 'OPENSCAD_PRIMITIVE_SIZE_DEFAULTED',
      message: `${kind} size was not a scalar or exact ${axes}-component numeric vector; unit size is used`,
      primitive: kind,
      field: 'size',
      value: input.size,
    })
  }

  const empty = values.some(value => !(value > 0) || !Number.isFinite(value))
  if (empty) rangeEmpty(kind, input.size, context)
  return Object.freeze({
    kind,
    empty,
    reduced: false,
    reduction: null,
    dimensions: axes === 3
      ? freeze3(values[0], values[1], values[2])
      : freeze2(values[0], values[1]),
    center: input.center === true,
    sizeSource,
  })
}

export function resolveOpenScadCube(
  input: OpenScadBoxPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadBoxPrimitivePlan {
  return boxPlan('cube', 3, input, context)
}

export function resolveOpenScadSquare(
  input: OpenScadBoxPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadBoxPrimitivePlan {
  return boxPlan('square', 2, input, context)
}

export function radiusPair(
  primitive: 'sphere' | 'circle' | 'cylinder',
  radiusField: string,
  diameterField: string,
  radius: unknown,
  diameter: unknown,
  context: OpenScadStablePrimitiveContext,
): RadiusPair {
  const radiusIsNumber = authoredNumber(radius)
  if (authoredNumber(diameter)) {
    if (radiusIsNumber) warn(context, {
      code: 'OPENSCAD_PRIMITIVE_RADIUS_SHADOWED',
      message: `${primitive} uses ${diameterField}; the paired ${radiusField} value has no effect`,
      primitive,
      field: radiusField,
      value: radius,
    })
    return Object.freeze({ value: diameter / 2, source: 'diameter', supplied: true })
  }
  if (radiusIsNumber) return Object.freeze({ value: radius, source: 'radius', supplied: true })
  return Object.freeze({ value: 1, source: 'default', supplied: false })
}

export function radialPlan(
  kind: 'sphere' | 'circle',
  input: OpenScadRadialPrimitiveInput,
  context: OpenScadStablePrimitiveContext,
): OpenScadRadialPrimitivePlan {
  const radius = radiusPair(kind, 'r', 'd', input.r, input.d, context)
  const empty = !(radius.value > 0) || !Number.isFinite(radius.value)
  if (empty) rangeEmpty(kind, radius.value, context)
  return Object.freeze({
    kind,
    radius: radius.value,
    radiusSource: radius.source,
    fragmentRadius: radius.value,
    empty,
    reduced: false,
    reduction: null,
  })
}

export function resolveOpenScadSphere(
  input: OpenScadRadialPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadRadialPrimitivePlan {
  return radialPlan('sphere', input, context)
}

export function resolveOpenScadCircle(
  input: OpenScadRadialPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadRadialPrimitivePlan {
  return radialPlan('circle', input, context)
}

export function resolveOpenScadCylinder(
  input: OpenScadCylinderPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadCylinderPrimitivePlan {
  const common = radiusPair('cylinder', 'r', 'd', input.r, input.d, context)
  const low = radiusPair('cylinder', 'r1', 'd1', input.r1, input.d1, context)
  const high = radiusPair('cylinder', 'r2', 'd2', input.r2, input.d2, context)
  if (common.supplied && (low.supplied || high.supplied)) warn(context, {
    code: 'OPENSCAD_CYLINDER_AMBIGUOUS_RADII',
    message: 'cylinder combines a shared radius with an end-specific radius',
    primitive: 'cylinder',
  })

  const height = authoredNumber(input.h) ? input.h : 1
  const radius1 = low.supplied ? low.value : common.supplied ? common.value : 1
  const radius2 = high.supplied ? high.value : common.supplied ? common.value : 1
  const radius1Source: OpenScadCylinderPrimitivePlan['radius1Source'] = low.supplied
    ? low.source === 'diameter' ? 'diameter1' : 'radius1'
    : common.supplied ? common.source : 'default'
  const radius2Source: OpenScadCylinderPrimitivePlan['radius2Source'] = high.supplied
    ? high.source === 'diameter' ? 'diameter2' : 'radius2'
    : common.supplied ? common.source : 'default'
  const empty = !(height > 0) || !Number.isFinite(height)
    || radius1 < 0 || radius2 < 0
    || !Number.isFinite(radius1) || !Number.isFinite(radius2)
    || (radius1 === 0 && radius2 === 0)
  if (empty) rangeEmpty('cylinder', { height, radius1, radius2 }, context)

  return Object.freeze({
    kind: 'cylinder',
    height,
    radius1,
    radius2,
    center: input.center === true,
    radius1Source,
    radius2Source,
    fragmentRadius: Math.max(radius1, radius2),
    empty,
    reduced: false,
    reduction: null,
  })
}

export function convexity(value: unknown): number {
  if (!authoredNumber(value) || !Number.isFinite(value)) return 1
  return Math.max(1, Math.trunc(value))
}

export function limit(value: number | undefined, label: string): number {
  if (value === undefined) return Number.POSITIVE_INFINITY
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new RangeError(`${label} must be a non-negative safe integer`)
  }
  return value
}

export function normalizeLimits(limits: OpenScadPrimitiveSafetyLimits | undefined): NormalizedLimits {
  return Object.freeze({
    maximumPoints: limit(limits?.maximumPoints, 'maximumPoints'),
    maximumFacesOrPaths: limit(limits?.maximumFacesOrPaths, 'maximumFacesOrPaths'),
    maximumIndices: limit(limits?.maximumIndices, 'maximumIndices'),
  })
}

export function reduction(
  primitive: 'polyhedron' | 'polygon',
  resource: OpenScadPrimitiveReduction['resource'],
  actual: number,
  maximum: number,
  context: OpenScadStablePrimitiveContext,
): OpenScadPrimitiveReduction {
  const result = Object.freeze({ reason: 'input-limit' as const, resource, actual, limit: maximum })
  warn(context, {
    code: 'OPENSCAD_PRIMITIVE_SAFETY_REDUCTION',
    message: `${primitive} was not expanded because its ${resource} exceed the configured engine limit`,
    primitive,
    field: resource,
    actual,
    limit: maximum,
  })
  return result
}

export function asVector(value: unknown): readonly unknown[] {
  return Array.isArray(value) ? value : []
}

export function point3(value: unknown): OpenScadVec3 | null {
  if (exactNumericVector(value, 2)) {
    if (!value.every(Number.isFinite)) return null
    return freeze3(value[0], value[1], 0)
  }
  if (exactNumericVector(value, 3)) {
    if (!value.every(Number.isFinite)) return null
    return freeze3(value[0], value[1], value[2])
  }
  return null
}

export function point2(value: unknown): OpenScadVec2 | null {
  // The pinned polygon conversion rejects infinities here, while NaN remains
  // a numeric coordinate for the later polygon sanitizer to handle.
  if (!exactNumericVector(value, 2)
    || value.some(component => component === Number.POSITIVE_INFINITY
      || component === Number.NEGATIVE_INFINITY)) return null
  return freeze2(value[0], value[1])
}

export function unsignedIndex(value: unknown): number {
  if (!authoredNumber(value)) return 0
  if (!Number.isFinite(value)) return Number.MAX_SAFE_INTEGER
  const truncated = Math.trunc(value)
  if (truncated < 0 || truncated > Number.MAX_SAFE_INTEGER) return Number.MAX_SAFE_INTEGER
  return Object.is(truncated, -0) ? 0 : truncated
}

export function frozenPolygons3(polygons: OpenScadVec3[][]): readonly (readonly OpenScadVec3[])[] {
  return Object.freeze(polygons.map(polygon => Object.freeze([...polygon])))
}

export function frozenOutlines2(outlines: OpenScadVec2[][]): readonly (readonly OpenScadVec2[])[] {
  return Object.freeze(outlines.map(outline => Object.freeze([...outline])))
}

export function resolveOpenScadPolyhedron(
  input: OpenScadPolyhedronPrimitiveInput = {},
  context: OpenScadStablePrimitiveContext = SILENT_CONTEXT,
): OpenScadPolyhedronPrimitivePlan {
  const limits = normalizeLimits(input.limits)
  const points = asVector(input.points)
  if (points.length > limits.maximumPoints) {
    const limited = reduction('polyhedron', 'points', points.length, limits.maximumPoints, context)
    return Object.freeze({
      kind: 'polyhedron', polygons: Object.freeze([]), facesSource: 'default', convexity: convexity(input.convexity),
      aborted: true, sourcePointCount: points.length, empty: true, reduced: true, reduction: limited,
    })
  }

  const facesSource: OpenScadPolyhedronPrimitivePlan['facesSource'] = input.faces !== undefined
    ? 'faces'
    : input.triangles !== undefined ? 'triangles' : 'default'
  if (facesSource === 'triangles') warn(context, {
    code: 'OPENSCAD_POLYHEDRON_TRIANGLES_DEPRECATED',
    message: 'polyhedron triangles is a legacy alias; faces is the stable spelling',
    primitive: 'polyhedron',
    field: 'triangles',
  })
  const faces = asVector(facesSource === 'faces' ? input.faces : facesSource === 'triangles' ? input.triangles : undefined)
  if (faces.length > limits.maximumFacesOrPaths) {
    const limited = reduction('polyhedron', 'faces', faces.length, limits.maximumFacesOrPaths, context)
    return Object.freeze({
      kind: 'polyhedron', polygons: Object.freeze([]), facesSource, convexity: convexity(input.convexity),
      aborted: true, sourcePointCount: points.length, empty: true, reduced: true, reduction: limited,
    })
  }

  const polygons: OpenScadVec3[][] = []
  let indexCount = 0
  for (let faceIndex = 0; faceIndex < faces.length; faceIndex++) {
    const authoredFace = asVector(faces[faceIndex])
    const polygon: OpenScadVec3[] = []
    polygons.push(polygon)
    for (let facePointIndex = 0; facePointIndex < authoredFace.length; facePointIndex++) {
      indexCount++
      if (indexCount > limits.maximumIndices) {
        const limited = reduction('polyhedron', 'indices', indexCount, limits.maximumIndices, context)
        return Object.freeze({
          kind: 'polyhedron', polygons: frozenPolygons3(polygons), facesSource,
          convexity: convexity(input.convexity), aborted: true, sourcePointCount: points.length,
          empty: polygons.every(candidate => candidate.length < 3), reduced: true, reduction: limited,
        })
      }
      const index = unsignedIndex(authoredFace[facePointIndex])
      if (index >= points.length) {
        warn(context, {
          code: 'OPENSCAD_PRIMITIVE_INDEX_OUT_OF_BOUNDS',
          message: 'polyhedron skipped a face entry whose point index is outside the point vector',
          primitive: 'polyhedron',
          field: `faces[${faceIndex}]`,
          index,
          value: authoredFace[facePointIndex],
        })
        continue
      }
      const converted = point3(points[index])
      if (converted === null) {
        warn(context, {
          code: 'OPENSCAD_PRIMITIVE_POINT_INVALID',
          message: 'polyhedron stopped after a referenced point failed exact vec2/vec3 conversion',
          primitive: 'polyhedron',
          field: `points[${index}]`,
          index,
          value: points[index],
        })
        return Object.freeze({
          kind: 'polyhedron', polygons: frozenPolygons3(polygons), facesSource,
          convexity: convexity(input.convexity), aborted: true, sourcePointCount: points.length,
          empty: polygons.every(candidate => candidate.length < 3), reduced: false, reduction: null,
        })
      }
      polygon.push(converted)
    }
  }

  return Object.freeze({
    kind: 'polyhedron',
    polygons: frozenPolygons3(polygons),
    facesSource,
    convexity: convexity(input.convexity),
    aborted: false,
    sourcePointCount: points.length,
    empty: polygons.every(polygon => polygon.length < 3),
    reduced: false,
    reduction: null,
  })
}
