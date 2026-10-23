import {
  deriveSemanticCapabilityClosure,
  SEMANTIC_PROGRAM_CAPABILITY_GRAPH,
  SEMANTIC_PROGRAM_EXECUTION_FEATURE,
  SEMANTIC_PROGRAM_IDENTITY,
  type SemanticDimension,
  type SemanticDynamicSlot,
  type SemanticMatrix4,
  type SemanticProgramCoreV1,
  type SemanticValueType,
} from '../core/semanticProgram'
import { clamp01 } from './math3d'
import { parseGeometrySourceRoutingHeader } from '../core/geometryRouting'
import {
  compileOpenSCAD,
  hasOpenScadViewportModifier,
  type CallNode,
  type ExpressionArgument,
  type FunctionNode,
  type ModuleNode,
  type Statement,
  type Value,
} from './openscadCompiler'
import { OpenSCADParseError } from './openscadErrors'
import {
  formatOpenScadValue,
  openScadTruthy,
} from './openScadValueSemantics'
import { OpenScadStableScope } from './openScadStableScope'
import {
  MAX_EVAL_DEPTH,
  MAX_EVAL_OPS,
  compactDiagnosticText,
  evaluationError,
  isStableProfile,
  overlayDynamicVariables,
  stableOverlayContext,
} from './openScadStableExpressionEval'
import { createOpenScadStableRuntimeVariables } from './openScadStableRuntime'
import {
  attestSemanticProgram,
  encodeSemanticProgram,
} from './semanticProgramCodec'
import { normalizeSemanticProgram, semanticSourceDescriptor } from './semanticProgramValidator'
import {
  type SemanticLoweringReport,
  type SemanticLoweringSuccess,
} from './semanticProgramTrust'
import {
  canonicalizeCompletedDag,
  canonicalNumber,
  countStatements,
  LegacyBindingCompatibilityError,
  MAX_EXTRUDE_SLICES,
  MAX_SHAPES,
  registerOperations,
  resultFromShapes,
  SemanticProgramBuilder,
  semanticValueType,
  StableViewportRootSelection,
  type EvalContext,
  type PassedCallChildren,
  type RGBA,
  type SemanticLoweringOptions,
  type SemanticShape,
} from '../semantic/lowererBuilder'
import {
  arg,
  bindAssertArguments,
  callExpressionArguments,
  collectFunctions,
  collectModules,
  detachedChildrenContinuationMap,
  enterStableStatementScope,
  evalExpression,
  evaluateSequentialBindings,
  finiteNumber,
  poll,
  resolveStableExpressionArguments,
  semanticProjectAssetRequired,
  stableCompatibilityDeprecation,
  stableIterable,
  stableWarning,
  truthy,
  valueToString,
  vectorValue,
} from '../semantic/lowererExpressionEval'
import {
  combineShapes,
  evalDxfLinearExtrude,
  evalDxfRotateExtrude,
  invoke,
  makeCylinder,
  makePolygon,
  makePolyhedron,
  matrixValue,
  multiplyMatrix,
  nodeValueType,
  parseColor,
  primitiveColor,
  scaleMatrix,
  semanticSegments,
  semanticSlot,
  transformShapes,
  translationMatrix,
  unionReduceInternal,
  viewportRootActivates,
  viewportSemanticSlots,
} from '../semantic/lowererGeometryOps'

export {
  type SemanticLoweringOptions,
  type SemanticLoweringReport,
  type SemanticLoweringSuccess,
}

function evalNodes(nodes: readonly Statement[], parent: EvalContext, scoped = true): SemanticShape[] {
  const ctx: EvalContext = isStableProfile(parent)
    ? enterStableStatementScope(nodes, parent)
    : { ...parent, env: scoped ? new Map(parent.env) : parent.env }
  return evalPreparedNodes(nodes, ctx)
}

