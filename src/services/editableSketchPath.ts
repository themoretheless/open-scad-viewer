import type {DirectSketch, Point2} from './directModeling'
import {flattenPath, pathToRing, type BezierPathJson} from './geometry/path2d'

export const SKETCH_PATH_TOLERANCE_MM = 0.02

/** Retain authored curves; samples are an explicit polygon/display approximation. */
export function withEditableSketchPath(sketch: DirectSketch, path: BezierPathJson): DirectSketch {
  if (sketch.retainedProfile || sketch.analytic || sketch.dimensions?.length) {
    throw Error('Editable paths cannot carry a second profile definition or polygon dimensions.')
  }
  if (!path || typeof path.closed !== 'boolean' || !Array.isArray(path.segments) ||
      path.segments.length < 1 || path.segments.length > 128) throw Error('Editable paths require 1–128 segments.')
  const validPt = (p: unknown): boolean =>
    Array.isArray(p) && p.length === 2 &&
    typeof p[0] === 'number' && Number.isFinite(p[0]) && Math.abs(p[0]) <= 1e6 &&
    typeof p[1] === 'number' && Number.isFinite(p[1]) && Math.abs(p[1]) <= 1e6
  if (!validPt(path.start)) throw Error('Invalid editable path control point.')
  for (let i = 0; i < path.segments.length; i++) {
    const s = path.segments[i]
    if (!s || (s.type === 'cubic' ? (!validPt(s.c1) || !validPt(s.c2) || !validPt(s.to)) : !validPt(s.to))) {
      throw Error('Invalid editable path control point.')
    }
  }
  const points = (path.closed ? pathToRing(path, SKETCH_PATH_TOLERANCE_MM) : flattenPath(path, SKETCH_PATH_TOLERANCE_MM)) as Point2[]
  if (points.length > 8192 || points.length < (path.closed ? 3 : 2)) throw Error('Editable path exceeds the sketch sample budget or is degenerate.')
  return {...sketch, editablePath: structuredClone(path), points, closed: path.closed}
}

/** Topological anchors, never the display tessellation vertices. */
export function editablePathAnchors(path: BezierPathJson): Point2[] {
  const end = path.closed ? Math.max(0, path.segments.length - 1) : path.segments.length
  const out = new Array<Point2>(1 + end)
  out[0] = path.start as Point2
  for (let i = 0; i < end; i++) out[i + 1] = path.segments[i].to as Point2
  return out
}
