// geometryEvaluator: segments.
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
import { stableFragmentInput, stableModuleSemanticsContext, stableTransformSemanticsContext, EvalNodesFn, stableEmptyShape } from './EvalNodesFn'


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


export function stableTransformPlan(node: CallNode, ctx: EvalContext): OpenScadStableTransformPlan {
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


export function axisAngleMatrix(axis: number[], degrees: number, ctx: EvalContext, p: number): number[] {
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
