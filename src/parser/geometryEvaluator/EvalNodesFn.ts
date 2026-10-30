// geometryEvaluator: EvalNodesFn.
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



export type EvalNodesFn = (nodes: readonly Statement[], parent: EvalContext, scoped?: boolean) => Promise<Shape[]>

export type EvalPreparedNodesFn = (nodes: readonly Statement[], ctx: EvalContext) => Promise<Shape[]>


export function requireStableProfile(ctx: EvalContext, node: CallNode): void {
  if (ctx.languageProfile !== 'openscad/stable-2021.01') {
    evaluationError(ctx, node.p, `Unsupported geometry operation ${node.name}()`)
  }
}


export function stableModuleSemanticsContext(ctx: EvalContext): OpenScadStableModuleSemanticsContext {
  return { warn: warning => warn(ctx, warning.message) }
}


export function stableGeometrySemanticsContext(ctx: EvalContext): OpenScadStableGeometryContext {
  return { warn: warning => warn(ctx, warning.message) }
}


export function stablePrimitiveSemanticsContext(ctx: EvalContext): OpenScadStablePrimitiveContext {
  return {
    checkParameterRanges: true,
    warn: warning => warn(ctx, warning.message),
  }
}


export function stableTransformSemanticsContext(ctx: EvalContext): OpenScadStableTransformContext {
  return {
    checkParameterRanges: true,
    warn: warning => warn(ctx, warning.message),
  }
}


export function stableFragmentInput(
  node: CallNode,
  ctx: EvalContext,
  radius: number,
): OpenScadFragmentResolutionInput {
  const special = (name: '$fn' | '$fa' | '$fs'): { present: boolean; value: Value } => {
    const local = node.args[name]
    if (local !== undefined) return { present: true, value: evalExpression(local, ctx) }
    const resolved = resolveStableVariable(name, ctx)
    return { present: resolved.found, value: resolved.value }
  }
  const fn = special('$fn'), fa = special('$fa'), fs = special('$fs')
  return {
    radius,
    quality: ctx.quality,
    ...(fn.present ? { fn: fn.value } : {}),
    ...(fa.present ? { fa: fa.value } : {}),
    ...(fs.present ? { fs: fs.value } : {}),
  }
}


export function stableFragmentInputFromEvaluated(
  evaluated: ReadonlyMap<string, Value>,
  ctx: EvalContext,
  radius: number,
): OpenScadFragmentResolutionInput {
  const special = (name: '$fn' | '$fa' | '$fs'): { present: boolean; value: Value } => {
    if (evaluated.has(name)) return { present: true, value: evaluated.get(name) }
    const resolved = resolveStableVariable(name, ctx)
    return { present: resolved.found, value: resolved.value }
  }
  const fn = special('$fn'), fa = special('$fa'), fs = special('$fs')
  return {
    radius,
    quality: ctx.quality,
    ...(fn.present ? { fn: fn.value } : {}),
    ...(fa.present ? { fa: fa.value } : {}),
    ...(fs.present ? { fs: fs.value } : {}),
  }
}


export function stableSegmentsFromEvaluated(
  evaluated: ReadonlyMap<string, Value>,
  ctx: EvalContext,
  radius: number,
): number {
  const resolution = resolveOpenScadFragments(
    stableFragmentInputFromEvaluated(evaluated, ctx, radius),
    stableModuleSemanticsContext(ctx),
  )
  if (resolution.reduced) ctx.reduced.value = true
  return resolution.fragments
}


export function stableEmptyShape(dimension: 2 | 3, color: RGBA, ctx: EvalContext): Shape {
  if (dimension === 3) {
    return {
      dimension: 3,
      geometry: ctx.kernel.empty3(),
      color,
      entityId: currentEntityId(ctx),
    }
  }
  return {
    dimension: 2,
    geometry: ctx.kernel.empty2(),
    color,
    entityId: currentEntityId(ctx),
  }
}


export function warnIgnoredPrimitiveChildren(node: CallNode, ctx: EvalContext): void {
  if (node.children.length > 0) warn(ctx, `${node.name}() ignores child geometry`)
}


