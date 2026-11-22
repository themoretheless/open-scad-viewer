import { generateVoronoiCells, type VoronoiGradientMode, type VoronoiPattern } from '../kernel/geometry/voronoi'
import { finiteNumber, vectorValue } from '../parser/astDiscovery'
import { arg, currentEntityId, kernelCall } from '../parser/expressionEvaluator'
import type { EvalContext, Shape, Shape2D } from '../parser/types'
import { evaluationError, isStableProfile } from './openScadStableExpressionEval'
import type { RGBA } from './openscadColors'
import type { CallNode, Value } from './openscadCompiler'
import {
  bboxOfPoly,
  clampMaxSegsForPanel,
  clipToBox,
  clipToInnerCavity,
  deterministicWallMaxSegs,
  emitCoupledVoronoiFloor,
  finalizeWallCell,
  generateTrabecularCells,
  overlapsKeepOut,
  roundVoronoiCell,
  sanitizeAndCCW,
  signedArea,
  survivesClipOut,
  type CoupleWallsSpec,
  type Pt,
} from './trabecularLattice'

type ViewerVoronoiPattern = VoronoiPattern | 'trabecular'
const VORONOI_PATTERNS = new Set<ViewerVoronoiPattern>([
  'trabecular',
  'voronoi',
  'web',
  'honeycomb',
  'isogrid',
  'triangles',
  'grid',
])
const GRADIENT_MODES = new Set<VoronoiGradientMode>(['none', 'vertical', 'radial'])

interface CachedPerimeterCell {
  poly: Pt[]
  minX: number
  maxX: number
}

const PERIMETER_CACHE_LIMIT = 16
const voronoiPerimeterCache = new Map<string, CachedPerimeterCell[]>()

function parseVec2(value: Value, ctx: EvalContext, p: number, label: string): [number, number] {
  const vector = vectorValue(value, ctx, p, label)
  if (vector.length < 2) evaluationError(ctx, p, `${label} must have at least two coordinates`)
  return [vector[0]!, vector[1]!]
}

function parseFlag(node: CallNode, name: string, ctx: EvalContext): boolean {
  const raw = arg(node, name, -1, 0, ctx)
  if (typeof raw === 'boolean') return raw
  return finiteNumber(raw, ctx, node.p, `viewer_voronoi ${name}`) > 0
}

function parseRectList(
  node: CallNode,
  name: string,
  ctx: EvalContext,
): Array<readonly [number, number, number, number]> {
  const raw = arg(node, name, -1, [], ctx)
  if (!Array.isArray(raw)) evaluationError(ctx, node.p, `viewer_voronoi ${name} must be a list of rectangles`)
  return raw
    .map(rect => {
      const v = vectorValue(rect, ctx, node.p, `viewer_voronoi ${name} entry`)
      if (v.length < 4) evaluationError(ctx, node.p, `Each ${name} rectangle needs [x0, y0, x1, y1]`)
      return [v[0]!, v[1]!, v[2]!, v[3]!] as const
    })
    .filter(r => Math.abs(r[2] - r[0]) > 1e-6 && Math.abs(r[3] - r[1]) > 1e-6)
}

function parseCoupleWalls(node: CallNode, ctx: EvalContext): CoupleWallsSpec | undefined {
  const raw = arg(node, 'couple_walls', -1, [], ctx)
  if (!Array.isArray(raw)) evaluationError(ctx, node.p, 'viewer_voronoi couple_walls must be a list')
  if (raw.length === 0) return undefined
  const v = vectorValue(raw, ctx, node.p, 'viewer_voronoi couple_walls')
  if (v.length < 14 || v[0]! <= 0) return undefined
  return {
    perimeter: v[0]!,
    sShift: v[1]!,
    spanX: v[2]!,
    sideSpan: v[3]!,
    cornerArc: v[4]!,
    innerR: v[5]!,
    cx: v[6]!,
    cyBack: v[7]!,
    cyFront: v[8]!,
    wallOverhang: v[9]!,
    wallCell: v[10]!,
    wallRib: v[11]!,
    wallJitter: v[12]!,
    wallSeed: v[13]!,
    wallHeight: v.length >= 15 ? v[14] : undefined,
  }
}

