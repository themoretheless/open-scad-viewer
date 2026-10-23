import { isRushFrontend } from './rushFrontendDetect'
import { brepGearFaceCount } from './geometry/brep'
import { evaluateRushGraphGeometry, requireRushGraphChecks } from './rushGraphChecks'
/**
 * Strict, intentionally documented OpenSCAD subset backed by the official geometry kernel.
 *
 * Supported language features: variables, arithmetic/boolean expressions,
 * ranges, for/if/let, statement assertions, user modules and children(). Supported geometry:
 * cube, sphere, cylinder, polyhedron, square, circle, polygon, transforms,
 * union/difference/intersection/hull, linear/rotate extrusion, projection and
 * 2D offset. Unsupported syntax fails loudly instead of rendering a wrong model.
 */
import { geometryAssetId } from '../core/scene'
import type {
  MeshData,
  MeshProvenanceRun,
  MeshSourceReference,
} from '../core/mesh'
import { clamp01, identity } from './math3d'
import { AbortedError, OpenSCADParseError } from './openscadErrors'
import { createBrepRecordingKernelOps } from './solid/brepRecorder'
import { bindOpenScad, prepareOpenScadFrontEnd } from './openscadBinder'
import {
  compileOpenSCAD,
  hasOpenScadViewportModifier,
  type CallNode,
  type ExpressionArgument,
  type ModuleNode,
  type Statement,
  type Value,
} from './openscadCompiler'
import {
  formatOpenScadValue,
  openScadTruthy,
} from './openScadValueSemantics'
import {
  resolveOpenScadColor,
  resolveOpenScadOffset,
} from './openScadStableModuleSemantics'
import {
  resolveOpenScadChildrenSelection,
} from './openScadStableGeometrySemantics'
import { OpenScadStableScope } from './openScadStableScope'
import {
  MAX_EVAL_DEPTH,
  MAX_EVAL_OPS,
  MAX_RANGE_ITEMS,
  compactDiagnosticText,
  evaluationError,
  isStableProfile,
  overlayDynamicVariables,
  stableOverlayContext,
} from './openScadStableExpressionEval'
import {
  type OpenScadProject,
} from './openScadProject'
import { compileOpenScadProject } from './openScadProjectCompiler'
import {
  prepareOpenScadImportAssets,
} from './openScadImport'
import {
  prepareOpenScadSurfaceAssets,
} from './openScadSurface'
import {
  prepareOpenScadTextAssets,
} from './openScadText'
import { defaultGeometryKernel } from './cadGeometryKernel'
import { createOpenScadStableRuntimeVariables } from './openScadStableRuntime'
import { nextColor, resetPalette, type RGBA } from './openscadColors'

export { AbortedError, OpenSCADParseError } from './openscadErrors'
import {
  CooperativeCheckpoint,
  isExclusiveView,
  MAX_SHAPES,
  MAX_SOURCE_LENGTH,
  MAX_TRIANGLES,
  StableViewportRootSelection,
  YIELD_EVERY_STATEMENTS,
  type EvalContext,
  type ParseOpenScadProjectOptions,
  type ParseOptions,
  type ParseResult,
  type Shape,
  type Shape3D,
  type Vec2,
  type Vec3,
} from '../parser/types'
export type {
  ParseOpenScadProjectOptions,
  ParseOptions,
  ParseResult,
} from '../parser/types'
import {
  bindAssertArguments,
  callExpressionArguments,
  finiteNumber,
  identityValue,
  stableCompatibilityForcedAssets,
  truthy,
  valueToString,
  vectorValue,
} from '../parser/astDiscovery'
import {
  arg,
  currentEntityId,
  enterStableStatementScope,
  evalExpression,
  evaluateSequentialBindings,
  evaluateStableBuiltinModuleArguments,
  kernelCall,
  resolveStableExpressionArguments,
  stableIterable,
  staticOperationId,
  trackedSolid,
  warn,
} from '../parser/expressionEvaluator'
import {
  booleanShapes,
  differenceChildren,
  dxfLinearExtrude,
  dxfRotateExtrude,
  hullShapes,
  linearExtrude,
  makeCylinder,
  makeImport,
  makePolygon,
  makePolyhedron,
  makeStableCircle,
  makeStableCube,
  makeStableCylinder,
  makeStablePolygon,
  makeStablePolyhedron,
  makeStableSphere,
  makeStableSquare,
  makeSurface,
  makeText,
  minkowskiShapes,
  parseLegacyColor,
  requireStableProfile,
  resizeChildren,
  rotateExtrude,
  rotateShapes,
  segments,
  stableGeometrySemanticsContext,
  stableModuleSemanticsContext,
  transformByMatrix,
  transformStableChildren,
} from '../parser/geometryEvaluator'

async function evalNodes(nodes: readonly Statement[], parent: EvalContext, scoped = true): Promise<Shape[]> {
  const ctx: EvalContext = isStableProfile(parent)
    ? enterStableStatementScope(nodes, parent)
    : { ...parent, env: scoped ? new Map(parent.env) : parent.env }
  return await evalPreparedNodes(nodes, ctx)
}

function stableViewportDisabled(statement: Statement, ctx: EvalContext): boolean {
  return isStableProfile(ctx)
    && statement.type === 'call'
    && hasOpenScadViewportModifier(statement, 'disable')
}

/**
 * GeometryEvaluationResult has one undifferentiated mesh collection, so it
 * cannot safely expose OpenSCAD's background preview layer as ordinary solid
 * geometry. Evaluate `%` for language effects in both qualities, omit its
 * shapes, and make the preview limitation explicit instead of claiming a
 * background layer that downstream consumers cannot distinguish.
 */
