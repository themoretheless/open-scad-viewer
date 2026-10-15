import { hasOpenScadViewportModifier } from '../openscadCompiler'
import type { CallNode, Value } from '../openscadCompiler'
import { MAX_EXTRUDE_SLICES, MAX_SHAPES, StableViewportRootSelection, enterStableStatementScope, evaluateSequentialBindings, resolveStableExpressionArguments, stableIterable } from '../openscadSemanticLowerer'
import type { EvalContext, RGBA, SemanticShape } from '../openscadSemanticLowerer'
import { MAX_EVAL_DEPTH, MAX_EVAL_OPS, compactDiagnosticText, evaluationError, isStableProfile, stableOverlayContext } from '../openScadStableExpressionEval'
import type { SemanticDynamicSlot, SemanticValueType } from '../../core/semanticProgram'
import { arg, bindAssertArguments, callExpressionArguments, evalExpression, evalNodes, semanticProjectAssetRequired, stableCompatibilityDeprecation, viewportRootActivates, viewportSemanticSlots } from './evalExpression'
import { axisAngleMatrix, axisAngleMatrixUnchecked, combineShapes, eulerMatrix, evalPreparedNodes, frameChildren, frameContext, invoke, matrixValue, mirrorMatrix, nodeValueType, parseColor, primitiveColor, scaleMatrix, semanticSegments, semanticSlot, transformShapes, translationMatrix, unionReduceInternal } from './evalPreparedNodes'
import { clamp01 } from '../math3d'
import { evalDxfLinearExtrude, evalDxfRotateExtrude, evalPassedCallChildren, makeCylinder, makePolygon, makePolyhedron } from './makeCylinder'
import { evalUserModule } from './evalUserModule'
import { finiteNumber, semanticValueType, stableWarning, truthy, valueToString, vectorValue } from './semanticValueType'
import { formatOpenScadValue, openScadTruthy } from '../openScadValueSemantics'

