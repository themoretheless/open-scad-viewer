import type { CallNode, Value } from '../openscadCompiler'
import { MAX_EXTRUDE_SLICES, MAX_SHAPES, YIELD_EVERY_STATEMENTS, stableIterable } from '../openscadParser'
import type { EvalContext, Shape, Vec2 } from '../openscadParser'
import { MAX_EVAL_OPS, evaluationError, isStableProfile, stableOverlayContext } from '../openScadStableExpressionEval'
import { OpenScadImportError, OpenScadImportPositionedError, loadPreparedOpenScadImport } from '../openScadImport'
import { arg, callExpressionArguments, evalNodes, finiteNumber, vectorValue } from './evalBuiltin'
import { booleanShapes } from './transformStableChildren'
import { compatibilityString, currentEntityId, evalExpression, evaluateStableBuiltinModuleArguments, evaluateStableCompatibilityArguments, trackedSolid, warn } from './isExclusiveView'
import { formatOpenScadValue } from '../openScadValueSemantics'
import { identityValue } from './warmGeometryKernel'
import { importCallError } from './surfaceCallSource'
import { nextColor } from '../openscadColors'
import { resolveOpenScadLinearExtrude, resolveOpenScadRotateExtrude } from '../openScadStableGeometrySemantics'
import { resolveOpenScadProjectPath } from '../openScadProject'
import { segments, stableFragmentInputFromEvaluated, stableGeometrySemanticsContext } from './requireStableProfile'

export function compatibilityDxfProfile(
  node: CallNode,
  ctx: EvalContext,
  values: ReadonlyMap<string, Value>,
  fileValue: string,
  importScale: number,
): Shape[] {
  if (ctx.project === undefined || ctx.importAssets === undefined) {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_PROJECT_REQUIRED',
      `${node.name}() file mode requires an OpenSCAD project inside the bounded project VFS.`,
      { specifier: fileValue, format: 'dxf' },
    )
  }
  if (!Number.isFinite(importScale) || importScale <= 0) return []

  const rawOrigin = values.get('origin')
  let origin: readonly [number, number] | undefined
  if (rawOrigin !== undefined) {
    if (Array.isArray(rawOrigin) && rawOrigin.length === 2
      && rawOrigin.every(value => typeof value === 'number' && Number.isFinite(value))) {
      origin = [rawOrigin[0] as number, rawOrigin[1] as number]
    } else {
      warn(ctx, `${node.name}(..., origin=${formatOpenScadValue(rawOrigin)}) could not be converted`)
    }
  }
  const layer = compatibilityString(values.get('layer'))
  const importer = node.sourcePath ?? ctx.project.entrypoint
  let assetPath: string | undefined
  try {
    const loaded = loadPreparedOpenScadImport(
      ctx.project,
      ctx.importAssets,
      importer,
      fileValue,
      {
        forcedFormat: 'dxf',
        layer,
        ...(origin === undefined ? {} : { origin }),
        scale: importScale,
      },
    )
    assetPath = resolveOpenScadProjectPath(importer, fileValue)
    if (loaded.dimension !== 2) {
      return importCallError(
        node,
        ctx,
        'E_IMPORT_INVALID_DATA',
        `${node.name}() file mode requires 2D DXF geometry.`,
        { specifier: fileValue, assetPath, format: 'dxf' },
      )
    }
    const sections = loaded.regions.map(region => ctx.kernel.polygon(
      region.contours.map(contour => contour.map(point => [point[0], point[1]] as Vec2)),
      region.fillRule === 'evenodd' ? 'EvenOdd' : 'NonZero',
    ))
    if (sections.length === 0) return []
    const geometry = sections.length === 1
      ? sections[0]
      : ctx.kernel.boolean2('union', sections)
    if (ctx.kernel.isEmpty(geometry)) return []
    return [{
      dimension: 2,
      geometry,
      color: nextColor(),
      entityId: currentEntityId(ctx),
    }]
  } catch (error) {
    if (error instanceof OpenScadImportPositionedError) throw error
    if (error instanceof OpenScadImportError) {
      return importCallError(node, ctx, error.code, error.message, {
        specifier: error.specifier,
        ...(error.assetPath === undefined ? {} : { assetPath: error.assetPath }),
        ...(error.format === undefined ? {} : { format: error.format }),
        ...(error.limit === undefined ? {} : { limit: error.limit }),
        ...(error.actual === undefined ? {} : { actual: error.actual }),
      })
    }
    return importCallError(
      node,
      ctx,
      'E_IMPORT_INVALID_DATA',
      `${node.name}() could not construct its DXF profile: ${error instanceof Error ? error.message : String(error)}`,
      { specifier: fileValue, ...(assetPath === undefined ? {} : { assetPath }), format: 'dxf' },
    )
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

export async function linearExtrude(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
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

export async function dxfLinearExtrude(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
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

export function stableRotateExtrudeSections(
  node: CallNode,
  ctx: EvalContext,
  evaluated: ReadonlyMap<string, Value>,
  sections: Shape[],
): Shape[] {
  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, `${node.name}() requires 2D children`)
  const bounds = ctx.kernel.bounds(sections[0].geometry)
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

export async function dxfRotateExtrude(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
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

export async function rotateExtrude(node: CallNode, ctx: EvalContext): Promise<Shape[]> {
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

export async function evalFor(node: CallNode, ctx: EvalContext, diagnosticName: 'for' | 'intersection_for'): Promise<Shape[]> {
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