async function evalPreparedCall(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
  const background = isStableProfile(ctx)
    && ctx.viewportRootOwner !== node
    && hasOpenScadViewportModifier(node, 'background')
  const shapes = await evalNode(node, ctx)
  if (!background) return shapes
  if (ctx.quality === 'preview' && shapes.length > 0) {
    warn(ctx, 'Viewport background (%) geometry is omitted in preview because the mesh result contract has no background-layer metadata')
  }
  return []
}

async function evalPreparedNodes(nodes: readonly Statement[], ctx: EvalContext): Promise<Shape[]> {
  const output: Shape[] = []
  let statementsSinceYield = 0
  for (const node of nodes) {
    if (ctx.control && statementsSinceYield >= YIELD_EVERY_STATEMENTS) {
      statementsSinceYield = 0
      await ctx.control.yieldIfDue()
    }
    statementsSinceYield++
    // Unlike every other viewport modifier, `*` suppresses argument, effect,
    // child, and geometry evaluation for the complete call subtree.
    if (stableViewportDisabled(node, ctx)) continue
    if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
    if (node.type === 'assign') {
      if (!isStableProfile(ctx)) ctx.env.set(node.name, evalExpression(node.value, ctx))
      continue
    }
    if (node.type === 'module' || node.type === 'function') continue
    output.push(...await evalPreparedCall(node, ctx))
    if (output.length > MAX_SHAPES) evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
  }
  return output
}

/**
 * Top-level evaluation with cooperative cancellation. Nested `for` / `if` /
 * module bodies share the same checkpoint, so a Worker can receive cancel
 * mid-loop. A single enormous kernel/BVH call still cannot yield mid-WASM;
 * the coordinator grace timer remains that hard boundary.
 */
async function evalTopLevel(nodes: readonly Statement[], ctx: EvalContext, control: CooperativeCheckpoint): Promise<Shape[]> {
  control.poll()
  try {
    return await evalPreparedNodes(nodes, { ...ctx, control })
  } catch (error) {
    if (error instanceof StableViewportRootSelection) return error.shapes
    throw error
  }
}

function viewportRootActivates(node: CallNode, shapes: readonly Shape[]): boolean {
  if (shapes.length > 0) return true
  // OpenSCAD does not create an empty CSG container for these transparent
  // effect/branch calls. Every other evaluated module instantiation can win a
  // root selection even when its resulting geometry is empty.
  return !['if', 'assert', 'echo', 'children', 'child'].includes(node.name)
}

function passedCallChildren(ctx: EvalContext): {
  readonly statements: readonly Statement[]
  readonly context: EvalContext
} {
  const passed = ctx.callChildren
  const statements = Array.isArray(passed) ? passed : passed?.statements ?? []
  if (Array.isArray(passed) || passed === undefined) return { statements, context: ctx }
  return {
    statements,
    context: {
      ...ctx,
      env: new Map(passed.env),
      stableScope: passed.scope,
      scopeVisibleBefore: Number.POSITIVE_INFINITY,
      callChildren: passed.continuation,
    },
  }
}

function compatibilityDeprecation(
  node: CallNode,
  ctx: EvalContext,
  replacement: string,
): void {
  const message = node.name === 'child'
    ? 'child() will be removed in future releases. Use children() instead.'
    : `The ${node.name}() module will be removed in future releases. Use ${replacement} instead.`
  warn(ctx, message)
}

