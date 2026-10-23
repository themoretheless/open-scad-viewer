import {callGeometryRust} from './kernel'
import type {NurbsBrep} from './brep'
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

export type CurveCurveIntersection =
 | {kind:'point';first:number;firstInterval:Vec2;second:number;secondInterval:Vec2;point:Vec3;residual:number;contact:'transverse'|'boundary'}
 | {kind:'overlap';firstInterval:Vec2;secondInterval:Vec2;reversed:boolean;maxControlResidual:number}
/** General positive-weight 3D curve/curve query. Tangencies, ambiguous coincidence clipping and exhausted budgets stay unresolved; never authorizes topology changes. */
export const intersectNurbsCurveCurve=(first:NurbsCurve,second:NurbsCurve,options:IntersectionOptions={}):IntersectionReport<CurveCurveIntersection>=>callGeometryRust('brep_intersect_curve_curve',{first,second,options})

export type CurveRuledSurfaceIntersection =
 | {kind:'point';t:number;tInterval:Vec2;uv:Vec2;uvBox:[number,number,number,number];point:Vec3;residual:number;contact:'transverse'|'boundary'}
 | {kind:'overlap';curveInterval:Vec2;uvStart:Vec2;uvEnd:Vec2;maxControlResidual:number;
  /** True when the component was unified across an exactly closed U seam (canonical u=u_min, or the UV path wraps u_max->u_min). */
  seamWrap?:boolean;
  /** Three (t,u,v) samples at curve-interval fractions 0, 1/2, 1 reconstructing the per-parameter map: a Möbius t->v cross-ratio map along a ruling ('mobius_v'), an affine t->u map along an iso-V ('affine_u'). Null where no single map spans a seam-merged interval. */
  correspondence?:{kind:'mobius_v'|'affine_u';samples:[number,number,number][]}|null}
/** 3D curve against a ruled NURBS surface (rational in one direction, linear with positive endpoint weights in the other). Points carry (t,u,v) with isolating boxes; overlaps carry the curve trim, lifted UV path endpoints and a sampled per-parameter correspondence. Exactly closed U seams unify seam-side contacts to a canonical u=u_min representative; near-closed seams stay duplicate behind explicit near_coincidence bands. Tangencies, near-surface bands and exhausted budgets stay unresolved; never authorizes topology changes. */
export const intersectNurbsCurveRuledSurface=(curve:NurbsCurve,surface:NurbsSurface,options:IntersectionOptions={}):IntersectionReport<CurveRuledSurfaceIntersection>=>callGeometryRust('brep_intersect_curve_ruled_surface',{curve,surface,options})

export type SurfaceSurfaceIntersection =
 | {kind:'overlap';firstBoundary:Vec2[];secondBoundary:Vec2[];points:Vec3[];maxSampleResidual:number}
 | {kind:'point';first:IntersectionSurfacePoint;second:IntersectionSurfacePoint;residual:number}
 | {kind:'curve';first:IntersectionSurfaceTrace;second:IntersectionSurfaceTrace;maxSampleResidual:number}
/** Finite affine patches; paired traces use the same fraction. Curved pairs remain unresolved. */
export const intersectNurbsSurfaceSurface=(first:NurbsSurface,second:NurbsSurface,options:IntersectionOptions={}):IntersectionReport<SurfaceSurfaceIntersection>=>callGeometryRust('brep_intersect_surface_surface',{first,second,options})

/** One canonical sphere patch's share of the lifted intersection circle in that patch's UV. */export interface SpherePatchCircleLift {patch:number;arcs:NurbsCurve[]}
export type SphereSphereIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;firstUv:SpherePatchCircleLift[];secondUv:SpherePatchCircleLift[];maxSampleResidual:number}
/** Analytic sphere/sphere for canonical sphere solids only (eight stereographic patches, optionally rigidly placed). Non-canonical operands are explicit unsupported_surface regions, never a numerical fallback; separate and contained pairs resolve empty, coincident spheres report coincident_trim, and every tangency or within-error tangency band stays tangency_or_multiple_root — tangent contacts are never points. Never authorizes topology changes. */
export const intersectSphereSphere=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<SphereSphereIntersection>=>callGeometryRust('brep_intersect_sphere_sphere',{first,second,options})

