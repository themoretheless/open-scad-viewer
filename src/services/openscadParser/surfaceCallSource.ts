import type { CadKernelHandle } from '../cadKernelOps'
import type { CallNode, ExpressionArgument, Value } from '../openscadCompiler'
import { IMPORT_NAMED_PARAMETERS, IMPORT_POSITIONAL_PARAMETERS, TEXT_NAMED_PARAMETERS, TEXT_POSITIONAL_PARAMETERS } from '../openscadParser'
import type { EvalContext, Shape, Vec2 } from '../openscadParser'
import { resolveOpenScadFragments } from '../openScadStableModuleSemantics'
import type { OpenScadFragmentResolutionInput } from '../openScadStableModuleSemantics'
import { OpenScadImportPositionedError } from '../openScadImport'
import type { OpenScadImportErrorCode } from '../openScadImport'
import { OpenScadTextError, OpenScadTextPositionedError, renderOpenScadText } from '../openScadText'
import type { OpenScadTextErrorCode, OpenScadTextParameters } from '../openScadText'
import { callExpressionArguments } from './evalBuiltin'
import { currentEntityId, evalExpression, resolveStableVariable, warn } from './isExclusiveView'
import { nextColor } from '../openscadColors'
import { stableModuleSemanticsContext } from './requireStableProfile'

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

export function textArgumentValues(node: CallNode, ctx: EvalContext): Map<string, Value> {
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

export function textCallError(
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

export function textStringParameter(
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

export function textNumberParameter(
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

export function importArgumentValues(node: CallNode, ctx: EvalContext): Map<string, Value> {
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
