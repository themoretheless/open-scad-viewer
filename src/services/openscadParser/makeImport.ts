import type { CallNode } from '../openscadCompiler'
import { IMPORT_POSITIONAL_PARAMETERS } from '../openscadParser'
import type { EvalContext, Shape, Vec2 } from '../openscadParser'
import { OpenScadImportError, OpenScadImportPositionedError, loadPreparedOpenScadImport } from '../openScadImport'
import type { OpenScad2021LegacyImportFormat } from '../openScadImport'
import { OpenScadProjectError, resolveOpenScadProjectPath } from '../openScadProject'
import { resolveOpenScadMirror, resolveOpenScadMultmatrix, resolveOpenScadRotate, resolveOpenScadScale, resolveOpenScadTranslate } from '../openScadStableTransformSemantics'
import type { OpenScadStableTransformPlan } from '../openScadStableTransformSemantics'
import { OpenScadSurfaceDataError, OpenScadSurfaceError, loadOpenScadSurfaceHeightMap, triangulateOpenScadSurface } from '../openScadSurface'
import { arg, vectorValue } from './evalBuiltin'
import { compatibilityString, currentEntityId, evalExpression, evaluateStableBuiltinModuleArguments, trackedSolid, warn } from './isExclusiveView'
import { evaluationError } from '../openScadStableExpressionEval'
import { formatOpenScadValue } from '../openScadValueSemantics'
import { importArgumentValues, importCallError, surfaceCallSource } from './surfaceCallSource'
import { nextColor } from '../openscadColors'
import { stableTransformSemanticsContext } from './requireStableProfile'

