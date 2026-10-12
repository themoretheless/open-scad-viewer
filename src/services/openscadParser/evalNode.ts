import { hasOpenScadViewportModifier } from '../openscadCompiler'
import type { CallNode } from '../openscadCompiler'
import { StableViewportRootSelection, evaluateSequentialBindings, resolveStableExpressionArguments } from '../openscadParser'
import type { EvalContext, Shape, Shape3D, Vec2, Vec3 } from '../openscadParser'
import { MAX_EVAL_DEPTH, MAX_RANGE_ITEMS, compactDiagnosticText, evaluationError, isStableProfile, stableOverlayContext } from '../openScadStableExpressionEval'
import { nextColor } from '../openscadColors'
import type { RGBA } from '../openscadColors'
import { arg, bindAssertArguments, callExpressionArguments, compatibilityDeprecation, evalNodes, finiteNumber, passedCallChildren, truthy, valueToString, vectorValue, viewportRootActivates } from './evalBuiltin'
import { booleanShapes, differenceChildren, hullShapes, minkowskiShapes, resizeChildren, rotateShapes, transformByMatrix, transformStableChildren } from './transformStableChildren'
import { brepGearFaceCount } from '../geometry/brep'
import { clamp01 } from '../math3d'
import { currentEntityId, evalExpression, evaluateStableBuiltinModuleArguments, kernelCall, staticOperationId, trackedSolid, warn } from './isExclusiveView'
import { dxfLinearExtrude, dxfRotateExtrude, evalFor, linearExtrude, rotateExtrude } from './compatibilityDxfProfile'
import { evalUserModule, parseLegacyColor } from './warmGeometryKernel'
import { formatOpenScadValue, openScadTruthy } from '../openScadValueSemantics'
import { makeCylinder, makePolyhedron, makeStableCircle, makeStableCube, makeStableCylinder, makeStablePolygon, makeStablePolyhedron, makeStableSphere, makeStableSquare, requireStableProfile, segments, stableGeometrySemanticsContext, stableModuleSemanticsContext } from './requireStableProfile'
import { makeImport, makePolygon, makeSurface } from './makeImport'
import { makeText } from './surfaceCallSource'
import { resolveOpenScadChildrenSelection } from '../openScadStableGeometrySemantics'
import { resolveOpenScadColor, resolveOpenScadOffset } from '../openScadStableModuleSemantics'

export async function evalNode(node: CallNode, parent: EvalContext): Promise<Shape[]> {
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
      if (isStableProfile(ctx)) return await transformStableChildren(node, ctx)
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
    case 'rotate': return isStableProfile(ctx) ? await transformStableChildren(node, ctx) : rotateShapes(await childShapes(), node, ctx)
    case 'scale': {
      if (isStableProfile(ctx)) return await transformStableChildren(node, ctx)
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
      return await resizeChildren(node, ctx)
    }
    case 'mirror': {
      if (isStableProfile(ctx)) return await transformStableChildren(node, ctx)
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
      ? await transformStableChildren(node, ctx)
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
    case 'difference': return await differenceChildren(node, ctx)
    case 'intersection': return booleanShapes(await childShapes(), 'intersection', ctx, node.p)
    case 'minkowski': {
      requireStableProfile(ctx, node)
      return minkowskiShapes(await childShapes(), node, ctx)
    }
    case 'hull': return hullShapes(await childShapes(), node, ctx)
    case 'linear_extrude': return await linearExtrude(node, ctx)
    case 'rotate_extrude': return await rotateExtrude(node, ctx)
    case 'dxf_linear_extrude':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'linear_extrude()')
      return await dxfLinearExtrude(node, ctx)
    case 'dxf_rotate_extrude':
      requireStableProfile(ctx, node)
      compatibilityDeprecation(node, ctx, 'rotate_extrude()')
      return await dxfRotateExtrude(node, ctx)
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