function estimateVoronoiSites(pattern: VoronoiPattern, w: number, h: number, cell: number): number {
  const nx = Math.max(1, Math.ceil(w / cell))
  const ny = Math.max(1, Math.ceil(h / cell))
  if (pattern === 'isogrid') {
    const rows = Math.ceil(h / ((cell * Math.sqrt(3)) / 2)) + 2
    return (nx + 2) * rows * 2
  }
  if (pattern === 'grid' || pattern === 'triangles') return nx * ny
  const rows = Math.max(1, Math.ceil(h / ((cell * Math.sqrt(3)) / 2)))
  return nx * rows
}

function generateTiledDeterministicCells(
  ctx: EvalContext,
  p: number,
  params: {
    pattern: VoronoiPattern
    minX: number
    maxX: number
    minY: number
    maxY: number
    cell: number
    rib: number
    seed: number
    jitter: number
    overhang: number
    gradient: number
    gradientMode: VoronoiGradientMode
    frame: number
  },
): Pt[][] | null {
  const h = params.maxY - params.minY
  const isoRows = Math.ceil(h / ((params.cell * Math.sqrt(3)) / 2)) + 2
  const ny = Math.max(1, Math.ceil(h / params.cell))
  const hexRows = Math.max(1, Math.ceil(h / ((params.cell * Math.sqrt(3)) / 2)))
  const rawMaxCols =
    params.pattern === 'isogrid'
      ? Math.floor(540 / (2 * isoRows)) - 2
      : params.pattern === 'grid' || params.pattern === 'triangles'
        ? Math.floor(540 / ny)
        : Math.floor(540 / hexRows)
  if (rawMaxCols < 4) return null
  const maxCols = rawMaxCols
  const overlapCols = Math.min(3, Math.floor(maxCols / 4))
  const strideCols = Math.max(1, maxCols - 2 * overlapCols)
  const out: Pt[][] = []

  for (let chunk = 0; chunk < 64; chunk++) {
    const startCol = chunk * strideCols
    const tileMinX = params.minX + startCol * params.cell
    const tileMaxX = tileMinX + maxCols * params.cell
    const isLast = tileMaxX >= params.maxX - 1e-6
    const keepMinX = chunk === 0 ? -Infinity : tileMinX + overlapCols * params.cell
    const keepMaxX = isLast ? Infinity : tileMinX + (overlapCols + strideCols) * params.cell

    const chunkCells = kernelCall(ctx, p, () =>
      generateVoronoiCells({
        min: [tileMinX, params.minY],
        max: [tileMaxX, params.maxY],
        pattern: params.pattern,
        cell: params.cell,
        rib: params.rib,
        seed: params.seed,
        jitter: params.jitter,
        overhangAngle: params.overhang > 0 ? params.overhang : null,
        stressGradient: params.gradient,
        gradientMode: params.gradientMode,
        cornerRadius: 0,
        slope: 0,
        frame: params.frame,
        keepOut: [],
        maxSites: 600,
      }),
    )
    for (const poly of chunkCells) {
      if (poly.length < 3) continue
      const cx = poly.reduce((sum, pt) => sum + pt[0], 0) / poly.length
      if (cx >= keepMinX && cx < keepMaxX) out.push(poly)
    }
    if (isLast) break
  }
  return out
}

