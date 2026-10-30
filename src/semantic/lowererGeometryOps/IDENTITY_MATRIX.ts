// lowererGeometryOps: IDENTITY_MATRIX.
import { resolveLegacyOpenScadSegments, roundLegacyOpenScadSegments } from '../../services/openScadLegacySegments'

import {
  type SemanticDimension,
  type SemanticDynamicSlot,
  type SemanticMatrix4,
  type SemanticValueType,
} from '../../core/semanticProgram'

import { clamp01 } from '../../services/math3d'

import {
  hasOpenScadViewportModifier,
  type CallNode,
  type ModuleNode,
  type Statement,
  type Value,
} from '../../services/openscadCompiler'

import {
  evaluationError,
  isStableProfile,
} from '../../services/openScadStableExpressionEval'

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
} from '.././lowererBuilder'

import {
  arg,
  evaluateStableCompatibilityArguments,
  finiteNumber,
  semanticProjectAssetRequired,
  stableCompatibilityDeprecation,
  stableWarning,
  vectorValue,
} from '.././lowererExpressionEval'



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
