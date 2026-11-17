import { DirectInputError } from '../i18n/kernelErrors'
import { cross3, dot3, type Vec3 } from '../kernel/geometry/math3d'
import type { DirectSketch, Point2 } from './directModeling'
import { sampleCurve, worldPoint, xyPlane, type AnalyticCurve, type SketchPlane } from './directSketchGeometry'

/** Planes whose normals differ by less than this (sine of the angle) count as parallel. */
export const PROJECTION_PARALLEL_TOLERANCE = 1e-9
const MIN_SEGMENT_MM = 1e-6

export interface ProjectedSketch {
  points: Point2[]
  closed: boolean
  analytic?: AnalyticCurve
  plane: SketchPlane
}

const sub = (a: Vec3, b: ArrayLike<number>): Vec3 => [a[0] - b[0]!, a[1] - b[1]!, a[2] - b[2]!]
const local = (p: Vec3, plane: SketchPlane): Point2 => {
  const d = sub(p, plane.origin)
  return [dot3(d, plane.u), dot3(d, plane.v)]
}
const refuseInput = (message: string) => new DirectInputError('DIRECT_SKETCH_PROJECTION_INPUT', message)

/**
 * Orthogonal projection of a sketch's edges onto `target` as a new sketch in target coordinates. Only exact
 * results are produced: polyline segments stay segments on any plane (a segment that collapses to a point is
 * refused), and circles and arcs are projected only from a plane parallel to the target, where projection is an
 * isometry (an arc seen from the opposite side keeps its geometry with the sweep reversed). A circle or arc on a
 * non-parallel plane would become an ellipse and is refused with `DIRECT_SKETCH_PROJECTION_NOT_EXACT`.
 */
export function projectSketchToPlane(
  source: Pick<DirectSketch, 'points' | 'closed' | 'analytic' | 'plane' | 'retainedProfile' | 'editablePath'>,
  target: SketchPlane,
): ProjectedSketch {
  if (source.retainedProfile || source.editablePath)
    throw refuseInput('Only polyline, circle and arc sketches can be projected.')
  const from = source.plane ?? xyPlane()
  const nTarget = cross3(target.u, target.v),
    nSource = cross3(from.u, from.v)
  const plane: SketchPlane = {
    origin: [...target.origin] as Vec3,
    u: [...target.u] as Vec3,
    v: [...target.v] as Vec3,
  }
  const curve = source.analytic
  if (curve) {
    const side = cross3(nSource, nTarget)
    if (Math.hypot(...side) > PROJECTION_PARALLEL_TOLERANCE)
      throw new DirectInputError(
        'DIRECT_SKETCH_PROJECTION_NOT_EXACT',
        'A circle or arc on a non-parallel plane projects to an ellipse; refusing an approximation.',
      )
    const center = local(worldPoint(curve.center, from), plane)
    const rad = (curve.start * Math.PI) / 180
    const dir: Vec3 = [0, 1, 2].map(k => Math.cos(rad) * from.u[k]! + Math.sin(rad) * from.v[k]!) as Vec3
    const start = (Math.atan2(dot3(dir, plane.v), dot3(dir, plane.u)) * 180) / Math.PI
    const flipped = dot3(nSource, nTarget) < 0
    const analytic: AnalyticCurve = {
      ...curve,
      center,
      start: curve.kind === 'circle' ? curve.start : start,
      sweep: flipped && curve.kind === 'arc' ? -curve.sweep : curve.sweep,
    }
    if (![center[0], center[1], analytic.start].every(Number.isFinite)) throw refuseInput('Projection is not finite.')
    return { points: sampleCurve(analytic), closed: source.closed === true, analytic, plane }
  }
  if (source.points.length < 2) throw refuseInput('A projected polyline needs at least two points.')
  const points = source.points.map(p => local(worldPoint(p, from), plane))
  if (!points.every(p => Number.isFinite(p[0]) && Number.isFinite(p[1]))) throw refuseInput('Projection is not finite.')
  const n = points.length
  for (let i = 0; i < (source.closed ? n : n - 1); i++) {
    const a = points[i]!,
      b = points[(i + 1) % n]!
    if (Math.hypot(b[0] - a[0], b[1] - a[1]) <= MIN_SEGMENT_MM)
      throw refuseInput('A projected segment collapses to a point.')
  }
  return { points, closed: source.closed === true, plane }
}

