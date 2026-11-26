import { callGeometryRust } from './kernel'
import type { Vec3 } from './math3d'
import type { NurbsCurve } from '../nurbsCurve'
import type { NurbsSurface } from '../nurbsSurface'

type Vec2 = [number, number]
export interface IntersectionPlane {
  normal: Vec3
  offset: number
}
export interface IntersectionOptions {
  distanceTolerance?: number
  parameterTolerance?: number
  maxDepth?: number
  maxBoxes?: number
}
export interface IntersectionReport<T> {
  components: T[]
  unresolved: {
    parameterBox: number[]
    reason:
      | 'budget_exceeded'
      | 'tangency_or_multiple_root'
      | 'near_coincidence'
      | 'boundary_crossing'
      | 'unsupported_surface'
      | 'coincident_trim'
  }[]
  boxesVisited: number
  bernsteinExcluded: number
  coverage: 'numerically_resolved' | 'incomplete'
  permitsTopologyChange: false
  evidence: 'numerical_uncertified'
}
export interface IntersectionCurvePoint {
  parameter: number
  parameterInterval: Vec2
  point: Vec3
  planeResidual: number
  contact: 'transverse' | 'boundary' | 'tangent'
  /** Certified root multiplicity: 1 for transverse and boundary contacts, at least 2 for a certified tangent contact. */
  multiplicity?: number
}
export interface IntersectionSurfacePoint {
  uv: Vec2
  point: Vec3
  planeResidual: number
}
/** Retained procedural geometry, never a fit through the report's samples. */
export type IntersectionSurfaceTrace =
  | { kind: 'line'; surface: NurbsSurface; plane: IntersectionPlane; start: Vec2; end: Vec2 }
  | { kind: 'ruled_u'; surface: NurbsSurface; plane: IntersectionPlane; vInterval: Vec2 }
  | { kind: 'ruled'; surface: NurbsSurface; plane: IntersectionPlane; uInterval: Vec2 }
export type CurvePlaneIntersection =
  | { kind: 'point'; curve: IntersectionCurvePoint }
  | { kind: 'overlap'; parameterInterval: Vec2; controlResidual: number }
export type SurfacePlaneIntersection =
  | {
      kind: 'curve'
      trace: IntersectionSurfaceTrace
      parameterBox: [number, number, number, number]
      samples: IntersectionSurfacePoint[]
      maxSampleResidual: number
    }
  | { kind: 'point'; surface: IntersectionSurfacePoint }
  | { kind: 'overlap'; parameterBox: [number, number, number, number]; controlResidual: number }
export type CurveSurfaceIntersection =
  | { kind: 'point'; curve: IntersectionCurvePoint; uv: Vec2; surfaceResidual: number }
  | { kind: 'overlap'; curveInterval: Vec2 }

/** Bounded Bernstein root isolation. Tangencies and exhausted budgets remain unresolved. */
export const intersectNurbsCurvePlane = (
  curve: NurbsCurve,
  plane: IntersectionPlane,
  options: IntersectionOptions = {},
): IntersectionReport<CurvePlaneIntersection> =>
  callGeometryRust('brep_intersect_curve_plane', { curve, plane, options })
/** Untrimmed affine/ruled patch sections. This does not sew a B-rep section or certify completeness. */
export const intersectNurbsSurfacePlane = (
  surface: NurbsSurface,
  plane: IntersectionPlane,
  options: IntersectionOptions = {},
): IntersectionReport<SurfacePlaneIntersection> =>
  callGeometryRust('brep_intersect_surface_plane', { surface, plane, options })
/** Admits affine rectangular planar support surfaces; curved targets report unsupported regions. */
export const intersectNurbsCurveSurface = (
  curve: NurbsCurve,
  surface: NurbsSurface,
  options: IntersectionOptions = {},
): IntersectionReport<CurveSurfaceIntersection> =>
  callGeometryRust('brep_intersect_curve_surface', { curve, surface, options })
export const evaluateIntersectionTrace = (
  trace: IntersectionSurfaceTrace,
  fraction: number,
): IntersectionSurfacePoint => callGeometryRust('brep_intersection_trace_evaluate', { trace, fraction })

