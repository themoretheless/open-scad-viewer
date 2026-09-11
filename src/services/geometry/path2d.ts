import { callGeometryRust } from './kernel'
import type { Vec2 } from './module'

export type PathPoint = Vec2
export type PathSegmentJson =
  | { type: 'line'; to: PathPoint }
  | { type: 'cubic'; c1: PathPoint; c2: PathPoint; to: PathPoint }
export type BezierPathJson = {
  start: PathPoint
  segments: PathSegmentJson[]
  closed: boolean
}
export type BBoxJson = { min: PathPoint; max: PathPoint }

const call = <T>(action: string, args: object = {}): T =>
  callGeometryRust<T>('path2d', { action, ...args })

export const pathFromRect = (min: PathPoint, max: PathPoint) =>
  call<BezierPathJson>('from_rect', { min, max })
export const pathFromCircle = (center: PathPoint, radius: number) =>
  call<BezierPathJson>('from_circle', { center, radius })
export const pathFromPolygon = (points: PathPoint[], closed = true) =>
  call<BezierPathJson>('from_polygon', { points, closed })
export const flattenPath = (path: BezierPathJson, tolerance = 0.25) =>
  call<PathPoint[]>('flatten', { path, tolerance })
export const pathToRing = (path: BezierPathJson, tolerance = 0.25) =>
  call<PathPoint[]>('to_ring', { path, tolerance })
export const insertPathAnchor = (path: BezierPathJson, index: number, t: number) =>
  call<BezierPathJson>('insert_anchor', { path, index, t })
export const deletePathAnchor = (path: BezierPathJson, node: number) =>
  call<BezierPathJson>('delete_anchor', { path, node })
export const reversePath = (path: BezierPathJson) => call<BezierPathJson>('reverse', { path })
export const simplifyPath = (path: BezierPathJson, tolerance: number) =>
  call<BezierPathJson>('simplify', { path, tolerance })
export const joinPaths = (
  head: BezierPathJson,
  tail: BezierPathJson,
  opts: { reverseHead?: boolean; reverseTail?: boolean; weld?: number } = {},
) =>
  call<BezierPathJson>('join', {
    head,
    tail,
    reverseHead: opts.reverseHead ?? false,
    reverseTail: opts.reverseTail ?? false,
    weld: opts.weld ?? 1e-6,
  })
export const outlineStroke = (path: BezierPathJson, width: number) =>
  call<BezierPathJson>('outline_stroke', { path, width })
export const outlineStrokeStyled = (
  path: BezierPathJson,
  width: number,
  opts: {
    cap?: 'butt' | 'round' | 'square'
    join?: 'miter' | 'round' | 'bevel'
    miterLimit?: number
    dash?: number[]
    dashOffset?: number
  } = {},
) =>
  call<BezierPathJson[]>('outline_stroke_styled', {
    path,
    width,
    cap: opts.cap ?? 'butt',
    join: opts.join ?? 'miter',
    miterLimit: opts.miterLimit ?? 4,
    dash: opts.dash,
    dashOffset: opts.dashOffset ?? 0,
  })
export const averageAnchors = (
  path: BezierPathJson,
  nodes: number[],
  axis: 'horizontal' | 'vertical' | 'both' = 'both',
) => call<BezierPathJson>('average_anchors', { path, nodes, axis })
export const setAnchorHandle = (
  path: BezierPathJson,
  node: number,
  side: 'in' | 'out',
  pos: PathPoint,
) => call<BezierPathJson>('set_anchor_handle', { path, node, side, pos })
export const applyHandleLink = (
  path: BezierPathJson,
  node: number,
  moved: 'in' | 'out',
  mode: 'free' | 'mirrored' | 'symmetric',
) => call<BezierPathJson>('apply_handle_link', { path, node, moved, mode })
export const roundCornersPath = (path: BezierPathJson, radius: number) =>
  call<BezierPathJson>('round_corners', { path, radius })