/** One cylinder face's share of an intersection circle in that face's UV: an iso-v line on a side patch, exact quarter arcs on a cap. */export interface CylinderPatchCurveLift {patch:number;arcs:NurbsCurve[]}
export type SphereCylinderIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;sphereUv:SpherePatchCircleLift[];cylinderUv:CylinderPatchCurveLift[];maxSampleResidual:number}
/** Analytic sphere/cylinder for the canonical solids in the axial configuration only (sphere center certified on the cylinder axis; rigid affine placements admitted). Non-canonical operands and clearly off-axis pairs are explicit unsupported_surface regions — never a numerical fallback; near-axial offsets report near_coincidence, the r==R band reports coincident_trim, and every rim or cap-plane tangency stays tangency_or_multiple_root — tangent contacts are never points or guessed circles. Never authorizes topology changes. */
export const intersectSphereCylinder=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<SphereCylinderIntersection>=>callGeometryRust('brep_intersect_sphere_cylinder',{first,second,options})

export type CylinderCylinderIntersection =
 | {kind:'line';curve:NurbsCurve;start:Vec3;end:Vec3;direction:Vec3;contact:'transverse'|'boundary';firstUv:CylinderPatchCurveLift[];secondUv:CylinderPatchCurveLift[];maxSampleResidual:number}
/** Analytic cylinder/cylinder for the canonical solids with parallel axes only (coaxial included; rigid affine placements admitted). Non-canonical operands and clearly non-parallel pairs are explicit unsupported_surface regions — never a numerical fallback (the general pair is a quartic); near-parallel or near-coaxial offsets inside the recognition band report near_coincidence, never forced. A transverse parallel pair yields two exact straight rulings (degree-1 lines from the planar circle/circle section) clipped by both finite heights, with per-patch iso-u lifts on both side walls (seam rulings duplicated on both adjacent patches); clip endpoints sit on cap planes, so contact is 'boundary', and a band-thin clip degenerates to tangency_or_multiple_root. External/internal tangencies, the coaxial equal-radius coincident band and stacked cap-plane coincidences stay unresolved — tangent contacts are never guessed; coaxial unequal radii without a coincident cap plane resolve empty. Never authorizes topology changes. */
export const intersectCylinderCylinder=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<CylinderCylinderIntersection>=>callGeometryRust('brep_intersect_cylinder_cylinder',{first,second,options})

/** One plane patch's share of a lifted intersection curve in that patch's UV (exact rational arcs in the unit square). */export interface PlanePatchCurveLift {patch:number;arcs:NurbsCurve[]}
export type PlaneSphereIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;full:boolean;planeUv:PlanePatchCurveLift[];sphereUv:SpherePatchCircleLift[];maxSampleResidual:number}
/** Analytic plane/sphere for a canonical finite rectangular planar patch (one-face open model, exact bilinear affine surface, plane operand first) against a canonical sphere solid. The section is the exact circle with exact lifts: ellipse arcs in the plane UV, clipped stereographic arcs on the sphere patches. Non-canonical operands are explicit unsupported_surface regions, never a numerical fallback; misses resolve empty, every tangency (plane at distance R or the circle touching the patch boundary within the band) stays tangency_or_multiple_root — tangent contacts are never points. Never authorizes topology changes. */
export const intersectPlaneSphere=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<PlaneSphereIntersection>=>callGeometryRust('brep_intersect_plane_sphere',{first,second,options})

export type PlaneCylinderIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;full:boolean;planeUv:PlanePatchCurveLift[];cylinderUv:CylinderPatchCurveLift[];maxSampleResidual:number}
 | {kind:'line';curve:NurbsCurve;start:Vec3;end:Vec3;direction:Vec3;contact:'transverse'|'boundary';planeUv:PlanePatchCurveLift[];cylinderUv:CylinderPatchCurveLift[];maxSampleResidual:number}
 | {kind:'ellipse';curve:NurbsCurve;center:Vec3;semiMajor:number;semiMinor:number;major:Vec3;minor:Vec3;normal:Vec3;full:boolean;planeUv:PlanePatchCurveLift[];cylinderUv:CylinderPatchCurveLift[]|null;maxSampleResidual:number}
/** Analytic plane/cylinder for a canonical finite rectangular planar patch (plane operand first) against a canonical cylinder solid; side-surface contacts only (cap-disk chords are not components). Perpendicular planes yield the exact circle with iso-v side lifts; parallel planes yield two exact rulings with iso-u side lifts; oblique planes yield the exact ellipse (semi-minor R, semi-major R/|axis.n|) whose cylinder side lift is null — the unrolled ellipse has no low-degree exact rational UV form. Cap-plane coincidence reports coincident_trim, recognition-scale tilts report near_coincidence, and every tangency stays tangency_or_multiple_root — tangent contacts are never guessed. Never authorizes topology changes. */
export const intersectPlaneCylinder=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<PlaneCylinderIntersection>=>callGeometryRust('brep_intersect_plane_cylinder',{first,second,options})

