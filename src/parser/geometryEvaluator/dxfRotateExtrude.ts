// geometryEvaluator: dxfRotateExtrude.

import { clamp01 } from '../../kernel/geometry/math3d'


import type { CallNode, Value } from '../../language/openscadCompiler'





import {
  resolveOpenScadRotateExtrude,
} from '../../language/semantic/openScadStableGeometrySemantics'

import {
  evaluationError,
  isStableProfile,
} from '../../language/openScadStableExpressionEval'

import { CSS_COLORS, type RGBA } from '../../language/openscadColors'

import {
  compatibilityDxfProfile,
} from '.././assetEvaluator'

import {
  type EvalContext,
  type Shape,
} from '.././types'

import {
  finiteNumber,
  vectorValue,
} from '.././astDiscovery'

import {
  arg,
  compatibilityString,
  evaluateStableBuiltinModuleArguments,
  evaluateStableCompatibilityArguments,
  trackedSolid,
} from '.././expressionEvaluator'
import { stableFragmentInputFromEvaluated, stableGeometrySemanticsContext, EvalNodesFn } from './EvalNodesFn'
import { booleanShapes, segments } from './segments'


export function stableRotateExtrudeSections(
  node: CallNode,
  ctx: EvalContext,
  evaluated: ReadonlyMap<string, Value>,
  sections: Shape[],
): Shape[] {
  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, `${node.name}() requires 2D children`)
  const bounds = ctx.kernel.isEmpty(sections[0].geometry)
    ? { min: [0, 0], max: [0, 0] }
    : ctx.kernel.bounds(sections[0].geometry)
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


export async function dxfRotateExtrude(
  node: CallNode,
  ctx: EvalContext,
  evalNodes: EvalNodesFn,
): Promise<Shape[]> {
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


export async function rotateExtrude(
  node: CallNode,
  ctx: EvalContext,
  evalNodes: EvalNodesFn,
): Promise<Shape[]> {
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


export function parseLegacyColor(value: Value, ctx: EvalContext, p: number): RGBA {
  if (Array.isArray(value)) {
    const channels = vectorValue(value, ctx, p, 'color')
    return [clamp01(channels[0] ?? 0.5), clamp01(channels[1] ?? 0.5), clamp01(channels[2] ?? 0.5), clamp01(channels[3] ?? 1)]
  }
  if (typeof value !== 'string') evaluationError(ctx, p, 'color() expects a name or RGB(A) vector')
  const named = CSS_COLORS[value.toLowerCase()]
  if (named) return [...named]
  const match = value.match(/^#([0-9a-f]{6}|[0-9a-f]{8})$/i)
  if (match) {
    const hex = match[1]
    return [parseInt(hex.slice(0, 2), 16) / 255, parseInt(hex.slice(2, 4), 16) / 255, parseInt(hex.slice(4, 6), 16) / 255, hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1]
  }
  evaluationError(ctx, p, `Unknown color ${value}`)
}