async function evalNode(node: CallNode, parent: EvalContext): Promise<Shape[]> {
  if (isStableProfile(parent) && !parent.viewportRootLocked
    && hasOpenScadViewportModifier(node, 'root')) {
    const shapes = await evalNode(node, {
      ...parent,
      viewportRootLocked: true,
      viewportRootOwner: node,
    })
    if (viewportRootActivates(node, shapes)) throw new StableViewportRootSelection(shapes)
    return shapes
  }
  if (parent.depth >= MAX_EVAL_DEPTH) evaluationError(parent, node.p, `Evaluation exceeds ${MAX_EVAL_DEPTH} nested calls`)
  const operationId = staticOperationId(node)
  const ctx: EvalContext = {
    ...parent,
    depth: parent.depth + 1,
    instancePath: `${parent.instancePath}>${operationId}`,
  }
  const childShapes = async () => await evalNodes(node.children, ctx)

  switch (node.name) {
    case 'assign': {
      requireStableProfile(ctx, node)
      const env = new Map(ctx.env)
      for (const argument of callExpressionArguments(node)) {
        // Historical assign() ignores positional arguments without evaluating
        // them, and each named RHS sees the caller rather than earlier args.
        if (argument.name === undefined) continue
        env.set(argument.name, evalExpression(argument.value, ctx))
      }
      return booleanShapes(
        await evalNodes(node.children, stableOverlayContext(ctx, env), false),
        'union',
        ctx,
        node.p,
        'assign',
      )
    }
    case 'assert': {
      if (isStableProfile(ctx)) {
        const args = callExpressionArguments(node)
        const resolved = resolveStableExpressionArguments(args, ['condition', 'message'], ctx)
        const conditionArgument = resolved.get('condition')
        const messageArgument = resolved.get('message')
        const condition = conditionArgument === undefined
          ? undefined
          : evalExpression(conditionArgument.value, ctx)
        const message = messageArgument === undefined
          ? undefined
          : evalExpression(messageArgument.value, ctx)
        if (!openScadTruthy(condition)) {
          const conditionText = conditionArgument === undefined
            ? 'undef'
            : compactDiagnosticText(ctx.source.slice(conditionArgument.p, conditionArgument.end))
          const detail = messageArgument === undefined
            ? ''
            : `: ${compactDiagnosticText(formatOpenScadValue(message))}`
          evaluationError(ctx, node.p, `Assertion '${conditionText}' failed${detail}`)
        }
        return await childShapes()
      }
      const bound = bindAssertArguments(node, ctx)
      const condition = evalExpression(bound.condition, ctx)
      // OpenSCAD binds call arguments before executing the module. Evaluate a
      // supplied message even when the assertion passes, but never evaluate
      // child geometry when the condition fails.
      const message = bound.message ? evalExpression(bound.message, ctx) : undefined
      if (!truthy(condition)) {
        const detail = bound.message
          ? `: ${compactDiagnosticText(valueToString(message))}`
          : ''
        evaluationError(ctx, node.p, `Assertion '${bound.conditionText}' failed${detail}`)
      }
      return await childShapes()
    }
    case 'cube': {
      if (isStableProfile(ctx)) return makeStableCube(node, ctx)
      const raw = arg(node, 'size', 0, 1, ctx)
      const size = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'cube size') : [finiteNumber(raw, ctx, node.p, 'cube size')]
      const dimensions: Vec3 = [size[0] ?? 1, size[1] ?? size[0] ?? 1, size[2] ?? size[0] ?? 1]
      if (dimensions.some(value => value <= 0)) evaluationError(ctx, node.p, 'Cube dimensions must be positive')
      const geometry = kernelCall(ctx, node.p, () => ctx.kernel.box(dimensions, arg(node, 'center', 1, false, ctx) === true))
      return [trackedSolid(geometry, nextColor(), node, ctx)]
    }
    case 'sphere': {
      if (isStableProfile(ctx)) return makeStableSphere(node, ctx)
      let radius = arg(node, 'r', 0, undefined, ctx)
      const diameter = arg(node, 'd', -1, undefined, ctx)
      if (radius === undefined) radius = diameter === undefined ? 1 : finiteNumber(diameter, ctx, node.p, 'sphere diameter') / 2
      const r = finiteNumber(radius, ctx, node.p, 'sphere radius')
      if (r <= 0) evaluationError(ctx, node.p, 'Sphere radius must be positive')
      return [trackedSolid(kernelCall(ctx, node.p, () => ctx.kernel.sphere(r, segments(node, ctx, 32, 4, r))), nextColor(), node, ctx)]
    }
    case 'brep_gear': {
      // Involute gear as an exact NURBS body (spur, helical, herringbone; external
      // with a bore or internal with a rim). Mesh gets the tessellation, Solid the body.
      const num = (name: string, position: number, fallback: number) =>
        finiteNumber(arg(node, name, position, fallback, ctx), ctx, node.p, `brep_gear ${name}`)
      const spec = {
        module: num('module', 0, 1),
        teeth: Math.round(num('teeth', 1, 20)),
        height: num('height', 2, 5),
        pressureAngle: num('pressure_angle', -1, 20),
        helixAngle: num('helix', -1, 0),
        herringbone: arg(node, 'herringbone', -1, false, ctx) === true,
        bore: num('bore', -1, 0),
        internal: arg(node, 'internal', -1, false, ctx) === true,
        rimWidth: num('rim_width', -1, 2),
        clearance: num('clearance', -1, 0.25),
        backlash: num('backlash', -1, 0),
      }
      if (spec.module <= 0 || spec.teeth < 3 || spec.height <= 0) evaluationError(ctx, node.p, 'brep_gear needs a positive module and height and at least 3 teeth')
      // Six wall faces per tooth (twice for a herringbone), each tessellated into
      // about 2*detail^2 triangles: keep every gear under the bridge's display
      // budget instead of letting a large ring gear fail at the user's $fn.
      const faces = brepGearFaceCount(spec)
      let detail = Math.max(1, Math.min(32, Math.round(segments(node, ctx, 32, 8, spec.module * spec.teeth / 2) / 8)))
      while (detail > 1 && 2 * faces * detail * detail > 16000) detail--
      return [trackedSolid(kernelCall(ctx, node.p, () => ctx.kernel.gear(spec, detail)), nextColor(), node, ctx)]
    }
    case 'cylinder': return isStableProfile(ctx) ? makeStableCylinder(node, ctx) : makeCylinder(node, ctx)
    case 'polyhedron': return isStableProfile(ctx) ? makeStablePolyhedron(node, ctx) : makePolyhedron(node, ctx)
    case 'import': {
      requireStableProfile(ctx, node)
      return makeImport(node, ctx)
    }
    case 'import_stl':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'import()')
      return makeImport(node, ctx, 'stl')
    case 'import_off':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'import()')
      return makeImport(node, ctx, 'off')
    case 'import_dxf':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'import()')
      return makeImport(node, ctx, 'dxf')
    case 'surface': {
      requireStableProfile(ctx, node)
      return makeSurface(node, ctx)
    }
    case 'text': {
      requireStableProfile(ctx, node)
      return makeText(node, ctx)
    }
    case 'square': {
      if (isStableProfile(ctx)) return makeStableSquare(node, ctx)
      const raw = arg(node, 'size', 0, 1, ctx)
      const size = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'square size') : [finiteNumber(raw, ctx, node.p, 'square size')]
      const dimensions: Vec2 = [size[0] ?? 1, size[1] ?? size[0] ?? 1]
      if (dimensions.some(value => value <= 0)) evaluationError(ctx, node.p, 'Square dimensions must be positive')
      return [{
        dimension: 2,
        geometry: ctx.kernel.rectangle(dimensions, arg(node, 'center', 1, false, ctx) === true),
        color: nextColor(),
        entityId: currentEntityId(ctx),
      }]
    }
    case 'circle': {
      if (isStableProfile(ctx)) return makeStableCircle(node, ctx)
      let radius = arg(node, 'r', 0, undefined, ctx)
      const diameter = arg(node, 'd', -1, undefined, ctx)
      if (radius === undefined) radius = diameter === undefined ? 1 : finiteNumber(diameter, ctx, node.p, 'circle diameter') / 2
      const r = finiteNumber(radius, ctx, node.p, 'circle radius')
      if (r <= 0) evaluationError(ctx, node.p, 'Circle radius must be positive')
      return [{
        dimension: 2,
        geometry: ctx.kernel.circle(r, segments(node, ctx, 48, 3, r)),
        color: nextColor(),
        entityId: currentEntityId(ctx),
      }]
    }
    case 'polygon': return isStableProfile(ctx) ? makeStablePolygon(node, ctx) : makePolygon(node, ctx)
    case 'translate': {
      if (isStableProfile(ctx)) return await transformStableChildren(node, ctx, evalNodes)
      const vector = vectorValue(arg(node, 'v', 0, [0, 0, 0], ctx), ctx, node.p, 'translate vector')
      return (await childShapes()).map(shape => ({
        ...shape,
        geometry: ctx.kernel.translate(
          shape.geometry,
          shape.dimension === 3
            ? [vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0]
            : [vector[0] ?? 0, vector[1] ?? 0],
        ),
      }))
    }
    case 'rotate': return isStableProfile(ctx)
      ? await transformStableChildren(node, ctx, evalNodes)
      : rotateShapes(await childShapes(), node, ctx)
    case 'scale': {
      if (isStableProfile(ctx)) return await transformStableChildren(node, ctx, evalNodes)
      const raw = arg(node, 'v', 0, [1, 1, 1], ctx)
      const values = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'scale vector') : [finiteNumber(raw, ctx, node.p, 'scale')]
      const sx = values[0] ?? 1, sy = values[1] ?? sx, sz = values[2] ?? sx
      if ([sx, sy, sz].some(value => value === 0)) evaluationError(ctx, node.p, 'Scale values cannot be zero')
      return (await childShapes()).map(shape => ({
        ...shape,
        geometry: ctx.kernel.scale(
          shape.geometry,
          shape.dimension === 3 ? [sx, sy, sz] : [sx, sy],
        ),
      }))
    }
    case 'resize': {
      requireStableProfile(ctx, node)
      return await resizeChildren(node, ctx, evalNodes)
    }
    case 'mirror': {
      if (isStableProfile(ctx)) return await transformStableChildren(node, ctx, evalNodes)
      const vector = vectorValue(arg(node, 'v', 0, [1, 0, 0], ctx), ctx, node.p, 'mirror normal')
      return (await childShapes()).map(shape => ({
        ...shape,
        geometry: ctx.kernel.mirror(
          shape.geometry,
          shape.dimension === 3
            ? [vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0]
            : [vector[0] ?? 0, vector[1] ?? 0],
        ),
      }))
    }
    case 'multmatrix': return isStableProfile(ctx)
      ? await transformStableChildren(node, ctx, evalNodes)
      : transformByMatrix(await childShapes(), arg(node, 'm', 0, undefined, ctx), node, ctx)
    case 'color': {
      const colorValue = arg(node, 'c', 0, isStableProfile(ctx) ? undefined : [0.5, 0.5, 0.5], ctx)
      const alpha = arg(node, 'alpha', 1, undefined, ctx)
      const color = isStableProfile(ctx)
        ? [...resolveOpenScadColor(colorValue, alpha, stableModuleSemanticsContext(ctx)).rgba] as RGBA
        : parseLegacyColor(colorValue, ctx, node.p)
      if (!isStableProfile(ctx) && alpha !== undefined) {
        color[3] = clamp01(finiteNumber(alpha, ctx, node.p, 'color alpha'))
      }
      return (await childShapes()).map(shape => ({ ...shape, color: [...color] as RGBA }))
    }
    case 'union': return booleanShapes(await childShapes(), 'union', ctx, node.p)
    case 'difference': return await differenceChildren(node, ctx, evalNodes, evalPreparedNodes)
    case 'intersection': return booleanShapes(await childShapes(), 'intersection', ctx, node.p)
    case 'minkowski': {
      requireStableProfile(ctx, node)
      return minkowskiShapes(await childShapes(), node, ctx)
    }
    case 'hull': return hullShapes(await childShapes(), node, ctx)
    case 'linear_extrude': return await linearExtrude(node, ctx, evalNodes)
    case 'rotate_extrude': return await rotateExtrude(node, ctx, evalNodes)
    case 'dxf_linear_extrude':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'linear_extrude()')
      return await dxfLinearExtrude(node, ctx, evalNodes)
    case 'dxf_rotate_extrude':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'rotate_extrude()')
      return await dxfRotateExtrude(node, ctx, evalNodes)
    case 'projection': {
      if (isStableProfile(ctx)) {
        const values = evaluateStableBuiltinModuleArguments(
          node,
          ['cut', 'convexity'],
          [],
          ctx,
        )
        const cut = values.get('cut') === true
        const children = await childShapes()
        const solids = children.filter((shape): shape is Shape3D => shape.dimension === 3)
        if (solids.length !== children.length) {
          warn(ctx, 'projection() ignored non-3D child geometry')
        }
        if (solids.length === 0) return []
        const color = solids[0].color
        const geometry = cut
          ? ctx.kernel.projection(ctx.kernel.boolean3('union', solids.map(shape => shape.geometry)), true)
          : ctx.kernel.boolean2('union', solids.map(shape => ctx.kernel.projection(shape.geometry, false)))
        return [{ dimension: 2, geometry, color, entityId: currentEntityId(ctx) }]
      }
      const cut = arg(node, 'cut', 0, false, ctx) === true
      return (await childShapes()).map(shape => {
        if (shape.dimension !== 3) evaluationError(ctx, node.p, 'projection() requires 3D children')
        return {
          dimension: 2,
          geometry: ctx.kernel.projection(shape.geometry, cut),
          color: shape.color,
          entityId: currentEntityId(ctx),
        }
      })
    }
    case 'offset': {
      if (isStableProfile(ctx)) {
        const rExpression = node.args.r ?? node.args._0
        const deltaExpression = node.args.delta
        const chamferExpression = node.args.chamfer
        const resolved = resolveOpenScadOffset({
          ...(rExpression === undefined ? {} : { r: evalExpression(rExpression, ctx) }),
          ...(deltaExpression === undefined ? {} : { delta: evalExpression(deltaExpression, ctx) }),
          ...(chamferExpression === undefined ? {} : { chamfer: evalExpression(chamferExpression, ctx) }),
        }, stableModuleSemanticsContext(ctx))
        const circularSegments = resolved.joinType === 'Round'
          ? segments(node, ctx, 48, 3, Math.abs(resolved.distance))
          : undefined
        return (await childShapes()).map(shape => {
          if (shape.dimension !== 2) evaluationError(ctx, node.p, 'offset() requires 2D children')
          return {
            ...shape,
            geometry: ctx.kernel.offset(
              shape.geometry,
              resolved.distance,
              resolved.joinType,
              resolved.joinType === 'Miter' ? 1_000_000_000 : 2,
              circularSegments,
            ),
            entityId: currentEntityId(ctx),
          }
        })
      }
      const distance = finiteNumber(arg(node, 'r', 0, arg(node, 'delta', 0, 1, ctx), ctx), ctx, node.p, 'offset distance')
      return (await childShapes()).map(shape => {
        if (shape.dimension !== 2) evaluationError(ctx, node.p, 'offset() requires 2D children')
        return { ...shape, geometry: ctx.kernel.offset(shape.geometry, distance), entityId: currentEntityId(ctx) }
      })
    }
    case 'group': {
      const shapes = await childShapes()
      return isStableProfile(ctx) ? booleanShapes(shapes, 'union', ctx, node.p, 'group') : shapes
    }
    case 'render': {
      if (isStableProfile(ctx)) {
        evaluateStableBuiltinModuleArguments(node, ['convexity'], [], ctx)
        return booleanShapes(await childShapes(), 'union', ctx, node.p, 'render')
      }
      return await childShapes()
    }
    case 'if': {
      const condition = arg(node, '_0', 0, false, ctx)
      const branch = isStableProfile(ctx) ? openScadTruthy(condition) : truthy(condition)
      return await evalNodes(branch ? node.children : node.alternative, ctx)
    }
    case 'let': {
      if (isStableProfile(ctx)) {
        const env = evaluateSequentialBindings(callExpressionArguments(node), ctx, 0)
        return await evalNodes(node.children, stableOverlayContext(ctx, env), false)
      }
      const env = new Map(ctx.env)
      for (const [name, expression] of Object.entries(node.args)) if (!name.startsWith('_')) env.set(name, evalExpression(expression, ctx))
      return await evalNodes(node.children, { ...ctx, env }, false)
    }
    case 'for': return await evalFor(node, ctx, 'for')
    case 'intersection_for': {
      requireStableProfile(ctx, node)
      return booleanShapes(
        await evalFor(node, ctx, 'intersection_for'),
        'intersection',
        ctx,
        node.p,
        'intersection_for',
      )
    }
    case 'echo': {
      requireStableProfile(ctx, node)
      const values = Object.entries(node.args).map(([name, expression]) => {
        const value = formatOpenScadValue(evalExpression(expression, ctx))
        return node.argKinds[name] === 'named' ? `${name} = ${value}` : value
      })
      ctx.warnings.push(`ECHO:${values.length ? ` ${values.join(', ')}` : ''}`)
      return await childShapes()
    }
    case 'children': {
      const passed = passedCallChildren(ctx)
      const all = passed.statements
      const childContext = passed.context
      if (isStableProfile(ctx)) {
        const selectorExpression = node.args._0
        const selected = resolveOpenScadChildrenSelection({
          provided: selectorExpression !== undefined,
          value: selectorExpression === undefined ? undefined : evalExpression(selectorExpression, ctx),
          childCount: all.length,
          maximumRangeItems: MAX_RANGE_ITEMS,
        }, stableGeometrySemanticsContext(ctx))
        if (selectorExpression === undefined) return await evalNodes(all, childContext)
        const output: Shape[] = []
        for (const [occurrence, childIndex] of selected.entries()) {
          output.push(...await evalNodes([all[childIndex]], {
            ...childContext,
            instancePath: `${childContext.instancePath}>children:${childIndex}#${occurrence}`,
          }))
        }
        return output
      }
      const index = arg(node, '_0', 0, undefined, ctx)
      if (index === undefined) return await evalNodes(all, childContext)
      const childIndex = Math.trunc(finiteNumber(index, ctx, node.p, 'children index'))
      return all[childIndex] ? await evalNodes([all[childIndex]], childContext) : []
    }
    case 'child': {
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'children()')
      const authored = callExpressionArguments(node)
      const indexValue = authored[0] === undefined
        ? 0
        : evalExpression(authored[0].value, ctx)
      const index = typeof indexValue === 'number' && Number.isFinite(indexValue)
        ? Math.trunc(indexValue)
        : 0
      if (index < 0) {
        warn(ctx, `Negative child index (${index}) not allowed`)
        return []
      }
      const passed = passedCallChildren(ctx)
      const statement = passed.statements[index]
      if (statement === undefined) {
        if (ctx.callChildren !== undefined) {
          warn(ctx, `Child index (${index}) out of bounds (${passed.statements.length} children)`)
        }
        return []
      }
      return await evalNodes([statement], {
        ...passed.context,
        instancePath: `${passed.context.instancePath}>child:${index}`,
      })
    }
  }

  if (isStableProfile(ctx)) {
    const declaration = ctx.stableScope?.moduleDeclaration(node.name)
    if (declaration !== undefined) return await evalUserModule(node, declaration.node, ctx, declaration.scope)
  } else {
    const module = ctx.modules.get(node.name)
    if (module) return await evalUserModule(node, module, ctx)
  }
  evaluationError(ctx, node.p, `Unsupported geometry operation ${node.name}()`)
}