export function makeStableCube(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(node, ['size', 'center'], [], ctx)
  const plan = resolveOpenScadCube({
    size: values.get('size'),
    center: values.get('center'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(3, color, ctx)]
  return [trackedSolid(
    ctx.kernel.box([...plan.dimensions] as Vec3, plan.center),
    color,
    node,
    ctx,
  )]
}


export function makeStableSquare(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(node, ['size', 'center'], [], ctx)
  const plan = resolveOpenScadSquare({
    size: values.get('size'),
    center: values.get('center'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(2, color, ctx)]
  return [{
    dimension: 2,
    geometry: ctx.kernel.rectangle([...plan.dimensions] as Vec2, plan.center),
    color,
    entityId: currentEntityId(ctx),
  }]
}


export function makeStableSphere(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(
    node,
    ['r'],
    ['d', '$fn', '$fa', '$fs'],
    ctx,
  )
  const plan = resolveOpenScadSphere({
    r: values.get('r'),
    d: values.get('d'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(3, color, ctx)]
  const fragments = stableSegmentsFromEvaluated(values, ctx, plan.fragmentRadius)
  return [trackedSolid(ctx.kernel.sphere(plan.radius, fragments), color, node, ctx)]
}


export function makeStableCircle(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(
    node,
    ['r'],
    ['d', '$fn', '$fa', '$fs'],
    ctx,
  )
  const plan = resolveOpenScadCircle({
    r: values.get('r'),
    d: values.get('d'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(2, color, ctx)]
  const fragments = stableSegmentsFromEvaluated(values, ctx, plan.fragmentRadius)
  return [{
    dimension: 2,
    geometry: ctx.kernel.circle(plan.radius, fragments),
    color,
    entityId: currentEntityId(ctx),
  }]
}


export function makeStableCylinder(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(
    node,
    ['h', 'r1', 'r2', 'center'],
    ['r', 'd', 'd1', 'd2', '$fn', '$fa', '$fs'],
    ctx,
  )
  const plan = resolveOpenScadCylinder({
    h: values.get('h'),
    r: values.get('r'),
    d: values.get('d'),
    r1: values.get('r1'),
    r2: values.get('r2'),
    d1: values.get('d1'),
    d2: values.get('d2'),
    center: values.get('center'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(3, color, ctx)]
  const fragments = stableSegmentsFromEvaluated(values, ctx, plan.fragmentRadius)
  let geometry: CadKernelHandle
  if (plan.radius1 > 0) {
    geometry = ctx.kernel.cylinder(
      plan.height,
      plan.radius1,
      plan.radius2,
      fragments,
      plan.center,
    )
  } else {
    geometry = ctx.kernel.mirrorZ(
      ctx.kernel.cylinder(plan.height, plan.radius2, 0, fragments, plan.center),
    )
    if (!plan.center) geometry = ctx.kernel.translateZ(geometry, plan.height)
  }
  return [trackedSolid(geometry, color, node, ctx)]
}


export function makeStablePolyhedron(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(
    node,
    ['points', 'faces', 'convexity'],
    ['triangles'],
    ctx,
  )
  const plan = resolveOpenScadPolyhedron({
    points: values.get('points'),
    faces: values.get('faces'),
    triangles: values.get('triangles'),
    convexity: values.get('convexity'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty) return [stableEmptyShape(3, color, ctx)]

  if (!plan.usable) {
    warn(ctx, 'polyhedron() produced no usable finite faces')
    return [stableEmptyShape(3, color, ctx)]
  }

  try {
    const geometry = ctx.kernel.ofMesh(new Float64Array(plan.vertices), new Uint32Array(plan.indices))
    if (ctx.kernel.isEmpty(geometry)) {
      warn(ctx, 'polyhedron() topology did not produce a manifold solid')
      return [stableEmptyShape(3, color, ctx)]
    }
    return [trackedSolid(geometry, color, node, ctx)]
  } catch {
    warn(ctx, 'polyhedron() topology did not produce a manifold solid')
    return [stableEmptyShape(3, color, ctx)]
  }
}


export function makeStablePolygon(node: CallNode, ctx: EvalContext): Shape[] {
  const values = evaluateStableBuiltinModuleArguments(
    node,
    ['points', 'paths', 'convexity'],
    [],
    ctx,
  )
  const plan = resolveOpenScadPolygon({
    points: values.get('points'),
    paths: values.get('paths'),
    convexity: values.get('convexity'),
  }, stablePrimitiveSemanticsContext(ctx))
  warnIgnoredPrimitiveChildren(node, ctx)
  if (plan.reduced) ctx.reduced.value = true
  const color = nextColor()
  if (plan.empty || plan.outlines.some(outline =>
    outline.some(point => !Number.isFinite(point[0]) || !Number.isFinite(point[1])))) {
    if (!plan.empty) warn(ctx, 'polygon() produced no usable finite outlines')
    return [stableEmptyShape(2, color, ctx)]
  }

  try {
    const polygons = plan.outlines.map(outline =>
      outline.map(point => [point[0], point[1]] as Vec2))
    const geometry = ctx.kernel.polygon(polygons, 'EvenOdd')
    if (ctx.kernel.isEmpty(geometry)) warn(ctx, 'polygon() outlines produced an empty cross-section')
    return [{
      dimension: 2,
      geometry,
      color,
      entityId: currentEntityId(ctx),
    }]
  } catch {
    warn(ctx, 'polygon() outlines did not produce a valid cross-section')
    return [stableEmptyShape(2, color, ctx)]
  }
}
