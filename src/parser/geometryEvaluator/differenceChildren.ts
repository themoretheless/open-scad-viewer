// geometryEvaluator: differenceChildren.
import type { CadKernelHandle } from '../../services/cadKernelOps'

import { clamp01 } from '../../services/math3d'

import { resolveLegacyOpenScadSegments } from '../../services/openScadLegacySegments'

import type { CallNode, ExpressionArgument, Statement, Value } from '../../services/openscadCompiler'

import { formatOpenScadValue } from '../../services/openScadValueSemantics'

import {
  resolveOpenScadFragments,
  type OpenScadFragmentResolutionInput,
  type OpenScadStableModuleSemanticsContext,
} from '../../services/openScadStableModuleSemantics'

import {
  resolveOpenScadCircle,
  resolveOpenScadCube,
  resolveOpenScadCylinder,
  resolveOpenScadPolygon,
  resolveOpenScadPolyhedron,
  resolveOpenScadSphere,
  resolveOpenScadSquare,
  type OpenScadStablePrimitiveContext,
} from '../../services/openScadStablePrimitiveSemantics'

import {
  resolveOpenScadMirror,
  resolveOpenScadMultmatrix,
  resolveOpenScadRotate,
  resolveOpenScadScale,
  resolveOpenScadTranslate,
  resolveOpenScadViewerAxisAngle,
  type OpenScadStableTransformContext,
  type OpenScadStableTransformPlan,
} from '../../services/openScadStableTransformSemantics'

import {
  resolveOpenScad2dMultmatrix,
  resolveOpenScadLinearExtrude,
  resolveOpenScadResize,
  resolveOpenScadRotateExtrude,
  type OpenScadStableGeometryContext,
} from '../../services/openScadStableGeometrySemantics'

import {
  evaluationError,
  isStableProfile,
} from '../../services/openScadStableExpressionEval'

import { CSS_COLORS, nextColor, type RGBA } from '../../services/openscadColors'

import {
  compatibilityDxfProfile,
  makeImport,
  makeSurface,
  makeText,
} from '.././assetEvaluator'

import {
  MAX_EXTRUDE_SLICES,
  type EvalContext,
  type Shape,
  type Shape2D,
  type Shape3D,
  type Vec2,
  type Vec3,
} from '.././types'

import {
  callExpressionArguments,
  finiteNumber,
  vectorValue,
} from '.././astDiscovery'

import {
  arg,
  compatibilityString,
  currentEntityId,
  enterStableStatementScope,
  evalExpression,
  evaluateStableBuiltinModuleArguments,
  evaluateStableCompatibilityArguments,
  kernelCall,
  resolveStableVariable,
  trackedSolid,
  warn,
} from '.././expressionEvaluator'
import { EvalNodesFn, EvalPreparedNodesFn, stableGeometrySemanticsContext, stableFragmentInputFromEvaluated } from './EvalNodesFn'
import { booleanShapes, homogeneousShapes } from './segments'


export async function differenceChildren(
  node: CallNode,
  ctx: EvalContext,
  evalNodes: EvalNodesFn,
  evalPreparedNodes: EvalPreparedNodesFn,
): Promise<Shape[]> {
  if (node.children.length === 0) return []
  const childContext = isStableProfile(ctx)
    ? enterStableStatementScope(node.children, ctx)
    : null
  const base = booleanShapes(
    childContext === null
      ? await evalNodes([node.children[0]], ctx)
      : await evalPreparedNodes([node.children[0]], childContext),
    'union',
    ctx,
    node.p,
  )
  // Every child statement is one cutter operand (a loop or module child is the
  // union of its own shapes), matching the SemanticProgram lane byte for byte.
  // The operands stay separate: A \ (B1 ∪ B2 …) equals ((A \ B1) \ B2) …, and
  // the kernel subtracts bound-separated cutters in small local batches
  // instead of first unioning them into one large cutter.
  const cutterOperands: Shape[] = []
  for (const child of node.children.slice(1)) {
    const shapes = childContext === null
      ? await evalNodes([child], ctx)
      : await evalPreparedNodes([child], childContext)
    cutterOperands.push(...booleanShapes(shapes, 'union', ctx, node.p))
  }
  const cutters = homogeneousShapes(cutterOperands, 'union', ctx, node.p)
  if (base.length === 0 || cutters.length === 0) return base
  if (base[0].dimension !== cutters[0].dimension) {
    if (isStableProfile(ctx)) {
      warn(ctx, 'difference() ignored child geometry with a different dimension')
      return base
    }
    evaluationError(ctx, node.p, 'difference() cannot mix 2D and 3D children')
  }
  ctx.control?.poll()
  if (base[0].dimension === 3) {
    const operands = [base[0].geometry, ...cutters.map(cutter => (cutter as Shape3D).geometry)]
    return [{
      dimension: 3,
      geometry: kernelCall(ctx, node.p, () => ctx.kernel.boolean3('difference', operands)),
      color: base[0].color,
      entityId: currentEntityId(ctx),
    }]
  }
  const operands = [base[0].geometry, ...cutters.map(cutter => (cutter as Shape2D).geometry)]
  return [{
    dimension: 2,
    geometry: kernelCall(ctx, node.p, () => ctx.kernel.boolean2('difference', operands)),
    color: base[0].color,
    entityId: currentEntityId(ctx),
  }]
}