async function evalFor(node: CallNode, ctx: EvalContext, diagnosticName: 'for' | 'intersection_for'): Promise<Shape[]> {
  const control = ctx.control
  if (isStableProfile(ctx)) {
    const bindings = callExpressionArguments(node)
    const output: Shape[] = []
    const occurrences = new Map<string, number>()
    let iterationsSinceYield = 0
    const visit = async (
      bindingIndex: number,
      iterationContext: EvalContext,
      path: readonly string[],
    ): Promise<void> => {
      if (bindingIndex >= bindings.length) {
        const valueKey = path.join(',')
        const occurrence = occurrences.get(valueKey) ?? 0
        occurrences.set(valueKey, occurrence + 1)
        if (control && iterationsSinceYield >= YIELD_EVERY_STATEMENTS) {
          iterationsSinceYield = 0
          await control.yieldIfDue()
        }
        iterationsSinceYield++
        control?.poll()
        output.push(...await evalNodes(node.children, {
          ...iterationContext,
          instancePath: `${ctx.instancePath}>loop:${valueKey}#${occurrence}`,
        }, false))
        if (output.length > MAX_SHAPES) {
          evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
        }
        return
      }
      const binding = bindings[bindingIndex]
      const values = stableIterable(
        evalExpression(binding.value, iterationContext),
        iterationContext,
        binding.p,
      )
      if (binding.name === undefined) {
        warn(ctx, `Ignoring ${diagnosticName}() iterator without variable name`)
        return
      }
      for (const value of values) {
        if (++ctx.budget.ops > MAX_EVAL_OPS) {
          evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
        }
        const env = new Map(iterationContext.env)
        env.set(binding.name, value)
        const segment = `${encodeURIComponent(binding.name)}=${encodeURIComponent(identityValue(value))}`
        await visit(bindingIndex + 1, stableOverlayContext(iterationContext, env), [...path, segment])
      }
    }
    await visit(0, ctx, [])
    return output
  }
  const entries = Object.entries(node.args).filter(([name]) => !name.startsWith('_'))
  if (entries.length !== 1) evaluationError(ctx, node.p, `${diagnosticName}() currently requires one named iterator`)
  const [name, expression] = entries[0]
  const values = evalExpression(expression, ctx)
  if (!Array.isArray(values)) evaluationError(ctx, node.p, `${diagnosticName}() iterator must be a vector or range`)
  const output: Shape[] = []
  const occurrences = new Map<string, number>()
  let iterationsSinceYield = 0
  for (const value of values) {
    if (control && iterationsSinceYield >= YIELD_EVERY_STATEMENTS) {
      iterationsSinceYield = 0
      await control.yieldIfDue()
    }
    iterationsSinceYield++
    control?.poll()
    if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
    const env = new Map(ctx.env)
    env.set(name, value)
    const valueKey = encodeURIComponent(identityValue(value))
    const occurrence = occurrences.get(valueKey) ?? 0
    occurrences.set(valueKey, occurrence + 1)
    output.push(...await evalNodes(node.children, {
      ...ctx,
      env,
      instancePath: `${ctx.instancePath}>loop:${encodeURIComponent(name)}=${valueKey}#${occurrence}`,
    }, false))
    if (output.length > MAX_SHAPES) evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
  }
  return output
}

