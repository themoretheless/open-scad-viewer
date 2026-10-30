// assetEvaluator: compatibilityDxfProfile.
import type { CadKernelHandle } from '../../services/cadKernelOps'

import type { CallNode, ExpressionArgument, Value } from '../../services/openscadCompiler'

import { formatOpenScadValue } from '../../services/openScadValueSemantics'

import {
  resolveOpenScadFragments,
  type OpenScadFragmentResolutionInput,
} from '../../services/openScadStableModuleSemantics'

import {
  OpenScadProjectError,
  resolveOpenScadProjectPath,
} from '../../services/openScadProject'

import {
  loadPreparedOpenScadImport,
  OpenScadImportError,
  OpenScadImportPositionedError,
  type OpenScad2021LegacyImportFormat,
  type OpenScadImportErrorCode,
} from '../../services/openScadImport'

import {
  loadOpenScadSurfaceHeightMap,
  OpenScadSurfaceDataError,
  OpenScadSurfaceError,
  triangulateOpenScadSurface,
} from '../../services/openScadSurface'

import {
  OpenScadTextError,
  OpenScadTextPositionedError,
  renderOpenScadText,
  type OpenScadTextErrorCode,
  type OpenScadTextParameters,
} from '../../services/openScadText'

import { nextColor } from '../../services/openscadColors'

import {
  type EvalContext,
  type Shape,
  type Vec2,
} from '.././types'

import { callExpressionArguments } from '.././astDiscovery'

import {
  arg,
  compatibilityString,
  currentEntityId,
  evalExpression,
  evaluateStableBuiltinModuleArguments,
  resolveStableVariable,
  trackedSolid,
  warn,
} from '.././expressionEvaluator'

import { stableModuleSemanticsContext } from '.././geometryEvaluator'
import { importCallError } from './surfaceCallSource'


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