const refuseNotExact = (message: string) => new DirectInputError('DIRECT_SKETCH_PROJECTION_NOT_EXACT', message)
const copyPlane = (target: SketchPlane): SketchPlane => ({
  origin: [...target.origin] as Vec3,
  u: [...target.u] as Vec3,
  v: [...target.v] as Vec3,
})
const dist3 = (a: Vec3, b: Vec3) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2])
const finite2 = (points: readonly Point2[]) => points.every(p => Number.isFinite(p[0]) && Number.isFinite(p[1]))

function closedPolyline(loop: readonly Vec3[], target: SketchPlane): ProjectedSketch {
  const points: Point2[] = []
  for (const p of loop) {
    const q = local([p[0], p[1], p[2]], target)
    const last = points.at(-1)
    if (!last || Math.hypot(q[0] - last[0], q[1] - last[1]) > MIN_SEGMENT_MM) points.push(q)
  }
  const first = points[0],
    last = points.at(-1)
  if (first && last && points.length > 1 && Math.hypot(first[0] - last[0], first[1] - last[1]) <= MIN_SEGMENT_MM)
    points.pop()
  if (points.length < 3) throw refuseInput('A projected face boundary collapses.')
  if (!finite2(points)) throw refuseInput('Projection is not finite.')
  return { points, closed: true, plane: copyPlane(target) }
}

function openPolyline(run: readonly Vec3[], target: SketchPlane): ProjectedSketch {
  const points = run.map(p => local([p[0], p[1], p[2]], target))
  if (!finite2(points)) throw refuseInput('Projection is not finite.')
  for (let i = 1; i < points.length; i++)
    if (Math.hypot(points[i]![0] - points[i - 1]![0], points[i]![1] - points[i - 1]![1]) <= MIN_SEGMENT_MM)
      throw refuseInput('A projected segment collapses to a point.')
  return { points, closed: false, plane: copyPlane(target) }
}

/**
 * Projects the boundary loops of a planar face of a polyhedral (mesh-only) body onto `target`. Mesh edges are
 * straight lines, so each loop projects exactly to a closed polyline on any plane; a loop that collapses (the face
 * seen edge-on) is refused with `DIRECT_SKETCH_PROJECTION_INPUT`.
 */
export function projectPolygonFaceToPlane(loops: readonly (readonly Vec3[])[], target: SketchPlane): ProjectedSketch[] {
  if (!loops.length) throw refuseInput('The face has no boundary to project.')
  return loops.map(loop => closedPolyline(loop, target))
}

/** Rational B-spline edge curve (the subset of `NurbsCurve` used for classification). */
export interface ProjectionEdgeCurve {
  degree: number
  knots: number[]
  controlPoints: number[][]
  weights: number[]
}

/** Rational B-spline point by Cox-de Boor, enough to certify edge shapes. */
function evaluate(curve: ProjectionEdgeCurve, u: number): Vec3 {
  const { degree: p, knots, controlPoints, weights } = curve
  const n = controlPoints.length
  let span = p
  while (span < n - 1 && u >= knots[span + 1]!) span++
  const left: number[] = [],
    right: number[] = [],
    basis = [1]
  for (let j = 1; j <= p; j++) {
    left[j] = u - knots[span + 1 - j]!
    right[j] = knots[span + j]! - u
    let saved = 0
    for (let r = 0; r < j; r++) {
      const temp = basis[r]! / (right[r + 1]! + left[j - r]!)
      basis[r] = saved + right[r + 1]! * temp
      saved = left[j - r]! * temp
    }
    basis[j] = saved
  }
  const out = [0, 0, 0, 0]
  for (let i = 0; i <= p; i++) {
    const k = span - p + i,
      w = weights[k]! * basis[i]!
    for (let d = 0; d < 3; d++) out[d]! += controlPoints[k]![d]! * w
    out[3]! += w
  }
  return [out[0]! / out[3]!, out[1]! / out[3]!, out[2]! / out[3]!]
}

type EdgePiece = { kind: 'line'; from: Vec3; to: Vec3 } | { kind: 'arc'; samples: Vec3[] }

/**
 * Classifies one edge curve as straight pieces (degree 1, or every control point on the chord) or a candidate
 * circular arc (degree 2 rational). The arc is certified later: each rational quadratic span is a conic, and a conic
 * through five points of a circle is that circle, so seven checked points per span prove it. Anything else is not
 * exact.
 */
