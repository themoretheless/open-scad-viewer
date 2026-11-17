import { DirectInputError } from '../i18n/kernelErrors'
import type { DirectSketch, Point2 } from './directModeling'
import type { AnalyticCurve } from './directSketchGeometry'
import type { SketchCurveConstraint } from './sketchCurveConstraints'

const LIMIT = 1e6
const finite = (...values: number[]) => values.every(v => Number.isFinite(v) && Math.abs(v) <= LIMIT)

export interface EllipseInput {
  center: Point2
  radiusA: number
  radiusB: number
  /** Degrees, counterclockwise from the local U axis. */
  rotation: number
}

/**
 * Sketch documents and retained profiles only carry circles and circular arcs, so an ellipse
 * stays exact only when both radii agree (it is a circle). Any other ellipse is refused with
 * a registered code instead of being approximated by a polyline.
 */
export function ellipseSketchCurve(input: EllipseInput): AnalyticCurve {
  const { center, radiusA, radiusB, rotation } = input
  if (
    !Array.isArray(center) ||
    center.length !== 2 ||
    !finite(center[0], center[1], radiusA, radiusB) ||
    !Number.isFinite(rotation) ||
    radiusA < 0.01 ||
    radiusB < 0.01
  ) {
    throw new DirectInputError(
      'DIRECT_SKETCH_ELLIPSE_INPUT',
      'Ellipse needs a finite center and radii from 0.01 to 1000000 mm.',
      'ellipse',
    )
  }
  if (Math.abs(radiusA - radiusB) > 1e-9 * Math.max(radiusA, radiusB)) {
    throw new DirectInputError(
      'DIRECT_SKETCH_ELLIPSE_NOT_EXACT',
      'Sketch profiles carry no exact rational ellipse; refusing a polyline approximation.',
    )
  }
  return { kind: 'circle', center: [center[0], center[1]], radius: radiusA, start: 0, sweep: 360 }
}

export interface SlotDefinition {
  a: Point2
  b: Point2
  width: number
}

/** Overall length (tip to tip) and full width of a center-line slot. */
export function slotDimensions(slot: SlotDefinition): { length: number; width: number; centerDistance: number } | null {
  const { a, b, width } = slot
  if (!finite(a[0], a[1], b[0], b[1], width) || width < 0.02) return null
  const centerDistance = Math.hypot(b[0] - a[0], b[1] - a[1])
  if (centerDistance < 1e-6) return null
  return { length: centerDistance + width, width, centerDistance }
}

/**
 * Applies a slot length and/or width dimension (an unchanged length is held), keeping the first center and the center-line
 * direction. Returns null when the length does not exceed the width by at least 0.000001 mm.
 */
export function resizeSlot(slot: SlotDefinition, change: { length?: number; width?: number }): SlotDefinition | null {
  const current = slotDimensions(slot)
  if (!current) return null
  const width = change.width ?? current.width
  const length = change.length ?? current.length
  if (!finite(width, length) || width < 0.02) return null
  const distance = length - width
  if (distance < 1e-6) return null
  const ux = (slot.b[0] - slot.a[0]) / current.centerDistance,
    uy = (slot.b[1] - slot.a[1]) / current.centerDistance
  const b: Point2 = [slot.a[0] + ux * distance, slot.a[1] + uy * distance]
  if (!finite(b[0], b[1])) return null
  return { a: [slot.a[0], slot.a[1]], b, width }
}

/** Exact boundary of a slot: two tangent lines and two semicircular caps, counterclockwise. */
export function analyticSlotBoundary(slot: SlotDefinition): {
  lines: [[Point2, Point2], [Point2, Point2]]
  caps: [AnalyticCurve, AnalyticCurve]
} | null {
  const d = slotDimensions(slot)
  if (!d) return null
  const { a, b } = slot,
    r = d.width / 2
  const angle = (Math.atan2(b[1] - a[1], b[0] - a[0]) * 180) / Math.PI
  const nx = (-(b[1] - a[1]) / d.centerDistance) * r,
    ny = ((b[0] - a[0]) / d.centerDistance) * r
  const rightA: Point2 = [a[0] - nx, a[1] - ny],
    rightB: Point2 = [b[0] - nx, b[1] - ny],
    leftB: Point2 = [b[0] + nx, b[1] + ny],
    leftA: Point2 = [a[0] + nx, a[1] + ny]
  return {
    lines: [
      [rightA, rightB],
      [leftB, leftA],
    ],
    caps: [
      { kind: 'arc', center: [b[0], b[1]], radius: r, start: angle - 90, sweep: 180 },
      { kind: 'arc', center: [a[0], a[1]], radius: r, start: angle + 90, sweep: 180 },
    ],
  }
}

/**
 * Arc that continues an open polyline from `end`, tangent to its end segment, and ends at
 * `target`. Refuses (null) a closed or analytic sketch, a degenerate end segment, a target
 * on the tangent line (no finite radius) and radii outside 0.01 to 1000000 mm.
 */
export function tangentArc(
  sketch: Pick<DirectSketch, 'points' | 'closed' | 'analytic'>,
  end: 'start' | 'end',
  target: Point2,
): AnalyticCurve | null {
  if (sketch.closed || sketch.analytic || sketch.points.length < 2) return null
  const pts = sketch.points
  const p = end === 'end' ? pts[pts.length - 1] : pts[0],
    q = end === 'end' ? pts[pts.length - 2] : pts[1]
  if (!p || !q || !finite(p[0], p[1], q[0], q[1], target[0], target[1])) return null
  const len = Math.hypot(p[0] - q[0], p[1] - q[1])
  if (len < 1e-9) return null
  const tx = (p[0] - q[0]) / len,
    ty = (p[1] - q[1]) / len
  const dx = target[0] - p[0],
    dy = target[1] - p[1],
    chord2 = dx * dx + dy * dy
  const normal = -ty * dx + tx * dy
  if (chord2 < 1e-12 || Math.abs(normal) <= 1e-9 * Math.sqrt(chord2)) return null
  const signed = chord2 / (2 * normal),
    radius = Math.abs(signed)
  if (!(radius >= 0.01 && radius <= LIMIT)) return null
  const center: Point2 = [p[0] - ty * signed, p[1] + tx * signed]
  const deg = (y: number, x: number) => (Math.atan2(y, x) * 180) / Math.PI
  const start = deg(p[1] - center[1], p[0] - center[0]),
    stop = deg(target[1] - center[1], target[0] - center[0])
  let sweep = (((stop - start) % 360) + 360) % 360
  if (signed < 0) sweep -= 360
  if (Math.abs(sweep) < 0.1 || Math.abs(sweep) >= 360) return null
  return { kind: 'arc', center, radius, start, sweep }
}

/**
 * Persistent tangency stored on a tangent arc: the arc circle stays tangent to the end segment of the
 * polyline it continues, so edits of the polyline re-solve the arc (or refuse) through the curve-constraint solver.
 */
export function tangentArcConstraint(
  sketch: Pick<DirectSketch, 'id' | 'points'>,
  end: 'start' | 'end',
): Extract<SketchCurveConstraint, { kind: 'tangent_line' }> {
  const n = sketch.points.length
  return end === 'end'
    ? { kind: 'tangent_line', ref: sketch.id, a: n - 2, b: n - 1 }
    : { kind: 'tangent_line', ref: sketch.id, a: 0, b: 1 }
}