export type CurveSegmentIntersection =
  | { kind: 'point'; curve: IntersectionCurvePoint; segmentParameter: number; lineResidual: number }
  | { kind: 'overlap'; curveInterval: Vec2 }
/** Finite 3D segment query; degree-one overlap clipping retains rational source parameters. */
export const intersectNurbsCurveSegment = (
  curve: NurbsCurve,
  start: Vec3,
  end: Vec3,
  options: IntersectionOptions = {},
): IntersectionReport<CurveSegmentIntersection> =>
  callGeometryRust('brep_intersect_curve_segment', { curve, start, end, options })

export type CurveCurveIntersection =
  | {
      kind: 'point'
      first: number
      firstInterval: Vec2
      second: number
      secondInterval: Vec2
      point: Vec3
      residual: number
      contact: 'transverse' | 'boundary'
    }
  | { kind: 'overlap'; firstInterval: Vec2; secondInterval: Vec2; reversed: boolean; maxControlResidual: number }
/** General positive-weight 3D curve/curve query. Tangencies, ambiguous coincidence clipping and exhausted budgets stay unresolved; never authorizes topology changes. */
export const intersectNurbsCurveCurve = (
  first: NurbsCurve,
  second: NurbsCurve,
  options: IntersectionOptions = {},
): IntersectionReport<CurveCurveIntersection> =>
  callGeometryRust('brep_intersect_curve_curve', { first, second, options })

export type CurveRuledSurfaceIntersection =
  | {
      kind: 'point'
      t: number
      tInterval: Vec2
      uv: Vec2
      uvBox: [number, number, number, number]
      point: Vec3
      residual: number
      contact: 'transverse' | 'boundary'
    }
  | {
      kind: 'overlap'
      curveInterval: Vec2
      uvStart: Vec2
      uvEnd: Vec2
      maxControlResidual: number
      /** True when the component was unified across an exactly closed U seam (canonical u=u_min, or the UV path wraps u_max->u_min). */
      seamWrap?: boolean
      /** Three (t,u,v) samples at curve-interval fractions 0, 1/2, 1 reconstructing the per-parameter map: a Möbius t->v cross-ratio map along a ruling ('mobius_v'), an affine t->u map along an iso-V ('affine_u'). Null where no single map spans a seam-merged interval. */
      correspondence?: { kind: 'mobius_v' | 'affine_u'; samples: [number, number, number][] } | null
    }
/** 3D curve against a ruled NURBS surface (rational in one direction, linear with positive endpoint weights in the other). Points carry (t,u,v) with isolating boxes; overlaps carry the curve trim, lifted UV path endpoints and a sampled per-parameter correspondence. Exactly closed U seams unify seam-side contacts to a canonical u=u_min representative; near-closed seams stay duplicate behind explicit near_coincidence bands. Tangencies, near-surface bands and exhausted budgets stay unresolved; never authorizes topology changes. */
export const intersectNurbsCurveRuledSurface = (
  curve: NurbsCurve,
  surface: NurbsSurface,
  options: IntersectionOptions = {},
): IntersectionReport<CurveRuledSurfaceIntersection> =>
  callGeometryRust('brep_intersect_curve_ruled_surface', { curve, surface, options })

export type SurfaceSurfaceIntersection =
  | { kind: 'overlap'; firstBoundary: Vec2[]; secondBoundary: Vec2[]; points: Vec3[]; maxSampleResidual: number }
  | { kind: 'point'; first: IntersectionSurfacePoint; second: IntersectionSurfacePoint; residual: number }
  | { kind: 'curve'; first: IntersectionSurfaceTrace; second: IntersectionSurfaceTrace; maxSampleResidual: number }
/** Finite affine patches; paired traces use the same fraction. Curved pairs remain unresolved. */
export const intersectNurbsSurfaceSurface = (
  first: NurbsSurface,
  second: NurbsSurface,
  options: IntersectionOptions = {},
): IntersectionReport<SurfaceSurfaceIntersection> =>
  callGeometryRust('brep_intersect_surface_surface', { first, second, options })

export * from './analyticIntersections'