function edgePieces(curve: ProjectionEdgeCurve, from: Vec3, to: Vec3, tolerance: number): EdgePiece[] {
  const { degree, knots, controlPoints, weights } = curve
  const n = controlPoints.length
  if (
    !Number.isInteger(degree) ||
    degree < 1 ||
    n <= degree ||
    knots.length !== n + degree + 1 ||
    weights.length !== n ||
    !weights.every(w => w > 0) ||
    !controlPoints.every(c => c.length >= 3 && c.slice(0, 3).every(Number.isFinite))
  )
    throw refuseNotExact('A face edge is not a supported curve.')
  const cps = controlPoints.map(c => [c[0]!, c[1]!, c[2]!] as Vec3)
  if (degree === 1) {
    const pts = [from, ...cps.slice(1, -1), to]
    return pts.slice(1).map((q, i) => ({ kind: 'line' as const, from: pts[i]!, to: q }))
  }
  const chord = sub(to, from),
    len = Math.hypot(...chord)
  if (
    len > tolerance &&
    cps.every(c => {
      const d = sub(c, from)
      const t = dot3(d, chord) / len
      return Math.hypot(...cross3(d, chord)) / len <= tolerance && t >= -tolerance && t <= len + tolerance
    })
  )
    return [{ kind: 'line', from, to }]
  if (degree !== 2) throw refuseNotExact('A face edge is neither a line nor a circular arc.')
  const lo = knots[degree]!,
    hi = knots[n]!
  const breaks = [...new Set(knots.filter(k => k >= lo && k <= hi))].sort((a, b) => a - b)
  const samples: Vec3[] = [from]
  for (let i = 0; i + 1 < breaks.length; i++)
    for (let j = 1; j <= 6; j++) samples.push(evaluate(curve, breaks[i]! + ((breaks[i + 1]! - breaks[i]!) * j) / 6))
  if (samples.length < 7) throw refuseNotExact('A face edge is neither a line nor a circular arc.')
  // The evaluated end must agree with the topological vertex before the vertex replaces it.
  if (dist3(samples.at(-1)!, to) > tolerance * Math.max(1, len)) throw refuseInput('A face edge misses its vertex.')
  samples[samples.length - 1] = to
  return [{ kind: 'arc', samples }]
}

function circleThrough(a: Vec3, b: Vec3, c: Vec3): { center: Vec3; radius: number; normal: Vec3 } | null {
  const ab = sub(b, a),
    ac = sub(c, a),
    normal = cross3(ab, ac),
    nn = dot3(normal, normal)
  if (!(nn > 1e-24)) return null
  const t1 = cross3(normal, ab),
    t2 = cross3(ac, normal)
  const k1 = dot3(ac, ac) / (2 * nn),
    k2 = dot3(ab, ab) / (2 * nn)
  const center: Vec3 = [0, 1, 2].map(i => a[i]! + t1[i]! * k1 + t2[i]! * k2) as Vec3
  return { center, radius: dist3(center, a), normal }
}

function projectArc(samples: readonly Vec3[], target: SketchPlane, tolerance: number): ProjectedSketch {
  const first = samples[0]!,
    last = samples.at(-1)!
  const closed = dist3(first, last) <= tolerance
  const mid = samples[Math.floor(samples.length / 2)]!
  const circle = closed
    ? circleThrough(first, samples[Math.floor(samples.length / 3)]!, samples[Math.floor((2 * samples.length) / 3)]!)
    : circleThrough(first, mid, last)
  if (!circle) throw refuseNotExact('A face edge is neither a line nor a circular arc.')
  const scale = Math.max(1, circle.radius),
    nLen = Math.hypot(...circle.normal)
  for (const p of samples)
    if (
      Math.abs(dist3(p, circle.center) - circle.radius) > tolerance * scale ||
      Math.abs(dot3(sub(p, circle.center), circle.normal)) / nLen > tolerance * scale
    )
      throw refuseNotExact('A face edge is neither a line nor a circular arc.')
  const center = local(circle.center, target)
  const angle = (p: Vec3) => {
    const q = local(p, target)
    return (Math.atan2(q[1] - center[1], q[0] - center[0]) * 180) / Math.PI
  }
  let analytic: AnalyticCurve
  if (closed) analytic = { kind: 'circle', center, radius: circle.radius, start: 0, sweep: 360 }
  else {
    const start = angle(first),
      ccw = (a: number) => (((a - start) % 360) + 360) % 360
    const end = ccw(angle(last)),
      through = ccw(angle(mid))
    analytic = { kind: 'arc', center, radius: circle.radius, start, sweep: through < end ? end : end - 360 }
  }
  if (!(circle.radius >= 0.01) || !finite2([center])) throw refuseInput('A projected arc is degenerate.')
  return { points: sampleCurve(analytic), closed, analytic, plane: copyPlane(target) }
}