async function evalUserModule(
  call: CallNode,
  module: ModuleNode,
  ctx: EvalContext,
  definitionScope?: OpenScadStableScope,
): Promise<Shape[]> {
  let env = new Map(ctx.env)
  if (isStableProfile(ctx)) {
    if (definitionScope === undefined) {
      evaluationError(ctx, call.p, `Stable module ${module.name}() is missing its lexical scope`)
    }
    const moduleStack = [...ctx.moduleStack, module.name]
    const definitionEnv = new Map(definitionScope.env)
    overlayDynamicVariables(definitionEnv, ctx.env)
    definitionEnv.set('$children', call.children.length)
    definitionEnv.set('$parent_modules', moduleStack.length)
    const definitionContext: EvalContext = {
      ...ctx,
      env: definitionEnv,
      stableScope: definitionScope,
      scopeVisibleBefore: Number.POSITIVE_INFINITY,
      moduleStack,
    }
    const args = callExpressionArguments(call)
    const resolved = resolveStableExpressionArguments(
      args,
      module.params.map(parameter => parameter.name),
      ctx,
    )
    const callerValues = new Map<ExpressionArgument, Value>()
    for (const argument of args) callerValues.set(argument, evalExpression(argument.value, ctx))
    const parameterValues = new Map<string, Value>()
    for (const parameter of module.params) {
      const supplied = resolved.get(parameter.name)
      const value = supplied !== undefined
        ? callerValues.get(supplied)
        : parameter.defaultValue !== undefined
          ? evalExpression(parameter.defaultValue, { ...definitionContext, env: new Map(definitionEnv) })
          : undefined
      parameterValues.set(parameter.name, value)
    }
    env = new Map(definitionEnv)
    for (const [name, value] of parameterValues) env.set(name, value)
    return booleanShapes(await evalNodes(module.children, {
      ...definitionContext,
      env,
      callChildren: {
        statements: call.children,
        env: new Map(ctx.env),
        scope: ctx.stableScope ?? definitionScope,
        continuation: ctx.callChildren,
      },
    }, false), 'union', ctx, call.p, module.name)
  } else {
    const positional = Object.entries(call.args).filter(([name]) => name.startsWith('_')).sort(([a], [b]) => Number(a.slice(1)) - Number(b.slice(1)))
    for (let i = 0; i < module.params.length; i++) {
      const param = module.params[i]
      const expression = call.args[param.name] ?? positional[i]?.[1] ?? param.defaultValue
      env.set(param.name, expression ? evalExpression(expression, { ...ctx, env }) : undefined)
    }
  }
  return await evalNodes(module.children, {
    ...ctx,
    env,
    callChildren: call.children,
    moduleStack: [...ctx.moduleStack, module.name],
  }, false)
}

