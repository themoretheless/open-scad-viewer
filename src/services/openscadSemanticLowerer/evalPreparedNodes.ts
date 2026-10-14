import { CSS_COLORS, IDENTITY_MATRIX, LegacyBindingCompatibilityError, MAX_SHAPES } from '../openscadSemanticLowerer'
import type { EvalContext, RGBA, SemanticNodeWithoutId, SemanticReducedShape, SemanticShape } from '../openscadSemanticLowerer'
import { hasOpenScadViewportModifier } from '../openscadCompiler'
import type { CallNode, ModuleNode, Statement, Value } from '../openscadCompiler'
import { MAX_EVAL_OPS, evaluationError, isStableProfile } from '../openScadStableExpressionEval'
import { OpenSCADParseError } from '../openscadErrors'
import type { SemanticDimension, SemanticDynamicSlot, SemanticMatrix4, SemanticValueType } from '../../core/semanticProgram'
import { arg, evalExpression, evalNodes, viewportSemanticSlots } from './evalExpression'
import { canonicalNumber, finiteNumber, identityValue, poll, semanticValueType, vectorValue } from './semanticValueType'
import { clamp01 } from '../math3d'
import { evalNode } from './evalNode'
import { resolveLegacyOpenScadSegments, roundLegacyOpenScadSegments } from '../openScadLegacySegments'