function cachedPerimeterVoronoiCells(
  ctx: EvalContext,
  p: number,
  params: {
    pattern: VoronoiPattern
    perimeter: number
    minY: number
    maxY: number
    cell: number
    rib: number
    seed: number
    jitter: number
    overhang: number
    gradient: number
    gradientMode: VoronoiGradientMode
    frame: number
    maxSites: number
  },
): CachedPerimeterCell[] | null {
  const key = [
    params.pattern,
    params.perimeter.toFixed(4),
    params.minY.toFixed(4),
    params.maxY.toFixed(4),
    params.cell.toFixed(4),
    params.rib.toFixed(4),
    params.seed,
    params.jitter.toFixed(4),
    params.overhang.toFixed(4),
    params.gradient.toFixed(4),
    params.gradientMode,
    params.frame.toFixed(4),
    params.maxSites,
  ].join('|')
  const cached = voronoiPerimeterCache.get(key)
  if (cached) return cached

  const pad = Math.max(params.cell * 1.5, 20)
  const minX = -pad
  const maxX = params.perimeter + pad
  const est = estimateVoronoiSites(params.pattern, maxX - minX, params.maxY - params.minY, params.cell)

  let raw: Pt[][] | null
  if (est <= 560) {
    raw = kernelCall(ctx, p, () =>
      generateVoronoiCells({
        min: [minX, params.minY],
        max: [maxX, params.maxY],
        pattern: params.pattern,
        cell: params.cell,
        rib: params.rib,
        seed: params.seed,
        jitter: params.jitter,
        overhangAngle: params.overhang > 0 ? params.overhang : null,
        stressGradient: params.gradient,
        gradientMode: params.gradientMode,
        cornerRadius: 0,
        slope: 0,
        frame: params.frame,
        keepOut: [],
        maxSites: Math.min(600, Math.max(Math.trunc(params.maxSites), 600)),
      }),
    )
  } else if (params.pattern !== 'voronoi' && params.pattern !== 'web') {
    raw = generateTiledDeterministicCells(ctx, p, {
      ...params,
      minX,
      maxX,
    })
  } else {
    return null
  }
  if (raw === null) return null
  const indexed: CachedPerimeterCell[] = raw.map(poly => {
    const bb = bboxOfPoly(poly)
    return { poly, minX: bb.minX, maxX: bb.maxX }
  })
  if (voronoiPerimeterCache.size >= PERIMETER_CACHE_LIMIT) {
    const oldest = voronoiPerimeterCache.keys().next().value
    if (oldest !== undefined) voronoiPerimeterCache.delete(oldest)
  }
  voronoiPerimeterCache.set(key, indexed)
  return indexed
}