export function evalNode(node: CallNode, parent: EvalContext): SemanticShape[] {
  if (isStableProfile(parent) && !parent.viewportRootLocked
    && hasOpenScadViewportModifier(node, 'root')) {
    // Selection is runtime-first and value-producing, not syntax-first: an
    // unvisited false branch, unused module, or empty call cannot suppress
    // later geometry. Probe the candidate with nested roots locked, then replay
    // a non-empty winner against the captured environment as a detached root.
    const diagnosticsBeforeCandidate = parent.builder.diagnosticCheckpoint()
    const shapes = evalNode(node, {
      ...parent,
      viewportRootLocked: true,
      viewportRootOwner: node,
    })
    if (viewportRootActivates(node, shapes)) {
      throw new StableViewportRootSelection(node, parent, diagnosticsBeforeCandidate)
    }
    return shapes
  }
  if (parent.depth >= MAX_EVAL_DEPTH) evaluationError(parent, node.p, `Evaluation exceeds ${MAX_EVAL_DEPTH} nested calls`)
  const ctx: EvalContext = { ...parent, depth: parent.depth + 1 }

  switch (node.name) {
    case 'assign': {
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'Unsupported geometry operation assign()')
      const env = new Map(ctx.env)
      const bindings = new Map<string, Value>()
      for (const argument of callExpressionArguments(node)) {
        // Historical assign() ignores positional arguments without evaluating
        // them, and evaluates every named RHS against the caller in parallel.
        if (argument.name === undefined) continue
        const value = evalExpression(argument.value, ctx)
        bindings.set(argument.name, value)
        env.set(argument.name, value)
      }
      const occurrence = invoke(
        node,
        ctx,
        [...bindings].map(([name, value]) => semanticSlot(name, value)),
      )
      return frameChildren(node, node.children, occurrence, stableOverlayContext(ctx, env))
    }
    case 'dxf_linear_extrude':
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'Unsupported geometry operation dxf_linear_extrude()')
      return evalDxfLinearExtrude(node, ctx)
    case 'dxf_rotate_extrude':
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'Unsupported geometry operation dxf_rotate_extrude()')
      return evalDxfRotateExtrude(node, ctx)
    case 'import_stl':
    case 'import_off':
    case 'import_dxf':
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, `Unsupported geometry operation ${node.name}()`)
      return semanticProjectAssetRequired(ctx, node.p, node.end, `${node.name}()`)
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
        const conditionText = conditionArgument === undefined
          ? 'undef'
          : compactDiagnosticText(ctx.source.slice(conditionArgument.p, conditionArgument.end))
        const occurrence = invoke(node, ctx, [semanticSlot('condition', condition), semanticSlot('message', message)])
        if (!openScadTruthy(condition)) {
          const detail = messageArgument === undefined
            ? ''
            : `: ${compactDiagnosticText(formatOpenScadValue(message))}`
          evaluationError(ctx, node.p, `Assertion '${conditionText}' failed${detail}`)
        }
        return ctx.builder.alias(occurrence, frameChildren(node, node.children, occurrence, ctx), false)
      }
      const bound = bindAssertArguments(node, ctx)
      const condition = evalExpression(bound.condition, ctx)
      const message = bound.message ? evalExpression(bound.message, ctx) : undefined
      const occurrence = invoke(node, ctx, [semanticSlot('condition', condition), semanticSlot('message', message)])
      if (!truthy(condition)) {
        const detail = bound.message ? `: ${compactDiagnosticText(valueToString(message))}` : ''
        evaluationError(ctx, node.p, `Assertion '${bound.conditionText}' failed${detail}`)
      }
      return ctx.builder.alias(occurrence, frameChildren(node, node.children, occurrence, ctx), false)
    }
    case 'cube': {
      const raw = arg(node, 'size', 0, 1, ctx)
      const size = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'cube size') : [finiteNumber(raw, ctx, node.p, 'cube size')]
      const dimensions = [size[0] ?? 1, size[1] ?? size[0] ?? 1, size[2] ?? size[0] ?? 1] as const
      if (dimensions.some(value => value <= 0)) evaluationError(ctx, node.p, 'Cube dimensions must be positive')
      const occurrence = invoke(node, ctx)
      return [ctx.builder.produce(occurrence, {
        kind: 'box',
        valueType: semanticValueType(ctx.builder.languageContract, ctx.builder.languageContract === 'legacy/current' ? 'solid-set' : 'solid', 'd3'),
        size: dimensions,
        center: arg(node, 'center', 1, false, ctx) === true,
      }, primitiveColor(ctx))]
    }
    case 'sphere': {
      let radius = arg(node, 'r', 0, undefined, ctx)
      const diameter = arg(node, 'd', -1, undefined, ctx)
      if (radius === undefined) radius = diameter === undefined ? 1 : finiteNumber(diameter, ctx, node.p, 'sphere diameter') / 2
      const parsedRadius = finiteNumber(radius, ctx, node.p, 'sphere radius')
      if (parsedRadius <= 0) evaluationError(ctx, node.p, 'Sphere radius must be positive')
      const occurrence = invoke(node, ctx)
      const segments = semanticSegments(node, ctx, occurrence, 32, 4)
      return [ctx.builder.produce(occurrence, segments === null ? {
        kind: 'sphere-analytic',
        valueType: semanticValueType(ctx.builder.languageContract, 'solid', 'd3'),
        radius: parsedRadius,
      } : {
        kind: 'sphere-polygonal',
        valueType: semanticValueType(ctx.builder.languageContract, 'solid-set', 'd3'),
        radius: parsedRadius,
        radialSegments: segments,
      }, primitiveColor(ctx))]
    }
    case 'cylinder': return makeCylinder(node, ctx)
    case 'polyhedron': return makePolyhedron(node, ctx)
    case 'square': {
      const raw = arg(node, 'size', 0, 1, ctx)
      const size = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'square size') : [finiteNumber(raw, ctx, node.p, 'square size')]
      const dimensions = [size[0] ?? 1, size[1] ?? size[0] ?? 1] as const
      if (dimensions.some(value => value <= 0)) evaluationError(ctx, node.p, 'Square dimensions must be positive')
      const occurrence = invoke(node, ctx)
      return [ctx.builder.produce(occurrence, {
        kind: 'rectangle',
        valueType: semanticValueType(ctx.builder.languageContract, 'region', 'd2'),
        size: dimensions,
        center: arg(node, 'center', 1, false, ctx) === true,
      }, primitiveColor(ctx))]
    }
    case 'circle': {
      let radius = arg(node, 'r', 0, undefined, ctx)
      const diameter = arg(node, 'd', -1, undefined, ctx)
      if (radius === undefined) radius = diameter === undefined ? 1 : finiteNumber(diameter, ctx, node.p, 'circle diameter') / 2
      const parsedRadius = finiteNumber(radius, ctx, node.p, 'circle radius')
      if (parsedRadius <= 0) evaluationError(ctx, node.p, 'Circle radius must be positive')
      const occurrence = invoke(node, ctx)
      const segments = semanticSegments(node, ctx, occurrence, 48, 3)
      return [ctx.builder.produce(occurrence, segments === null ? {
        kind: 'circle-analytic',
        valueType: semanticValueType(ctx.builder.languageContract, 'region', 'd2'),
        radius: parsedRadius,
      } : {
        kind: 'circle-polygonal',
        valueType: semanticValueType(ctx.builder.languageContract, 'region', 'd2'),
        radius: parsedRadius,
        radialSegments: segments,
      }, primitiveColor(ctx))]
    }
    case 'polygon': return makePolygon(node, ctx)
    case 'translate': {
      const vector = vectorValue(arg(node, 'v', 0, [0, 0, 0], ctx), ctx, node.p, 'translate vector')
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      const matrix = translationMatrix(vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0)
      return transformShapes(occurrence, shapes, () => matrix, ctx)
    }
    case 'rotate': {
      const angle = arg(node, 'a', 0, 0, ctx)
      const axis = arg(node, 'v', 1, undefined, ctx)
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      return transformShapes(occurrence, shapes, shape => {
        if (shape.dimension === 'region2') {
          const degrees = Array.isArray(angle)
            ? vectorValue(angle, ctx, node.p, 'rotation')[2] ?? 0
            : finiteNumber(angle, ctx, node.p, 'rotation')
          return axisAngleMatrixUnchecked([0, 0, 1], degrees)
        }
        if (Array.isArray(angle)) {
          const vector = vectorValue(angle, ctx, node.p, 'rotation')
          return eulerMatrix(vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0)
        }
        const degrees = finiteNumber(angle, ctx, node.p, 'rotation')
        return axis === undefined
          ? axisAngleMatrixUnchecked([0, 0, 1], degrees)
          : axisAngleMatrix(vectorValue(axis, ctx, node.p, 'rotation axis'), degrees, ctx, node.p)
      }, ctx)
    }
    case 'scale': {
      const raw = arg(node, 'v', 0, [1, 1, 1], ctx)
      const values = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'scale vector') : [finiteNumber(raw, ctx, node.p, 'scale')]
      const x = values[0] ?? 1, y = values[1] ?? x, z = values[2] ?? x
      if ([x, y, z].some(value => value === 0)) evaluationError(ctx, node.p, 'Scale values cannot be zero')
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      const matrix = scaleMatrix(x, y, z)
      return transformShapes(occurrence, shapes, () => matrix, ctx)
    }
    case 'mirror': {
      const vector = vectorValue(arg(node, 'v', 0, [1, 0, 0], ctx), ctx, node.p, 'mirror normal')
      const occurrence = invoke(node, ctx)
      return transformShapes(
        occurrence,
        frameChildren(node, node.children, occurrence, ctx),
        shape => mirrorMatrix(vector, shape.dimension, ctx, node.p),
        ctx,
      )
    }
    case 'multmatrix': {
      const matrix = matrixValue(arg(node, 'm', 0, undefined, ctx), ctx, node.p)
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      if (shapes.some(shape => shape.dimension !== 'solid3')) {
        if (ctx.builder.languageContract !== 'openscad-viewer/brep-1') {
          evaluationError(ctx, node.p, 'multmatrix currently supports 3D children only')
        }
        // A retained 2D profile has no implicit projection back from space.
        // Admit only matrices whose image of the entire XY plane is still XY;
        // the backend separately checks singularity and retained curve kinds.
        if (matrix[2] !== 0 || matrix[6] !== 0 || matrix[14] !== 0) {
          evaluationError(ctx, node.p, '2D B-rep multmatrix must preserve the XY plane')
        }
      }
      return transformShapes(occurrence, shapes, () => matrix, ctx)
    }
    case 'color': {
      const color = parseColor(arg(node, 'c', 0, [0.5, 0.5, 0.5], ctx), ctx, node.p)
      const alpha = arg(node, 'alpha', 1, undefined, ctx)
      if (alpha !== undefined) color[3] = clamp01(finiteNumber(alpha, ctx, node.p, 'color alpha'))
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx).map(shape => ({ ...shape, color: [...color] as RGBA }))
      return ctx.builder.alias(occurrence, shapes, false)
    }
    case 'union':
    case 'intersection':
    case 'hull': {
      const occurrence = invoke(node, ctx)
      return combineShapes(occurrence, frameChildren(node, node.children, occurrence, ctx), node.name, ctx, node.p)
    }
    case 'difference': {
      const occurrence = invoke(node, ctx)
      const framed = frameContext(node, occurrence, ctx)
      // The pinned evaluator completes the first-child value (including its
      // implicit union) before it starts evaluating any cutter statement.
      // Keep those two scoped evalNodes calls separate: kernel failure in the
      // base must win over a later cutter-language diagnostic.
      const childContext = framed !== null && isStableProfile(ctx)
        ? enterStableStatementScope(node.children, framed)
        : null
      const baseShapes = framed === null || node.children.length === 0
        ? []
        : childContext === null
          ? evalNodes(node.children.slice(0, 1), framed)
          : evalPreparedNodes(node.children.slice(0, 1), childContext)
      const base = unionReduceInternal(baseShapes, ctx, node.p)
      const cutterShapes = framed === null
        ? []
        : childContext === null
          ? evalNodes(node.children.slice(1), framed)
          : evalPreparedNodes(node.children.slice(1), childContext)
      const cutters = unionReduceInternal(cutterShapes, ctx, node.p)
      if (base === null) {
        if (cutters !== null) {
          ctx.builder.discardedEffects.push({
            tag: 'legacy-difference-cutters',
            root: cutters.node,
            ownerOccurrence: occurrence,
          })
        }
        return []
      }
      if (cutters === null) {
        if (base.identityOccurrence !== null && baseShapes.length === 1) {
          return ctx.builder.alias(occurrence, baseShapes, false)
        }
        return ctx.builder.alias(occurrence, [{
          node: base.node,
          dimension: base.dimension,
          producerOccurrence: occurrence,
          identityOccurrence: occurrence,
          color: base.color,
        }], true)
      }
      if (base.dimension !== cutters.dimension) evaluationError(ctx, node.p, 'difference() cannot mix 2D and 3D children')
      const inputType = ctx.builder.nodes[base.node].valueType
      // Cutting a single solid can split it into multiple disconnected solids.
      // The Boolean carrier is therefore SolidSet even when its base is Solid.
      const valueType: SemanticValueType = {
        ...inputType,
        geometryKind: inputType.space === 'd2' ? 'region' : 'solid-set',
      }
      return [ctx.builder.produce(occurrence, {
        kind: 'boolean', operation: 'difference', valueType,
        inputs: [base.node, cutters.node],
      }, base.color)]
    }
    case 'linear_extrude': {
      const height = finiteNumber(arg(node, 'height', 0, 1, ctx), ctx, node.p, 'extrusion height')
      if (height <= 0) evaluationError(ctx, node.p, 'linear_extrude() height must be positive')
      const twistDegrees = finiteNumber(arg(node, 'twist', -1, 0, ctx), ctx, node.p, 'extrusion twist')
      const rawSlices = Math.max(0, Math.trunc(finiteNumber(arg(node, 'slices', -1, 0, ctx), ctx, node.p, 'extrusion slices')))
      const slices = Math.min(MAX_EXTRUDE_SLICES, rawSlices)
      const rawScale = arg(node, 'scale', -1, [1, 1], ctx)
      const scaleValues = Array.isArray(rawScale) ? vectorValue(rawScale, ctx, node.p, 'extrusion scale') : [finiteNumber(rawScale, ctx, node.p, 'extrusion scale')]
      const scale = [scaleValues[0] ?? 1, scaleValues[1] ?? scaleValues[0] ?? 1] as const
      if (ctx.builder.languageContract === 'openscad-viewer/brep-1' && scale.some(value => value === 0)) {
        evaluationError(ctx, node.p, 'brep-1 extrusion scale cannot contain zero')
      }
      const center = arg(node, 'center', -1, false, ctx) === true
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      if (shapes.length === 0) return []
      if (shapes.some(shape => shape.dimension !== 'region2')) evaluationError(ctx, node.p, 'linear_extrude() requires 2D children')
      const profile = unionReduceInternal(shapes, ctx, node.p)!
      const inputType = ctx.builder.nodes[profile.node].valueType
      return [ctx.builder.produce(occurrence, {
        kind: 'linear-extrude',
        valueType: { ...inputType, geometryKind: 'solid-set', space: 'd3' },
        input: profile.node,
        height, twistDegrees, slices, scale, center,
      }, profile.color)]
    }
    case 'rotate_extrude': {
      const angleDegrees = finiteNumber(arg(node, 'angle', -1, 360, ctx), ctx, node.p, 'revolve angle')
      if (ctx.builder.languageContract === 'openscad-viewer/brep-1' && (angleDegrees <= 0 || angleDegrees > 360)) {
        evaluationError(ctx, node.p, 'brep-1 revolve angle must be in (0, 360] degrees')
      }
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      if (shapes.length === 0) return []
      if (shapes.some(shape => shape.dimension !== 'region2')) evaluationError(ctx, node.p, 'rotate_extrude() requires 2D children')
      const profile = unionReduceInternal(shapes, ctx, node.p)!
      const segments = semanticSegments(node, ctx, occurrence, 48, 3)
      const valueType = { ...ctx.builder.nodes[profile.node].valueType, geometryKind: 'solid-set' as const, space: 'd3' as const }
      return [ctx.builder.produce(occurrence, segments === null ? {
        kind: 'rotate-extrude-analytic', valueType, input: profile.node, angleDegrees,
      } : {
        kind: 'rotate-extrude-polygonal', valueType, input: profile.node, angleDegrees, radialSegments: segments,
      }, profile.color)]
    }
    case 'projection': {
      const cut = arg(node, 'cut', 0, false, ctx) === true
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      return shapes.map(shape => {
        if (shape.dimension !== 'solid3') evaluationError(ctx, node.p, 'projection() requires 3D children')
        return ctx.builder.produce(occurrence, {
          kind: 'projection',
          valueType: { ...nodeValueType(shape, ctx), geometryKind: 'region', space: 'd2' },
          input: shape.node,
          cut,
        }, shape.color)
      })
    }
    case 'offset': {
      const distance = finiteNumber(arg(node, 'r', 0, arg(node, 'delta', 0, 1, ctx), ctx), ctx, node.p, 'offset distance')
      const occurrence = invoke(node, ctx)
      const shapes = frameChildren(node, node.children, occurrence, ctx)
      return shapes.map(shape => {
        if (shape.dimension !== 'region2') evaluationError(ctx, node.p, 'offset() requires 2D children')
        return ctx.builder.produce(occurrence, {
          kind: 'offset', valueType: nodeValueType(shape, ctx), input: shape.node, distance,
        }, shape.color)
      })
    }
    case 'group':
    case 'render': {
      const occurrence = invoke(node, ctx)
      return frameChildren(node, node.children, occurrence, ctx)
    }
    case 'echo': {
      if (!isStableProfile(ctx)) {
        evaluationError(ctx, node.p, 'Unsupported geometry operation echo()')
      }
      const values = Object.entries(node.args).map(([name, expression]) => {
        const value = formatOpenScadValue(evalExpression(expression, ctx))
        return node.argKinds[name] === 'named' ? `${name} = ${value}` : value
      })
      const message = `ECHO:${values.length ? ` ${values.join(', ')}` : ''}`
      ctx.builder.warn(
        null,
        'W_OPENSCAD_ECHO',
        [],
        message,
        { start: node.p, end: node.end },
        false,
      )
      const viewportSlots = viewportSemanticSlots(node, ctx)
      return evalNodes(node.children, viewportSlots.length === 0
        ? ctx
        : { ...ctx, pendingSlots: [...ctx.pendingSlots, ...viewportSlots] })
    }
    case 'if': {
      const condition = arg(node, '_0', 0, false, ctx)
      const branch = isStableProfile(ctx) ? openScadTruthy(condition) : truthy(condition)
      const occurrence = invoke(node, ctx, [semanticSlot('$branch', branch)])
      return frameChildren(node, branch ? node.children : node.alternative, occurrence, ctx, !branch)
    }
    case 'let': {
      if (isStableProfile(ctx)) {
        const args = callExpressionArguments(node)
        const env = evaluateSequentialBindings(args, ctx, 0)
        const slots: SemanticDynamicSlot[] = []
        for (const argument of args) {
          if (argument.name !== undefined && env.has(argument.name)) {
            slots.push(semanticSlot(argument.name, env.get(argument.name)))
          }
        }
        const occurrence = invoke(node, ctx, slots)
        return frameChildren(node, node.children, occurrence, stableOverlayContext(ctx, env))
      }
      const env = new Map(ctx.env)
      const slots: SemanticDynamicSlot[] = []
      const bindings = Object.entries(node.args)
        .filter(([name]) => !name.startsWith('_'))
        .map(([name, expression]) => [name, evalExpression(expression, ctx)] as const)
      for (const [name, value] of bindings) {
        env.set(name, value)
        slots.push(semanticSlot(name, value))
      }
      const occurrence = invoke(node, ctx, slots)
      return frameChildren(node, node.children, occurrence, { ...ctx, env })
    }
    case 'for': {
      if (isStableProfile(ctx)) {
        const bindings = callExpressionArguments(node)
        const output: SemanticShape[] = []
        const visit = (
          bindingIndex: number,
          iterationContext: EvalContext,
          slots: readonly SemanticDynamicSlot[],
        ): void => {
          if (bindingIndex >= bindings.length) {
            const occurrence = invoke(node, ctx, slots)
            output.push(...frameChildren(node, node.children, occurrence, iterationContext))
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
            stableWarning(ctx, binding.p, 'Ignoring for() iterator without variable name')
            return
          }
          for (const value of values) {
            if (++ctx.budget.ops > MAX_EVAL_OPS) {
              evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
            }
            const env = new Map(iterationContext.env)
            env.set(binding.name, value)
            visit(
              bindingIndex + 1,
              stableOverlayContext(iterationContext, env),
              [...slots, semanticSlot(binding.name, value)],
            )
          }
        }
        visit(0, ctx, [])
        return output
      }
      const entries = Object.entries(node.args).filter(([name]) => !name.startsWith('_'))
      if (entries.length !== 1) evaluationError(ctx, node.p, 'for() currently requires one named iterator')
      const [name, expression] = entries[0]
      const values = evalExpression(expression, ctx)
      if (!Array.isArray(values)) evaluationError(ctx, node.p, 'for() iterator must be a vector or range')
      const output: SemanticShape[] = []
      for (const value of values) {
        if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
        const env = new Map(ctx.env)
        env.set(name, value)
        const occurrence = invoke(node, ctx, [semanticSlot(name, value)])
        output.push(...frameChildren(node, node.children, occurrence, { ...ctx, env }))
        if (output.length > MAX_SHAPES) evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
      }
      return output
    }
    case 'child': {
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'Unsupported geometry operation child()')
      stableCompatibilityDeprecation(node, ctx, 'children()')
      const authored = callExpressionArguments(node)
      const indexValue = authored[0] === undefined
        ? 0
        : evalExpression(authored[0].value, ctx)
      const childIndex = typeof indexValue === 'number' && Number.isFinite(indexValue)
        ? Math.trunc(indexValue)
        : 0
      const occurrence = invoke(node, ctx, [semanticSlot('$index', indexValue)])
      if (childIndex < 0) {
        stableWarning(
          ctx,
          node.p,
          `Negative child index (${childIndex}) not allowed`,
          'W_OPENSCAD_CHILD',
          false,
        )
        return []
      }
      const passed = ctx.callChildren
      if (!passed) return []
      const statement = passed.statements[childIndex]
      if (statement === undefined) {
        stableWarning(
          ctx,
          node.p,
          `Child index (${childIndex}) out of bounds (${passed.statements.length} children)`,
          'W_OPENSCAD_CHILD',
          false,
        )
        return []
      }
      return evalPassedCallChildren(node, ctx, occurrence, indexValue, passed, [statement])
    }
    case 'children': {
      const indexValue = arg(node, '_0', 0, undefined, ctx)
      const childIndex = indexValue === undefined
        ? null
        : Math.trunc(finiteNumber(indexValue, ctx, node.p, 'children index'))
      const occurrence = invoke(node, ctx, [semanticSlot('$index', indexValue)])
      const passed = ctx.callChildren
      if (!passed) return []
      let statements = passed.statements
      if (childIndex !== null) {
        statements = passed.statements[childIndex] ? [passed.statements[childIndex]] : []
      }
      return evalPassedCallChildren(node, ctx, occurrence, indexValue, passed, statements)
    }
  }

  if (isStableProfile(ctx)) {
    const declaration = ctx.stableScope?.moduleDeclaration(node.name)
    if (declaration !== undefined) return evalUserModule(node, declaration.node, ctx, declaration.scope)
  } else {
    const module = ctx.modules.get(node.name)
    if (module) return evalUserModule(node, module, ctx)
  }
  evaluationError(ctx, node.p, `Unsupported geometry operation ${node.name}()`)
}
