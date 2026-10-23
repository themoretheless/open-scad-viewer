import type { CadKernelHandle } from '../services/cadKernelOps'
import { clamp01 } from '../services/math3d'
import { resolveLegacyOpenScadSegments } from '../services/openScadLegacySegments'
import type { CallNode, ExpressionArgument, Statement, Value } from '../services/openscadCompiler'
import { formatOpenScadValue } from '../services/openScadValueSemantics'
import {
  resolveOpenScadFragments,
  type OpenScadFragmentResolutionInput,
  type OpenScadStableModuleSemanticsContext,
} from '../services/openScadStableModuleSemantics'
import {
  resolveOpenScadCircle,
  resolveOpenScadCube,
  resolveOpenScadCylinder,
  resolveOpenScadPolygon,
  resolveOpenScadPolyhedron,
  resolveOpenScadSphere,
  resolveOpenScadSquare,
  type OpenScadStablePrimitiveContext,
} from '../services/openScadStablePrimitiveSemantics'
import {
  resolveOpenScadMirror,
  resolveOpenScadMultmatrix,
  resolveOpenScadRotate,
  resolveOpenScadScale,
  resolveOpenScadTranslate,
  resolveOpenScadViewerAxisAngle,
  type OpenScadStableTransformContext,
  type OpenScadStableTransformPlan,
} from '../services/openScadStableTransformSemantics'
import {
  resolveOpenScad2dMultmatrix,
  resolveOpenScadLinearExtrude,
  resolveOpenScadResize,
  resolveOpenScadRotateExtrude,
  type OpenScadStableGeometryContext,
} from '../services/openScadStableGeometrySemantics'
import {
  evaluationError,
  isStableProfile,
} from '../services/openScadStableExpressionEval'
import { CSS_COLORS, nextColor, type RGBA } from '../services/openscadColors'
import {
  compatibilityDxfProfile,
  makeImport,
  makeSurface,
  makeText,
} from './assetEvaluator'
import {
  MAX_EXTRUDE_SLICES,
  type EvalContext,
  type Shape,
  type Shape2D,
  type Shape3D,
  type Vec2,
  type Vec3,
} from './types'
import {
  callExpressionArguments,
  finiteNumber,
  vectorValue,
} from './astDiscovery'
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
} from './expressionEvaluator'

export {
  compatibilityDxfProfile,
  makeImport,
  makeSurface,
  makeText,
}

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
  const requested = raw === undefined || raw === 0 ? null : finiteNumber(raw, ctx, node.p, '$fn')
  const selection = resolveLegacyOpenScadSegments(requested, fallback, minimum, ctx.quality)
  if (selection.clamped) warn(ctx, `$fn=${selection.beforeCap} was clamped to ${selection.maximum} for ${ctx.quality} rendering`)
  if (selection.reduced) ctx.reduced.value = true
  return selection.segments
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

export function makePolygon(node: CallNode, ctx: EvalContext): Shape[] {
  const pointsValue = arg(node, 'points', 0, [], ctx)
  if (!Array.isArray(pointsValue)) evaluationError(ctx, node.p, 'polygon points must be a vector')
  const points = pointsValue.map(point => {
    const vector = vectorValue(point, ctx, node.p, 'polygon point')
    if (vector.length < 2) evaluationError(ctx, node.p, 'Each polygon point needs two coordinates')
    return [vector[0], vector[1]] as Vec2
  })
  const pathsValue = arg(node, 'paths', 1, undefined, ctx)
  let polygons: Vec2[][] = [points]
  if (pathsValue !== undefined) {
    if (!Array.isArray(pathsValue)) evaluationError(ctx, node.p, 'polygon paths must be a vector')
    polygons = pathsValue.map(path => vectorValue(path, ctx, node.p, 'polygon path').map(index => {
      const point = points[Math.trunc(index)]
      if (!point) evaluationError(ctx, node.p, 'Polygon path index is out of bounds')
      return point
    }))
  }
  // EvenOdd: the default Positive fill rule silently yields an EMPTY shape
  // for clockwise-wound point lists, which are perfectly valid in OpenSCAD.
  return [{
    dimension: 2,
    geometry: ctx.kernel.polygon(polygons, 'EvenOdd'),
    color: nextColor(),
    entityId: currentEntityId(ctx),
  }]
}

