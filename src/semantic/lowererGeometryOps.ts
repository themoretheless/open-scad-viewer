import { resolveLegacyOpenScadSegments, roundLegacyOpenScadSegments } from '../services/openScadLegacySegments'
import {
  type SemanticDimension,
  type SemanticDynamicSlot,
  type SemanticMatrix4,
  type SemanticValueType,
} from '../core/semanticProgram'
import { clamp01 } from '../services/math3d'
import {
  hasOpenScadViewportModifier,
  type CallNode,
  type ModuleNode,
  type Statement,
  type Value,
} from '../services/openscadCompiler'
import {
  evaluationError,
  isStableProfile,
} from '../services/openScadStableExpressionEval'
import {
  canonicalNumber,
  CSS_COLORS,
  identityValue,
  LegacyBindingCompatibilityError,
  MAX_EXTRUDE_SLICES,
  semanticValueType,
  type EvalContext,
  type RGBA,
  type SemanticNodeWithoutId,
  type SemanticShape,
} from './lowererBuilder'
import {
  arg,
  evaluateStableCompatibilityArguments,
  finiteNumber,
  semanticProjectAssetRequired,
  stableCompatibilityDeprecation,
  stableWarning,
  vectorValue,
} from './lowererExpressionEval'

export const IDENTITY_MATRIX: SemanticMatrix4 = [
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
]

export function multiplyMatrix(left: SemanticMatrix4, right: SemanticMatrix4): SemanticMatrix4 {
  const output = new Array<number>(16).fill(0)
  for (let column = 0; column < 4; column++) {
    for (let row = 0; row < 4; row++) {
      for (let inner = 0; inner < 4; inner++) output[column * 4 + row] += left[inner * 4 + row] * right[column * 4 + inner]
      output[column * 4 + row] = canonicalNumber(output[column * 4 + row])
    }
  }
  return output as unknown as SemanticMatrix4
}

export function translationMatrix(x: number, y: number, z: number): SemanticMatrix4 {
  const matrix = [...IDENTITY_MATRIX] as number[]
  matrix[12] = x; matrix[13] = y; matrix[14] = z
  return matrix as unknown as SemanticMatrix4
}

export function scaleMatrix(x: number, y: number, z: number): SemanticMatrix4 {
  const matrix = [...IDENTITY_MATRIX] as number[]
  matrix[0] = x; matrix[5] = y; matrix[10] = z
  return matrix as unknown as SemanticMatrix4
}

export function matrixValue(value: Value, ctx: EvalContext, position: number): SemanticMatrix4 {
  if (!Array.isArray(value) || value.length < 3) evaluationError(ctx, position, 'multmatrix requires a 4x4 matrix')
  const rows = value.map(row => vectorValue(row, ctx, position, 'matrix row'))
  if (rows.some(row => row.length < 4)) evaluationError(ctx, position, 'multmatrix requires a 4x4 matrix')
  const matrix: number[] = []
  for (let column = 0; column < 4; column++) {
    for (let row = 0; row < 4; row++) matrix.push(canonicalNumber(rows[row]?.[column] ?? (row === column ? 1 : 0)))
  }
  if (matrix[3] !== 0 || matrix[7] !== 0 || matrix[11] !== 0 || matrix[15] !== 1) {
    if (ctx.builder.languageContract === 'legacy/current') {
      // The pinned Manifold transform consumes only the affine 3x4 portion and
      // ignores the authored fourth row. Canonicalize that legacy equivalence
      // instead of weakening the SPE1 affine-matrix invariant.
      matrix[3] = 0
      matrix[7] = 0
      matrix[11] = 0
      matrix[15] = 1
    } else {
      evaluationError(ctx, position, 'multmatrix must be affine')
    }
  }
  return matrix as unknown as SemanticMatrix4
}