export type SphereConeIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;sphereUv:SpherePatchCircleLift[];coneUv:CylinderPatchCurveLift[];maxSampleResidual:number}
/** Analytic sphere/cone (frustum) for the canonical solids in the axial configuration only (sphere center certified on the cone axis; rigid affine placements admitted; true-apex cones admitted, an equal-radius frustum is a cylinder and is refused). Non-canonical operands and clearly off-axis pairs are explicit unsupported_surface regions — never a numerical fallback (the general pair is a quartic); near-axial offsets report near_coincidence. Side contacts solve one exact quadratic in the axial offset: two distinct in-height roots yield exact circles of linearly interpolated ring radius with iso-v side lifts and per-patch sphere lifts, cap-plane crossings inside a ring disk yield exact cap-plane circles with UV-circle cap lifts, and provable misses resolve empty. The side tangency (double root), rim contacts, apex contacts and cap-plane touches stay tangency_or_multiple_root — tangent contacts are never points or guessed circles. Never authorizes topology changes. */
export const intersectSphereCone=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<SphereConeIntersection>=>callGeometryRust('brep_intersect_sphere_cone',{first,second,options})

export type ConeConeIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;firstUv:CylinderPatchCurveLift[];secondUv:CylinderPatchCurveLift[];maxSampleResidual:number}
/** Analytic cone/cone (frustum) for the canonical solids in the coaxial configuration only (axes parallel and coincident, either orientation — apex-to-apex and base-to-base anti-axial pairs included; rigid affine placements admitted; true-apex cones admitted, an equal-radius frustum is a cylinder and is refused). Non-canonical operands and clearly non-parallel or off-axis pairs are explicit unsupported_surface regions — never a numerical fallback (the general pair is a quartic); recognition-scale tilts and offsets report near_coincidence, never forced coaxial. In the shared axial coordinate the side contact solves one linear equation rho_1(s) == rho_2(s): different tapers yield at most one exact circle of radius rho(s*) when s* is certified strictly inside both height ranges (a root clipped by either finite height is honestly absent), with iso-v lifts on all four side patches of both cones; equal-taper profiles coincident over the overlap report coincident_trim (never a surface), clearly distinct profiles resolve empty, and equality at a single ring plane only stays tangency_or_multiple_root. A root within the band of any ring plane (rim contact), a radius collapsing into the band (apex meeting) and any two ring planes coinciding within the band (rim/rim, rim-on-cap, apex-on-cap) stay tangency_or_multiple_root — tangent contacts are never guessed circles. Never authorizes topology changes. */
export const intersectConeCone=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<ConeConeIntersection>=>callGeometryRust('brep_intersect_cone_cone',{first,second,options})

export type PlaneConeIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;full:boolean;planeUv:PlanePatchCurveLift[];coneUv:CylinderPatchCurveLift[];maxSampleResidual:number}
 | {kind:'line';curve:NurbsCurve;start:Vec3;end:Vec3;direction:Vec3;contact:'transverse'|'boundary';planeUv:PlanePatchCurveLift[];coneUv:CylinderPatchCurveLift[];maxSampleResidual:number}
 | {kind:'ellipse';curve:NurbsCurve;center:Vec3;semiMajor:number;semiMinor:number;major:Vec3;minor:Vec3;normal:Vec3;full:boolean;planeUv:PlanePatchCurveLift[];coneUv:CylinderPatchCurveLift[]|null;maxSampleResidual:number}
 | {kind:'parabola';curve:NurbsCurve;vertex:Vec3;direction:Vec3;focalLength:number;normal:Vec3;planeUv:PlanePatchCurveLift[];coneUv:CylinderPatchCurveLift[]|null;maxSampleResidual:number}
 | {kind:'hyperbola';curve:NurbsCurve;center:Vec3;semiTransverse:number;semiConjugate:number;transverse:Vec3;conjugate:Vec3;normal:Vec3;planeUv:PlanePatchCurveLift[];coneUv:CylinderPatchCurveLift[]|null;maxSampleResidual:number}