function stableTransformPlan(node: CallNode, ctx: EvalContext): OpenScadStableTransformPlan {
  const context = stableTransformSemanticsContext(ctx)
  switch (node.name) {
    case 'translate': {
      const values = evaluateStableBuiltinModuleArguments(node, ['v'], [], ctx)
      return resolveOpenScadTranslate(values.get('v'), context)
    }
    case 'scale': {
      const values = evaluateStableBuiltinModuleArguments(node, ['v'], [], ctx)
      return resolveOpenScadScale(values.get('v'), context)
    }
    case 'mirror': {
      const values = evaluateStableBuiltinModuleArguments(node, ['v'], [], ctx)
      return resolveOpenScadMirror(values.get('v'), context)
    }
    case 'rotate': {
      const values = evaluateStableBuiltinModuleArguments(node, ['a', 'v'], [], ctx)
      return resolveOpenScadRotate({ a: values.get('a'), v: values.get('v') }, context)
    }
    case 'multmatrix': {
      const values = evaluateStableBuiltinModuleArguments(node, ['m'], [], ctx)
      return resolveOpenScadMultmatrix(values.get('m'), context)
    }
    default:
      throw new Error(`Internal stable transform dispatch failed for ${node.name}()`)
  }
}

export async function transformStableChildren(
  node: CallNode,
  ctx: EvalContext,
  evalNodes: EvalNodesFn,
): Promise<Shape[]> {
  // Argument binding and expression effects happen before child instantiation.
  const plan = stableTransformPlan(node, ctx)
  const shapes = booleanShapes(
    await evalNodes(node.children, ctx),
    'union',
    ctx,
    node.p,
    node.name,
  )
  if (shapes.length === 0) return []

  const shape = shapes[0]
  const singular = shape.dimension === 3 ? plan.matrix3dSingular : plan.matrix2dSingular
  if (plan.dropsChildren || singular) {
    if (singular) warn(ctx, `${node.name}() produced empty geometry from a singular transform`)
    return [stableEmptyShape(shape.dimension, shape.color, ctx)]
  }

  if (shape.dimension === 2 && !plan.affine2d) {
    warn(ctx, `${node.name}() transform could not be represented by the geometry kernel`)
    return [stableEmptyShape(shape.dimension, shape.color, ctx)]
  }

  try {
    if (shape.dimension === 3) {
      const matrix = [...plan.matrix]
      // Stable multmatrix honors an authored projective fourth row through
      // the dedicated kernel operation; every other transform is affine.
      const projective = matrix[3] !== 0 || matrix[7] !== 0 || matrix[11] !== 0 || matrix[15] !== 1
      return [{
        ...shape,
        geometry: projective
          ? ctx.kernel.transform3Projective(shape.geometry, matrix)
          : ctx.kernel.transform3(shape.geometry, matrix),
        entityId: currentEntityId(ctx),
      }]
    }
    return [{
      ...shape,
      geometry: ctx.kernel.transform2(shape.geometry, [...plan.matrix2d]),
      entityId: currentEntityId(ctx),
    }]
  } catch {
    warn(ctx, `${node.name}() transform could not be represented by the geometry kernel`)
    return [stableEmptyShape(shape.dimension, shape.color, ctx)]
  }
}

export function rotateShapes(shapes: Shape[], node: CallNode, ctx: EvalContext): Shape[] {
  const angle = arg(node, 'a', 0, 0, ctx)
  const axis = arg(node, 'v', 1, undefined, ctx)
  return shapes.map(shape => {
    if (shape.dimension === 2) {
      const degrees = Array.isArray(angle) ? vectorValue(angle, ctx, node.p, 'rotation')[2] ?? 0 : finiteNumber(angle, ctx, node.p, 'rotation')
      return { ...shape, geometry: ctx.kernel.rotate(shape.geometry, degrees) }
    }
    if (Array.isArray(angle)) {
      const vector = vectorValue(angle, ctx, node.p, 'rotation')
      return { ...shape, geometry: ctx.kernel.rotate(shape.geometry, [vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0]) }
    }
    const degrees = finiteNumber(angle, ctx, node.p, 'rotation')
    if (axis === undefined) return { ...shape, geometry: ctx.kernel.rotate(shape.geometry, [0, 0, degrees]) }
    const vector = vectorValue(axis, ctx, node.p, 'rotation axis')
    return { ...shape, geometry: ctx.kernel.transform3(shape.geometry, axisAngleMatrix(vector, degrees, ctx, node.p)) }
  })
}