export function evalPreparedNodes(nodes: readonly Statement[], ctx: EvalContext): SemanticShape[] {
  const output: SemanticShape[] = []
  for (const statement of nodes) {
    if (isStableProfile(ctx) && statement.type === 'call'
      && hasOpenScadViewportModifier(statement, 'disable')) {
      // `*` is the sole modifier which suppresses language evaluation itself.
      continue
    }
    const occurrenceStart = ctx.builder.occurrences.length
    const background = isStableProfile(ctx) && statement.type === 'call'
      && ctx.viewportRootOwner !== statement
      && hasOpenScadViewportModifier(statement, 'background')
    const backgroundCheckpoint = background && ctx.quality === 'full'
      ? ctx.builder.runtimeCheckpoint()
      : null
    try {
      poll(ctx)
      if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, statement.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
      if (statement.type === 'assign') {
        ctx.builder.invocation(statement, ctx.parentOccurrence, ctx.pendingSlots)
        if (!isStableProfile(ctx)) ctx.env.set(statement.name, evalExpression(statement.value, ctx))
        continue
      }
      if (statement.type === 'module') {
        ctx.builder.invocation(statement, ctx.parentOccurrence, ctx.pendingSlots)
        continue
      }
      if (statement.type === 'function') continue
      const shapes = evalNode(statement, ctx)
      if (backgroundCheckpoint !== null) {
        ctx.builder.rollbackRuntime(backgroundCheckpoint)
      } else {
        output.push(...shapes)
        if (background && shapes.length > 0) ctx.reduced.value = true
      }
      if (output.length > MAX_SHAPES) evaluationError(ctx, statement.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
    } catch (error) {
      const capturable = error instanceof OpenSCADParseError
        || error instanceof LegacyBindingCompatibilityError
      if (capturable && ctx.builder.terminalOccurrence === null) {
        const operation = ctx.builder.registered.byStatement.get(statement)
        // Full-profile echo is intentionally transparent in semantic-program
        // v1. An expression error inside that effect has no static occurrence
        // to attach to, so preserve the positioned language error instead of
        // replacing it with an internal registry failure.
        if (operation === undefined) throw error
        let occurrence = -1
        for (let index = ctx.builder.occurrences.length - 1; index >= occurrenceStart; index--) {
          const candidate = ctx.builder.occurrences[index]
          if (candidate.operation === operation && candidate.parent === ctx.parentOccurrence) {
            occurrence = index
            break
          }
        }
        if (occurrence < 0) {
          occurrence = ctx.builder.invocation(statement, ctx.parentOccurrence, ctx.pendingSlots)
        }
        ctx.builder.captureTerminalOccurrence(occurrence)
      }
      throw error
    }
  }
  return output
}

export function frameChildren(
  owner: CallNode | ModuleNode,
  statements: readonly Statement[],
  ownerOccurrence: number,
  ctx: EvalContext,
  alternative = false,
): SemanticShape[] {
  const framed = frameContext(owner, ownerOccurrence, ctx, alternative)
  return framed ? evalNodes(statements, framed) : []
}

export function frameContext(
  owner: CallNode | ModuleNode,
  ownerOccurrence: number,
  ctx: EvalContext,
  alternative = false,
): EvalContext | null {
  const operation = alternative && owner.type === 'call'
    ? ctx.builder.registered.alternativeByCall.get(owner)
    : ctx.builder.registered.bodyByStatement.get(owner)
  if (operation === undefined) return null
  const frame = ctx.builder.invocationOperation(operation, ownerOccurrence)
  return { ...ctx, parentOccurrence: frame, pendingSlots: [] }
}

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

export function axisAngleMatrix(axis: readonly number[], degrees: number, ctx: EvalContext, position: number): SemanticMatrix4 {
  const length = Math.hypot(axis[0] ?? 0, axis[1] ?? 0, axis[2] ?? 0)
  if (length === 0) evaluationError(ctx, position, 'Rotation axis cannot be zero')
  const x = (axis[0] ?? 0) / length, y = (axis[1] ?? 0) / length, z = (axis[2] ?? 0) / length
  const angle = degrees * Math.PI / 180, c = Math.cos(angle), s = Math.sin(angle), t = 1 - c
  return [
    canonicalNumber(t*x*x+c), canonicalNumber(t*x*y+s*z), canonicalNumber(t*x*z-s*y), 0,
    canonicalNumber(t*x*y-s*z), canonicalNumber(t*y*y+c), canonicalNumber(t*y*z+s*x), 0,
    canonicalNumber(t*x*z+s*y), canonicalNumber(t*y*z-s*x), canonicalNumber(t*z*z+c), 0,
    0, 0, 0, 1,
  ]
}

export function eulerMatrix(x: number, y: number, z: number): SemanticMatrix4 {
  const rx = axisAngleMatrixUnchecked([1, 0, 0], x)
  const ry = axisAngleMatrixUnchecked([0, 1, 0], y)
  const rz = axisAngleMatrixUnchecked([0, 0, 1], z)
  return multiplyMatrix(rz, multiplyMatrix(ry, rx))
}

export function axisAngleMatrixUnchecked(axis: readonly [number, number, number], degrees: number): SemanticMatrix4 {
  const angle = degrees * Math.PI / 180, c = Math.cos(angle), s = Math.sin(angle), t = 1 - c
  const [x, y, z] = axis
  return [
    canonicalNumber(t*x*x+c), canonicalNumber(t*x*y+s*z), canonicalNumber(t*x*z-s*y), 0,
    canonicalNumber(t*x*y-s*z), canonicalNumber(t*y*y+c), canonicalNumber(t*y*z+s*x), 0,
    canonicalNumber(t*x*z+s*y), canonicalNumber(t*y*z-s*x), canonicalNumber(t*z*z+c), 0,
    0, 0, 0, 1,
  ]
}

export function mirrorMatrix(
  vector: readonly number[],
  dimension: SemanticDimension,
  ctx: EvalContext,
  position: number,
): SemanticMatrix4 {
  const zValue = dimension === 'region2' ? 0 : vector[2] ?? 0
  const length = Math.hypot(vector[0] ?? 0, vector[1] ?? 0, zValue)
  if (length === 0) {
    if (ctx.builder.languageContract === 'openscad-viewer/brep-1') {
      evaluationError(ctx, position, 'Mirror normal cannot be zero')
    }
    return [
      0, 0, 0, 0,
      0, 0, 0, 0,
      0, 0, dimension === 'region2' ? 1 : 0, 0,
      0, 0, 0, 1,
    ]
  }
  const x = (vector[0] ?? 0) / length, y = (vector[1] ?? 0) / length, z = zValue / length
  return [
    canonicalNumber(1-2*x*x), canonicalNumber(-2*x*y), canonicalNumber(-2*x*z), 0,
    canonicalNumber(-2*y*x), canonicalNumber(1-2*y*y), canonicalNumber(-2*y*z), 0,
    canonicalNumber(-2*z*x), canonicalNumber(-2*z*y), canonicalNumber(1-2*z*z), 0,
    0, 0, 0, 1,
  ]
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
  const selection=resolveLegacyOpenScadSegments(requested,fallback,minimum,ctx.quality)
  const maximum=selection.maximum,segments=selection.beforeCap
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
  if(selection.reduced)ctx.reduced.value=true
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