/** Analytic plane/cone (frustum) for a canonical finite rectangular planar patch (plane operand first) against a canonical conical frustum solid (true-apex cone admitted; an equal-radius frustum is a cylinder and is refused); side-surface contacts only. Perpendicular planes yield the exact circle with the radius linearly interpolated between the rings and iso-v side lifts; through-axis planes yield two exact rulings through the apex with iso-u side lifts; oblique planes yield the exact ellipse (|axis.n| > sin(alpha)), parabola (pure-rounding snap to the ruling-parallel angle, weights 1) or hyperbola arcs clipped by the height rings and the patch rectangle in closed form — the cone-side lift of those conics is null (no exact low-degree rational UV form on the ruled side). Ring-plane coincidence reports coincident_trim, the apex contact and the recognition-scale parabola threshold band stay tangency_or_multiple_root, misses resolve empty. Never authorizes topology changes. */
export const intersectPlaneCone=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<PlaneConeIntersection>=>callGeometryRust('brep_intersect_plane_cone',{first,second,options})

/** One torus face's share of an intersection circle in that face's UV: an exact degree-1 iso-v segment for parallels, an iso-u segment for meridians, clipped per patch quadrant. */export interface TorusPatchCurveLift {patch:number;arcs:NurbsCurve[]}
export type PlaneTorusIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;full:boolean;planeUv:PlanePatchCurveLift[];torusUv:TorusPatchCurveLift[];maxSampleResidual:number}
/** Analytic plane/torus for a canonical finite rectangular planar patch (plane operand first) against a canonical ring-torus solid (sixteen exact rational biquadratic patches, strict ring R-r>=1e-5 only — the constructor refuses horn and spindle tori; rigid affine placements admitted), axial configurations only. Perpendicular planes at axial height h resolve empty for |h|>r, yield the exact parallel pair R+-sqrt(r^2-h^2) (h=0 is the equator pair R+-r) with per-quadrant iso-v torus lifts and exact plane-UV ellipse arcs, clipped to the patch rectangle in closed form; through-axis planes yield the exact meridian pair of radius r at +-R with per-quadrant iso-u torus lifts. Tangencies (|h|==r, domain-edge touches, the inner-radius collapse band) stay tangency_or_multiple_root — never guessed circles; recognition-scale tilts and near-through-axis offsets report near_coincidence; oblique planes (quartic, Villarceau) and offset axis-parallel planes (Cassini ovals) are explicit unsupported_surface regions, never a numerical fallback; plane/torus coincidence cannot arise, so coincident_trim is unused. Never authorizes topology changes. */
export const intersectPlaneTorus=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<PlaneTorusIntersection>=>callGeometryRust('brep_intersect_plane_torus',{first,second,options})

export type SphereTorusIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;sphereUv:SpherePatchCircleLift[];torusUv:TorusPatchCurveLift[];maxSampleResidual:number}
/** Analytic sphere/torus for the canonical solids in the axial configuration only (sphere center certified on the torus axis; rigid affine placements admitted; strict ring torus R-r>=1e-5 — the constructor refuses horn and spindle tori). Non-canonical operands and clearly off-axis pairs are explicit unsupported_surface regions — never a numerical fallback (the general pair is a quartic); near-axial offsets report near_coincidence. In the meridian half-plane the section is circle/circle: two distinct roots revolve into two exact circles (radius rho*, height z*) with iso-v torus lifts at the exact rational-arc v parameter and per-patch stereographic sphere lifts; the meridian tangencies (double roots, external d==r_t+r_s or internal d==|r_t-r_s|) and the pole-collapse guard stay tangency_or_multiple_root — tangent contacts are never guessed circles; provably separate or contained meridian circles resolve empty. Never authorizes topology changes. */
export const intersectSphereTorus=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<SphereTorusIntersection>=>callGeometryRust('brep_intersect_sphere_torus',{first,second,options})

export type CylinderTorusIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;cylinderUv:CylinderPatchCurveLift[];torusUv:TorusPatchCurveLift[];maxSampleResidual:number}
/** Analytic cylinder/torus for the canonical solids in the coaxial configuration only (the cylinder axis certified coincident with the torus axis; rigid affine placements admitted; strict ring torus R-r>=1e-5 — the constructor refuses horn and spindle tori). Non-canonical operands and clearly tilted or off-axis pairs are explicit unsupported_surface regions — never a numerical fallback (the general pair is a quartic); recognition-scale tilts and near-coaxial offsets report near_coincidence, never forced coaxial. In the meridian half-plane the cylinder side is the line rho=R_c against the torus circle (rho-R)^2+z^2=r^2: a provable miss resolves empty, the tangent line (|R_c-R|==r, double root at z=0) stays tangency_or_multiple_root, and otherwise the two distinct exact circles of radius R_c at z=+-sqrt(r^2-(R_c-R)^2) are clipped by the finite height (a circle on a cap plane is the rim tangency, unresolved). Cap planes crossing the tube (|h_c|<r) cut the exact circle pair R+-sqrt(r^2-h_c^2): radii inside the cap disk are cap circle components, a radius equal to R_c within the band is the rim tangency. Resolved circles carry exact lifts: iso-v side lines or cap UV circles on the cylinder, iso-v parallels at the exact rational-arc v parameter on the torus. Tangent contacts are never guessed; coincident_trim is unused (a cylinder side never coincides with a curved torus patch). Never authorizes topology changes. */
export const intersectCylinderTorus=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<CylinderTorusIntersection>=>callGeometryRust('brep_intersect_cylinder_torus',{first,second,options})