/** The B-rep topology a face projection reads (a structural subset of `NurbsBrep`). */
export interface ProjectionBrep {
  vertices: { point: [number, number, number] }[]
  edges: { vertices: [number, number]; curve: ProjectionEdgeCurve }[]
  loops: { coedges: { edge: number; reversed: boolean }[] }[]
  faces: { outer: number; holes: number[] }[]
}

/**
 * Projects the edges of planar B-rep face `face` onto `target`. Line edges project exactly on any plane and are
 * joined into polylines (a loop of lines becomes a closed polyline); a circular-arc edge projects exactly only when
 * the face is parallel to the target and becomes its own arc or circle sketch. An arc on a non-parallel face, any
 * other curve and a non-planar face are refused with `DIRECT_SKETCH_PROJECTION_NOT_EXACT`.
 */
export function projectBrepFaceToPlane(
  brep: ProjectionBrep,
  face: number,
  target: SketchPlane,
  tolerance = 1e-7,
): ProjectedSketch[] {
  const f = brep.faces[face]
  if (!f) throw refuseInput('The selected face is unavailable.')
  const all = [f.outer, ...f.holes].map(i => brep.loops[i])
  const facePoints: Vec3[] = []
  const loops = all.map(loop => {
    if (!loop?.coedges.length) throw refuseInput('The selected face has an empty loop.')
    return loop.coedges.flatMap(ce => {
      const e = brep.edges[ce.edge]
      if (!e) throw refuseInput('A face edge is unavailable.')
      const va = brep.vertices[e.vertices[0]]?.point,
        vb = brep.vertices[e.vertices[1]]?.point
      if (!va || !vb) throw refuseInput('A face vertex is unavailable.')
      const pieces = edgePieces(e.curve, [...va] as Vec3, [...vb] as Vec3, tolerance)
      for (const piece of pieces) facePoints.push(...(piece.kind === 'line' ? [piece.from, piece.to] : piece.samples))
      if (!ce.reversed) return pieces
      return pieces
        .reverse()
        .map(piece =>
          piece.kind === 'line'
            ? { kind: 'line' as const, from: piece.to, to: piece.from }
            : { kind: 'arc' as const, samples: [...piece.samples].reverse() },
        )
    })
  })
  // The face must be planar: every boundary point lies on the plane through the most spread boundary points.
  const p0 = facePoints[0]!
  const far = facePoints.reduce((best, p) => (dist3(p, p0) > dist3(best, p0) ? p : best), p0)
  let normal: Vec3 = [0, 0, 0]
  for (const p of facePoints) {
    const n = cross3(sub(far, p0), sub(p, p0))
    if (Math.hypot(...n) > Math.hypot(...normal)) normal = n
  }
  const nLen = Math.hypot(...normal)
  if (!(nLen > 1e-18)) throw refuseInput('The selected face is degenerate.')
  if (facePoints.some(p => Math.abs(dot3(sub(p, p0), normal)) / nLen > tolerance))
    throw refuseNotExact('The selected face is not planar.')
  const m = cross3(target.u, target.v)
  const isParallel = Math.hypot(...cross3(normal, m)) <= PROJECTION_PARALLEL_TOLERANCE * nLen * Math.hypot(...m)
  const out: ProjectedSketch[] = []
  for (const loop of loops) {
    const onlyLines = loop.every(piece => piece.kind === 'line')
    let run: Vec3[] = []
    const flush = () => {
      if (run.length >= 2) out.push(onlyLines ? closedPolyline(run, target) : openPolyline(run, target))
      run = []
    }
    for (const piece of loop) {
      if (piece.kind === 'line') {
        if (!run.length) run.push(piece.from)
        run.push(piece.to)
        continue
      }
      flush()
      if (!isParallel)
        throw refuseNotExact(
          'A circular face edge on a non-parallel plane projects to an ellipse; refusing an approximation.',
        )
      out.push(projectArc(piece.samples, target, tolerance))
    }
    flush()
  }
  return out
}