function emitCoupledRustFloor(
  ctx: EvalContext,
  p: number,
  opts: {
    pattern: VoronoiPattern
    minPt: readonly [number, number]
    maxPt: readonly [number, number]
    cell: number
    rib: number
    seed: number
    jitter: number
    gradient: number
    radius: number
    keepOut: readonly (readonly [number, number, number, number])[]
    coupleWalls: CoupleWallsSpec
  },
): Pt[][] | null {
  const { pattern, minPt, maxPt, cell, rib, seed, jitter, gradient, radius, keepOut, coupleWalls } = opts
  if (pattern === 'voronoi' || pattern === 'web' || pattern === 'honeycomb') {
    const wallMaxY =
      coupleWalls.wallHeight !== undefined
        ? Math.max(0.5, coupleWalls.wallHeight + coupleWalls.wallRib * 0.5)
        : Math.max(20, coupleWalls.wallCell * 3)
    const effJitter = pattern === 'honeycomb' ? 0 : coupleWalls.wallJitter
    const cached = cachedPerimeterVoronoiCells(ctx, p, {
      pattern,
      perimeter: coupleWalls.perimeter,
      minY: -coupleWalls.wallRib,
      maxY: wallMaxY,
      cell: coupleWalls.wallCell,
      rib: coupleWalls.wallRib,
      seed: coupleWalls.wallSeed,
      jitter: effJitter,
      overhang: coupleWalls.wallOverhang,
      gradient,
      gradientMode: 'vertical',
      frame: 0,
      maxSites: 600,
    })
    if (!cached) return null
    const pad = Math.max(coupleWalls.wallCell * 1.5, 20)
    const wallBaseMaxSegs = deterministicWallMaxSegs(
      coupleWalls.perimeter,
      coupleWalls.spanX,
      wallMaxY,
      coupleWalls.wallCell,
      coupleWalls.wallCell,
    )
    const yProbe = 0.01
    const rawIntervals: Array<{ bMinX: number; bMaxX: number; mid: number }> = []
    for (const entry of cached) {
      if (entry.maxX < -1 || entry.minX > coupleWalls.perimeter + 1) continue
      const finalized = finalizeWallCell(sanitizeAndCCW(entry.poly), {
        min: [-pad, 0],
        max: [coupleWalls.perimeter + pad, wallMaxY],
        rib: coupleWalls.wallRib,
        overhang: coupleWalls.wallOverhang,
        slope: 0,
        frame: 0,
        radius,
        maxSegs: wallBaseMaxSegs,
        keepOut: [],
        openLeft: true,
        openRight: true,
        openBottom: true,
      })
      if (!finalized) continue
      const xs: number[] = []
      for (let i = 0; i < finalized.length; i++) {
        const a = finalized[i]!
        const b = finalized[(i + 1) % finalized.length]!
        if ((a[1] <= yProbe && b[1] >= yProbe) || (b[1] <= yProbe && a[1] >= yProbe)) {
          const dy = b[1] - a[1]
          if (Math.abs(dy) <= 1e-9) xs.push(a[0], b[0])
          else xs.push(a[0] + ((yProbe - a[1]) / dy) * (b[0] - a[0]))
        }
      }
      if (xs.length < 2) continue
      const bMinX = Math.min(...xs)
      const bMaxX = Math.max(...xs)
      if (bMaxX - bMinX <= 0.1) continue
      const mid = 0.5 * (bMinX + bMaxX)
      if (mid >= 0 && mid < coupleWalls.perimeter) rawIntervals.push({ bMinX, bMaxX, mid })
    }
    rawIntervals.sort((a, b) => a.mid - b.mid)
    const intervals: typeof rawIntervals = []
    for (const item of rawIntervals) {
      const prev = intervals[intervals.length - 1]
      if (prev && item.mid - prev.mid < coupleWalls.wallRib * 0.4) continue
      intervals.push(item)
    }
    if (intervals.length < 3) return null
    const ribs: number[] = []
    for (let i = 0; i < intervals.length - 1; i++) {
      ribs.push(0.5 * (intervals[i]!.bMaxX + intervals[i + 1]!.bMinX) + coupleWalls.sShift)
    }
    ribs.push(
      0.5 * (intervals[intervals.length - 1]!.bMaxX + intervals[0]!.bMinX + coupleWalls.perimeter) + coupleWalls.sShift,
    )
    const wallOpenings = intervals.map(iv => [iv.bMinX + coupleWalls.sShift, iv.bMaxX + coupleWalls.sShift] as const)
    return emitCoupledVoronoiFloor(
      {
        min: minPt,
        max: maxPt,
        cell,
        rib,
        seed,
        jitter: pattern === 'honeycomb' ? 0 : jitter,
        gradient,
        radius,
        keepOut,
      },
      coupleWalls,
      ribs,
      wallOpenings,
    )
  }

  const alignedMinX = minPt[0] + coupleWalls.innerR - Math.ceil((coupleWalls.innerR + cell) / cell) * cell
  const nx = Math.max(1, Math.ceil((maxPt[0] - alignedMinX + cell) / cell))
  const alignedMaxX = alignedMinX + nx * cell
  const alignedMinY = minPt[1] + coupleWalls.innerR - Math.ceil((coupleWalls.innerR + cell) / cell) * cell
  const ny = Math.max(1, Math.ceil((maxPt[1] - alignedMinY + cell) / cell))
  const alignedMaxY = alignedMinY + ny * cell
  if (estimateVoronoiSites(pattern, alignedMaxX - alignedMinX, alignedMaxY - alignedMinY, cell) > 580) return null

  const rawCells = kernelCall(ctx, p, () =>
    generateVoronoiCells({
      min: [alignedMinX, alignedMinY],
      max: [alignedMaxX, alignedMaxY],
      pattern,
      cell,
      rib,
      seed,
      jitter,
      overhangAngle: null,
      stressGradient: gradient,
      gradientMode: 'radial',
      cornerRadius: 0,
      slope: 0,
      frame: 0,
      keepOut: [],
      maxSites: 600,
    }),
  )
  const flareOut = radius > 0.1 ? radius + 1.5 : 0
  const clipMin: [number, number] = [minPt[0] - flareOut, minPt[1] - flareOut]
  const clipMax: [number, number] = [maxPt[0] + flareOut, maxPt[1] + flareOut]
  const clipR = coupleWalls.innerR > 0 ? coupleWalls.innerR + flareOut : 0
  const kept: Pt[][] = []
  for (const poly of rawCells) {
    const inner = clipToInnerCavity(poly, minPt, maxPt, coupleWalls.innerR)
    if (inner.length < 3) continue
    const bb = bboxOfPoly(inner)
    if (bb.width < rib * 1.1 || bb.height < rib * 1.1 || Math.abs(signedArea(inner)) < rib * rib * 0.75) continue
    if (!keepOut.some(rect => overlapsKeepOut(inner, rect))) {
      kept.push(flareOut > 0 ? clipToInnerCavity(poly, clipMin, clipMax, clipR) : inner)
    }
  }
  if (radius <= 1e-4) return kept
  const maxSegs = clampMaxSegsForPanel(kept, 8)
  return kept.map(poly => roundVoronoiCell(poly, radius, 0, maxSegs))
}