export function parseColor(value: Value, ctx: EvalContext, position: number): RGBA {
  if (Array.isArray(value)) {
    const channels = vectorValue(value, ctx, position, 'color')
    return [clamp01(channels[0] ?? 0.5), clamp01(channels[1] ?? 0.5), clamp01(channels[2] ?? 0.5), clamp01(channels[3] ?? 1)]
  }
  if (typeof value !== 'string') evaluationError(ctx, position, 'color() expects a name or RGB(A) vector')
  const named = CSS_COLORS[value.toLowerCase()]
  if (named) return [...named]
  const match = value.match(/^#([0-9a-f]{6}|[0-9a-f]{8})$/i)
  if (!match) evaluationError(ctx, position, `Unknown color ${value}`)
  const hex = match[1]
  return [
    parseInt(hex.slice(0, 2), 16) / 255,
    parseInt(hex.slice(2, 4), 16) / 255,
    parseInt(hex.slice(4, 6), 16) / 255,
    hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1,
  ]
}

export function semanticSlot(name: string, value: Value): SemanticDynamicSlot {
  return { name, value: identityValue(value), duplicateOrdinal: 0 }
}

export function viewportSemanticSlots(node: CallNode, ctx: EvalContext): SemanticDynamicSlot[] {
  if (!isStableProfile(ctx) || ctx.viewportRootOwner === node) return []
  const slots: SemanticDynamicSlot[] = []
  if (hasOpenScadViewportModifier(node, 'highlight')) {
    slots.push(semanticSlot('$viewport-highlight', true))
  }
  if (hasOpenScadViewportModifier(node, 'background')) {
    slots.push(semanticSlot('$viewport-background', true))
  }
  return slots
}

export function viewportRootActivates(node: CallNode, shapes: readonly SemanticShape[]): boolean {
  if (shapes.length > 0) return true
  // These effect/branch calls do not contribute an empty CSG container in
  // OpenSCAD 2021.01. Other evaluated module instantiations do, so a `!` on
  // union(), transform{}, group{}, for(empty), or an empty user module still
  // wins and deliberately yields an empty top-level result.
  return !['if', 'assert', 'echo', 'children', 'child'].includes(node.name)
}

export function invoke(
  node: CallNode | ModuleNode,
  ctx: EvalContext,
  slots: readonly SemanticDynamicSlot[] = [],
): number {
  const viewportSlots = node.type === 'call' ? viewportSemanticSlots(node, ctx) : []
  return ctx.builder.invocation(
    node,
    ctx.parentOccurrence,
    [...ctx.pendingSlots, ...viewportSlots, ...slots],
  )
}

export function nodeValueType(shape: SemanticShape, ctx: EvalContext): SemanticValueType {
  return ctx.builder.nodes[shape.node].valueType
}

export function requestedSegments(node: CallNode, ctx: EvalContext): number | null {
  const local = arg(node, '$fn', -1, undefined, ctx)
  const global = ctx.env.get('$fn')
  const raw = local === undefined || local === 0 ? global : local
  return raw === undefined || raw === 0
    ? null
    : roundLegacyOpenScadSegments(finiteNumber(raw, ctx, node.p, '$fn'))
}

export function semanticSegments(
  node: CallNode,
  ctx: EvalContext,
  occurrence: number,
  fallback: number,
  minimum: number,
): number | null {
  const requested = requestedSegments(node, ctx)
  return semanticSegmentsFromRequested(node, ctx, occurrence, requested, fallback, minimum)
}

export function semanticSegmentsFromRequested(
  node: CallNode,
  ctx: EvalContext,
  occurrence: number,
  requested: number | null,
  fallback: number,
  minimum: number,
): number | null {
  if (ctx.builder.languageContract === 'openscad-viewer/brep-1') {
    ctx.builder.addTessellationIntent(occurrence, requested, ctx.env)
    return null
  }
  const selection = resolveLegacyOpenScadSegments(requested, fallback, minimum, ctx.quality)
  const maximum = selection.maximum, segments = selection.beforeCap
  if (selection.clamped) {
    const operation = ctx.builder.registered.byStatement.get(node) ?? null
    ctx.builder.warn(
      operation,
      'SEMANTIC_SEGMENTS_CLAMPED',
      [
        { name: 'requested', value: identityValue(segments) },
        { name: 'maximum', value: identityValue(maximum) },
      ],
      `$fn=${segments} was clamped to ${maximum} for ${ctx.quality} rendering`,
      { start: node.p, end: node.end },
    )
  }
  if (selection.reduced) ctx.reduced.value = true
  return selection.segments
}

export function primitiveColor(ctx: EvalContext): RGBA { return ctx.builder.nextColor() }

export function transformShapes(
  occurrence: number,
  shapes: readonly SemanticShape[],
  matrixFor: (shape: SemanticShape) => SemanticMatrix4,
  ctx: EvalContext,
): SemanticShape[] {
  return shapes.map(shape => ctx.builder.produce(occurrence, {
    kind: 'transform',
    valueType: nodeValueType(shape, ctx),
    input: shape.node,
    matrix: matrixFor(shape),
  }, shape.color, shape.identityOccurrence))
}

export function combineShapes(
  occurrence: number,
  shapes: readonly SemanticShape[],
  operation: 'union' | 'intersection' | 'difference' | 'hull',
  ctx: EvalContext,
  position: number,
): SemanticShape[] {
  if (shapes.length === 0) return []
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) {
    evaluationError(ctx, position, `${operation}() cannot mix 2D and 3D children`)
  }
  if (shapes.length === 1) return ctx.builder.alias(occurrence, shapes, operation === 'hull')
  const firstType = nodeValueType(shapes[0], ctx)
  const valueType: SemanticValueType = {
    ...firstType,
    geometryKind: dimension === 'region2' ? 'region' : 'solid-set',
    space: dimension === 'region2' ? 'd2' : 'd3',
  }
  const common = {
    valueType,
    inputs: shapes.map(shape => shape.node),
  }
  const semanticNode: SemanticNodeWithoutId = operation === 'hull'
    ? { kind: 'hull', ...common }
    : { kind: 'boolean', operation, ...common }
  return [ctx.builder.produce(occurrence, semanticNode, shapes[0].color)]
}

