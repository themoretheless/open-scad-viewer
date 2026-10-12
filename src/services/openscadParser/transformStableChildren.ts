import type { CallNode, Value } from '../openscadCompiler'
import { enterStableStatementScope } from '../openscadParser'
import type { EvalContext, Shape, Shape2D, Shape3D, Vec3 } from '../openscadParser'
import { arg, evalNodes, evalPreparedNodes, finiteNumber, vectorValue } from './evalBuiltin'
import { currentEntityId, evaluateStableBuiltinModuleArguments, kernelCall, trackedSolid, warn } from './isExclusiveView'
import { evaluationError, isStableProfile } from '../openScadStableExpressionEval'
import { openScadMinkowski2dViaProduct, resolveOpenScad2dMultmatrix, resolveOpenScadResize } from '../openScadStableGeometrySemantics'
import { stableEmptyShape, stableGeometrySemanticsContext } from './requireStableProfile'
import { stableTransformPlan } from './makeImport'

export async function transformStableChildren(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
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

  try {
    if (shape.dimension === 3) {
      return [{
        ...shape,
        geometry: ctx.kernel.transform3(shape.geometry, [...plan.matrix]),
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

export function axisAngleMatrix(axis: number[], degrees: number, ctx: EvalContext, p: number): number[] {
  const length = Math.hypot(axis[0] ?? 0, axis[1] ?? 0, axis[2] ?? 0)
  if (length === 0) evaluationError(ctx, p, 'Rotation axis cannot be zero')
  const x = (axis[0] ?? 0) / length, y = (axis[1] ?? 0) / length, z = (axis[2] ?? 0) / length
  const angle = degrees * Math.PI / 180, c = Math.cos(angle), s = Math.sin(angle), t = 1 - c
  // Kernel matrices are column-major.
  return [
    t*x*x+c, t*x*y+s*z, t*x*z-s*y, 0,
    t*x*y-s*z, t*y*y+c, t*y*z+s*x, 0,
    t*x*z+s*y, t*y*z-s*x, t*z*z+c, 0,
    0, 0, 0, 1,
  ]
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

export async function differenceChildren(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
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

export async function resizeChildren(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
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

  const axes = dimension === 3 ? 3 : 2
  const min = Array.from({ length: axes }, () => Infinity)
  const max = Array.from({ length: axes }, () => -Infinity)
  let hasGeometry = false
  for (const shape of shapes) {
    if (ctx.kernel.isEmpty(shape.geometry)) continue
    hasGeometry = true
    const bounds = ctx.kernel.bounds(shape.geometry)
    for (let axis = 0; axis < axes; axis++) {
      min[axis] = Math.min(min[axis], bounds.min[axis] ?? Infinity)
      max[axis] = Math.max(max[axis], bounds.max[axis] ?? -Infinity)
    }
  }
  if (!hasGeometry) return shapes

  const resolved = resolveOpenScadResize({
    dimension,
    extents: max.map((value, axis) => value - min[axis]),
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
      const geometry = openScadMinkowski2dViaProduct(
        shapes.map(shape => (shape as Shape2D).geometry),
        {
          anchor: section => {
            // The kernel exposes Minkowski as an anchored dilation. Anchor at an
            // actual contour vertex (not a bounding-box corner, which may lie
            // outside a circle) so the normalized structuring set contains 0.
            const point = ctx.kernel.polygons(section)[0]?.[0]
            return point === undefined ? [0, 0] : [point[0], point[1]]
          },
          translate: (section, offset) => ctx.kernel.translate(section, offset),
          extrudeUnitPrism: section => ctx.kernel.linearExtrude(
            section, 1, 0, 0, [1, 1], false,
          ),
          minkowskiSum: (left, right) => ctx.kernel.minkowskiSum3(left, right),
          projectTo2d: solid => ctx.kernel.projection(solid, false),
        },
      )
      if (geometry === undefined) return []
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
