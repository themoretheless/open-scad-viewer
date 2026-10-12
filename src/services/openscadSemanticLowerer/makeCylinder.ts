import type { CallNode, Statement, Value } from '../openscadCompiler'
import { LegacyBindingCompatibilityError, MAX_EXTRUDE_SLICES } from '../openscadSemanticLowerer'
import type { EvalContext, PassedCallChildren, SemanticShape } from '../openscadSemanticLowerer'
import { arg, evalNodes, evaluateStableCompatibilityArguments, semanticProjectAssetRequired, stableCompatibilityDeprecation } from './evalExpression'
import { canonicalNumber, finiteNumber, semanticValueType, stableWarning, vectorValue } from './semanticValueType'
import { evaluationError, isStableProfile } from '../openScadStableExpressionEval'
import { frameChildren, invoke, primitiveColor, semanticSegments, semanticSegmentsFromRequested, semanticSlot, unionReduceInternal } from './evalPreparedNodes'

export function makeCylinder(node: CallNode, ctx: EvalContext): SemanticShape[] {
  const height = finiteNumber(arg(node, 'h', 0, 1, ctx), ctx, node.p, 'cylinder height')
  if (height <= 0) evaluationError(ctx, node.p, 'Cylinder height must be positive')
  let low = arg(node, 'r1', 1, undefined, ctx)
  let high = arg(node, 'r2', 2, undefined, ctx)
  const radius = arg(node, 'r', 1, undefined, ctx)
  const diameter = arg(node, 'd', -1, undefined, ctx)
  const diameterLow = arg(node, 'd1', -1, undefined, ctx)
  const diameterHigh = arg(node, 'd2', -1, undefined, ctx)
  if (diameterLow !== undefined) low = finiteNumber(diameterLow, ctx, node.p, 'd1') / 2
  if (diameterHigh !== undefined) high = finiteNumber(diameterHigh, ctx, node.p, 'd2') / 2
  if (low === undefined && high === undefined) {
    const base = diameter !== undefined
      ? finiteNumber(diameter, ctx, node.p, 'diameter') / 2
      : radius !== undefined ? finiteNumber(radius, ctx, node.p, 'radius') : 1
    low = base
    high = base
  }
  if (low === undefined) low = high
  if (high === undefined) high = low
  const radiusBottom = finiteNumber(low, ctx, node.p, 'r1')
  const radiusTop = finiteNumber(high, ctx, node.p, 'r2')
  if (radiusBottom < 0 || radiusTop < 0 || (radiusBottom === 0 && radiusTop === 0)) {
    evaluationError(ctx, node.p, 'Cylinder radii must be non-negative and not both zero')
  }
  const center = arg(node, 'center', 3, false, ctx) === true
  const occurrence = invoke(node, ctx)
  const segments = semanticSegments(node, ctx, occurrence, 32, 3)
  return [ctx.builder.produce(occurrence, segments === null ? {
    kind: 'cylinder-analytic',
    valueType: semanticValueType(ctx.builder.languageContract, 'solid', 'd3'),
    height, radiusBottom, radiusTop, center,
  } : {
    kind: 'cylinder-polygonal',
    valueType: semanticValueType(ctx.builder.languageContract, 'solid-set', 'd3'),
    height, radiusBottom, radiusTop, center, radialSegments: segments,
  }, primitiveColor(ctx))]
}

export function makePolyhedron(node: CallNode, ctx: EvalContext): SemanticShape[] {
  const points = arg(node, 'points', 0, [], ctx)
  const faces = arg(node, 'faces', 1, arg(node, 'triangles', 1, [], ctx), ctx)
  if (!Array.isArray(points) || !Array.isArray(faces)) evaluationError(ctx, node.p, 'polyhedron points and faces must be vectors')
  const vertices = points.map((point, index) => {
    const vector = vectorValue(point, ctx, node.p, `polyhedron point ${index}`)
    if (vector.length < 3) evaluationError(ctx, node.p, 'Each polyhedron point needs three coordinates')
    return [vector[0], vector[1], vector[2]] as const
  })
  const triangles: Array<readonly [number, number, number]> = []
  for (const face of faces) {
    const polygon = vectorValue(face, ctx, node.p, 'polyhedron face')
      .map(value => canonicalNumber(Math.trunc(value)))
    if (polygon.length < 3) evaluationError(ctx, node.p, 'Each polyhedron face needs at least three vertices')
    for (const index of polygon) {
      if (index < 0 || index >= vertices.length) evaluationError(ctx, node.p, 'Polyhedron face index is out of bounds')
    }
    for (let index = 1; index < polygon.length - 1; index++) {
      triangles.push([polygon[0], polygon[index + 1], polygon[index]])
    }
  }
  if (vertices.length < 4) {
    if (ctx.builder.languageContract === 'legacy/current') {
      if (vertices.length !== 0 || faces.length !== 0) {
        evaluationError(ctx, node.p, 'Invalid manifold polyhedron: Not manifold')
      }
    } else {
      evaluationError(ctx, node.p, 'A solid polyhedron needs at least four points')
    }
  }
  const occurrence = invoke(node, ctx)
  return [ctx.builder.produce(occurrence, {
    kind: 'polyhedron',
    valueType: semanticValueType(ctx.builder.languageContract, 'solid-set', 'd3'),
    vertices,
    triangles,
  }, primitiveColor(ctx))]
}

