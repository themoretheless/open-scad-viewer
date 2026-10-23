import type { CadKernelHandle } from '../services/cadKernelOps'
import type { CallNode, ExpressionArgument, Value } from '../services/openscadCompiler'
import { formatOpenScadValue } from '../services/openScadValueSemantics'
import {
  resolveOpenScadFragments,
  type OpenScadFragmentResolutionInput,
} from '../services/openScadStableModuleSemantics'
import {
  OpenScadProjectError,
  resolveOpenScadProjectPath,
} from '../services/openScadProject'
import {
  loadPreparedOpenScadImport,
  OpenScadImportError,
  OpenScadImportPositionedError,
  type OpenScad2021LegacyImportFormat,
  type OpenScadImportErrorCode,
} from '../services/openScadImport'
import {
  loadOpenScadSurfaceHeightMap,
  OpenScadSurfaceDataError,
  OpenScadSurfaceError,
  triangulateOpenScadSurface,
} from '../services/openScadSurface'
import {
  OpenScadTextError,
  OpenScadTextPositionedError,
  renderOpenScadText,
  type OpenScadTextErrorCode,
  type OpenScadTextParameters,
} from '../services/openScadText'
import { nextColor } from '../services/openscadColors'
import {
  type EvalContext,
  type Shape,
  type Vec2,
} from './types'
import { callExpressionArguments } from './astDiscovery'
import {
  arg,
  compatibilityString,
  currentEntityId,
  evalExpression,
  evaluateStableBuiltinModuleArguments,
  resolveStableVariable,
  trackedSolid,
  warn,
} from './expressionEvaluator'
import { stableModuleSemanticsContext } from './geometryEvaluator'

export function surfaceCallSource(node: CallNode, ctx: EvalContext): {
  readonly source: string
  readonly sourcePath: string
} {
  const sourcePath = node.sourcePath ?? ctx.project?.entrypoint ?? '<inline>'
  const file = ctx.project?.read(sourcePath)
  return {
    source: file?.kind === 'source' ? file.source : ctx.source,
    sourcePath,
  }
}

const TEXT_POSITIONAL_PARAMETERS = Object.freeze([
  'text',
  'size',
  'font',
] as const)

const TEXT_NAMED_PARAMETERS = new Set([
  ...TEXT_POSITIONAL_PARAMETERS,
  'direction',
  'language',
  'script',
  'halign',
  'valign',
  'spacing',
  '$fn',
  '$fa',
  '$fs',
])

function textArgumentValues(node: CallNode, ctx: EvalContext): Map<string, Value> {
  const resolved = new Map<string, ExpressionArgument>()
  for (const argument of callExpressionArguments(node)) {
    if (argument.name === undefined) {
      const name = TEXT_POSITIONAL_PARAMETERS.find(parameter => !resolved.has(parameter))
      if (name === undefined) {
        warn(ctx, 'Ignoring excess positional argument')
        continue
      }
      resolved.set(name, argument)
      continue
    }
    if (!TEXT_NAMED_PARAMETERS.has(argument.name)) {
      warn(ctx, `Ignoring unknown argument ${argument.name}`)
      continue
    }
    if (resolved.has(argument.name)) warn(ctx, `Argument ${argument.name} was specified more than once`)
    resolved.set(argument.name, argument)
  }

  const values = new Map<string, Value>()
  for (const [name, argument] of resolved) values.set(name, evalExpression(argument.value, ctx))
  return values
}

function textCallError(
  node: CallNode,
  ctx: EvalContext,
  code: OpenScadTextErrorCode,
  message: string,
  details: Omit<ConstructorParameters<typeof OpenScadTextPositionedError>[4], 'sourcePath'> = {},
): never {
  const authored = surfaceCallSource(node, ctx)
  throw new OpenScadTextPositionedError(authored.source, node, code, message, {
    sourcePath: authored.sourcePath,
    ...details,
  })
}

function textStringParameter(
  values: ReadonlyMap<string, Value>,
  name: string,
  fallback: string,
  node: CallNode,
  ctx: EvalContext,
): string {
  const value = values.get(name) ?? fallback
  if (typeof value !== 'string') {
    return textCallError(node, ctx, 'E_TEXT_PARAMETER_INVALID', `text() ${name} must be a string.`)
  }
  return value
}

function textNumberParameter(
  value: Value,
  name: string,
  fallback: number,
  node: CallNode,
  ctx: EvalContext,
): number {
  const selected = value ?? fallback
  if (typeof selected !== 'number' || !Number.isFinite(selected)) {
    return textCallError(node, ctx, 'E_TEXT_PARAMETER_INVALID', `text() ${name} must be a finite number.`)
  }
  return selected
}

