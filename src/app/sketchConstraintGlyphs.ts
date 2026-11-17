import type { Point2 } from './directModeling'
import type { SketchConstraint } from './sketchConstraints'
import type { SketchCurveConstraint } from './sketchCurveConstraints'
import type { AnalyticCurve } from './directSketchGeometry'

/** Which stored list a glyph points at: polyline `constraints` or circle/arc `curveConstraints`. */
export type SketchConstraintSource = 'polyline' | 'curve'

export interface SketchConstraintRef {
  sketchId: string
  source: SketchConstraintSource
  index: number
}

export interface SketchConstraintGlyph extends SketchConstraintRef {
  key: string
  kind: SketchConstraint['kind'] | 'concentric' | 'equal_radius' | 'tangent' | 'coincident'
  symbol: string
  at: Point2
}

interface GlyphSketch {
  id: string
  points: Point2[]
  analytic?: Pick<AnalyticCurve, 'center' | 'radius'>
  constraints?: SketchConstraint[]
  curveConstraints?: SketchCurveConstraint[]
}

const SYMBOLS: Record<SketchConstraintGlyph['kind'], string> = {
  horizontal: 'H',
  vertical: 'V',
  parallel: '∥',
  perpendicular: '⊥',
  equal_length: '=',
  concentric: '◎',
  equal_radius: 'R=',
  tangent: 'T',
  coincident: '●',
}

const sameRef = (a: SketchConstraintRef | null | undefined, b: SketchConstraintRef) =>
  !!a && a.sketchId === b.sketchId && a.source === b.source && a.index === b.index

export const isSketchConstraintRef = sameRef

/**
 * Glyph positions for the persistent constraints of the given sketches, in sketch-plane millimetres.
 * Polyline constraints sit beside the midpoint of each constrained segment (pair constraints get one glyph per
 * segment); several glyphs on one segment are stacked along its normal. Circle and arc constraints sit just
 * outside the curve, a tangent-to-segment glyph at the tangent point. `offset` is the screen-sized gap in mm.
 */
export function sketchConstraintGlyphs(sketches: readonly GlyphSketch[], offset: number): SketchConstraintGlyph[] {
  const out: SketchConstraintGlyph[] = []
  for (const s of sketches) {
    const stack = new Map<string, number>()
    s.constraints?.forEach((c, index) => {
      const segments: [number, number][] = [[c.a, c.b]]
      if ('c' in c) segments.push([c.c, c.d])
      segments.forEach(([i, j], part) => {
        const a = s.points[i],
          b = s.points[j]
        if (!a || !b) return
        const len = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1
        const n: Point2 = [-(b[1] - a[1]) / len, (b[0] - a[0]) / len]
        const segKey = `${Math.min(i, j)}-${Math.max(i, j)}`
        const level = (stack.get(segKey) ?? 0) + 1
        stack.set(segKey, level)
        const mid: Point2 = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2]
        out.push({
          key: `${s.id}:p${index}:${part}`,
          sketchId: s.id,
          source: 'polyline',
          index,
          kind: c.kind,
          symbol: SYMBOLS[c.kind],
          at: [mid[0] + n[0] * offset * level, mid[1] + n[1] * offset * level],
        })
      })
    })
    const curve = s.analytic
    s.curveConstraints?.forEach((c, index) => {
      if (c.kind === 'coincident') {
        // Beside the coincident vertex, diagonally so it does not cover the segments meeting there.
        const p = s.points[c.point]
        if (!p) return
        const level = (stack.get(`v${c.point}`) ?? 0) + 1
        stack.set(`v${c.point}`, level)
        const d = (offset * level) / Math.SQRT2
        out.push({
          key: `${s.id}:k${index}`,
          sketchId: s.id,
          source: 'curve',
          index,
          kind: 'coincident',
          symbol: SYMBOLS.coincident,
          at: [p[0] + d, p[1] - d],
        })
        return
      }
      if (!curve) return
      const kind: SketchConstraintGlyph['kind'] =
        c.kind === 'tangent_circle' || c.kind === 'tangent_line' ? 'tangent' : c.kind
      const angle = Math.PI / 4 + (index * Math.PI) / 6
      let at: Point2 = [
        curve.center[0] + (curve.radius + offset) * Math.cos(angle),
        curve.center[1] + (curve.radius + offset) * Math.sin(angle),
      ]
      if (c.kind === 'concentric')
        at = [curve.center[0] + offset * Math.cos(angle), curve.center[1] + offset * Math.sin(angle)]
      if (c.kind === 'tangent_line') {
        const ref = sketches.find(o => o.id === c.ref)
        const a = ref?.points[c.a],
          b = ref?.points[c.b]
        if (a && b) {
          const dx = b[0] - a[0],
            dy = b[1] - a[1],
            l2 = dx * dx + dy * dy
          if (l2 > 0) {
            const t = ((curve.center[0] - a[0]) * dx + (curve.center[1] - a[1]) * dy) / l2
            const foot: Point2 = [a[0] + t * dx, a[1] + t * dy]
            const away = Math.hypot(foot[0] - curve.center[0], foot[1] - curve.center[1]) || 1
            at = [
              foot[0] + ((foot[0] - curve.center[0]) / away) * offset,
              foot[1] + ((foot[1] - curve.center[1]) / away) * offset,
            ]
          }
        }
      }
      out.push({ key: `${s.id}:k${index}`, sketchId: s.id, source: 'curve', index, kind, symbol: SYMBOLS[kind], at })
    })
  }
  return out
}

/** Returns the sketch without the referenced constraint, or `null` when the reference does not match it. */
export function removeSketchConstraintRef<T extends GlyphSketch>(sketch: T, ref: SketchConstraintRef): T | null {
  if (sketch.id !== ref.sketchId) return null
  const list = ref.source === 'polyline' ? sketch.constraints : sketch.curveConstraints
  if (!list || !Number.isInteger(ref.index) || ref.index < 0 || ref.index >= list.length) return null
  const kept = list.filter((_, i) => i !== ref.index)
  const next = { ...sketch }
  const key = ref.source === 'polyline' ? 'constraints' : 'curveConstraints'
  if (kept.length) (next as GlyphSketch)[key] = kept as never
  else delete (next as GlyphSketch)[key]
  return next
}