function axisAngleMatrix(axis: number[], degrees: number, ctx: EvalContext, p: number): number[] {
  const matrix = resolveOpenScadViewerAxisAngle(axis, degrees)
  if (matrix === null) evaluationError(ctx, p, 'Rotation axis cannot be zero')
  return [...matrix]
}

export function transformByMatrix(shapes: Shape[], value: Value, node: CallNode, ctx: EvalContext): Shape[] {
  const has2d = shapes.some(shape => shape.dimension === 2)
  const has3d = shapes.some(shape => shape.dimension === 3)
  let matrix2d: number[] | null = null
  if (has2d && isStableProfile(ctx)) {
    const resolved = resolveOpenScad2dMultmatrix(value)
    if (resolved === null) {
      warn(ctx, 'Invalid multmatrix() value leaves 2D children unchanged')
    } else {
      matrix2d = [...resolved]
    }
  }

  let matrix3d: number[] | null = null
  if (has3d || (has2d && !isStableProfile(ctx))) {
    if (!Array.isArray(value) || value.length < 3) evaluationError(ctx, node.p, 'multmatrix requires a 4x4 matrix')
    const rows = value.map(row => vectorValue(row, ctx, node.p, 'matrix row'))
    if (rows.some(row => row.length < 4)) evaluationError(ctx, node.p, 'multmatrix requires a 4x4 matrix')
    const matrix: number[] = []
    for (let column = 0; column < 4; column++) for (let row = 0; row < 4; row++) matrix.push(rows[row]?.[column] ?? (row === column ? 1 : 0))
    matrix3d = matrix
  }
  return shapes.map(shape => {
    if (shape.dimension === 2) {
      if (matrix2d === null) return shape
      return { ...shape, geometry: ctx.kernel.transform2(shape.geometry, matrix2d) }
    }
    return { ...shape, geometry: ctx.kernel.transform3(shape.geometry, matrix3d!) }
  })
}

/**
 * Mixed-dimension policy shared by every Boolean module: the legacy profile
 * rejects mixed children, the stable profile warns and keeps the first child's
 * dimension (an intersection with a foreign dimension is empty).
 */
export function homogeneousShapes(
  shapes: Shape[],
  operation: 'union' | 'intersection' | 'difference',
  ctx: EvalContext,
  p: number,
  diagnosticName: string = operation,
): Shape[] {
  if (shapes.length === 0) return []
  const dimension = shapes[0].dimension
  if (!shapes.some(shape => shape.dimension !== dimension)) return shapes
  if (!isStableProfile(ctx)) {
    evaluationError(ctx, p, `${diagnosticName}() cannot mix 2D and 3D children`)
  }
  warn(ctx, `${diagnosticName}() ignored child geometry with a different dimension`)
  if (operation === 'intersection') return []
  return shapes.filter(shape => shape.dimension === dimension)
}

export function booleanShapes(
  sourceShapes: Shape[],
  operation: 'union' | 'intersection',
  ctx: EvalContext,
  p: number,
  diagnosticName: string = operation,
): Shape[] {
  const shapes = homogeneousShapes(sourceShapes, operation, ctx, p, diagnosticName)
  if (shapes.length <= 1) return shapes
  ctx.control?.poll()
  if (shapes[0].dimension === 3) {
    const solids = shapes.map(shape => (shape as Shape3D).geometry)
    const geometry = kernelCall(ctx, p, () => ctx.kernel.boolean3(operation, solids))
    return [{ dimension: 3, geometry, color: shapes[0].color, entityId: currentEntityId(ctx) }]
  }
  const sections = shapes.map(shape => (shape as Shape2D).geometry)
  const geometry = kernelCall(ctx, p, () => ctx.kernel.boolean2(operation, sections))
  return [{ dimension: 2, geometry, color: shapes[0].color, entityId: currentEntityId(ctx) }]
}

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