export function makeText(node: CallNode, ctx: EvalContext): Shape[] {
  if (ctx.project === undefined || ctx.textAssets === undefined) {
    return textCallError(
      node,
      ctx,
      'E_TEXT_PROJECT_REQUIRED',
      'text() requires an OpenSCAD project so fonts are resolved inside the bounded project VFS.',
    )
  }

  const authored = surfaceCallSource(node, ctx)
  const argumentContext = authored.source === ctx.source
    ? ctx
    : { ...ctx, source: authored.source }
  const values = textArgumentValues(node, argumentContext)
  const size = textNumberParameter(values.get('size'), 'size', 10, node, argumentContext)
  const textSpecial = (name: '$fn' | '$fa' | '$fs'): { present: boolean; value: Value } => {
    if (values.has(name)) return { present: true, value: values.get(name) }
    const inherited = resolveStableVariable(name, argumentContext)
    return { present: inherited.found, value: inherited.value }
  }
  const textFn = textSpecial('$fn'), textFa = textSpecial('$fa'), textFs = textSpecial('$fs')
  const textFragmentInput: OpenScadFragmentResolutionInput = {
    radius: Math.max(0, size),
    quality: argumentContext.quality,
    ...(textFn.present ? { fn: textFn.value } : {}),
    ...(textFa.present ? { fa: textFa.value } : {}),
    ...(textFs.present ? { fs: textFs.value } : {}),
  }
  const textFragments = resolveOpenScadFragments(
    textFragmentInput,
    stableModuleSemanticsContext(argumentContext),
  )
  if (textFragments.reduced) argumentContext.reduced.value = true
  const parameters: OpenScadTextParameters = {
    text: textStringParameter(values, 'text', '', node, argumentContext),
    size,
    font: textStringParameter(values, 'font', '', node, argumentContext),
    direction: textStringParameter(values, 'direction', '', node, argumentContext),
    language: textStringParameter(values, 'language', 'en', node, argumentContext),
    script: textStringParameter(values, 'script', '', node, argumentContext),
    halign: textStringParameter(values, 'halign', 'left', node, argumentContext),
    valign: textStringParameter(values, 'valign', 'baseline', node, argumentContext),
    spacing: textNumberParameter(values.get('spacing'), 'spacing', 1, node, argumentContext),
    // Passing the bounded integer as explicit $fn makes the text flattener a
    // consumer of the same 2021 fragment decision as geometric primitives.
    $fn: textFragments.fragments,
    $fa: textFragments.effectiveFa,
    $fs: textFragments.effectiveFs,
  }

  try {
    const layout = renderOpenScadText(
      ctx.project,
      ctx.textAssets,
      parameters,
      authored.sourcePath,
    )
    const sections: CadKernelHandle[] = []
    for (const glyph of layout.glyphs) {
      if (glyph.contours.length === 0) continue
      const section = ctx.kernel.polygon(
        glyph.contours.map(contour => contour.map(point => [point[0], point[1]] as Vec2)),
        'EvenOdd',
      )
      if (!ctx.kernel.isEmpty(section)) sections.push(section)
    }
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
    if (error instanceof OpenScadTextPositionedError) throw error
    if (error instanceof OpenScadTextError) {
      const { sourcePath: _sourcePath, ...details } = error.details
      return textCallError(node, ctx, error.code, error.message, details)
    }
    return textCallError(
      node,
      ctx,
      'E_TEXT_SHAPING_FAILED',
      `text() could not construct valid 2D glyph geometry: ${error instanceof Error ? error.message : String(error)}`,
      { font: parameters.font },
    )
  }
}

const IMPORT_POSITIONAL_PARAMETERS = Object.freeze([
  'file',
  'layer',
  'convexity',
  'origin',
  'scale',
] as const)

const IMPORT_NAMED_PARAMETERS = new Set([
  ...IMPORT_POSITIONAL_PARAMETERS,
  'width',
  'height',
  'center',
  'dpi',
  '$fn',
  '$fa',
  '$fs',
])

function importArgumentValues(node: CallNode, ctx: EvalContext): Map<string, Value> {
  const resolved = new Map<string, ExpressionArgument>()
  for (const argument of callExpressionArguments(node)) {
    if (argument.name === undefined) {
      const name = IMPORT_POSITIONAL_PARAMETERS.find(parameter => !resolved.has(parameter))
      if (name === undefined) {
        warn(ctx, 'Ignoring excess positional argument')
        continue
      }
      resolved.set(name, argument)
      continue
    }
    if (!IMPORT_NAMED_PARAMETERS.has(argument.name)) {
      warn(ctx, `Ignoring unknown argument ${argument.name}`)
      continue
    }
    if (resolved.has(argument.name)) warn(ctx, `Argument ${argument.name} was specified more than once`)
    resolved.set(argument.name, argument)
  }

  const values = new Map<string, Value>()
  for (const [name, argument] of resolved) values.set(name, evalExpression(argument.value, ctx))
  return values
}

export function importCallError(
  node: CallNode,
  ctx: EvalContext,
  code: OpenScadImportErrorCode,
  message: string,
  details: {
    readonly specifier?: string
    readonly assetPath?: string
    readonly format?: ConstructorParameters<typeof OpenScadImportPositionedError>[4]['format']
    readonly limit?: number
    readonly actual?: number
  } = {},
): never {
  const authored = surfaceCallSource(node, ctx)
  throw new OpenScadImportPositionedError(authored.source, node, code, message, {
    sourcePath: authored.sourcePath,
    specifier: details.specifier ?? '',
    ...(details.assetPath === undefined ? {} : { assetPath: details.assetPath }),
    ...(details.format === undefined ? {} : { format: details.format }),
    ...(details.limit === undefined ? {} : { limit: details.limit }),
    ...(details.actual === undefined ? {} : { actual: details.actual }),
  })
}

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

function surfaceError(
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