export function hullShapes(shapes: Shape[], node: CallNode, ctx: EvalContext): Shape[] {
  if (shapes.length === 0) return []
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) {
    if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'hull() cannot mix 2D and 3D children')
    warn(ctx, 'hull() ignored child geometry with a different dimension')
    shapes = shapes.filter(shape => shape.dimension === dimension)
  }
  ctx.control?.poll()
  if (dimension === 3) {
    const geometry = kernelCall(ctx, node.p, () => ctx.kernel.hull3(shapes.map(shape => (shape as Shape3D).geometry)))
    return [trackedSolid(geometry, shapes[0].color, node, ctx)]
  }
  return [{
    dimension: 2,
    geometry: kernelCall(ctx, node.p, () => ctx.kernel.hull2(shapes.map(shape => (shape as Shape2D).geometry))),
    color: shapes[0].color,
    entityId: currentEntityId(ctx),
  }]
}


export async function resizeChildren(
  node: CallNode,
  ctx: EvalContext,
  evalNodes: EvalNodesFn,
): Promise<Shape[]> {
  const evaluated = evaluateStableBuiltinModuleArguments(
    node,
    ['newsize', 'auto', 'convexity'],
    [],
    ctx,
  )
  const rawNewsize = evaluated.get('newsize')
  const rawAuto = evaluated.get('auto') ?? false
  const shapes = await evalNodes(node.children, ctx)
  if (shapes.length === 0) return shapes

  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) {
    evaluationError(ctx, node.p, 'resize() cannot mix 2D and 3D children')
  }

  const bounds = shapes
    .filter(shape => !ctx.kernel.isEmpty(shape.geometry))
    .map(shape => ctx.kernel.bounds(shape.geometry))
  if (bounds.length === 0) return shapes

  const resolved = resolveOpenScadResize({
    dimension,
    bounds,
    // Omitted newsize has the stable identity default. An explicitly invalid
    // non-vector still reaches the helper and produces a soft diagnostic.
    newsize: rawNewsize ?? [],
    auto: rawAuto,
  }, stableGeometrySemanticsContext(ctx))
  if (!resolved.valid || !resolved.applied) return shapes

  return shapes.map(shape => ({
    ...shape,
    geometry: ctx.kernel.scale(
      shape.geometry,
      shape.dimension === 3
        ? [resolved.scales[0], resolved.scales[1], resolved.scales[2]]
        : [resolved.scales[0], resolved.scales[1]],
    ),
  }))
}