export function makePolygon(node: CallNode, ctx: EvalContext): SemanticShape[] {
  const pointsValue = arg(node, 'points', 0, [], ctx)
  if (!Array.isArray(pointsValue)) evaluationError(ctx, node.p, 'polygon points must be a vector')
  const points = pointsValue.map(point => {
    const vector = vectorValue(point, ctx, node.p, 'polygon point')
    if (vector.length < 2) evaluationError(ctx, node.p, 'Each polygon point needs two coordinates')
    return [vector[0], vector[1]] as const
  })
  const pathsValue = arg(node, 'paths', 1, undefined, ctx)
  let rings: Array<readonly (readonly [number, number])[]>
  if (pathsValue === undefined) {
    if (ctx.builder.languageContract === 'legacy/current' && points.length === 0) {
      // The pinned JS binding receives the flat empty point vector here and
      // throws before it can reinterpret it as one empty ring.
      throw new LegacyBindingCompatibilityError("Cannot read properties of undefined (reading 'length')")
    }
    rings = [points]
  } else {
    if (!Array.isArray(pathsValue)) evaluationError(ctx, node.p, 'polygon paths must be a vector')
    rings = pathsValue.map(path => vectorValue(path, ctx, node.p, 'polygon path').map(rawIndex => {
      const point = points[Math.trunc(rawIndex)]
      if (!point) evaluationError(ctx, node.p, 'Polygon path index is out of bounds')
      return point
    }))
  }
  if (rings.length === 0) {
    if (ctx.builder.languageContract === 'legacy/current') {
      throw new LegacyBindingCompatibilityError("Cannot read properties of undefined (reading 'length')")
    }
    evaluationError(ctx, node.p, 'Polygon rings need at least three points')
  }
  if (ctx.builder.languageContract !== 'legacy/current' && rings.some(ring => ring.length < 3)) {
    evaluationError(ctx, node.p, 'Polygon rings need at least three points')
  }
  const occurrence = invoke(node, ctx)
  return [ctx.builder.produce(occurrence, {
    kind: 'polygon',
    valueType: semanticValueType(ctx.builder.languageContract, 'region', 'd2'),
    rings,
    fillRule: 'even-odd',
  }, primitiveColor(ctx))]
}

export function compatibilityRequestedSegments(
  values: ReadonlyMap<string, Value>,
  node: CallNode,
  ctx: EvalContext,
): number | null {
  const local = values.get('$fn')
  const global = ctx.env.get('$fn')
  const raw = local === undefined || local === 0 ? global : local
  return raw === undefined || raw === 0
    ? null
    : Math.round(finiteNumber(raw, ctx, node.p, '$fn'))
}

export function compatibilityPlanarChildren(
  node: CallNode,
  occurrence: number,
  ctx: EvalContext,
): SemanticShape[] {
  const children = frameChildren(node, node.children, occurrence, ctx)
  const planar = children.filter(shape => shape.dimension === 'region2')
  if (planar.length !== children.length) {
    stableWarning(
      ctx,
      node.p,
      `${node.name}() ignored non-2D child geometry`,
      'W_OPENSCAD_COMPATIBILITY_CHILD',
    )
  }
  return planar
}