function stableLinearExtrudeSections(
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

function stableRotateExtrudeSections(
  node: CallNode,
  ctx: EvalContext,
  evaluated: ReadonlyMap<string, Value>,
  sections: Shape[],
): Shape[] {
  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, `${node.name}() requires 2D children`)
  const bounds = ctx.kernel.isEmpty(sections[0].geometry)
    ? { min: [0, 0], max: [0, 0] }
    : ctx.kernel.bounds(sections[0].geometry)
  const radius = Math.max(Math.abs(bounds.min[0] ?? 0), Math.abs(bounds.max[0] ?? 0))
  const fragmentInput = stableFragmentInputFromEvaluated(evaluated, ctx, radius)
  const resolved = resolveOpenScadRotateExtrude({
    angle: evaluated.get('angle'),
    profileXMin: bounds.min[0],
    profileXMax: bounds.max[0],
    fn: fragmentInput.fn,
    fa: fragmentInput.fa,
    fs: fragmentInput.fs,
    quality: ctx.quality,
  }, stableGeometrySemanticsContext(ctx))
  if (resolved.crossesAxis) {
    evaluationError(ctx, node.p, `${node.name}() profile may not cross the Y axis`)
  }
  if (resolved.reduced) ctx.reduced.value = true
  if (resolved.empty) return []
  const profile = resolved.reflectProfileX
    ? ctx.kernel.transform2(sections[0].geometry, [
        -1, 0, 0,
        0, 1, 0,
        0, 0, 1,
      ])
    : sections[0].geometry
  try {
    let geometry = ctx.kernel.rotateExtrude(
      profile,
      resolved.circularSegments,
      resolved.angle,
    )
    if (resolved.postRotateDegrees !== 0) {
      geometry = ctx.kernel.rotate(geometry, [0, 0, resolved.postRotateDegrees])
    }
    return [trackedSolid(geometry, sections[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `${node.name}() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}

export async function dxfRotateExtrude(
  node: CallNode,
  ctx: EvalContext,
  evalNodes: EvalNodesFn,
): Promise<Shape[]> {
  const evaluated = evaluateStableCompatibilityArguments(
    node,
    ['file', 'layer', 'origin', 'scale'],
    ['convexity', 'angle', '$fn', '$fa', '$fs'],
    ctx,
  )
  const values = new Map(evaluated.values)
  const rawScale = values.get('scale')
  const importScale = typeof rawScale === 'number' && Number.isFinite(rawScale) && rawScale > 0
    ? rawScale
    : 1
  const rawFile = values.get('file')
  const file = rawFile === undefined ? '' : compatibilityString(rawFile)
  const sourceSections = file.length > 0
    ? compatibilityDxfProfile(node, ctx, values, file, importScale)
    : await evalNodes(node.children, ctx)
  const sections = booleanShapes(sourceSections, 'union', ctx, node.p, node.name)
  return stableRotateExtrudeSections(node, ctx, values, sections)
}

export async function rotateExtrude(
  node: CallNode,
  ctx: EvalContext,
  evalNodes: EvalNodesFn,
): Promise<Shape[]> {
  const evaluated = isStableProfile(ctx)
    ? evaluateStableBuiltinModuleArguments(
        node,
        ['angle', 'convexity'],
        ['$fn', '$fa', '$fs'],
        ctx,
      )
    : null
  const sections = booleanShapes(await evalNodes(node.children, ctx), 'union', ctx, node.p)
  if (evaluated !== null) {
    return stableRotateExtrudeSections(node, ctx, evaluated, sections)
  }

  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, 'rotate_extrude() requires 2D children')
  const bounds = ctx.kernel.bounds(sections[0].geometry)
  const radius = Math.max(Math.abs(bounds.min[0] ?? 0), Math.abs(bounds.max[0] ?? 0))
  const angle = finiteNumber(arg(node, 'angle', -1, 360, ctx), ctx, node.p, 'revolve angle')
  const circularSegments = segments(node, ctx, 48, 3, radius)
  try {
    const geometry = ctx.kernel.rotateExtrude(sections[0].geometry, circularSegments, angle)
    return [trackedSolid(geometry, sections[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `rotate_extrude() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}

export function parseLegacyColor(value: Value, ctx: EvalContext, p: number): RGBA {
  if (Array.isArray(value)) {
    const channels = vectorValue(value, ctx, p, 'color')
    return [clamp01(channels[0] ?? 0.5), clamp01(channels[1] ?? 0.5), clamp01(channels[2] ?? 0.5), clamp01(channels[3] ?? 1)]
  }
  if (typeof value !== 'string') evaluationError(ctx, p, 'color() expects a name or RGB(A) vector')
  const named = CSS_COLORS[value.toLowerCase()]
  if (named) return [...named]
  const match = value.match(/^#([0-9a-f]{6}|[0-9a-f]{8})$/i)
  if (match) {
    const hex = match[1]
    return [parseInt(hex.slice(0, 2), 16) / 255, parseInt(hex.slice(2, 4), 16) / 255, parseInt(hex.slice(4, 6), 16) / 255, hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1]
  }
  evaluationError(ctx, p, `Unknown color ${value}`)
}