export async function evalViewerBend(
  node: CallNode,
  ctx: EvalContext,
  childShapes: () => Promise<Shape[]>,
): Promise<Shape[]> {
  if (isStableProfile(ctx))
    evaluationError(ctx, node.p, 'viewer_bend is a viewer mesh extension, not standard OpenSCAD')
  const radius = finiteNumber(arg(node, 'radius', 0, 0, ctx), ctx, node.p, 'bend radius')
  const center = finiteNumber(arg(node, 'center', 1, 0, ctx), ctx, node.p, 'bend center')
  const step = finiteNumber(arg(node, 'step', 2, 5, ctx), ctx, node.p, 'bend mesh step')
  if (radius <= 0 || step <= 0) evaluationError(ctx, node.p, 'Bend radius and mesh step must be positive')
  const reverse = arg(node, 'reverse', 3, false, ctx) === true
  const shapes = await childShapes()
  if (!ctx.kernel.bend3) evaluationError(ctx, node.p, 'Bend is unavailable for this geometry backend')
  return shapes.map(shape => {
    if (shape.dimension !== 3) evaluationError(ctx, node.p, 'viewer_bend requires a 3D solid')
    return {
      ...shape,
      geometry: kernelCall(ctx, node.p, () => ctx.kernel.bend3!(shape.geometry, radius, center, reverse, step)),
    }
  })
}