/**
 * Warm the official geometry kernel, cached per JS realm. Exported so a
 * hosting worker can eagerly compile it at startup instead of paying the
 * download+compile cost on the first request. A rejected load is NOT cached:
 * one transient network failure must not brick every future parse, so the
 * cached promise is cleared on rejection and the next call retries.
 */
export function warmGeometryKernel(): Promise<void> {
  return defaultGeometryKernel.warm().then(() => undefined)
}

async function parseInternal(
  source: string,
  options: ParseOptions,
  compileAst: () => readonly Statement[] = () => compileOpenSCAD(source, {
    languageProfile: options.languageProfile,
  }),
  project?: OpenScadProject,
): Promise<ParseResult> {
  const now = options.now ?? (() => performance.now())
  const startedAt = now()
  if (project === undefined && source.length > MAX_SOURCE_LENGTH) {
    throw new OpenSCADParseError(source, 0, `Source exceeds ${MAX_SOURCE_LENGTH.toLocaleString()} characters`)
  }
  resetPalette()
  const languageProfile = options.languageProfile ?? 'openscad-viewer-subset@1'
  let ast: readonly Statement[]
  let bound
  let parseMs: number
  let bindMs: number
  if (project === undefined) {
    const prepared = prepareOpenScadFrontEnd(source, {
      languageProfile,
      compile: () => compileAst(),
      now,
    })
    ast = prepared.program
    bound = prepared.bound
    parseMs = prepared.compileMs
    bindMs = prepared.bindMs
  } else {
    ast = compileAst()
    const compiledAt = now()
    bound = bindOpenScad(ast, { languageProfile, source })
    const boundAt = now()
    parseMs = Math.max(0, compiledAt - startedAt)
    bindMs = Math.max(0, boundAt - compiledAt)
  }
  const forcedImportAssets = project === undefined
    ? []
    : stableCompatibilityForcedAssets(project, ast)
  const [importAssets, surfaceAssets, textAssets] = project === undefined
    ? [undefined, undefined, undefined]
    : await Promise.all([
        prepareOpenScadImportAssets(project, { forcedAssets: forcedImportAssets }),
        prepareOpenScadSurfaceAssets(project),
        prepareOpenScadTextAssets(project),
      ])
  const parsedAt = now()
  const kernelSession = await defaultGeometryKernel.openEvalSession()
  // Recording costs only a small JSON graph, but it is opt-in because the result
  // crosses a protocol that validates an exact set of keys.
  const exactRecorder = options.recordExactSolids === true
    ? createBrepRecordingKernelOps(kernelSession.kernel)
    : null
  const warnings: string[] = []
  const modules = new Map(bound.modules)
  const functions = new Map(bound.functions)
  const quality = options.quality ?? 'full'
  const env = languageProfile === 'openscad/stable-2021.01'
    ? new Map<string, Value>(Array.from(
        createOpenScadStableRuntimeVariables({ quality, animationTime: options.animationTime }),
        ([name, value]) => [name, Array.isArray(value) ? [...value] : value as Value],
      ))
    : new Map<string, Value>([['$fn', 0], ['$fa', 12], ['$fs', 2]])
  const sourceReferences = new Map<number, MeshSourceReference>()
  const reduced = { value: false }
  let ctx: EvalContext = {
    kernel: exactRecorder?.ops ?? kernelSession.kernel,
    source,
    project,
    importAssets,
    surfaceAssets,
    textAssets,
    languageProfile,
    env,
    functions,
    modules,
    warnings,
    quality,
    sourceReferences,
    depth: 0,
    functionStack: [],
    moduleStack: [],
    budget: { ops: 0 },
    reduced,
    instancePath: 'root',
    valueBudget: { used: 0 },
    valueWeights: new WeakMap(),
    valueDepths: new WeakMap(),
  }
  if (isStableProfile(ctx)) ctx = enterStableStatementScope(ast, ctx)
  const initializedAt = now()
  const control = new CooperativeCheckpoint(options.shouldAbort, options.onYield, now, options.yieldControl)
  ctx = { ...ctx, control }

  try {
    let shapes = await evalTopLevel(ast, ctx, control)
    if (isStableProfile(ctx)) shapes = booleanShapes(shapes, 'union', ctx, 0, 'union')
    // A single giant statement may occupy a whole macrotask. Always yield
    // before post-processing so a queued cancel can skip mesh extraction.
    await control.yieldIfDue(true)
    const evaluatedAt = now()
    // Mesh extraction (normals/BVH/edges) is often the dominant cost — a
    // superseded request must not pay it in full before the newest starts.
    if (options.shouldAbort?.()) throw new AbortedError()
    const sections = shapes.filter(shape => shape.dimension === 2)
    if (sections.length) warn(ctx, `${sections.length} top-level 2D object(s) are not displayed; wrap them in linear_extrude() or rotate_extrude()`)
    const solids = shapes.filter((shape): shape is Shape3D => shape.dimension === 3 && !ctx.kernel.isEmpty(shape.geometry))
    const meshes: MeshData[] = []
    let volume = 0
    let surfaceArea = 0
    let triangleCount = 0
    const exactRoots: NonNullable<ParseResult['exactSolids']>['roots'][number][] = []
    for (const [index, shape] of solids.entries()) {
      if (options.shouldAbort?.()) throw new AbortedError()
      // Resolved here, while the handle is still alive and before its mesh is extracted.
      if (exactRecorder) {
        const name = `Body ${index + 1}`
        const resolved = exactRecorder.recording.resolve(shape.geometry)
        exactRoots.push('id' in resolved ? { name, id: resolved.id } : { name, inexact: resolved.inexact })
      }
      const analysis = options.shouldAbort || options.onYield
        ? await ctx.kernel.analyzeSolidCooperatively(shape.geometry, async () => {
          control.poll()
          await control.yieldIfDue()
        })
        : ctx.kernel.analyzeSolid(shape.geometry)
      volume += analysis.volume
      surfaceArea += analysis.surfaceArea
      const mesh = analysis.mesh
      await control.yieldIfDue()
      triangleCount += mesh.numTri
      if (triangleCount > MAX_TRIANGLES) evaluationError(ctx, 0, `Rendered model exceeds ${MAX_TRIANGLES.toLocaleString()} triangles`)
      if (mesh.numProp < 6) evaluationError(ctx, 0, 'Geometry kernel did not produce normals')
      let vertices: Float32Array
      if (mesh.numProp === 6 && mesh.vertProperties instanceof Float32Array && isExclusiveView(mesh.vertProperties, mesh.numVert * 6)) {
        // The kernel copied this array out for this analysis alone; publish it
        // without another copy. The Worker protocol requires exclusive buffers.
        vertices = mesh.vertProperties
      } else {
        vertices = new Float32Array(mesh.numVert * 6)
        for (let vertex = 0; vertex < mesh.numVert; vertex++) {
          const sourceOffset = vertex * mesh.numProp
          const targetOffset = vertex * 6
          for (let channel = 0; channel < 6; channel++) vertices[targetOffset + channel] = mesh.vertProperties[sourceOffset + channel]
          if ((vertex & 0x3fff) === 0x3fff) await control.yieldIfDue()
        }
      }
      const indices = isExclusiveView(mesh.triVerts, mesh.triVerts.length) ? mesh.triVerts : new Uint32Array(mesh.triVerts)
      const { bvh, semanticEdges } = analysis
      const provenance: MeshProvenanceRun[] = []
      for (let run = 0; run < mesh.runOriginalID.length; run++) {
        const triangleStart = (mesh.runIndex[run] ?? 0) / 3
        const triangleEnd = (mesh.runIndex[run + 1] ?? mesh.triVerts.length) / 3
        if (triangleEnd <= triangleStart) continue
        const originalId = mesh.runOriginalID[run]
        provenance.push({
          triangleStart,
          triangleEnd,
          source: sourceReferences.get(originalId) ?? null,
          backside: ((mesh.runFlags[run] ?? 0) & 1) !== 0,
        })
        if ((run & 0x1fff) === 0x1fff) await control.yieldIfDue()
      }
      if (provenance.length === 0 && mesh.numTri > 0) {
        provenance.push({ triangleStart: 0, triangleEnd: mesh.numTri, source: null, backside: false })
      }
      meshes.push({
        entityId: shape.entityId,
        geometryAssetId: geometryAssetId(vertices, indices),
        vertices,
        indices,
        bvh,
        edgeIndices: semanticEdges.indices,
        color: [...shape.color],
        transform: identity(),
        faceIds: isExclusiveView(mesh.faceID, mesh.faceID.length) ? mesh.faceID : new Uint32Array(mesh.faceID),
        provenance,
        topology: semanticEdges.diagnostics,
      })
      await control.yieldIfDue()
    }
    const analyzedAt = now()
    return {
      meshes, warnings, volume, surfaceArea, quality, reduced: reduced.value,
      ...(exactRecorder ? { exactSolids: { nodes: exactRecorder.recording.nodes, roots: exactRoots } } : {}),
      timings: {
        parseMs,
        bindMs,
        initializeMs: Math.max(0, initializedAt - parsedAt),
        evaluateMs: Math.max(0, evaluatedAt - initializedAt),
        analyzeMs: Math.max(0, analyzedAt - evaluatedAt),
      },
    }
  } finally {
    kernelSession.dispose()
  }
}