export interface SemanticReducedShape {
  readonly node: number
  readonly dimension: SemanticDimension
  readonly color: RGBA
  readonly identityOccurrence: number | null
}

export function unionReduceInternal(
  shapes: readonly SemanticShape[],
  ctx: EvalContext,
  position: number,
): SemanticReducedShape | null {
  if (shapes.length === 0) return null
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) evaluationError(ctx, position, 'union() cannot mix 2D and 3D children')
  if (shapes.length === 1) return {
    node: shapes[0].node,
    dimension,
    color: shapes[0].color,
    identityOccurrence: shapes[0].identityOccurrence,
  }
  const firstType = nodeValueType(shapes[0], ctx)
  const node = ctx.builder.addInternalNode({
    kind: 'boolean',
    operation: 'union',
    inputs: shapes.map(shape => shape.node),
    valueType: {
      ...firstType,
      geometryKind: dimension === 'region2' ? 'region' : 'solid-set',
      space: dimension === 'region2' ? 'd2' : 'd3',
    },
  })
  return { node, dimension, color: shapes[0].color, identityOccurrence: null }
}

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
    : roundLegacyOpenScadSegments(finiteNumber(raw, ctx, node.p, '$fn'))
}

export function compatibilityPlanarChildren(
  node: CallNode,
  occurrence: number,
  ctx: EvalContext,
  frameChildrenFn: (owner: CallNode | ModuleNode, statements: readonly Statement[], ownerOccurrence: number, ctx: EvalContext) => SemanticShape[],
): SemanticShape[] {
  const children = frameChildrenFn(node, node.children, occurrence, ctx)
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

export function evalDxfLinearExtrude(
  node: CallNode,
  ctx: EvalContext,
  frameChildrenFn: (owner: CallNode | ModuleNode, statements: readonly Statement[], ownerOccurrence: number, ctx: EvalContext) => SemanticShape[],
): SemanticShape[] {
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
  const shapes = compatibilityPlanarChildren(node, occurrence, ctx, frameChildrenFn)
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

export function evalDxfRotateExtrude(
  node: CallNode,
  ctx: EvalContext,
  frameChildrenFn: (owner: CallNode | ModuleNode, statements: readonly Statement[], ownerOccurrence: number, ctx: EvalContext) => SemanticShape[],
): SemanticShape[] {
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
  const shapes = compatibilityPlanarChildren(node, occurrence, ctx, frameChildrenFn)
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