export function evalDxfLinearExtrude(node: CallNode, ctx: EvalContext): SemanticShape[] {
  stableCompatibilityDeprecation(node, ctx, 'linear_extrude()')
  const evaluated = evaluateStableCompatibilityArguments(
    node,
    ['file', 'layer', 'height', 'origin', 'scale', 'center', 'twist', 'slices'],
    ['convexity', '$fn', '$fa', '$fs'],
    ctx,
  )
  const file = evaluated.values.get('file')
  if (typeof file === 'string' && file.length > 0) {
    semanticProjectAssetRequired(ctx, node.p, node.end, 'dxf_linear_extrude()')
  }

  let heightValue = evaluated.values.get('height')
  const first = evaluated.authored[0]
  if (heightValue === undefined && first?.name === undefined) {
    const firstValue = evaluated.evaluated.get(first)
    if (typeof firstValue === 'number') heightValue = firstValue
  }
  let height = typeof heightValue === 'number' && Number.isFinite(heightValue) ? heightValue : 100
  if (height <= 0) height = 0

  const rawScale = evaluated.values.get('scale')
  let scale: readonly [number, number] = [1, 1]
  if (typeof rawScale === 'number' && Number.isFinite(rawScale)) {
    scale = [Math.max(0, rawScale), Math.max(0, rawScale)]
  } else if (Array.isArray(rawScale) && rawScale.length === 2) {
    const scaleX = rawScale[0]
    const scaleY = rawScale[1]
    if (typeof scaleX === 'number' && Number.isFinite(scaleX)
      && typeof scaleY === 'number' && Number.isFinite(scaleY)) {
      scale = [Math.max(0, scaleX), Math.max(0, scaleY)]
    }
  }
  const twistValue = evaluated.values.get('twist')
  const twistDegrees = typeof twistValue === 'number' && Number.isFinite(twistValue) ? twistValue : 0
  const slicesValue = evaluated.values.get('slices')
  const slices = typeof slicesValue === 'number' && Number.isFinite(slicesValue) && slicesValue > 0
    ? Math.min(MAX_EXTRUDE_SLICES, Math.trunc(slicesValue))
    : 0
  const center = evaluated.values.get('center') === true

  const occurrence = invoke(node, ctx)
  const shapes = compatibilityPlanarChildren(node, occurrence, ctx)
  if (height === 0 || shapes.length === 0) return []
  const profile = unionReduceInternal(shapes, ctx, node.p)!
  const inputType = ctx.builder.nodes[profile.node].valueType
  return [ctx.builder.produce(occurrence, {
    kind: 'linear-extrude',
    valueType: { ...inputType, geometryKind: 'solid-set', space: 'd3' },
    input: profile.node,
    height,
    twistDegrees,
    slices,
    scale,
    center,
  }, profile.color)]
}

export function evalDxfRotateExtrude(node: CallNode, ctx: EvalContext): SemanticShape[] {
  stableCompatibilityDeprecation(node, ctx, 'rotate_extrude()')
  const evaluated = evaluateStableCompatibilityArguments(
    node,
    ['file', 'layer', 'origin', 'scale'],
    ['convexity', 'angle', '$fn', '$fa', '$fs'],
    ctx,
  )
  const file = evaluated.values.get('file')
  if (file !== undefined && (typeof file !== 'string' || file.length > 0)) {
    semanticProjectAssetRequired(ctx, node.p, node.end, 'dxf_rotate_extrude()')
  }
  const rawAngle = evaluated.values.get('angle')
  let angleDegrees = typeof rawAngle === 'number' && Number.isFinite(rawAngle) ? rawAngle : 360
  if (angleDegrees <= -360 || angleDegrees > 360) angleDegrees = 360

  const occurrence = invoke(node, ctx)
  const shapes = compatibilityPlanarChildren(node, occurrence, ctx)
  if (angleDegrees === 0 || shapes.length === 0) return []
  const profile = unionReduceInternal(shapes, ctx, node.p)!
  const requested = compatibilityRequestedSegments(evaluated.values, node, ctx)
  const segments = semanticSegmentsFromRequested(node, ctx, occurrence, requested, 48, 3)
  const valueType = {
    ...ctx.builder.nodes[profile.node].valueType,
    geometryKind: 'solid-set' as const,
    space: 'd3' as const,
  }
  return [ctx.builder.produce(occurrence, segments === null ? {
    kind: 'rotate-extrude-analytic', valueType, input: profile.node, angleDegrees,
  } : {
    kind: 'rotate-extrude-polygonal', valueType, input: profile.node, angleDegrees, radialSegments: segments,
  }, profile.color)]
}

export function evalPassedCallChildren(
  node: CallNode,
  ctx: EvalContext,
  occurrence: number,
  indexValue: Value,
  passed: Readonly<PassedCallChildren>,
  statements: readonly Statement[],
): SemanticShape[] {
  if (statements.length === 0) return []
  const currentOperation = ctx.builder.registered.byStatement.get(node)
  const expansionOperation = ctx.builder.registered.expansionByCall.get(node)
    ?? passed.expansionOperation
  const expansionPath = ctx.builder.registered.operations[expansionOperation].structuralPath
  const currentPath = currentOperation === undefined
    ? []
    : ctx.builder.registered.operations[currentOperation].structuralPath
  const legacyReentry = !isStableProfile(ctx)
    && ctx.builder.languageContract === 'legacy/current'
    && expansionPath.length < currentPath.length
    && expansionPath.every((segment, index) => {
      const candidate = currentPath[index]
      return segment.kind === candidate.kind
        && segment.name === candidate.name
        && segment.ordinal === candidate.ordinal
    })
  const expansion = legacyReentry
    ? occurrence
    : ctx.builder.invocationOperation(
      expansionOperation,
      occurrence,
      [semanticSlot('$index', indexValue)],
    )
  return evalNodes(statements, {
    ...ctx,
    env: new Map(passed.env),
    stableScope: isStableProfile(ctx) ? passed.scope ?? ctx.stableScope : ctx.stableScope,
    scopeVisibleBefore: Number.POSITIVE_INFINITY,
    parentOccurrence: expansion,
    pendingSlots: [],
    callChildren: !isStableProfile(ctx) && ctx.builder.languageContract === 'legacy/current'
      ? passed
      : passed.continuation,
  })
}
