import type { CadKernelHandle } from '../cadKernelOps'
import type { CallNode, Value } from '../openscadCompiler'
import { MAX_FN } from '../openscadParser'
import type { EvalContext, Shape, Vec2, Vec3 } from '../openscadParser'
import { resolveOpenScadFragments } from '../openScadStableModuleSemantics'
import type { OpenScadFragmentResolutionInput, OpenScadStableModuleSemanticsContext } from '../openScadStableModuleSemantics'
import type { OpenScadStableGeometryContext } from '../openScadStableGeometrySemantics'
import { resolveOpenScadCircle, resolveOpenScadCube, resolveOpenScadCylinder, resolveOpenScadPolygon, resolveOpenScadPolyhedron, resolveOpenScadSphere, resolveOpenScadSquare } from '../openScadStablePrimitiveSemantics'
import type { OpenScadStablePrimitiveContext } from '../openScadStablePrimitiveSemantics'
import type { OpenScadStableTransformContext } from '../openScadStableTransformSemantics'
import { nextColor } from '../openscadColors'
import type { RGBA } from '../openscadColors'
import { arg, finiteNumber, vectorValue } from './evalBuiltin'
import { currentEntityId, evalExpression, evaluateStableBuiltinModuleArguments, resolveStableVariable, trackedSolid, warn } from './isExclusiveView'
import { evaluationError, isStableProfile } from '../openScadStableExpressionEval'

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

  const vertices: number[] = []
  const indices: number[] = []
  for (const polygon of plan.polygons) {
    if (polygon.length < 3) continue
    const base = vertices.length / 3
    for (const point of polygon) vertices.push(point[0], point[1], point[2])
    // OpenSCAD's authored exterior winding is opposite the kernel mesh input.
    for (let index = 1; index < polygon.length - 1; index++) {
      indices.push(base, base + index + 1, base + index)
    }
  }
  if (indices.length === 0 || vertices.some(value => !Number.isFinite(Math.fround(value)))) {
    warn(ctx, 'polyhedron() produced no usable finite faces')
    return [stableEmptyShape(3, color, ctx)]
  }

  try {
    const geometry = ctx.kernel.ofMesh(new Float64Array(vertices), new Uint32Array(indices))
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

export function segments(node: CallNode, ctx: EvalContext, fallback: number, minimum: number, radius: number): number {
  if (isStableProfile(ctx)) {
    const resolution = resolveOpenScadFragments(
      stableFragmentInput(node, ctx, radius),
      stableModuleSemanticsContext(ctx),
    )
    if (resolution.reduced) ctx.reduced.value = true
    return resolution.fragments
  }
  const local = arg(node, '$fn', -1, undefined, ctx)
  const global = ctx.env.get('$fn')
  const raw = local === undefined || local === 0 ? global : local
  const requested = raw === undefined || raw === 0 ? undefined : Math.round(finiteNumber(raw, ctx, node.p, '$fn'))
  const maxSegments = ctx.quality === 'preview' ? 48 : MAX_FN
  const previewFallback = ctx.quality === 'preview' ? Math.min(fallback, 24) : fallback
  let value = requested ?? previewFallback
  if (value > maxSegments) {
    warn(ctx, `$fn=${value} was clamped to ${maxSegments} for ${ctx.quality} rendering`)
    value = maxSegments
  }
  value = Math.max(minimum, value)
  if (ctx.quality === 'preview') {
    // This is the only place quality changes evaluation. Record whether the
    // preview reduction actually altered the segment count a full-quality
    // evaluation of the same call would have used.
    const fullValue = Math.max(minimum, Math.min(requested ?? fallback, MAX_FN))
    if (value !== fullValue) ctx.reduced.value = true
  }
  return value
}

export function makeCylinder(node: CallNode, ctx: EvalContext): Shape[] {
  const height = finiteNumber(arg(node, 'h', 0, 1, ctx), ctx, node.p, 'cylinder height')
  if (height <= 0) evaluationError(ctx, node.p, 'Cylinder height must be positive')
  let low = arg(node, 'r1', 1, undefined, ctx)
  let high = arg(node, 'r2', 2, undefined, ctx)
  const radius = arg(node, 'r', 1, undefined, ctx)
  const diameter = arg(node, 'd', -1, undefined, ctx)
  const d1 = arg(node, 'd1', -1, undefined, ctx)
  const d2 = arg(node, 'd2', -1, undefined, ctx)
  if (d1 !== undefined) low = finiteNumber(d1, ctx, node.p, 'd1') / 2
  if (d2 !== undefined) high = finiteNumber(d2, ctx, node.p, 'd2') / 2
  if (low === undefined && high === undefined) {
    const base = diameter !== undefined ? finiteNumber(diameter, ctx, node.p, 'diameter') / 2
      : radius !== undefined ? finiteNumber(radius, ctx, node.p, 'radius') : 1
    low = high = base
  }
  if (low === undefined) low = high
  if (high === undefined) high = low
  const r1 = finiteNumber(low, ctx, node.p, 'r1')
  const r2 = finiteNumber(high, ctx, node.p, 'r2')
  if (r1 < 0 || r2 < 0 || (r1 === 0 && r2 === 0)) evaluationError(ctx, node.p, 'Cylinder radii must be non-negative and not both zero')
  const center = arg(node, 'center', 3, false, ctx) === true
  const fn = segments(node, ctx, 32, 3, Math.max(r1, r2))
  let geometry: CadKernelHandle
  if (r1 > 0) geometry = ctx.kernel.cylinder(height, r1, r2, fn, center)
  else {
    geometry = ctx.kernel.mirrorZ(ctx.kernel.cylinder(height, r2, 0, fn, center))
    if (!center) geometry = ctx.kernel.translateZ(geometry, height)
  }
  return [trackedSolid(geometry, nextColor(), node, ctx)]
}

export function makePolyhedron(node: CallNode, ctx: EvalContext): Shape[] {
  const points = arg(node, 'points', 0, [], ctx)
  const faces = arg(node, 'faces', 1, arg(node, 'triangles', 1, [], ctx), ctx)
  if (!Array.isArray(points) || !Array.isArray(faces)) evaluationError(ctx, node.p, 'polyhedron points and faces must be vectors')
  const vertices: number[] = []
  for (const point of points) {
    const vector = vectorValue(point, ctx, node.p, 'polyhedron point')
    if (vector.length < 3) evaluationError(ctx, node.p, 'Each polyhedron point needs three coordinates')
    vertices.push(vector[0], vector[1], vector[2])
  }
  const indices: number[] = []
  for (const face of faces) {
    const polygon = vectorValue(face, ctx, node.p, 'polyhedron face').map(Math.trunc)
    if (polygon.length < 3) evaluationError(ctx, node.p, 'Each polyhedron face needs at least three vertices')
    for (const index of polygon) if (index < 0 || index >= points.length) evaluationError(ctx, node.p, 'Polyhedron face index is out of bounds')
    // OpenSCAD faces are wound clockwise viewed from outside; the kernel
    // requires counter-clockwise — reverse the fan so spec-correct
    // polyhedra build outward-facing instead of inside-out.
    for (let i = 1; i < polygon.length - 1; i++) indices.push(polygon[0], polygon[i + 1], polygon[i])
  }
  try {
    // Weld duplicated coordinates first: OpenSCAD accepts point lists with
    // repeated positions, but kernel halfedge pairing rejects them.
    return [trackedSolid(ctx.kernel.ofMesh(new Float64Array(vertices), new Uint32Array(indices)), nextColor(), node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `Invalid manifold polyhedron: ${error instanceof Error ? error.message : String(error)}`)
  }
}