export type ConeTorusIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;coneUv:CylinderPatchCurveLift[];torusUv:TorusPatchCurveLift[];maxSampleResidual:number}
/** Analytic cone/torus (frustum/torus) for the canonical solids in the coaxial configuration only (the cone axis certified coincident with the torus axis, either orientation — anti-axial pairs oriented by the axes' dot sign; rigid affine placements admitted; true-apex cones admitted, an equal-radius frustum is a cylinder and is refused; strict ring torus R-r>=1e-5 — the constructor refuses horn and spindle tori). Non-canonical operands and clearly tilted or off-axis pairs are explicit unsupported_surface regions — never a numerical fallback (the general pair is a quartic); recognition-scale tilts and near-coaxial offsets report near_coincidence, never forced coaxial. In the meridian half-plane the cone side is the slanted line (rho,z)=(r_b+m t,z_b+s t) against the torus circle (rho-R)^2+z^2=r^2 — one exact quadratic in the cone axial parameter: a provable miss resolves empty, the tangent line (double root) stays tangency_or_multiple_root, and distinct roots revolve into exact circles (radius rho*, height z*) clipped by the finite height (a root on a ring plane is the rim contact, unresolved; the rho*≈0 apex/pole guard is structurally unreachable for a strict ring torus and kept honest). Ring planes crossing the tube (|h_c|<r) cut the exact circle pair R+-sqrt(r^2-h_c^2): radii inside the ring disk are cap circle components, a radius equal to the ring radius within the band is the rim tangency. Resolved circles carry exact lifts: iso-v side lines or cap UV circles on the cone, iso-v parallels at the exact rational-arc v parameter on the torus. Tangent contacts are never guessed; coincident_trim is unused (a cone side never coincides with a curved torus patch). Never authorizes topology changes. */
export const intersectConeTorus=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<ConeTorusIntersection>=>callGeometryRust('brep_intersect_cone_torus',{first,second,options})

export type TorusTorusIntersection =
 | {kind:'circle';curve:NurbsCurve;center:Vec3;radius:number;normal:Vec3;firstUv:TorusPatchCurveLift[];secondUv:TorusPatchCurveLift[];maxSampleResidual:number}
/** Analytic torus/torus for the canonical strict ring-torus solids in the coaxial configuration only (axes certified parallel and coincident, either orientation; the center planes may differ by an axial offset h; rigid affine placements admitted; R-r>=1e-5 — the constructor refuses horn and spindle tori). Non-canonical operands and clearly tilted or off-axis pairs are explicit unsupported_surface regions — never a numerical fallback (the general pair is a quartic); recognition-scale tilts and near-coaxial offsets report near_coincidence, never forced coaxial. In the meridian half-plane the section is circle/circle: two distinct roots revolve into two exact circles (radius rho*, height z*) with iso-v parallel lifts at the exact rational-arc v parameter on both 4x4 tilings; the meridian tangencies (double roots, external d==r1+r2 or internal d==|r1-r2|) and the pole-collapse guard stay tangency_or_multiple_root — tangent contacts are never guessed circles; equal meridian circles report coincident_trim; provably separate or contained meridian circles resolve empty. Never authorizes topology changes. */
export const intersectTorusTorus=(first:NurbsBrep,second:NurbsBrep,options:IntersectionOptions={}):IntersectionReport<TorusTorusIntersection>=>callGeometryRust('brep_intersect_torus_torus',{first,second,options})

/** Algebraic conversion without fitting samples; UV lines may lift to curved 3D geometry.
 * Conversion does not certify plane membership or authorize topology changes. */
export const intersectionTraceToNurbsCurve=(trace:IntersectionSurfaceTrace):NurbsCurve=>callGeometryRust('brep_intersection_trace_curve',{trace})
/** Separate numerical pieces; their knot domains use the original trace fraction. No sewing implied. */
export const intersectionTraceToNurbsCurveSegments=(trace:IntersectionSurfaceTrace):NurbsCurve[]=>callGeometryRust('brep_intersection_trace_curve_segments',{trace})
