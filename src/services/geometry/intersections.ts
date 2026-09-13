import {callGeometryRust} from './kernel'
import type {NurbsCurve} from '../nurbsCurve'
import type {NurbsSurface} from '../nurbsSurface'

type Vec2 = [number,number]
type Vec3 = [number,number,number]
export interface IntersectionPlane {normal:Vec3;offset:number}
export interface IntersectionOptions {
 distanceTolerance?:number
 parameterTolerance?:number
 maxDepth?:number
 maxBoxes?:number
}
export interface IntersectionReport<T> {
 components:T[]
 unresolved:{parameterBox:number[];reason:'budget_exceeded'|'tangency_or_multiple_root'|'near_coincidence'|'boundary_crossing'|'unsupported_surface'|'coincident_trim'}[]
 boxesVisited:number
 bernsteinExcluded:number
 coverage:'numerically_resolved'|'incomplete'
 permitsTopologyChange:false
 evidence:'numerical_uncertified'
}
export interface IntersectionCurvePoint {
 parameter:number
 parameterInterval:Vec2
 point:Vec3
 planeResidual:number
 contact:'transverse'|'boundary'
}
export interface IntersectionSurfacePoint {uv:Vec2;point:Vec3;planeResidual:number}
/** Retained procedural geometry, never a fit through the report's samples. */
export type IntersectionSurfaceTrace =
 | {kind:'line';surface:NurbsSurface;plane:IntersectionPlane;start:Vec2;end:Vec2}
 | {kind:'ruled_u';surface:NurbsSurface;plane:IntersectionPlane;vInterval:Vec2}
 | {kind:'ruled';surface:NurbsSurface;plane:IntersectionPlane;uInterval:Vec2}
export type CurvePlaneIntersection =
 | {kind:'point';curve:IntersectionCurvePoint}
 | {kind:'overlap';parameterInterval:Vec2;controlResidual:number}
export type SurfacePlaneIntersection =
 | {kind:'curve';trace:IntersectionSurfaceTrace;parameterBox:[number,number,number,number];samples:IntersectionSurfacePoint[];maxSampleResidual:number}
 | {kind:'point';surface:IntersectionSurfacePoint}
 | {kind:'overlap';parameterBox:[number,number,number,number];controlResidual:number}
export type CurveSurfaceIntersection =
 | {kind:'point';curve:IntersectionCurvePoint;uv:Vec2;surfaceResidual:number}
 | {kind:'overlap';curveInterval:Vec2}

/** Bounded Bernstein root isolation. Tangencies and exhausted budgets remain unresolved. */
export const intersectNurbsCurvePlane=(curve:NurbsCurve,plane:IntersectionPlane,options:IntersectionOptions={}):IntersectionReport<CurvePlaneIntersection>=>callGeometryRust('brep_intersect_curve_plane',{curve,plane,options})
/** Untrimmed affine/ruled patch sections. This does not sew a B-rep section or certify completeness. */
export const intersectNurbsSurfacePlane=(surface:NurbsSurface,plane:IntersectionPlane,options:IntersectionOptions={}):IntersectionReport<SurfacePlaneIntersection>=>callGeometryRust('brep_intersect_surface_plane',{surface,plane,options})
/** Admits affine rectangular planar support surfaces; curved targets report unsupported regions. */
export const intersectNurbsCurveSurface=(curve:NurbsCurve,surface:NurbsSurface,options:IntersectionOptions={}):IntersectionReport<CurveSurfaceIntersection>=>callGeometryRust('brep_intersect_curve_surface',{curve,surface,options})
export const evaluateIntersectionTrace=(trace:IntersectionSurfaceTrace,fraction:number):IntersectionSurfacePoint=>callGeometryRust('brep_intersection_trace_evaluate',{trace,fraction})

export type CurveSegmentIntersection =
 | {kind:'point';curve:IntersectionCurvePoint;segmentParameter:number;lineResidual:number}
 | {kind:'overlap';curveInterval:Vec2}
/** Finite 3D segment query; degree-one overlap clipping retains rational source parameters. */
export const intersectNurbsCurveSegment=(curve:NurbsCurve,start:Vec3,end:Vec3,options:IntersectionOptions={}):IntersectionReport<CurveSegmentIntersection>=>callGeometryRust('brep_intersect_curve_segment',{curve,start,end,options})

export type SurfaceSurfaceIntersection =
 | {kind:'overlap';firstBoundary:Vec2[];secondBoundary:Vec2[];points:Vec3[];maxSampleResidual:number}
 | {kind:'point';first:IntersectionSurfacePoint;second:IntersectionSurfacePoint;residual:number}
 | {kind:'curve';first:IntersectionSurfaceTrace;second:IntersectionSurfaceTrace;maxSampleResidual:number}
/** Finite affine patches; paired traces use the same fraction. Curved pairs remain unresolved. */
export const intersectNurbsSurfaceSurface=(first:NurbsSurface,second:NurbsSurface,options:IntersectionOptions={}):IntersectionReport<SurfaceSurfaceIntersection>=>callGeometryRust('brep_intersect_surface_surface',{first,second,options})

/** Algebraic conversion without fitting samples; UV lines may lift to curved 3D geometry.
 * Conversion does not certify plane membership or authorize topology changes. */
export const intersectionTraceToNurbsCurve=(trace:IntersectionSurfaceTrace):NurbsCurve=>callGeometryRust('brep_intersection_trace_curve',{trace})
/** Separate numerical pieces; their knot domains use the original trace fraction. No sewing implied. */
export const intersectionTraceToNurbsCurveSegments=(trace:IntersectionSurfaceTrace):NurbsCurve[]=>callGeometryRust('brep_intersection_trace_curve_segments',{trace})