export const roundedRectPath = (
  min: PathPoint,
  max: PathPoint,
  radii: number[],
  styles: Array<'round' | 'chamfer' | 'inverted' | 'notch'> = [],
) => call<BezierPathJson>('rounded_rect', { min, max, radii, styles })
export const roundedPolygonPath = (
  points: PathPoint[],
  radii: number[],
  styles: Array<'round' | 'chamfer' | 'inverted' | 'notch'> = [],
) => call<BezierPathJson>('rounded_polygon', { points, radii, styles })
export const concentricOffsetRings = (
  rings: PathPoint[][],
  count: number,
  step: number,
  join = 'Miter',
  segments = 8,
) => call<PathPoint[][]>('concentric_offset', { rings, count, step, join, segments })

export const spiralPath = (
  min: PathPoint,
  max: PathPoint,
  turns: number,
  innerRatio = 0,
  clockwise = true,
) => call<BezierPathJson>('spiral', { min, max, turns, innerRatio, clockwise })
export const polarGridPaths = (
  min: PathPoint,
  max: PathPoint,
  circles: number,
  spokes: number,
  innerRatio = 0,
  fullSpokes = true,
) => call<BezierPathJson[]>('polar_grid', { min, max, circles, spokes, innerRatio, fullSpokes })
export const stepAndRepeat = (path: BezierPathJson, count: number, delta: PathPoint) =>
  call<BezierPathJson[]>('step_and_repeat', { path, count, delta })
export const radialRepeat = (path: BezierPathJson, count: number, center: PathPoint) =>
  call<BezierPathJson[]>('radial_repeat', { path, count, center })
export const gridArray = (path: BezierPathJson, rows: number, cols: number, spacing: PathPoint) =>
  call<BezierPathJson[]>('grid_array', { path, rows, cols, spacing })
export const hatchRing = (ring: PathPoint[], spacing: number, angle = 0, cross = false) =>
  call<BezierPathJson[]>('hatch', { ring, spacing, angle, cross })
export const stippleRing = (
  ring: PathPoint[],
  spacing: number,
  size: number,
  kind: 'dot' | 'ring' | 'cross' | 'square' = 'dot',
) => call<BezierPathJson[]>('stipple', { ring, spacing, size, kind })
export const zigZagPath = (path: BezierPathJson, amplitude: number, wavelength: number) =>
  call<BezierPathJson>('zig_zag', { path, amplitude, wavelength })
export const puckerBloatPath = (path: BezierPathJson, amount: number) =>
  call<BezierPathJson>('pucker_bloat', { path, amount })
export const roughenPath = (path: BezierPathJson, amount: number, seed = 1) =>
  call<BezierPathJson>('roughen', { path, amount, seed })
export const twistPath = (path: BezierPathJson, radians: number) =>
  call<BezierPathJson>('twist', { path, radians })
export const scatterPath = (path: BezierPathJson, count: number, radius: number, seed = 1) =>
  call<BezierPathJson[]>('scatter', { path, count, radius, seed })
export const blendPaths = (a: BezierPathJson, b: BezierPathJson, steps: number) =>
  call<BezierPathJson[]>('blend', { a, b, steps })
export const freeDistortPath = (path: BezierPathJson, quad: [PathPoint, PathPoint, PathPoint, PathPoint]) =>
  call<BezierPathJson>('free_distort', { path, quad })
export const arcPath = (
  min: PathPoint,
  max: PathPoint,
  start: number,
  sweep: number,
  mode: 'arc' | 'pie' | 'segment' = 'arc',
) => call<BezierPathJson>('arc', { min, max, start, sweep, mode })
export const starPath = (min: PathPoint, max: PathPoint, points: number, innerRatio = 0.4) =>
  call<BezierPathJson>('star', { min, max, points, innerRatio })

export const findPathSnap = (
  cursor: PathPoint,
  threshold: number,
  paths: BezierPathJson[],
  grid?: number,
) =>
  call<{ point: PathPoint; kind: string; distance: number } | null>('snap', {
    cursor,
    threshold,
    paths,
    grid,
  })
export const alignBoxes = (
  boxes: BBoxJson[],
  reference: BBoxJson,
  h?: 'left' | 'center' | 'right',
  v?: 'top' | 'center' | 'bottom',
) => call<PathPoint[]>('align', { boxes, reference, h, v })
export const distributeBoxes = (boxes: BBoxJson[], horizontal = true) =>
  call<PathPoint[]>('distribute', { boxes, horizontal })