export function makeImport(
  node: CallNode,
  ctx: EvalContext,
  forcedFormat?: OpenScad2021LegacyImportFormat,
): Shape[] {
  const displayName = node.name
  if (ctx.project === undefined || ctx.importAssets === undefined) {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_PROJECT_REQUIRED',
      `${displayName}() requires an OpenSCAD project so its file is resolved inside the bounded project VFS.`,
    )
  }

  const authored = surfaceCallSource(node, ctx)
  const argumentContext = authored.source === ctx.source
    ? ctx
    : { ...ctx, source: authored.source }
  const compatibility = forcedFormat !== undefined
  const values = compatibility
    ? evaluateStableBuiltinModuleArguments(
        node,
        IMPORT_POSITIONAL_PARAMETERS,
        ['width', 'height', 'filename', 'layername', 'center', 'dpi', '$fn', '$fa', '$fs'],
        argumentContext,
      )
    : importArgumentValues(node, argumentContext)
  const rawFileValue = values.get('file') ?? values.get('filename')
  const fileValue = compatibility ? compatibilityString(rawFileValue) : rawFileValue
  if (typeof fileValue !== 'string' || fileValue.length === 0) {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_FILE_REQUIRED',
      `${displayName}() requires a non-empty string file path.`,
    )
  }

  const rawLayerValue = values.get('layer') ?? values.get('layername')
  const layerValue = compatibility && rawLayerValue !== undefined
    ? compatibilityString(rawLayerValue)
    : rawLayerValue
  if (layerValue !== undefined && typeof layerValue !== 'string') {
    return importCallError(node, ctx, 'E_IMPORT_ARGUMENT_INVALID', `${displayName}() layer must be a string.`, {
      specifier: fileValue,
    })
  }
  const originValue = values.get('origin')
  let origin: [number, number] | undefined
  if (originValue !== undefined) {
    if (!Array.isArray(originValue) || originValue.length !== 2
      || originValue.some(value => typeof value !== 'number' || !Number.isFinite(value))) {
      if (compatibility) {
        warn(ctx, `linear_extrude(..., origin=${formatOpenScadValue(originValue)}) could not be converted`)
      } else {
      return importCallError(
        node,
        ctx,
        'E_IMPORT_ARGUMENT_INVALID',
        `${displayName}() origin must contain two finite numbers.`,
        { specifier: fileValue },
      )
      }
    } else {
      origin = [originValue[0] as number, originValue[1] as number]
    }
  }
  const rawScaleValue = values.get('scale')
  const scaleValue = compatibility
    ? (typeof rawScaleValue === 'number' && Number.isFinite(rawScaleValue) && rawScaleValue > 0
        ? rawScaleValue
        : 1)
    : rawScaleValue
  if (!compatibility && scaleValue !== undefined
    && (typeof scaleValue !== 'number' || !Number.isFinite(scaleValue) || scaleValue <= 0)) {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_ARGUMENT_INVALID',
      `${displayName}() scale must be a positive finite number.`,
      { specifier: fileValue },
    )
  }
  const rawCenterValue = values.get('center')
  const centerValue = compatibility ? rawCenterValue === true : rawCenterValue
  if (!compatibility && centerValue !== undefined && typeof centerValue !== 'boolean') {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_ARGUMENT_INVALID',
      `${displayName}() center must be boolean.`,
      { specifier: fileValue },
    )
  }
  const rawDpiValue = values.get('dpi')
  const dpiValue = compatibility
    ? (typeof rawDpiValue === 'number' && Number.isFinite(rawDpiValue) && rawDpiValue >= 0.001
        ? rawDpiValue
        : undefined)
    : rawDpiValue
  if (compatibility && typeof rawDpiValue === 'number'
    && (!Number.isFinite(rawDpiValue) || rawDpiValue < 0.001)) {
    warn(ctx, 'Invalid dpi value given; using the default 72 dpi')
  }
  if (!compatibility && dpiValue !== undefined
    && (typeof dpiValue !== 'number' || !Number.isFinite(dpiValue) || dpiValue < 0.001)) {
    return importCallError(
      node,
      ctx,
      'E_IMPORT_ARGUMENT_INVALID',
      `${displayName}() dpi must be a finite number greater than or equal to 0.001.`,
      { specifier: fileValue },
    )
  }

  // These 2021.01 parameters are evaluated even though they only affect the
  // legacy preview or are inert in the corresponding upstream importer.
  values.get('convexity')
  values.get('width')
  values.get('height')
  values.get('$fn')
  values.get('$fa')
  values.get('$fs')

  const importer = node.sourcePath ?? ctx.project.entrypoint
  const loadedScale = typeof scaleValue === 'number' ? scaleValue : undefined
  const loadedCenter = typeof centerValue === 'boolean' ? centerValue : undefined
  const loadedDpi = typeof dpiValue === 'number' ? dpiValue : undefined
  let assetPath: string | undefined
  try {
    const loaded = loadPreparedOpenScadImport(
      ctx.project,
      ctx.importAssets,
      importer,
      fileValue,
      {
        ...(layerValue === undefined ? {} : { layer: layerValue }),
        ...(origin === undefined ? {} : { origin }),
        ...(loadedScale === undefined ? {} : { scale: loadedScale }),
        ...(loadedCenter === undefined ? {} : { center: loadedCenter }),
        ...(loadedDpi === undefined ? {} : { dpi: loadedDpi }),
        ...(forcedFormat === undefined ? {} : { forcedFormat }),
      },
    )
    assetPath = resolveOpenScadProjectPath(importer, fileValue)

    if (loaded.dimension === 2) {
      const sections = loaded.regions.map(region => ctx.kernel.polygon(
        region.contours.map(contour => contour.map(point => [point[0], point[1]] as Vec2)),
        region.fillRule === 'evenodd' ? 'EvenOdd' : 'NonZero',
      ))
      const geometry = sections.length === 1
        ? sections[0]
        : ctx.kernel.boolean2('union', sections)
      if (ctx.kernel.isEmpty(geometry)) {
        return importCallError(
          node,
          ctx,
          'E_IMPORT_EMPTY',
          `${displayName}() produced empty 2D geometry.`,
          { specifier: fileValue, assetPath, format: loaded.format },
        )
      }
      return [{
        dimension: 2,
        geometry,
        color: nextColor(),
        entityId: currentEntityId(ctx),
      }]
    }

    const geometry = ctx.kernel.ofMesh(loaded.vertices, loaded.triangles)
    if (ctx.kernel.isEmpty(geometry)) {
      return importCallError(
        node,
        ctx,
        'E_IMPORT_EMPTY',
        `${displayName}() produced empty 3D geometry.`,
        { specifier: fileValue, assetPath, format: loaded.format },
      )
    }
    return [trackedSolid(geometry, nextColor(), node, ctx)]
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
      'E_IMPORT_NON_MANIFOLD',
      `${displayName}() could not construct geometry: ${error instanceof Error ? error.message : String(error)}`,
      { specifier: fileValue, ...(assetPath === undefined ? {} : { assetPath }) },
    )
  }
}