function evalPreparedNodes(nodes: readonly Statement[], ctx: EvalContext): SemanticShape[] {
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

function frameChildren(
  owner: CallNode | ModuleNode,
  statements: readonly Statement[],
  ownerOccurrence: number,
  ctx: EvalContext,
  alternative = false,
): SemanticShape[] {
  const framed = frameContext(owner, ownerOccurrence, ctx, alternative)
  return framed ? evalNodes(statements, framed) : []
}

function frameContext(
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

function axisAngleMatrix(axis: readonly number[], degrees: number, ctx: EvalContext, position: number): SemanticMatrix4 {
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

function eulerMatrix(x: number, y: number, z: number): SemanticMatrix4 {
  const rx = axisAngleMatrixUnchecked([1, 0, 0], x)
  const ry = axisAngleMatrixUnchecked([0, 1, 0], y)
  const rz = axisAngleMatrixUnchecked([0, 0, 1], z)
  return multiplyMatrix(rz, multiplyMatrix(ry, rx))
}

function axisAngleMatrixUnchecked(axis: readonly [number, number, number], degrees: number): SemanticMatrix4 {
  const angle = degrees * Math.PI / 180, c = Math.cos(angle), s = Math.sin(angle), t = 1 - c
  const [x, y, z] = axis
  return [
    canonicalNumber(t*x*x+c), canonicalNumber(t*x*y+s*z), canonicalNumber(t*x*z-s*y), 0,
    canonicalNumber(t*x*y-s*z), canonicalNumber(t*y*y+c), canonicalNumber(t*y*z+s*x), 0,
    canonicalNumber(t*x*z+s*y), canonicalNumber(t*y*z-s*x), canonicalNumber(t*z*z+c), 0,
    0, 0, 0, 1,
  ]
}

function mirrorMatrix(
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

function evalPassedCallChildren(
  node: CallNode,
  ctx: EvalContext,
  occurrence: number,
  indexValue: Value,
  passed: Readonly<PassedCallChildren>,
  statements: readonly Statement[],
): SemanticShape[] {
  if (statements.length === 0) return []
  const currentOperation = ctx.builder.registered.byStatement.get(node)
  const expansionOperation = ctx.builder.registered.expansionByCall.get(node)
    ?? passed.expansionOperation
  const expansionPath = ctx.builder.registered.operations[expansionOperation].structuralPath
  const currentPath = currentOperation === undefined
    ? []
    : ctx.builder.registered.operations[currentOperation].structuralPath
  const legacyReentry = !isStableProfile(ctx)
    && ctx.builder.languageContract === 'legacy/current'
    && expansionPath.length < currentPath.length
    && expansionPath.every((segment, index) => {
      const candidate = currentPath[index]
      return segment.kind === candidate.kind
        && segment.name === candidate.name
        && segment.ordinal === candidate.ordinal
    })
  const expansion = legacyReentry
    ? occurrence
    : ctx.builder.invocationOperation(
      expansionOperation,
      occurrence,
      [semanticSlot('$index', indexValue)],
    )
  return evalNodes(statements, {
    ...ctx,
    env: new Map(passed.env),
    stableScope: isStableProfile(ctx) ? passed.scope ?? ctx.stableScope : ctx.stableScope,
    scopeVisibleBefore: Number.POSITIVE_INFINITY,
    parentOccurrence: expansion,
    pendingSlots: [],
    callChildren: !isStableProfile(ctx) && ctx.builder.languageContract === 'legacy/current'
      ? passed
      : passed.continuation,
  })
}

function evalNode(node: CallNode, parent: EvalContext): SemanticShape[] {
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
      return evalDxfLinearExtrude(node, ctx, frameChildren)
    case 'dxf_rotate_extrude':
      if (!isStableProfile(ctx)) evaluationError(ctx, node.p, 'Unsupported geometry operation dxf_rotate_extrude()')
      return evalDxfRotateExtrude(node, ctx, frameChildren)
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

function evalUserModule(
  call: CallNode,
  module: ModuleNode,
  ctx: EvalContext,
  definitionScope?: OpenScadStableScope,
): SemanticShape[] {
  let env = new Map(ctx.env)
  const slots: SemanticDynamicSlot[] = []
  let moduleContext = ctx
  if (isStableProfile(ctx)) {
    if (definitionScope === undefined) {
      evaluationError(ctx, call.p, `Stable module ${module.name}() is missing its lexical scope`)
    }
    const moduleStack = [...ctx.moduleStack, module.name]
    const definitionEnv = new Map(definitionScope.env)
    overlayDynamicVariables(definitionEnv, ctx.env)
    definitionEnv.set('$children', call.children.length)
    definitionEnv.set('$parent_modules', moduleStack.length)
    moduleContext = {
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
          ? evalExpression(parameter.defaultValue, { ...moduleContext, env: new Map(definitionEnv) })
          : undefined
      parameterValues.set(parameter.name, value)
    }
    env = new Map(definitionEnv)
    for (const parameter of module.params) {
      const value = parameterValues.get(parameter.name)
      env.set(parameter.name, value)
      slots.push(semanticSlot(parameter.name, value))
    }
  } else {
    const positional = Object.entries(call.args)
      .filter(([name]) => name.startsWith('_'))
      .sort(([left], [right]) => Number(left.slice(1)) - Number(right.slice(1)))
    for (let index = 0; index < module.params.length; index++) {
      const parameter = module.params[index]
      const expression = call.args[parameter.name] ?? positional[index]?.[1] ?? parameter.defaultValue
      const value = expression ? evalExpression(expression, { ...ctx, env }) : undefined
      env.set(parameter.name, value)
      slots.push(semanticSlot(parameter.name, value))
    }
  }
  const callOccurrence = invoke(call, ctx, slots)
  const bodyOperation = ctx.builder.registered.bodyByStatement.get(call)
  const expansionOperation = ctx.builder.registered.expansionByCall.get(call)
  if (bodyOperation === undefined || expansionOperation === undefined) {
    evaluationError(ctx, call.p, 'User-module call is missing its semantic expansion frame')
  }
  const bodyOccurrence = ctx.builder.invocationOperation(bodyOperation, callOccurrence)
  const definitionOccurrence = ctx.builder.invocation(module, bodyOccurrence, slots)
  return frameChildren(module, module.children, definitionOccurrence, {
    ...moduleContext,
    env,
    moduleStack: isStableProfile(ctx) ? moduleContext.moduleStack : [...ctx.moduleStack, module.name],
    parentOccurrence: definitionOccurrence,
    pendingSlots: [],
    callChildren: {
      owner: call,
      bodyOccurrence,
      expansionOperation,
      statements: call.children,
      env: new Map(ctx.env),
      scope: isStableProfile(ctx) ? ctx.stableScope : undefined,
      continuation: ctx.callChildren,
    },
  })
}

/** @internal Use the trust-owning semanticProgramLowerer facade. */
export function lowerOpenSCADToSemanticProgramUnchecked(
  source: string,
  options: SemanticLoweringOptions = {},
): SemanticLoweringSuccess {
  const route = parseGeometrySourceRoutingHeader(source)
  const ast = compileOpenSCAD(source, { languageProfile: options.languageProfile })
  const modules = new Map<string, ModuleNode>()
  const functions = new Map<string, FunctionNode>()
  collectModules(ast, modules)
  collectFunctions(ast, functions)
  const moduleNames = new Set(modules.keys())
  const languageProfile = options.languageProfile ?? 'openscad-viewer-subset@1'
  const registered = registerOperations(
    ast,
    moduleNames,
    languageProfile,
  )
  let builder = new SemanticProgramBuilder(source, route.languageContract, registered)
  const budget = { ops: 0 }
  const reduced = { value: false }
  const valueBudget = { used: 0 }
  const quality = options.quality ?? 'full'
  const env = languageProfile === 'openscad/stable-2021.01'
    ? new Map<string, Value>(Array.from(
        createOpenScadStableRuntimeVariables({ quality, animationTime: options.animationTime }),
        ([name, value]) => [name, Array.isArray(value) ? [...value] : value as Value],
      ))
    : new Map<string, Value>([['$fn', 0], ['$fa', 12], ['$fs', 2]])
  const context: EvalContext = {
    source,
    languageProfile,
    env,
    functions,
    modules,
    builder,
    quality,
    depth: 0,
    functionStack: [],
    moduleStack: [],
    budget,
    reduced,
    valueBudget,
    valueWeights: new WeakMap(),
    valueDepths: new WeakMap(),
    parentOccurrence: null,
    pendingSlots: [],
    shouldAbort: options.shouldAbort,
  }
  let shapes: SemanticShape[] = []
  let terminalError: OpenSCADParseError | LegacyBindingCompatibilityError | null = null
  let terminalDiagnostic: number | null = null
  const captureFailure = (error: unknown): void => {
    const capturable = error instanceof OpenSCADParseError
      || error instanceof LegacyBindingCompatibilityError
    if (!options.captureTerminalFailure || !capturable) throw error
    if (builder.terminalOccurrence === null) {
      throw new Error('Capturable semantic failure has no interrupted occurrence')
    }
    terminalError = error
    terminalDiagnostic = builder.terminal(error, builder.terminalOccurrence)
  }
  try {
    shapes = evalNodes(ast, context, false)
  } catch (error) {
    if (error instanceof StableViewportRootSelection) {
      const detachedContinuations = detachedChildrenContinuationMap(
        error.node,
        error.context.callChildren,
      )
      const rootBuilder = new SemanticProgramBuilder(
        source,
        route.languageContract,
        registerOperations(ast, moduleNames, languageProfile, error.node, detachedContinuations),
      )
      rootBuilder.inheritDiagnostics(builder, error.diagnosticsBeforeCandidate)
      builder = rootBuilder
      try {
        shapes = evalPreparedNodes([error.node], {
          ...error.context,
          builder,
          parentOccurrence: null,
          pendingSlots: [],
          viewportRootLocked: true,
          viewportRootOwner: error.node,
        })
      } catch (rootError) {
        captureFailure(rootError)
      }
    } else {
      captureFailure(error)
    }
  }
  const canonical = canonicalizeCompletedDag(
    builder,
    terminalError === null ? resultFromShapes(shapes) : { tag: 'empty', type: 'never' },
    terminalDiagnostic,
    builder.terminalOccurrence,
  )
  const coreDraft: SemanticProgramCoreV1 = {
    schema: 'semantic-program-core',
    schemaVersion: { major: 1, minor: 2 },
    requiredFeatures: [SEMANTIC_PROGRAM_EXECUTION_FEATURE],
    identityVersion: SEMANTIC_PROGRAM_IDENTITY,
    language: {
      contract: route.languageContract,
      semanticsRevision: route.languageContract === 'legacy/current' ? '1.0.0' : 'brep-1.0.0',
      capabilityGraphVersion: SEMANTIC_PROGRAM_CAPABILITY_GRAPH,
    },
    units: {
      length: 'millimeter', angle: 'degree', handedness: 'right', upAxis: 'z',
      matrixLayout: 'column-major', composition: 'parent-times-local',
    },
    operations: builder.registered.operations,
    occurrences: builder.occurrences,
    nodes: builder.nodes,
    execution: canonical.execution,
    result: canonical.result,
    declaredCapabilities: route.requiredCapabilities,
    capabilityClosure: [],
    diagnosticTemplates: builder.diagnosticTemplates,
  }
  const core: SemanticProgramCoreV1 = {
    ...coreDraft,
    capabilityClosure: deriveSemanticCapabilityClosure(coreDraft),
  }
  const envelope = {
    schema: 'semantic-program-envelope' as const,
    schemaVersion: { major: 1 as const, minor: 2 as const },
    source: semanticSourceDescriptor(source),
    core,
    provenance: builder.registered.provenance,
    tessellationIntents: [...builder.tessellationIntents].sort((left, right) => left.occurrence - right.occurrence),
    diagnostics: builder.diagnostics,
  }
  const program = normalizeSemanticProgram(envelope, source)
  const canonicalSnapshot = encodeSemanticProgram(program)
  const attestation = attestSemanticProgram(program)
  const report = Object.freeze({
    astStatements: countStatements(ast),
    operations: program.core.operations.length,
    occurrences: program.core.occurrences.length,
    nodes: program.core.nodes.length,
    evaluationSteps: budget.ops,
    allocatedValueUnits: valueBudget.used,
  })
  if (terminalError !== null) Object.freeze(terminalError)
  const success: SemanticLoweringSuccess = Object.freeze({
    tag: 'success' as const,
    program,
    sourceText: source,
    get canonicalBytes() { return new Uint8Array(canonicalSnapshot) },
    attestation,
    warnings: Object.freeze([...builder.warnings]),
    fullEquivalent: !reduced.value,
    terminalError,
    report,
  })
  return success
}
