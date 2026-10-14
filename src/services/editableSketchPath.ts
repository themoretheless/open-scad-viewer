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
  const controlPoints = [path.start, ...path.segments.flatMap(s => s.type === 'cubic' ? [s.c1, s.c2, s.to] : [s.to])]
  if (!controlPoints.every(p => Array.isArray(p) && p.length === 2 && p.every(v => Number.isFinite(v) && Math.abs(v) <= 1e6))) {
    throw Error('Invalid editable path control point.')
  }
  const points = (path.closed ? pathToRing(path, SKETCH_PATH_TOLERANCE_MM) : flattenPath(path, SKETCH_PATH_TOLERANCE_MM)) as Point2[]
  if (points.length > 8192 || points.length < (path.closed ? 3 : 2)) throw Error('Editable path exceeds the sketch sample budget or is degenerate.')
  return {...sketch, editablePath: structuredClone(path), points, closed: path.closed}
}

/** Topological anchors, never the display tessellation vertices. */
export function editablePathAnchors(path: BezierPathJson): Point2[] {
  return [path.start, ...path.segments.slice(0, path.closed ? -1 : undefined).map(s => s.to)] as Point2[]
}