export function surfaceError(
  node: CallNode,
  ctx: EvalContext,
  code: ConstructorParameters<typeof OpenScadSurfaceError>[2],
  message: string,
  details: Omit<ConstructorParameters<typeof OpenScadSurfaceError>[4], 'sourcePath'> = {},
): never {
  const authored = surfaceCallSource(node, ctx)
  throw new OpenScadSurfaceError(authored.source, node, code, message, {
    sourcePath: authored.sourcePath,
    ...details,
  })
}

export function makeSurface(node: CallNode, ctx: EvalContext): Shape[] {
  if (ctx.project === undefined || ctx.surfaceAssets === undefined) {
    return surfaceError(
      node,
      ctx,
      'E_SURFACE_PROJECT_REQUIRED',
      'surface() requires an OpenSCAD project so its file is resolved inside the bounded project VFS.',
    )
  }

  const authored = surfaceCallSource(node, ctx)
  const argumentContext = authored.source === ctx.source
    ? ctx
    : { ...ctx, source: authored.source }
  const fileValue = arg(node, 'file', 0, undefined, argumentContext)
  if (typeof fileValue !== 'string' || fileValue.length === 0) {
    return surfaceError(
      node,
      ctx,
      'E_SURFACE_FILE_REQUIRED',
      'surface() requires a non-empty string file path.',
    )
  }
  const center = arg(node, 'center', 1, false, argumentContext) === true
  // OpenSCAD 2021 stores convexity as a display/preview hint; evaluating the
  // argument preserves side effects and errors, but it cannot change the mesh.
  arg(node, 'convexity', 2, 1, argumentContext)
  const invert = node.args.invert === undefined
    ? false
    : evalExpression(node.args.invert, argumentContext) === true
  const importer = node.sourcePath ?? ctx.project.entrypoint

  let assetPath: string | undefined
  try {
    const loaded = loadOpenScadSurfaceHeightMap(
      ctx.project,
      ctx.surfaceAssets,
      importer,
      fileValue,
      invert,
    )
    assetPath = loaded.path
    const surface = triangulateOpenScadSurface(loaded.map, center)
    const geometry = ctx.kernel.ofMesh(surface.vertices, surface.triangles)
    return [trackedSolid(geometry, nextColor(), node, ctx)]
  } catch (error) {
    if (error instanceof OpenScadSurfaceError) throw error
    if (error instanceof OpenScadProjectError) {
      return surfaceError(
        node,
        ctx,
        'E_SURFACE_PATH_INVALID',
        error.message,
        {
          specifier: fileValue,
          ...(error.details.path === undefined ? {} : { assetPath: error.details.path }),
          ...(error.details.limit === undefined ? {} : { limit: error.details.limit }),
          ...(error.details.actual === undefined ? {} : { actual: error.details.actual }),
        },
      )
    }
    if (error instanceof OpenScadSurfaceDataError) {
      return surfaceError(
        node,
        ctx,
        error.code,
        error.message,
        {
          specifier: fileValue,
          ...(error.assetPath === undefined && assetPath === undefined
            ? {}
            : { assetPath: error.assetPath ?? assetPath }),
          ...(error.limit === undefined ? {} : { limit: error.limit }),
          ...(error.actual === undefined ? {} : { actual: error.actual }),
        },
      )
    }
    return surfaceError(
      node,
      ctx,
      'E_SURFACE_NON_MANIFOLD',
      `surface() could not construct manifold geometry: ${error instanceof Error ? error.message : String(error)}`,
      { specifier: fileValue, ...(assetPath === undefined ? {} : { assetPath }) },
    )
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