export function minkowskiShapes(shapes: Shape[], node: CallNode, ctx: EvalContext): Shape[] {
  if (shapes.length === 0) return []
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) {
    evaluationError(ctx, node.p, 'minkowski() cannot mix 2D and 3D children')
  }
  if (shapes.length === 1) return shapes

  try {
    if (dimension === 2) {
      const geometry = ctx.kernel.minkowskiSum2(shapes.map(shape => (shape as Shape2D).geometry))
      return [{
        dimension: 2,
        geometry,
        color: shapes[0].color,
        entityId: currentEntityId(ctx),
      }]
    }
    let geometry = (shapes[0] as Shape3D).geometry
    if (ctx.kernel.isEmpty(geometry)) return []
    for (let index = 1; index < shapes.length; index++) {
      const right = (shapes[index] as Shape3D).geometry
      if (ctx.kernel.isEmpty(right)) return []
      // The kernel exposes the right operand as an anchored dilation. Normalize
      // it around a point that is genuinely in the solid (a real mesh vertex),
      // then restore that translation after the ordered sum. Bounding-box
      // corners are insufficient because they need not belong to curved solids.
      const vertex = ctx.kernel.firstVertex3(right)
      if (vertex === null) return []
      const anchor: Vec3 = [vertex[0], vertex[1], vertex[2]]
      geometry = ctx.kernel.translate(
        ctx.kernel.minkowskiSum3(
          geometry,
          ctx.kernel.translate(right, [-anchor[0], -anchor[1], -anchor[2]]),
        ),
        anchor,
      )
    }
    return [trackedSolid(geometry, shapes[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `minkowski() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}


export function stableLinearExtrudeSections(
  node: CallNode,
  ctx: EvalContext,
  evaluated: ReadonlyMap<string, Value>,
  sections: Shape[],
): Shape[] {
  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, `${node.name}() requires 2D children`)
  const profilePoints = ctx.kernel.polygons(sections[0].geometry).flatMap(polygon => (
    polygon.map(point => [point[0], point[1]] as const)
  ))
  const fragmentInput = stableFragmentInputFromEvaluated(evaluated, ctx, 0)
  const resolved = resolveOpenScadLinearExtrude({
    height: evaluated.get('height'),
    scale: evaluated.get('scale'),
    center: evaluated.get('center'),
    twist: evaluated.get('twist'),
    slices: evaluated.get('slices'),
    fn: fragmentInput.fn,
    fa: fragmentInput.fa,
    fs: fragmentInput.fs,
    profilePoints,
    maximumSlices: MAX_EXTRUDE_SLICES,
  }, stableGeometrySemanticsContext(ctx))
  if (resolved.reduced) ctx.reduced.value = true
  if (resolved.empty) return []
  try {
    const geometry = ctx.kernel.linearExtrude(
      sections[0].geometry,
      resolved.height,
      resolved.manifoldNDivisions,
      resolved.twist,
      [...resolved.scale] as Vec2,
      resolved.center,
    )
    return [trackedSolid(geometry, sections[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `${node.name}() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}


export async function linearExtrude(
  node: CallNode,
  ctx: EvalContext,
  evalNodes: EvalNodesFn,
): Promise<Shape[]> {
  const evaluated = isStableProfile(ctx)
    ? evaluateStableBuiltinModuleArguments(
        node,
        ['height', 'center', 'convexity', 'twist', 'slices', 'scale'],
        ['$fn', '$fa', '$fs'],
        ctx,
      )
    : null
  const sections = booleanShapes(await evalNodes(node.children, ctx), 'union', ctx, node.p)

  if (evaluated !== null) {
    return stableLinearExtrudeSections(node, ctx, evaluated, sections)
  }

  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, 'linear_extrude() requires 2D children')

  const height = finiteNumber(arg(node, 'height', 0, 1, ctx), ctx, node.p, 'extrusion height')
  if (height <= 0) evaluationError(ctx, node.p, 'linear_extrude() height must be positive')
  const twist = finiteNumber(arg(node, 'twist', -1, 0, ctx), ctx, node.p, 'extrusion twist')
  // Cap slices: the value goes straight into the geometry kernel, which
  // allocates per-slice cross-sections long before MAX_TRIANGLES can fire.
  const slices = Math.min(MAX_EXTRUDE_SLICES, Math.max(0, Math.trunc(finiteNumber(arg(node, 'slices', -1, 0, ctx), ctx, node.p, 'extrusion slices'))))
  const rawScale = arg(node, 'scale', -1, [1, 1], ctx)
  const scaleValues = Array.isArray(rawScale) ? vectorValue(rawScale, ctx, node.p, 'extrusion scale') : [finiteNumber(rawScale, ctx, node.p, 'extrusion scale')]
  const scale: Vec2 = [scaleValues[0] ?? 1, scaleValues[1] ?? scaleValues[0] ?? 1]
  try {
    const geometry = ctx.kernel.linearExtrude(sections[0].geometry, height, slices, twist, scale, arg(node, 'center', -1, false, ctx) === true)
    return [trackedSolid(geometry, sections[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `linear_extrude() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}


export async function dxfLinearExtrude(
  node: CallNode,
  ctx: EvalContext,
  evalNodes: EvalNodesFn,
): Promise<Shape[]> {
  const evaluated = evaluateStableCompatibilityArguments(
    node,
    ['file', 'layer', 'height', 'origin', 'scale', 'center', 'twist', 'slices'],
    ['convexity', '$fn', '$fa', '$fs'],
    ctx,
  )
  const values = new Map(evaluated.values)
  if (values.get('height') === undefined) {
    const first = evaluated.authored[0]
    const firstValue = first?.name === undefined ? evaluated.evaluated.get(first) : undefined
    if (typeof firstValue === 'number' && Number.isFinite(firstValue)) values.set('height', firstValue)
  }

  const rawScale = values.get('scale')
  let importScale = 1
  if (typeof rawScale === 'number' && Number.isFinite(rawScale)) {
    importScale = Math.max(0, rawScale)
  } else if (Array.isArray(rawScale) && rawScale.length === 2) {
    const scaleX = rawScale[0]
    const scaleY = rawScale[1]
    if (typeof scaleX === 'number' && Number.isFinite(scaleX)
      && typeof scaleY === 'number' && Number.isFinite(scaleY)) {
      importScale = Math.max(0, scaleX)
    }
  }
  const file = values.get('file')
  const sourceSections = typeof file === 'string' && file.length > 0
    ? compatibilityDxfProfile(node, ctx, values, file, importScale)
    : await evalNodes(node.children, ctx)
  const sections = booleanShapes(sourceSections, 'union', ctx, node.p, node.name)
  return stableLinearExtrudeSections(node, ctx, values, sections)
}