export const scissorsCutPath = (path: BezierPathJson, click: PathPoint, maxDist: number) =>
  call<BezierPathJson[]>('scissors', { path, click, maxDist })
export const knifeCutPath = (path: BezierPathJson, a: PathPoint, b: PathPoint) =>
  call<BezierPathJson[]>('knife', { path, a, b })
export const measurePoints = (a: PathPoint, b: PathPoint) =>
  call<{ a: PathPoint; b: PathPoint; distance: number; angleDeg: number; delta: PathPoint }>(
    'measure',
    { a, b },
  )

export const offsetPath = (
  path: BezierPathJson,
  distance: number,
  join = 'Miter',
  segments = 8,
) => call<BezierPathJson[]>('offset', { path, distance, join, segments })
export const smoothPath = (path: BezierPathJson) => call<BezierPathJson>('smooth', { path })
export const openPath = (path: BezierPathJson) => call<BezierPathJson>('open_path', { path })
export const deletePathSegments = (path: BezierPathJson, nodes: number[]) =>
  call<BezierPathJson[]>('delete_segments', { path, nodes })
export const mergeByColor = (shapes: Array<{ rings: PathPoint[][]; color: [number, number, number, number] }>) =>
  call<Array<{ rings: PathPoint[][]; color: [number, number, number, number] }>>('merge_by_color', { shapes })
export const snapToAngle = (start: PathPoint, end: PathPoint, stepDeg = 45) =>
  call<PathPoint>('snap_angle', { start, end, stepDeg })
export const dragAlignGuides = (moving: BBoxJson, targets: BBoxJson[], threshold: number) =>
  call<{ delta: PathPoint; guides: Array<{ vertical: boolean; position: number }> }>(
    'drag_guides',
    { moving, targets, threshold },
  )
export const alignBoxesRelative = (
  boxes: BBoxJson[],
  relative: 'selection' | 'key' | 'page' | 'last',
  opts: {
    key?: number
    last?: number
    page?: BBoxJson
    h?: 'left' | 'center' | 'right'
    v?: 'top' | 'center' | 'bottom'
  } = {},
) =>
  call<PathPoint[]>('align_relative', {
    boxes,
    relative,
    key: opts.key,
    last: opts.last,
    page: opts.page,
    h: opts.h,
    v: opts.v,
  })
export const distributeObjects = (
  boxes: BBoxJson[],
  horizontal = true,
  anchor: 'center' | 'start' | 'end' = 'center',
) => call<PathPoint[]>('distribute_objects', { boxes, horizontal, anchor })
export const distributeSpacing = (boxes: BBoxJson[], gap: number, horizontal = true) =>
  call<PathPoint[]>('distribute_spacing', { boxes, horizontal, gap })
export const pathArrowMarkers = (
  path: BezierPathJson,
  width: number,
  start: 'none' | 'arrow' | 'dot' | 'bar' = 'none',
  end: 'none' | 'arrow' | 'dot' | 'bar' = 'none',
) => call<BezierPathJson[]>('arrow_markers', { path, width, start, end })
export const recolorRgba = (
  color: [number, number, number, number],
  hue: number,
  sat: number,
  light: number,
) => call<[number, number, number, number]>('recolor', { color, hue, sat, light })
export const sampleGradient = (
  kind: 'linear' | 'radial' | 'conic',
  point: PathPoint,
  stops: Array<{ offset: number; color: [number, number, number, number] }>,
  opts: {
    p1?: PathPoint
    p2?: PathPoint
    center?: PathPoint
    radius?: number
    angle?: number
    spread?: 'pad' | 'repeat' | 'reflect'
  } = {},
) =>
  call<[number, number, number, number]>('sample_gradient', {
    kind,
    point,
    stops,
    p1: opts.p1,
    p2: opts.p2,
    center: opts.center,
    radius: opts.radius,
    angle: opts.angle,
    spread: opts.spread ?? 'pad',
  })