let parseQueue: Promise<void> = Promise.resolve()

/** Parse and evaluate the supported OpenSCAD subset in a serialized WASM scope. */
export function parseOpenSCAD(source: string, options: ParseOptions = {}): Promise<ParseResult> {
  const run = async () => {
    if (isRushFrontend(source)) {
      const { compileRushFrontend } = await import('./rushFrontend')
      const compiled = compileRushFrontend(source)
      if (compiled.execution_target === 'own-nurbs') {
        const { buildTextNurbsScene } = await import('./rushFrontendScene')
        if (options.shouldAbort?.()) throw new AbortedError()
        try {
          const result = await buildTextNurbsScene(compiled.document, options.quality, { shouldAbort: options.shouldAbort, onYield: options.onYield, onSweepPreview: options.onSweepPreview })
          if (options.shouldAbort?.()) throw new AbortedError()
          return result
        } catch (error) {
          if (error instanceof DOMException && error.name === 'AbortError') throw new AbortedError()
          throw error
        }
      }
      const result = await parseInternal(compiled.source, options)
      requireRushGraphChecks(await evaluateRushGraphGeometry(compiled.geometry_assertions, result.meshes, async source => (await parseInternal(source, options)).meshes))
      // Generated SCAD spans are not spans in the authored compact document.
      for (const mesh of result.meshes) for (const run of mesh.provenance) run.source = null
      return result
    }
    return parseInternal(source, options)
  }
  const result = parseQueue.then(run, run)
  parseQueue = result.then(() => undefined, () => undefined)
  return result
}

/** Compile and evaluate one bounded project snapshot with the independent full profile. */
export function parseOpenScadProject(
  project: OpenScadProject,
  options: ParseOpenScadProjectOptions = {},
): Promise<ParseResult> {
  const run = () => parseInternal(
    project.readEntrypoint().source,
    { ...options, languageProfile: 'openscad/stable-2021.01' },
    () => compileOpenScadProject(project),
    project,
  )
  const result = parseQueue.then(run, run)
  parseQueue = result.then(() => undefined, () => undefined)
  return result
}