export function evalViewerVoronoi(node: CallNode, ctx: EvalContext, color: RGBA): Shape2D[] {
  if (isStableProfile(ctx))
    evaluationError(ctx, node.p, 'viewer_voronoi is a viewer mesh extension, not standard OpenSCAD')
  if (ctx.traceHierarchyOnly === true) {
    return [{ dimension: 2, geometry: ctx.kernel.rectangle([1, 1], false), color, entityId: currentEntityId(ctx) }]
  }
  const rawMin = arg(node, 'min', -1, undefined, ctx)
  const rawMax = arg(node, 'max', -1, undefined, ctx)
  const rawSize = arg(node, 'size', 0, undefined, ctx)
  const minPt: [number, number] = rawMin !== undefined ? parseVec2(rawMin, ctx, node.p, 'viewer_voronoi min') : [0, 0]
  let maxPt: [number, number]
  if (rawMax !== undefined) {
    maxPt = parseVec2(rawMax, ctx, node.p, 'viewer_voronoi max')
  } else if (rawSize !== undefined) {
    const sizePt = parseVec2(rawSize, ctx, node.p, 'viewer_voronoi size')
    maxPt = [minPt[0] + sizePt[0], minPt[1] + sizePt[1]]
  } else {
    evaluationError(ctx, node.p, 'viewer_voronoi requires size or min/max bounds')
  }
  const cell = finiteNumber(arg(node, 'cell', 1, 16, ctx), ctx, node.p, 'viewer_voronoi cell')
  const rib = finiteNumber(arg(node, 'rib', 2, 2.4, ctx), ctx, node.p, 'viewer_voronoi rib')
  const patternRaw = arg(node, 'pattern', -1, 'voronoi', ctx)
  if (typeof patternRaw !== 'string' || !VORONOI_PATTERNS.has(patternRaw as ViewerVoronoiPattern))
    evaluationError(
      ctx,
      node.p,
      'viewer_voronoi pattern must be trabecular, voronoi, honeycomb, isogrid, triangles or grid',
    )
  const seed = finiteNumber(arg(node, 'seed', -1, 1, ctx), ctx, node.p, 'viewer_voronoi seed')
  const jitter = finiteNumber(arg(node, 'jitter', -1, 0.7, ctx), ctx, node.p, 'viewer_voronoi jitter')
  const overhang = finiteNumber(arg(node, 'overhang', -1, 0, ctx), ctx, node.p, 'viewer_voronoi overhang')
  if (overhang < 0) evaluationError(ctx, node.p, 'viewer_voronoi overhang must be non-negative')
  const gradient = finiteNumber(arg(node, 'gradient', -1, 0, ctx), ctx, node.p, 'viewer_voronoi gradient')
  const modeRaw = arg(node, 'gradient_mode', -1, 'none', ctx)
  if (typeof modeRaw !== 'string' || !GRADIENT_MODES.has(modeRaw as VoronoiGradientMode))
    evaluationError(ctx, node.p, 'viewer_voronoi gradient_mode must be none, vertical or radial')
  const radius = finiteNumber(arg(node, 'radius', -1, 0, ctx), ctx, node.p, 'viewer_voronoi radius')
  const slope = finiteNumber(arg(node, 'slope', -1, 0, ctx), ctx, node.p, 'viewer_voronoi slope')
  const frame = finiteNumber(arg(node, 'frame', -1, 0, ctx), ctx, node.p, 'viewer_voronoi frame')
  const maxSites = finiteNumber(arg(node, 'max_sites', -1, 400, ctx), ctx, node.p, 'viewer_voronoi max_sites')
  const seamOffset = finiteNumber(arg(node, 'seam_offset', -1, 0, ctx), ctx, node.p, 'viewer_voronoi seam_offset')
  const seamDir = finiteNumber(arg(node, 'seam_dir', -1, 1, ctx), ctx, node.p, 'viewer_voronoi seam_dir') < 0 ? -1 : 1
  const perimeter = finiteNumber(arg(node, 'perimeter', -1, 0, ctx), ctx, node.p, 'viewer_voronoi perimeter')
  const openLeft = parseFlag(node, 'open_left', ctx)
  const openRight = parseFlag(node, 'open_right', ctx)
  const openBottom = parseFlag(node, 'open_bottom', ctx)
  finiteNumber(arg(node, 'fold_span', -1, 0, ctx), ctx, node.p, 'viewer_voronoi fold_span')
  const keepOut = parseRectList(node, 'keep_out', ctx)
  const clipOut = parseRectList(node, 'clip_out', ctx)
  const coupleWalls = parseCoupleWalls(node, ctx)

  let outCells: Pt[][]
  if (patternRaw === 'trabecular') {
    outCells = generateTrabecularCells({
      min: minPt,
      max: maxPt,
      cell,
      rib,
      seed,
      jitter,
      overhang,
      gradient,
      gradientMode: modeRaw as VoronoiGradientMode,
      slope,
      frame,
      radius,
      keepOut,
      seamOffset,
      seamDir,
      perimeter,
      openLeft,
      openRight,
      openBottom,
      coupleWalls: modeRaw === 'radial' ? coupleWalls : undefined,
    })
  } else {
    const coupledRust =
      modeRaw === 'radial' && coupleWalls
        ? emitCoupledRustFloor(ctx, node.p, {
            pattern: patternRaw as VoronoiPattern,
            minPt,
            maxPt,
            cell,
            rib,
            seed,
            jitter,
            gradient,
            radius,
            keepOut,
            coupleWalls,
          })
        : null
    if (coupledRust) {
      outCells = coupledRust
    } else {
      const cached =
        perimeter > 0 && Math.abs(slope) < 1e-6
          ? cachedPerimeterVoronoiCells(ctx, node.p, {
              pattern: patternRaw as VoronoiPattern,
              perimeter,
              minY: openBottom ? minPt[1] - rib : minPt[1],
              maxY: maxPt[1],
              cell,
              rib,
              seed,
              jitter,
              overhang,
              gradient,
              gradientMode: modeRaw as VoronoiGradientMode,
              frame,
              maxSites,
            })
          : null
      if (cached) {
        const span = maxPt[0] - minPt[0]
        const sA = seamOffset
        const sB = seamOffset + seamDir * span
        const sMin = Math.min(sA, sB)
        const sMax = Math.max(sA, sB)
        const baseMaxSegs = deterministicWallMaxSegs(perimeter, span, maxPt[1] - minPt[1], cell, cell)
        const selected = cached.filter(entry => entry.maxX >= sMin - 1 && entry.minX <= sMax + 1)
        const maxSegs = clampMaxSegsForPanel(
          selected.map(e => e.poly),
          baseMaxSegs,
        )
        outCells = []
        for (const entry of selected) {
          const mapped: Pt[] = entry.poly.map(([sx, sy]) => [minPt[0] + (sx - seamOffset) * seamDir, sy])
          const localPoly = sanitizeAndCCW(mapped)
          const finalized = finalizeWallCell(localPoly, {
            min: minPt,
            max: maxPt,
            rib,
            overhang,
            slope: 0,
            frame,
            radius,
            maxSegs,
            keepOut,
            openLeft,
            openRight,
            openBottom,
          })
          if (finalized) outCells.push(finalized)
        }
      } else {
        const cells = kernelCall(ctx, node.p, () =>
          generateVoronoiCells({
            min: minPt,
            max: maxPt,
            pattern: patternRaw as VoronoiPattern,
            cell,
            rib,
            seed,
            jitter,
            overhangAngle: overhang > 0 ? overhang : null,
            stressGradient: gradient,
            gradientMode: modeRaw as VoronoiGradientMode,
            cornerRadius: 0,
            slope,
            frame,
            keepOut,
            maxSites: Math.trunc(maxSites),
          }),
        )
        const totalCorners = radius > 1e-4 ? cells.reduce((sum, cellPoly) => sum + cellPoly.length, 0) : 0
        const baseMaxSegs = totalCorners > 0 ? Math.max(3, Math.min(12, Math.floor(2000 / totalCorners))) : 12
        const maxSegs = clampMaxSegsForPanel(cells, baseMaxSegs)
        outCells = radius > 1e-4 ? cells.map(cellPoly => roundVoronoiCell(cellPoly, radius, overhang, maxSegs)) : cells
      }
    }
  }

  if (clipOut.length > 0) {
    const checkRegion = coupleWalls && modeRaw === 'radial'
    outCells = outCells.filter(poly =>
      survivesClipOut(checkRegion ? clipToBox(poly, minPt, maxPt) : poly, clipOut, rib),
    )
  }
  if (outCells.length === 0) return []
  return [
    {
      dimension: 2,
      geometry: kernelCall(ctx, node.p, () => ctx.kernel.polygon(outCells, 'NonZero')),
      color: [...color],
      entityId: currentEntityId(ctx),
    },
  ]
}
