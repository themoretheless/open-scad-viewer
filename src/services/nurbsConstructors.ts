/** Surface construction is implemented by the own Rust kernel. */
import { type NurbsCurve } from './nurbsCurve'
import { type NurbsSurface } from './nurbsSurface'
import { callNurbsRust } from './geometry/nurbs'
import {inspectAuthoredFrameRegularity,type AuthoredFrameRegularity} from './nurbsAuthoredFrameRegularity'
import {inspectRetainedPatchRegularity,type RetainedPatchRegularity} from './nurbsRetainedPatchRegularity'
import {inspectFrenetFrameRegularity,type FrenetFrameRegularity} from './nurbsFrenetFrameRegularity'
import {inspectFixedNormalFrameRegularity,type FixedNormalFrameRegularity} from './nurbsFixedNormalFrameRegularity'
/** Rational sphere surface; its poles are intentional parameter singularities. */
export const sphereNurbsSurface = (center: [number,number,number], radius: number): NurbsSurface => callNurbsRust('surface_sphere',{center,radius})
/** Circular cylinder side, without caps. */
export const cylinderNurbsSurface = (center: [number,number,number], radius: number, height: number): NurbsSurface => callNurbsRust('surface_cylinder',{center,radius,height})
/** Circular cone side, without a base cap; singular apex. */
export const coneNurbsSurface = (center: [number,number,number], radius: number, height: number): NurbsSurface => callNurbsRust('surface_cone',{center,radius,height})
/** Bounded reverse Polish coordinate expressions; constants and results use model mm coordinates. */
export type NurbsFormulaToken = number | 't' | 'neg' | '+' | '-' | '*' | '/'
export const formulaNurbsCurve = (domain: [number,number], expressions: [NurbsFormulaToken[] | string,NurbsFormulaToken[] | string,NurbsFormulaToken[] | string]): NurbsCurve => callNurbsRust('curve_formula',{domain,expressions})
export type NurbsSurfaceFormulaToken = Exclude<NurbsFormulaToken,'t'> | 'u' | 'v'
/** Two-variable rational coordinate formulas, converted algebraically on [0,1]^2. */
export const formulaNurbsSurface = (domain: [number,number,number,number], expressions: [NurbsSurfaceFormulaToken[] | string,NurbsSurfaceFormulaToken[] | string,NurbsSurfaceFormulaToken[] | string]): NurbsSurface => callNurbsRust('surface_formula',{domain,expressions})
/** Natural tensor cubic interpolation of rectangular 2..11 site grids per axis. */
export const gridSplineNurbsSurface = (points: [number,number,number][][], parametersU: number[], parametersV: number[]): NurbsSurface => callNurbsRust('surface_grid_spline',{points,parameters_u:parametersU,parameters_v:parametersV})
export type NurbsCornerJets = [[[number,number,number],[number,number,number]],[[number,number,number],[number,number,number]]]
/** Bicubic patch from normalized corner positions, u/v tangents and mixed uv jets. */
export const hermiteNurbsPatch = (corners: NurbsCornerJets, tangentU: NurbsCornerJets, tangentV: NurbsCornerJets, twist: NurbsCornerJets): NurbsSurface => callNurbsRust('surface_hermite_patch',{corners,tangent_u:tangentU,tangent_v:tangentV,twist})
/** Closed cubic site interpolation, repeated endpoint required; clamped knot encoding. */
export const closedSplineNurbsCurve = (points: [number,number,number][], parameters: number[]): NurbsCurve => callNurbsRust('curve_closed_spline',{points,parameters})
/** Cubic interpolation with endpoint dP/dt tangents, normalized parameter domain. */
export const clampedSplineNurbsCurve = (points: [number,number,number][], parameters: number[], startTangent: [number,number,number], endTangent: [number,number,number]): NurbsCurve => callNurbsRust('curve_clamped_spline',{points,parameters,start_tangent:startTangent,end_tangent:endTangent})
/** Natural cubic interpolation through sites; normalized domain and zero endpoint second derivatives. */
export const naturalSplineNurbsCurve = (points: [number,number,number][], parameters: number[]): NurbsCurve => callNurbsRust('curve_natural_spline',{points,parameters})
/** Cubic Hermite interpolation; tangents are dP/dt for authored parameters, output domain [0,1]. */
export const hermiteNurbsCurve = (points: [number,number,number][], tangents: [number,number,number][], parameters: number[]): NurbsCurve => callNurbsRust('curve_hermite',{points,tangents,parameters})
export interface NurbsHelixApproximation {
 curve:NurbsCurve
 report:{spans:number;budget:number;realArithmeticErrorEstimate:number;continuousBound:false;roundingCertified:false;method:'uniform-angle-cubic-Hermite-fourth-derivative-estimate'|'uniform-abscissa-cubic-Hermite-fourth-derivative-estimate'|'catenary-profile-Hermite-rational-revolution'|'radial-ruled-helix-Hermite'|'clothoid-Hermite-composite-Simpson'|'profile-screw-helix-Hermite'}
}
/** Budget limits the ideal Hermite remainder estimate; binary64 rounding remains uncertified. */
export const approximateHelixNurbsCurve=(center:[number,number,number],radius:number,height:number,turns:number,phaseDegrees:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_helix',{center,radius,height,turns,phase_degrees:phaseDegrees,max_deviation:maxDeviation})
/** Elliptical constant XY radii; the estimate excludes binary64 rounding. */
export const approximateEllipticHelixNurbsCurve=(center:[number,number,number],radiusX:number,radiusY:number,height:number,turns:number,phaseDegrees:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_elliptic_helix',{center,radius_x:radiusX,radius_y:radiusY,height,turns,phase_degrees:phaseDegrees,max_deviation:maxDeviation})
/** Circular radius varies linearly between nonnegative endpoints; zero-radius apex allowed. */
export const approximateConicalHelixNurbsCurve=(center:[number,number,number],startRadius:number,endRadius:number,height:number,turns:number,phaseDegrees:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_conical_helix',{center,start_radius:startRadius,end_radius:endRadius,height,turns,phase_degrees:phaseDegrees,max_deviation:maxDeviation})
export const planeNurbsPatch = (origin: [number,number,number], axisU: [number,number,number], axisV: [number,number,number]): NurbsSurface => callNurbsRust('surface_plane',{origin,axisU,axisV})
export const bilinearNurbsPatch = (corners: [[ [number,number,number], [number,number,number] ],[ [number,number,number], [number,number,number] ]]): NurbsSurface => callNurbsRust('surface_bilinear',{corners})
/** Rational tensor-product Bezier surface; degree inferred from control net. */
export const bezierNurbsSurface = (points: [number,number,number][][], weights?: number[][]): NurbsSurface => callNurbsRust('surface_bezier',{points,weights})
/** One rational Bezier span, degree inferred from 2..26 control points. */
export const bezierNurbsCurve = (points: [number,number,number][], weights?: number[]): NurbsCurve => callNurbsRust('curve_bezier',{points,weights})
/** Clamped curves with exactly matching endpoints; degree elevation and C0 seams. */
export const miterNurbsProfileSections = (profiles: NurbsCurve[], points: [number,number,number][], normal: [number,number,number], miterLimit = 4, closed = false): NurbsCurve[][] => callNurbsRust('curve_miter_sections',{profiles,points,normal,miter_limit:miterLimit,closed})
export const transitionPolylineNurbsCurve = (points: [number,number,number][], setback: number, closed = false): NurbsCurve => callNurbsRust('curve_transition_polyline',{points,setback,closed})
export const roundPolylineNurbsCurve = (points: [number,number,number][], radius: number, closed = false): NurbsCurve => callNurbsRust('curve_round_polyline',{points,radius,closed})
export const composeNurbsCurves = (curves: NurbsCurve[]): NurbsCurve => callNurbsRust('curve_compose',{curves})
export const lineNurbsCurve = (start: [number,number,number], end: [number,number,number]): NurbsCurve => callNurbsRust('curve_line',{start,end})
/** Equal normalized parameter intervals per segment; closure repeats the first point. */
export const polylineNurbsCurve = (points: [number,number,number][], closed = false): NurbsCurve => callNurbsRust('curve_polyline',{points,closed})
/** Start direction is the least-aligned Cartesian axis projected into the circle plane. */
export const circleNurbsCurve = (center: [number,number,number], normal: [number,number,number], radius: number): NurbsCurve => callNurbsRust('curve_circle',{center,normal,radius})
export const circleNurbsArc = (center: [number,number,number], normal: [number,number,number], radius: number, startDegrees: number, sweepDegrees: number): NurbsCurve => callNurbsRust('curve_circle_arc',{center,normal,radius,startDegrees,sweepDegrees})
/** Rational ellipse arc; axes are independent 3D radius vectors. Angles use degrees. */
export const ellipseNurbsArc = (center: [number, number, number], axisU: [number, number, number], axisV: [number, number, number], startDegrees = 0, sweepDegrees = 360): NurbsCurve =>
  callNurbsRust('curve_ellipse_arc', { center, axisU, axisV, startDegrees, sweepDegrees })
/** Rational ellipsoid surface; poles are parameter singularities. */
export const ellipsoidNurbsSurface = (center: [number, number, number], radii: [number, number, number]): NurbsSurface =>
  callNurbsRust('surface_ellipsoid', { center, radii })
/** Ring torus surface with an elliptical tube; majorRadius must exceed radialRadius. */
export const torusNurbsSurface = (center: [number, number, number], majorRadius: number, radialRadius: number, axialRadius = radialRadius): NurbsSurface =>
  callNurbsRust('surface_torus', { center, majorRadius, radialRadius, axialRadius })
/** Polynomial parabola; t is dimensionless and maps affinely from the curve parameter. */
export const parabolaNurbsCurve = (center: [number,number,number], axisU: [number,number,number], axisV: [number,number,number], start: number, end: number): NurbsCurve => callNurbsRust('curve_parabola',{center,axisU,axisV,start,end})
/** Rational hyperbola branch; its parameter is not affine in the analytic t. */
export const hyperbolaNurbsCurve = (center: [number,number,number], axisU: [number,number,number], axisV: [number,number,number], start: number, end: number): NurbsCurve => callNurbsRust('curve_hyperbola',{center,axisU,axisV,start,end})
export const ellipticCylinderNurbsSurface = (center: [number,number,number], radiusX: number, radiusY: number, height: number): NurbsSurface => callNurbsRust('surface_elliptic_cylinder',{center,radiusX,radiusY,height})
export const coneFrustumNurbsSurface = (center: [number,number,number], bottomRadius: number, topRadius: number, height: number): NurbsSurface => callNurbsRust('surface_cone_frustum',{center,bottomRadius,topRadius,height})
/** z=a*x²+b*x*y+c*y²+d*x+e*y+f in mm coordinates. */
export const quadraticNurbsPatch = (bounds: [number,number,number,number], coefficients: [number,number,number,number,number,number]): NurbsSurface => callNurbsRust('surface_quadratic_patch',{bounds,coefficients})
/** Uncapped one-sheet hyperboloid; start/end bound the analytic hyperbolic parameter. */
export const hyperboloidOneSheetNurbsSurface = (center: [number,number,number], radii: [number,number,number], start: number, end: number): NurbsSurface =>
  callNurbsRust('surface_hyperboloid_one_sheet', {center,radii,start,end})
/** One connected sheet; start >= 0. A zero start introduces a singular pole. */
export const hyperboloidTwoSheetNurbsSurface = (center: [number,number,number], radii: [number,number,number], start: number, end: number, lower = false): NurbsSurface =>
  callNurbsRust('surface_hyperboloid_two_sheet', {center,radii,start,end,lower})
/** Power coefficients c[i][j] for z=sum(c[i][j]*x^i*y^j); exact basis conversion. */
export const polynomialGraphNurbsSurface = (bounds: [number,number,number,number], coefficients: number[][]): NurbsSurface =>
  callNurbsRust('surface_polynomial_graph', {bounds,coefficients})
/** XYZ power coefficients by increasing degree; output parameter is normalized to [0,1]. */
export const polynomialNurbsCurve = (domain: [number,number], coefficients: [number,number,number][]): NurbsCurve =>
  callNurbsRust('curve_polynomial_parametric', {domain,coefficients})
/** XYZ power coefficients c[i][j]; normalized output domains, degree <= 12 per direction. */
export const polynomialNurbsSurface = (domain: [number,number,number,number], coefficients: [number,number,number][][]): NurbsSurface =>
  callNurbsRust('surface_polynomial_parametric', {domain,coefficients})
/** Homogeneous XYZ/W power coefficients. Mixed-sign Bernstein weights are rejected. */
export const rationalPolynomialNurbsCurve = (domain: [number,number], coefficients: [number,number,number,number][]): NurbsCurve =>
  callNurbsRust('curve_rational_polynomial', {domain,coefficients})
/** Homogeneous XYZ/W power coefficients c[i][j]; conservative positive-weight admission. */
export const rationalPolynomialNurbsSurface = (domain: [number,number,number,number], coefficients: [number,number,number,number][][]): NurbsSurface =>
  callNurbsRust('surface_rational_polynomial', {domain,coefficients})
export function loftNurbsCurves(curves: NurbsCurve[]): NurbsSurface { return callNurbsRust('loft', { curves }) }
export function extrudeNurbsCurve(curve: NurbsCurve, vector: number[]): NurbsSurface { return callNurbsRust('extrude', { curve, vector }) }
export function revolveNurbsCurve(curve: NurbsCurve, origin: number[], axis: number[], angle: number): NurbsSurface { return callNurbsRust('revolve', { curve, origin, axis, angle }) }
/** Exact translation sweep; the profile frame does not rotate along the path. */
export const sweepNurbsCurve=(profile:NurbsCurve,path:NurbsCurve):NurbsSurface=>callNurbsRust('surface_sweep',{profile,path})
/** Fixed-axis rotation with rational conic angle parameterization, plus path translation. */
export const twistSweepNurbsCurve=(profile:NurbsCurve,path:NurbsCurve,origin:[number,number,number],axis:[number,number,number],startDegrees:number,sweepDegrees:number):NurbsSurface=>
 callNurbsRust('surface_twist_sweep',{profile,path,origin,axis,start_degrees:startDegrees,sweep_degrees:sweepDegrees})
/** Local profile x/width interpolates the guides; transverse axes are dimensionless. */
export const twoGuideSweepNurbsCurve=(profile:NurbsCurve,guideA:NurbsCurve,guideB:NurbsCurve,width:number,axisY:[number,number,number],axisZ:[number,number,number]):NurbsSurface=>
 callNurbsRust('surface_two_guide_sweep',{profile,guide_a:guideA,guide_b:guideB,width,axis_y:axisY,axis_z:axisZ})
/** Dimensionless positive rational scale law; knots define its authored domain. */
export interface NurbsScaleLaw {
 degree:number
 knots:number[]
 values:number[]
 weights:number[]
}
/** Fixed orientation; origin is the profile scaling center in model coordinates. */
export const scaledSweepNurbsCurve=(profile:NurbsCurve,path:NurbsCurve,scale:NurbsScaleLaw,origin:[number,number,number]):NurbsSurface=>
 callNurbsRust('surface_scaled_sweep',{profile,path,origin,scale:{degree:scale.degree,knots:scale.knots,controlPoints:scale.values.map(r=>[r,0,0]),weights:scale.weights,periodic:false}})
/** RMF transport with positive dimensionless scale; sampled refinement only. */
export const checkedProfileSweepNurbsSurface=(profile:NurbsCurve,path:NurbsCurve,scale:NurbsScaleLaw,normal:[number,number,number],sections:number,maxDeviation:number):FramedSweepResult=>
 callNurbsRust('surface_profile_sweep',{profile,path,scale:{degree:scale.degree,knots:scale.knots,controlPoints:scale.values.map(r=>[r,0,0]),weights:scale.weights,periodic:false},normal,sections,max_deviation:maxDeviation})
export interface NurbsVectorLaw {degree:number;knots:number[];values:[number,number,number][];weights:number[]}
export interface ProgressiveSweepOptions {
 axisScale?:NurbsVectorLaw
 centerLaw?:NurbsVectorLaw
 normal:[number,number,number]
 orientation?:'rmf'|'fixed'|'fixed_normal'|'frenet'|'corrected_frenet'
 spacing?:'parameter'|'arc_length'
 initialSections?:number
 maxSections?:number
 maxDeviation:number
 lengthTolerance?:number
 lengthMaxCells?:number
 rmfTransportSteps?:number
 errorMaxCells?:number
 errorMaxProducts?:number
}
/** Complete orientation independent of path tangent; native reports distinguish
 * certified retained-patch error from sampled transport acceptance. */
export interface AuthoredProgressiveSweepOptions extends Omit<ProgressiveSweepOptions,'orientation'> {
 orientation:'authored'
 frameAxis:NurbsVectorLaw
 frameNormal:NurbsVectorLaw
 frameRegularityMaxCells?:number
}
export type ProgressiveSurfaceSweepOptions=ProgressiveSweepOptions|AuthoredProgressiveSweepOptions
/** A spatial rail controls normal direction; this does not force profile contact. */
export interface GuidedProgressiveSweepOptions extends Omit<ProgressiveSweepOptions,'orientation'> {
 orientation?:'rmf'|'fixed_normal'|'frenet'|'corrected_frenet'
 orientationGuide:NurbsCurve
 /** Contact parameter in selected profile domain; default reference index is zero. */
 contactAnchor?:{profileIndex?:number;parameter:number}
}
export type ProgressiveGuidedSurfaceSweepOptions=ProgressiveSurfaceSweepOptions|GuidedProgressiveSweepOptions
export const sweepGuidePayload=(options:ProgressiveGuidedSurfaceSweepOptions)=>
 'orientationGuide' in options?{orientation_guide:options.orientationGuide,...(options.contactAnchor?{contact_profile:options.contactAnchor.profileIndex??0,contact_parameter:options.contactAnchor.parameter}:{})}:{}
export const sweepFrameLawPayload=(options:{orientation?:string;frameAxis?:NurbsVectorLaw;frameNormal?:NurbsVectorLaw})=>{
 if(options.orientation!=='authored') return {}
 const curve=(law:NurbsVectorLaw)=>({degree:law.degree,knots:law.knots,controlPoints:law.values,weights:law.weights,periodic:false})
 if(!options.frameAxis||!options.frameNormal) throw new Error('Authored sweep requires frameAxis and frameNormal')
 return {frame_axis:curve(options.frameAxis),frame_normal:curve(options.frameNormal)}
}
/** Positive dimensionless axis scale and local-frame center offsets in mm. */
export const sweepAffineLawPayload=(options:Pick<ProgressiveSweepOptions,'axisScale'|'centerLaw'>)=>({
 axis_scale:options.axisScale?{degree:options.axisScale.degree,knots:options.axisScale.knots,controlPoints:options.axisScale.values,weights:options.axisScale.weights,periodic:false}:null,
 center_law:options.centerLaw?{degree:options.centerLaw.degree,knots:options.centerLaw.knots,controlPoints:options.centerLaw.values,weights:options.centerLaw.weights,periodic:false}:null,
})
export interface ProgressiveSweepReport {
 authoredFrameRegularity?:AuthoredFrameRegularity
 retainedPatchRegularity?:RetainedPatchRegularity
 sourceFrameRegularity?:FrenetFrameRegularity|FixedNormalFrameRegularity
 sourceFrameSmoothness?:ProgressiveOriginalFrameSmoothness
 closedSourceFrameSmoothnessC1?:ProgressiveClosedPathFrameSmoothness
 closedSourceFrameSmoothness?:ProgressiveClosedAuthoredFrameSmoothness|ProgressiveClosedPathFrameSmoothness|ProgressiveClosedGuidedFrameSmoothness
 retainedDecompositionSmoothness?:ProgressiveRetainedDecompositionJoins
 retainedDecompositionG1Fallback?:ProgressiveRetainedDecompositionSmoothness
 accepted:boolean
 sections:number
 stations:number
 continuousErrorUpper:number|null
 /** Maximum of certified profiles; does not certify unresolved profiles. */
 knownProfileErrorUpper:number|null
 /** Original stored end contours; excludes decomposition, correction and filled caps. */
 originalSectionEndpointErrorUpper?:number|null
 /** Retained end contours including decomposition; excludes correction/filled caps. */
 endpointContourErrorUpper?:[number,number]|null
 errorCertificateCells:number
 decompositionProducts:number
 errorCertificateReason:string|null
 continuousErrorScope:'retained-patches-relative-to-original-profile-transport'
 sampledControlDeviation:number
 budget:number
 closedPath:boolean
 lengthResidualUpper:number|null
 continuousBound:boolean
 roundingCertified:boolean
 seamContinuity:'C0'|'open'
 method:'progressive-fourfold-section-refinement'
}
/** Attach a separate whole-law Rust premise without changing native admission. */
const sweepAngularLawCurve=(twist:NurbsScaleLaw):NurbsCurve=>({degree:twist.degree,knots:twist.knots,
 controlPoints:twist.values.map(a=>[a*Math.PI/180,0,0]),weights:twist.weights,periodic:false})
function withSweepFrameRegularity<T extends {patches:NurbsSurface[]|null;report:ProgressiveSweepReport;levels?:ProgressiveSweepReport[]}>(result:T,options:ProgressiveGuidedSurfaceSweepOptions,path:NurbsCurve,twist:NurbsScaleLaw,profiles:NurbsCurve[],scale:NurbsScaleLaw):T {
 const smoothness=inspectProgressiveOriginalFrameSmoothness(profiles,path,scale,twist,options,2,10000,1000000)
 result.report.sourceFrameSmoothness=smoothness
 for(const level of result.levels??[])level.sourceFrameSmoothness=smoothness
 if(result.report.closedPath&&options.orientation==='authored'){
  const closed=inspectProgressiveClosedAuthoredFrameSmoothness(profiles,path,scale,twist,options,2,10000,1000000)
  result.report.closedSourceFrameSmoothness=closed
  for(const level of result.levels??[])level.closedSourceFrameSmoothness=closed
 }

 if(result.report.closedPath&&!('frameAxis' in options&&options.frameAxis)&&!('orientationGuide' in options&&options.orientationGuide)&&['rmf','fixed_normal','corrected_frenet'].includes(options.orientation??'rmf')){
  const closed=inspectProgressiveClosedPathFrameSmoothness(profiles,path,scale,twist,options,2,10000,1000000)
  result.report.closedSourceFrameSmoothness=closed
  for(const level of result.levels??[])level.closedSourceFrameSmoothness=closed
  if(!closed.closedSourceFrameSmoothnessCertified){
   const c1=inspectProgressiveClosedPathFrameSmoothness(profiles,path,scale,twist,options,1,10000,1000000)
   result.report.closedSourceFrameSmoothnessC1=c1
   for(const level of result.levels??[])level.closedSourceFrameSmoothnessC1=c1
  }
 }

 if(result.report.closedPath&&'orientationGuide' in options&&options.orientationGuide&&!('frameAxis' in options&&options.frameAxis)){
  const closed=inspectProgressiveClosedGuidedFrameSmoothness(profiles,path,scale,twist,options,2,10000,1000000)
  result.report.closedSourceFrameSmoothness=closed
  for(const level of result.levels??[])level.closedSourceFrameSmoothness=closed
 }

 // Native retained Jacobian proof remains independent of refinement admission.
 // Audit published levels; rejected previews can request the separate API.
 if(result.report.accepted&&result.patches?.length){
  const regularity=inspectRetainedPatchRegularity(result.patches,10000)
  result.report.retainedPatchRegularity=regularity
  const last=result.levels?.at(-1)
  if(last)last.retainedPatchRegularity=regularity
  const fallback=inspectProgressiveRetainedDecompositionSmoothness(profiles,path,scale,twist,options,
   result.report.sections,1,1000000)
  const decomposition={...fallback.g2,profilePatchRanges:fallback.profilePatchRanges}
  result.report.retainedDecompositionSmoothness=decomposition
  if(last)last.retainedDecompositionSmoothness=decomposition
  result.report.retainedDecompositionG1Fallback=fallback
  if(last)last.retainedDecompositionG1Fallback=fallback
  if(!('orientationGuide' in options)&&options.orientation==='frenet'){
   const frame=inspectFrenetFrameRegularity(path,sweepAngularLawCurve(twist),10000)
   result.report.sourceFrameRegularity=frame
   if(last)last.sourceFrameRegularity=frame
  }else if(!('orientationGuide' in options)&&options.orientation==='fixed_normal'){
   const frame=inspectFixedNormalFrameRegularity(path,options.normal,sweepAngularLawCurve(twist),10000)
   result.report.sourceFrameRegularity=frame
   if(last)last.sourceFrameRegularity=frame
  }
 }
 if(options.orientation!=='authored')return result
 const frame=sweepFrameLawPayload(options)
 const regularity=inspectAuthoredFrameRegularity(frame.frame_axis!,frame.frame_normal!,options.frameRegularityMaxCells??10000)
 result.report.authoredFrameRegularity=regularity
 for(const level of result.levels??[])level.authoredFrameRegularity=regularity
 return result
}
const progressiveSweepSourcePayload=(path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,construction=false)=>({path,...sweepAffineLawPayload(options),...sweepFrameLawPayload(options),...sweepGuidePayload(options),
 ...(!construction||options.rmfTransportSteps===undefined?{}:{rmf_transport_steps:options.rmfTransportSteps}),
 ...(!construction||options.errorMaxCells===undefined?{}:{error_max_cells:options.errorMaxCells}),
 ...(!construction||options.errorMaxProducts===undefined?{}:{error_max_products:options.errorMaxProducts}),
 scale:{degree:scale.degree,knots:scale.knots,controlPoints:scale.values.map(r=>[r,0,0]),weights:scale.weights,periodic:false},
 twist:sweepAngularLawCurve(twist),normal:options.normal,orientation:options.orientation??'rmf',spacing:options.spacing??'parameter',
 initial_sections:options.initialSections??5,max_sections:options.maxSections??257,max_deviation:options.maxDeviation,
 length_tolerance:options.lengthTolerance??0.001,length_max_cells:options.lengthMaxCells??100000})
export interface ProgressiveSweepResult {patches:NurbsSurface[]|null;report:ProgressiveSweepReport;levels:ProgressiveSweepReport[]}
/** Simultaneous scale/twist; twist values use degrees, normalized traversal. */
export const progressiveSweepNurbsPatches=(profile:NurbsCurve,path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions):ProgressiveSweepResult=>
 withSweepFrameRegularity(callNurbsRust('surface_progressive_sweep',{ profile,...progressiveSweepSourcePayload(path,scale,twist,options,true)}),options,path,twist,[profile],scale)
export interface ProgressiveMultiSweepResult extends ProgressiveSweepResult {profilePatchRanges:[number,number][]|null}
/** Shared stations/budget for ordered curves; preserves boundaries, without sewing or caps. */
export const progressiveSweepNurbsProfiles=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions):ProgressiveMultiSweepResult=>
 withSweepFrameRegularity(callNurbsRust('surface_progressive_sweep_profiles',{ profiles,...progressiveSweepSourcePayload(path,scale,twist,options,true)}),options,path,twist,profiles,scale)
export interface ProgressiveSweepPreview<R=ProgressiveSweepReport|ProgressiveMiterReport> {
 preview:true
 patches:NurbsSurface[]
 profilePatchRanges:[number,number][]
 report:R
}
/** One preview level; unaccepted patches are not construction results. */
export const previewProgressiveNurbsProfiles=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,sections:number):ProgressiveSweepPreview<ProgressiveSweepReport>=>
 withSweepFrameRegularity(callNurbsRust('surface_progressive_sweep_level',{ preview_sections:sections,profiles,...progressiveSweepSourcePayload(path,scale,twist,options,true)}),options,path,twist,profiles,scale)
export interface ProgressiveRetainedStationSeams {
 requestedOrder:1|2
 allStationSeamsCertified:boolean
 exactWork:number
 seams:{patch:number;station:number;closure:boolean;c0Identity:boolean;certified:boolean;regularityCertified:boolean;exactWork:number;reason:string}[]
 reason:string|null
 method:'exact-retained-station-strip-jets'
 scope:'retained-station-seams-only'
 sourceFrameSmoothnessCertified:false
 profileJoinsCertified:false
 capJoinsCertified:false
 solidCertified:false
}
/** Native full-domain retained seam audit; source-frame and Solid proofs remain separate. */
export const inspectProgressiveRetainedStationSeams=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,sections:number,order:1|2,maxExactWork:number):ProgressiveRetainedStationSeams=>
 callNurbsRust('surface_progressive_sweep_station_seams',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),preview_sections:sections,order,maxExactWork})
export interface ProgressiveRetainedProfileJoin {
 requestedOrder:1|2
 leftPatch:number
 rightPatch:number
 profilePatchRanges:[number,number][]
 certified:boolean
 exactIdentity:boolean
 regularityCertified:boolean
 exactWork:number
 reason:string
 method:'exact-retained-profile-strip-jets'
 scope:'explicit-retained-profile-join-only'
 allProfileJoinsCertified:false
 sourceFrameSmoothnessCertified:false
 capJoinsCertified:false
 continuousBound:false
 solidCertified:false
}
/** Rust reconstructs the requested level and checks this explicit uMax/uMin pair. */
export const inspectProgressiveRetainedProfileJoin=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,sections:number,leftPatch:number,rightPatch:number,order:1|2,transverseScale:number,maxExactWork:number):ProgressiveRetainedProfileJoin=>
 callNurbsRust('surface_progressive_sweep_profile_join',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),preview_sections:sections,leftPatch,rightPatch,order,transverseScale,maxExactWork})
export interface ProgressiveRetainedDecompositionJoins {
 requestedOrder:1|2
 expectedJoins:number
 checkedJoins:number
 coverageComplete:boolean
 decompositionJoinsCertified:boolean
 exactWork:number
 joins:{leftPatch:number;rightPatch:number;certified:boolean;exactIdentity:boolean;regularityCertified:boolean;exactWork:number;reason:string}[]
 reason:string|null
 profilePatchRanges:[number,number][]
 method:'exact-retained-decomposition-strip-jets'
 scope:'within-source-profile-decomposition-only'
 allProfileJoinsCertified:false
 closedProfileSeamsCertified:false
 sourceFrameSmoothnessCertified:false
 capJoinsCertified:false
 continuousBound:false
 solidCertified:false
}
/** Native ownership extraction and all internal decomposition joins share one budget. */
export const inspectProgressiveRetainedDecompositionJoins=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,sections:number,order:1|2,transverseScale:number,maxExactWork:number):ProgressiveRetainedDecompositionJoins=>
 callNurbsRust('surface_progressive_sweep_decomposition_joins',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),preview_sections:sections,order,transverseScale,maxExactWork})
export interface ProgressiveRetainedDecompositionSmoothness {
 g2:Omit<ProgressiveRetainedDecompositionJoins,'profilePatchRanges'>
 g1:Omit<ProgressiveRetainedDecompositionJoins,'profilePatchRanges'>|null
 decompositionG1Certified:boolean
 exactWork:number
 maxExactWork:number
 profilePatchRanges:[number,number][]
 method:'exact-retained-decomposition-smoothness'
 scope:'within-source-profile-decomposition-only'
 allProfileJoinsCertified:false
 closedProfileSeamsCertified:false
 sourceFrameSmoothnessCertified:false
 capJoinsCertified:false
 continuousBound:false
 solidCertified:false
}
/** Native G2 with budget reserved for an independently complete G1 fallback. */
export const inspectProgressiveRetainedDecompositionSmoothness=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,sections:number,transverseScale:number,maxExactWork:number):ProgressiveRetainedDecompositionSmoothness=>
 callNurbsRust('surface_progressive_sweep_decomposition_smoothness',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),preview_sections:sections,transverseScale,maxExactWork})
export interface ProgressiveOriginalFrameSmoothness {
 requestedOrder:1|2
 sourceFrameSmoothnessCertified:boolean
 cells:number
 exactWork:number
 reason:string|null
 method:'original-frame-continuity-and-nondegeneracy-cover'
 scope:'open-original-frame-only'
 retainedSeamsCertified:false
 profileJoinsCertified:false
 capJoinsCertified:false
 continuousBound:false
 solidCertified:false
}
/** Original frame proof is computed exclusively by Rust, independently of retained surface seams. */
export const inspectProgressiveOriginalFrameSmoothness=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,order:1|2,maxCells:number,maxExactWork:number):ProgressiveOriginalFrameSmoothness=>
 callNurbsRust('surface_progressive_sweep_frame_smoothness',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),order,maxCells,maxExactWork})
export interface ProgressiveClosedAuthoredFrameSmoothness {
 requestedOrder:1|2
 closedSourceFrameSmoothnessCertified:boolean
 cells:number
 exactWork:number
 reason:string|null
 method:'exact-original-authored-endpoint-jets-and-frame-cover'
 scope:'closed-original-authored-frame-only'
 pathSeamCertified:false
 retainedSeamsCertified:false
 profileJoinsCertified:false
 capJoinsCertified:false
 continuousBound:false
 solidCertified:false
}
/** Rust original closed-frame proof; it is independent of the path and retained surface seam. */
export const inspectProgressiveClosedAuthoredFrameSmoothness=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,order:1|2,maxCells:number,maxExactWork:number):ProgressiveClosedAuthoredFrameSmoothness=>
 callNurbsRust('surface_progressive_sweep_closed_frame_smoothness',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),order,maxCells,maxExactWork})
export interface ProgressiveClosedPathFrameSmoothness extends Omit<ProgressiveClosedAuthoredFrameSmoothness,'method'|'scope'> {
 method:'exact-original-path-twist-endpoint-jets-and-frame-cover'
 scope:'closed-original-path-frame-only'
}
/** Rust proof of the closed original path frame; retained joins remain separate. */
export const inspectProgressiveClosedPathFrameSmoothness=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,order:1|2,maxCells:number,maxExactWork:number):ProgressiveClosedPathFrameSmoothness=>
 callNurbsRust('surface_progressive_sweep_closed_path_frame_smoothness',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),order,maxCells,maxExactWork})
export interface ProgressiveClosedGuidedFrameSmoothness extends Omit<ProgressiveClosedAuthoredFrameSmoothness,'method'|'scope'> {
 method:'exact-original-guided-endpoint-jets-and-joint-frame-cover'
 scope:'closed-original-guided-frame-only'
}
/** Rust proof of the closed original guided frame; retained joins remain separate. */
export const inspectProgressiveClosedGuidedFrameSmoothness=(profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions,order:1|2,maxCells:number,maxExactWork:number):ProgressiveClosedGuidedFrameSmoothness=>
 callNurbsRust('surface_progressive_sweep_closed_guided_frame_smoothness',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),order,maxCells,maxExactWork})
export interface ProgressiveSweepIdealCapDomains {
 idealCapDomainsCertified:boolean
 localDomainCertified:boolean
 sourcePlaneAxis:number|null
 endpointFrameAxes:[[[number,number],[number,number],[number,number]],[[number,number],[number,number],[number,number]]]|null
 cells:number;pairs:number;exactWork:number;reason:string|null
 method:'original-progressive-endpoint-material-domains'
 continuousBound:false;retainedCapRegionsCertified:false;globalEmbeddingCertified:false;solidCertified:false
}
/** Original material domains only. All geometry/proofs are evaluated in Rust. */
export function inspectProgressiveSweepIdealCapDomains(
 profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveGuidedSurfaceSweepOptions,loopSizes:number[],
 budgets={tolerance:.001,maxPairs:1000,maxCells:10000,maxExactWork:1000000},
):ProgressiveSweepIdealCapDomains {
 return callNurbsRust('surface_progressive_sweep_cap_domains',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),loopSizes,...budgets})
}
export interface ProgressiveSweepCapProjection {
 capProjectionCertified:boolean;idealCapDomainsCertified:boolean
 endpointNormals:ProgressiveSweepIdealCapDomains['endpointFrameAxes']
 normalDots:[[number,number],[number,number]]|null;reversesOrientation:[boolean,boolean]|null
 cells:number;exactWork:number;reason:string|null
 method:'original-progressive-endpoint-plane-projection'
 continuousBound:false;retainedCapRegionsCertified:false;globalEmbeddingCertified:false;solidCertified:false
}
/** Plane projection only; no cap ownership or boundary error promotion. */
export function inspectProgressiveSweepCapProjection(
 profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveGuidedSurfaceSweepOptions,loopSizes:number[],caps:[NurbsSurface,NurbsSurface],
 budgets={tolerance:.001,maxPairs:1000,maxCells:10000,maxExactWork:1000000},
):ProgressiveSweepCapProjection {
 return callNurbsRust('surface_progressive_sweep_cap_projection',{profiles,...progressiveSweepSourcePayload(path,scale,twist,options),loopSizes,caps,...budgets})
}
export interface ProgressiveSweepStreamOptions {
 /** Worker cancellation state, sampled at the same boundaries as signal. */
 shouldAbort?:()=>boolean
 signal?:AbortSignal
}
/** With authored frames and a guide, frameAxis defines the axis and the projected guide defines the normal. */
export interface ProgressiveMiterOptions {
 orientationGuide?:NurbsCurve
 frameAxis?:NurbsVectorLaw
 frameNormal?:NurbsVectorLaw
 axisScale?:NurbsVectorLaw
 centerLaw?:NurbsVectorLaw
 capDomainBudgets?:{tolerance:number;maxPairs:number;maxCells:number;maxExactWork:number}
 retainedDecompositionBudgets?:{maxProducts:number;maxFaces:number}
 capProjectionBudgets?:{maxCells:number;maxExactWork:number}
 circleCorrection?:{quantum:number;tolerance:number;maxWork:number}
 capCorrection?:{quantum:number;tolerance:number;maxWork:number;authoredFrame?:boolean}
 capWallMaxWalls?:number
 contourAuditBudgets?:{tolerance:number;maxPairs:number;maxCells:number}
 retainedWallMaxInjectivityCells?:number
 capPairAuditBudgets?:Omit<import('./nurbsSweepAudit').SweepWallAuditOptions,'sharedBoundaries'>
 embeddingBudgets?:import('./nurbsSweepEmbedding').SweepEmbeddingBudgets
 volumeBudgets?:import('./nurbsSweepEmbedding').SweepVolumeBudgets
 wallAuditBudgets?:Omit<import('./nurbsSweepAudit').SweepWallAuditOptions,'sharedBoundaries'>
 normal:[number,number,number]
 closed?:boolean
 miterLimit?:number
 initialSteps?:number
 maxSteps?:number
 maxDeviation:number
}
export interface ProgressiveMiterReport {
 accepted:boolean;profileRegularityCertified:boolean;wallRegularityCertified:boolean|null;regularityCells:number;unresolvedWallPatches:[number,number][];certifiedErrorUpper:number|null;endpointContourErrorUpper:[number,number]|null;errorCertificateCells:number;errorCertificateReason:string|null;phaseResolved:boolean;frameTransportCertified:boolean;frameTransportReason:string|null;continuousErrorUpper:number;affineLawsApplied:boolean;authoredFramesApplied:boolean;orientationGuideApplied:boolean;continuousErrorMethod:'interval-authored-axis-guide-frame-interpolation'|'rational-law-derivative-interpolation-real-arithmetic'|'interval-affine-law-interpolation'|'interval-authored-frame-interpolation'|'interval-guide-frame-interpolation';steps:number;sections:number;stations:number;sampledControlDeviation:number;budget:number
 closedPath:boolean;holonomyCorrectionRadians:number;continuousBound:false;roundingCertified:false
 seamContinuity:'C0'|'open';method:'progressive-miter-fourfold-section-refinement'
}
export interface ProgressiveMiterResult {sections:NurbsCurve[][]|null;levels:ProgressiveMiterReport[];report:ProgressiveMiterReport}
export interface ProgressiveMiterPreview {preview:true;sections:NurbsCurve[][];report:ProgressiveMiterReport}
const progressiveMiterPayload=(profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions)=>({profiles,points,...sweepAffineLawPayload(options),
 ...(options.orientationGuide?{orientation_guide:options.orientationGuide}:{}),
 ...((options.frameAxis||options.frameNormal)?sweepFrameLawPayload({orientation:'authored',frameAxis:options.frameAxis,frameNormal:options.frameNormal}):{}),
 scale:{degree:scale.degree,knots:scale.knots,controlPoints:scale.values.map(value=>[value,0,0]),weights:scale.weights,periodic:false},
 twist:{degree:twist.degree,knots:twist.knots,controlPoints:twist.values.map(value=>[value*Math.PI/180,0,0]),weights:twist.weights,periodic:false},
 normal:options.normal,closed:options.closed??false,miter_limit:options.miterLimit??4,initial_steps:options.initialSteps??1,max_steps:options.maxSteps??64,max_deviation:options.maxDeviation,
})
/** Miter stations are retained at every level; laws and holonomy use normalized polyline length. Twist values are degrees. */
export const progressiveMiterNurbsProfiles=(profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions):ProgressiveMiterResult=>
 callNurbsRust('curve_progressive_miter',progressiveMiterPayload(profiles,points,scale,twist,options))
export const previewProgressiveMiterNurbsProfiles=(profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions,steps:number):ProgressiveMiterPreview=>
 callNurbsRust('curve_progressive_miter_level',{...progressiveMiterPayload(profiles,points,scale,twist,options),preview_steps:steps})
/** Geometric prerequisite only: material ownership and boundary pairing are separate. */
export interface ProgressiveMiterCapProjection {
 normalDots:[[number,number],[number,number]]|null
 reversesOrientation:[boolean,boolean]|null
 cells:number;exactWork:number;reason:string|null
 method:'original-endpoint-plane-projection';continuousBound:false
}
export function inspectProgressiveMiterCapProjection(
 profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,caps:[NurbsSurface,NurbsSurface],maxCells=10000,maxExactWork=1000000,
):ProgressiveMiterCapProjection {
 return callNurbsRust('curve_progressive_miter_cap_projection',{...progressiveMiterPayload(profiles,points,scale,twist,options),caps,maxCells,maxExactWork})
}
export interface ProgressiveMiterCapParallelism {
 parallel:[boolean,boolean]|null
 cells:number;exactWork:number;reason:string|null
 method:'original-endpoint-plane-parallelism';continuousBound:false
}
export function inspectProgressiveMiterCapParallelism(
 profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,caps:[NurbsSurface,NurbsSurface],maxCells=10000,maxExactWork=1000000,
):ProgressiveMiterCapParallelism {
 return callNurbsRust('curve_progressive_miter_cap_parallelism',{...progressiveMiterPayload(profiles,points,scale,twist,options),caps,maxCells,maxExactWork})
}
export interface ProgressiveMiterIdealCapDomains {
 idealCapDomainsCertified:boolean;localDomainCertified:boolean;sourcePlaneAxis:number|null
 endpointNormals:[[[number,number],[number,number],[number,number]],[[number,number],[number,number],[number,number]]]|null
 cells:number;pairs:number;exactWork:number;reason:string|null
 method:'original-profile-endpoint-material-domains';continuousBound:false
}
/** Original material ownership only. Filled-cap error and shell orientation are separate. */
export function inspectProgressiveMiterIdealCapDomains(
 profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,loopSizes:number[],budgets={tolerance:.001,maxPairs:1000,maxCells:10000,maxExactWork:1000000},
):ProgressiveMiterIdealCapDomains {
 return callNurbsRust('curve_progressive_miter_cap_domains',{...progressiveMiterPayload(profiles,points,scale,twist,options),loopSizes,...budgets})
}
/** Audit retained walls; path-neighbor declarations are derived by Rust. */
export function inspectProgressiveMiterWalls(
 profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,sections:NurbsCurve[][],
 budgets:Omit<import('./nurbsSweepAudit').SweepWallAuditOptions,'sharedBoundaries'>,
 loopSizes?:number[],
):import('./nurbsSweepAudit').SweepWallAudit {
 return callNurbsRust('curve_progressive_miter_wall_audit',{
  ...progressiveMiterPayload(profiles,points,scale,twist,options),sections,...budgets,...(loopSizes?{loopSizes}:{}),
 })
}
export async function* streamProgressiveMiterNurbsProfiles(profiles:NurbsCurve[],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions,control:ProgressiveSweepStreamOptions={}):AsyncGenerator<ProgressiveMiterPreview,ProgressiveMiterResult,void>{
 const checkAbort=()=>{control.signal?.throwIfAborted();if(control.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')}
 checkAbort()
 // Every level and final audit belongs to one source snapshot.
 ;({profiles,points,scale,twist,options}=structuredClone({profiles,points,scale,twist,options}))
 const levels:ProgressiveMiterReport[]=[]
 let steps=options.initialSteps??1
 const maximum=options.maxSteps??64
 for(;;){
  checkAbort();await new Promise<void>(resolve=>setTimeout(resolve,0));checkAbort()
  const level=previewProgressiveMiterNurbsProfiles(profiles,points,scale,twist,options,steps)
  checkAbort();levels.push(level.report);yield structuredClone(level);checkAbort()
  if(level.report.accepted||steps===maximum)return {sections:level.report.accepted?level.sections:null,levels,report:level.report}
  steps=Math.min(2*steps,maximum)
 }
}
/** Yields bounded previews and returns construction patches only after acceptance.
 * Cancellation is cooperative between synchronous kernel calls, not within one call.
 */
export async function* streamProgressiveNurbsProfiles(
 profiles:NurbsCurve[],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveGuidedSurfaceSweepOptions,stream:ProgressiveSweepStreamOptions={},
):AsyncGenerator<ProgressiveSweepPreview<ProgressiveSweepReport>,ProgressiveMultiSweepResult,void> {
 const checkAbort=()=>{
  stream.signal?.throwIfAborted()
  if(stream.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')
 }
 checkAbort()
 // Preview consumers can edit their copies without changing final geometry.
 ;({profiles,path,scale,twist,options}=structuredClone({profiles,path,scale,twist,options}))
 const levels:ProgressiveSweepReport[]=[]
 const maximum=options.maxSections??257
 let sections=options.initialSections??5
 for(;;){
  checkAbort()
  // Yield to worker message dispatch before each bounded kernel request.
  await new Promise<void>(resolve=>setTimeout(resolve,0))
  checkAbort()
  const preview=previewProgressiveNurbsProfiles(profiles,path,scale,twist,options,sections)
  checkAbort()
  levels.push(preview.report)
  yield structuredClone(preview)
  checkAbort()
  if(preview.report.accepted||sections>=maximum){
   return {patches:preview.report.accepted?preview.patches:null,
    profilePatchRanges:preview.report.accepted?preview.profilePatchRanges:null,
    report:preview.report,levels}
  }
  sections=Math.min(2*(sections-1)+1,maximum)
 }
}
export const clampedLoftNurbsCurves=(curves:NurbsCurve[],parameters:number[],startTangent:[number,number,number],endTangent:[number,number,number]):NurbsSurface=>callNurbsRust('surface_clamped_loft',{curves,parameters,start_tangent:startTangent,end_tangent:endTangent})
export const boundaryFillNurbsSurfaces=(boundaries:NurbsCurve[],center:[number,number,number]):NurbsSurface[]=>callNurbsRust('surface_boundary_fill',{boundaries,center})
export const triangularNurbsPatch=(base:NurbsCurve,sideA:NurbsCurve,sideB:NurbsCurve):NurbsSurface=>callNurbsRust('surface_triangular_patch',{base,side_a:sideA,side_b:sideB})
export const gordonNurbsSurface=(uCurves:NurbsCurve[],vCurves:NurbsCurve[],parametersU:number[],parametersV:number[]):NurbsSurface=>callNurbsRust('surface_gordon',{u_curves:uCurves,v_curves:vCurves,parameters_u:parametersU,parameters_v:parametersV})
export const closedLoftNurbsCurves=(curves:NurbsCurve[],parameters:number[]):NurbsSurface=>callNurbsRust('surface_closed_loft',{curves,parameters})
export const naturalLoftNurbsCurves=(curves:NurbsCurve[],parameters:number[]):NurbsSurface=>callNurbsRust('surface_natural_loft',{curves,parameters})
export const loftAlignedNurbsCurves=(curves:NurbsCurve[]):NurbsSurface=>callNurbsRust('loft_aligned',{curves})

import {validateSculptBrush,type GeometryDeformation,type GeometryBrush,type SculptBrush} from './geometryEditing'
/** Sculpts the control polygon/net; knots and weights are untouched. */
export const sculptNurbsCurve=(curve:NurbsCurve,brush:SculptBrush):NurbsCurve=>{validateSculptBrush(brush);return callNurbsRust('nurbs_sculpt_curve',{curve,brush})}
export const sculptNurbsSurface=(surface:NurbsSurface,brush:SculptBrush):NurbsSurface=>{validateSculptBrush(brush);return callNurbsRust('nurbs_sculpt_surface',{surface,brush})}
/** Nonlinear edits affect control points, not the exact pointwise surface image. */
export const deformNurbsCurve=(curve:NurbsCurve,deformation:GeometryDeformation):NurbsCurve=>callNurbsRust('nurbs_deform_curve',{curve,deformation})
export const deformNurbsSurface=(surface:NurbsSurface,deformation:GeometryDeformation):NurbsSurface=>callNurbsRust('nurbs_deform_surface',{surface,deformation})
export const brushNurbsCurve=(curve:NurbsCurve,brush:GeometryBrush):NurbsCurve=>callNurbsRust('nurbs_brush_curve',{curve,brush})
export const brushNurbsSurface=(surface:NurbsSurface,brush:GeometryBrush):NurbsSurface=>callNurbsRust('nurbs_brush_surface',{surface,brush})

/** Homogeneous Coons patch with compatible corner weights and positive control weights; boundaries: bottom, top, left, right. */
export const coonsNurbsPatch=(boundaries:NurbsCurve[]):NurbsSurface=>callNurbsRust('surface_coons_patch',{boundaries})

export interface FramedSweepResult {
 surface:NurbsSurface|null
 report:{accepted:boolean;sampledControlDeviation:number;budget:number;stations:number;sections:number;closedPath?:boolean;seamContinuity?:'C0'|'open';continuousBound:false;method:'double-reflection-fourfold-section-refinement'}
}
/** Sampled refinement diagnostic only; not a certified continuous error bound. */
export const framedSweepNurbsCurve=(profile:NurbsCurve,path:NurbsCurve,normal:[number,number,number],sections:number,maxDeviation:number):FramedSweepResult=>
 callNurbsRust('surface_framed_sweep',{profile,path,normal,sections,maxDeviation})

export type SurfaceJetBoundary='uMin'|'uMax'|'vMin'|'vMax'
export interface SurfaceSeamRegularity {
 certified:boolean
 unresolvedIntervals?:[number,number][]
 reason?:string
}
export interface SurfaceJetMatchResult {
 surface:NurbsSurface
 report:{
  continuityOrder:1|2
  normalScale:number
  method:'homogeneous-normalized-boundary-jets'
  regularityCertified:boolean
  referenceRegularity?:SurfaceSeamRegularity
  editedRegularity?:SurfaceSeamRegularity
  seamBasis:'exact-affine-knot-correspondence'
  modifiedLayers:number
  accepted:boolean
  maxError:number
  errorUpper:number
  tangentialSmoothnessCertified:boolean
  reason:'accepted'|'unproven-regularity'|'unproven-tangential-smoothness'|'deviation-exceeds-budget'
  errorBounds:{
   wholeSeam:true
   normalizedParameters:true
   method:'outward-homogeneous-jet-difference-hull'
   positionUpper:number
   firstDerivativeUpper:number
   secondDerivativeUpper:number|null
   mixedDerivativeUpper:number|null
  }
 }
}
/** Native whole-seam regularity and derivative-error gate; apply only accepted results. */
export const matchNurbsSurfaceJets=(reference:NurbsSurface,edited:NurbsSurface,referenceBoundary:SurfaceJetBoundary,editedBoundary:SurfaceJetBoundary,order:1|2,scale=1,reverse=false,maxError=1e-6):SurfaceJetMatchResult=>
 callNurbsRust('surface_match_jets',{reference,edited,referenceBoundary,editedBoundary,order,scale,reverse,maxError})

export interface SurfacePreparationResult {
 reference:NurbsSurface
 edited:NurbsSurface
 report:{accepted:boolean;reason:'accepted'|'deviation-exceeds-budget'|'unproven-parameter-normalization';budget:number;referenceErrorUpper?:number;editedErrorUpper?:number;wholeSurface?:true;method?:'outward-homogeneous-Bernstein-difference'}
 basis:{degree:number;controlCount:number;normalizedSeam:true;editedReversed:boolean;referenceDomain:[number,number];editedDomain:[number,number];periodicityRemoved:{reference:boolean;edited:boolean}}
}
export const prepareNurbsSurfaceSeams=(reference:NurbsSurface,edited:NurbsSurface,referenceBoundary:SurfaceJetBoundary,editedBoundary:SurfaceJetBoundary,reverse:boolean,maxError:number,openPeriodic=false):SurfacePreparationResult=>
 callNurbsRust('surface_prepare_seams',{reference,edited,referenceBoundary,editedBoundary,reverse,maxError,openPeriodic})

export interface CurveMatchResult {
 curve:NurbsCurve
 report:{accepted:boolean;positionErrorUpper:number;sineAngleUpper:number|null;angleDegreesUpper:number|null;maxAngleDegrees:number;regularityCertified:boolean;orientationCertified:boolean;method:'outward-endpoint-handle-wedge';reason:string}
}
export const matchNurbsCurveG1=(reference:NurbsCurve,edited:NurbsCurve,referenceEnd:'start'|'end',editedEnd:'start'|'end',maxAngleDegrees=1e-6):CurveMatchResult=>
 callNurbsRust('curve_match_g1',{reference,edited,referenceEnd,editedEnd,maxAngleDegrees})

export interface CoonsBoundaryPreparation {
 curve:NurbsCurve
 report:{accepted:boolean;errorUpper:number;budget:number;wholeCurve:true;method:'outward-homogeneous-Bernstein-difference'}
}
/** Preserve boundary parameters; return the source definition if the bound exceeds budget. */
export const prepareCoonsBoundaryWeights=(curve:NurbsCurve,maxError:number):CoonsBoundaryPreparation=>
 {const result=callNurbsRust<CoonsBoundaryPreparation>('curve_prepare_coons_weights',{curve,maxError});return result.report.accepted?result:{...result,curve:structuredClone(curve)}}

export const approximateVariablePitchHelixNurbsCurve=(center:[number,number,number],radius:number,height:number,turns:number,startPitch:number,endPitch:number,phaseDegrees:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_variable_pitch_helix',{center,radius,height,turns,start_pitch:startPitch,end_pitch:endPitch,phase_degrees:phaseDegrees,max_deviation:maxDeviation})

export const approximateInvoluteNurbsCurve=(center:[number,number,number],radius:number,startRadians:number,endRadians:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_involute',{center,radius,start_radians:startRadians,end_radians:endRadians,max_deviation:maxDeviation})

export const approximateLogarithmicSpiralNurbsCurve=(center:[number,number,number],radius:number,growth:number,startRadians:number,endRadians:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_logarithmic_spiral',{center,radius,growth,start_radians:startRadians,end_radians:endRadians,max_deviation:maxDeviation})

export const approximateLissajousNurbsCurve=(center:[number,number,number],amplitudes:[number,number,number],frequencies:[number,number,number],phasesDegrees:[number,number,number],maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_lissajous',{center,amplitudes,frequencies,phases_degrees:phasesDegrees,max_deviation:maxDeviation})

export const approximateTrochoidNurbsCurve=(center:[number,number,number],rollingRadius:number,tracingRadius:number,startRadians:number,endRadians:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_trochoid',{center,rolling_radius:rollingRadius,tracing_radius:tracingRadius,start_radians:startRadians,end_radians:endRadians,max_deviation:maxDeviation})
export const approximateCycloidNurbsCurve=(center:[number,number,number],radius:number,startRadians:number,endRadians:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_cycloid',{center,radius,start_radians:startRadians,end_radians:endRadians,max_deviation:maxDeviation})

export const approximateEpicycloidNurbsCurve=(center:[number,number,number],fixedRadius:number,rollingRadius:number,startRadians:number,endRadians:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_epicycloid',{center,fixed_radius:fixedRadius,rolling_radius:rollingRadius,start_radians:startRadians,end_radians:endRadians,max_deviation:maxDeviation})

export const approximateHypocycloidNurbsCurve=(center:[number,number,number],fixedRadius:number,rollingRadius:number,startRadians:number,endRadians:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_hypocycloid',{center,fixed_radius:fixedRadius,rolling_radius:rollingRadius,start_radians:startRadians,end_radians:endRadians,max_deviation:maxDeviation})

export const approximateArchimedeanSpiralNurbsCurve=(center:[number,number,number],startRadius:number,endRadius:number,startDegrees:number,endDegrees:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_archimedean_spiral',{center,start_radius:startRadius,end_radius:endRadius,start_degrees:startDegrees,end_degrees:endDegrees,max_deviation:maxDeviation})

export const approximateCatenaryNurbsCurve=(center:[number,number,number],scale:number,startX:number,endX:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_catenary',{center,scale,start_x:startX,end_x:endX,max_deviation:maxDeviation})

export interface NurbsCatenoidApproximation {surface:NurbsSurface;report:NurbsHelixApproximation["report"]}
export const approximateCatenoidNurbsSurface=(center:[number,number,number],scale:number,startZ:number,endZ:number,maxDeviation:number):NurbsCatenoidApproximation=>
 callNurbsRust('surface_catenoid',{center,scale,start_z:startZ,end_z:endZ,max_deviation:maxDeviation})

export interface NurbsHelicoidApproximation {surface:NurbsSurface;report:NurbsHelixApproximation["report"]}
export const approximateHelicoidNurbsSurface=(center:[number,number,number],innerRadius:number,outerRadius:number,height:number,turns:number,phaseDegrees:number,maxDeviation:number):NurbsHelicoidApproximation=>
 callNurbsRust('surface_helicoid',{center,inner_radius:innerRadius,outer_radius:outerRadius,height,turns,phase_degrees:phaseDegrees,max_deviation:maxDeviation})

export const approximateToroidalSpiralNurbsCurve=(center:[number,number,number],majorRadius:number,minorRadius:number,majorTurns:number,minorTurns:number,majorPhaseDegrees:number,minorPhaseDegrees:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_toroidal_spiral',{center,major_radius:majorRadius,minor_radius:minorRadius,major_turns:majorTurns,minor_turns:minorTurns,major_phase_degrees:majorPhaseDegrees,minor_phase_degrees:minorPhaseDegrees,max_deviation:maxDeviation})
export const approximateTorusKnotNurbsCurve=(center:[number,number,number],majorRadius:number,minorRadius:number,p:number,q:number,majorPhaseDegrees:number,minorPhaseDegrees:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_torus_knot',{center,major_radius:majorRadius,minor_radius:minorRadius,p,q,major_phase_degrees:majorPhaseDegrees,minor_phase_degrees:minorPhaseDegrees,max_deviation:maxDeviation})

export const extrudeNurbsCurvePatches=(curve:NurbsCurve,vector:[number,number,number]):NurbsSurface[]=>callNurbsRust('surface_extrude_patches',{curve,vector})

export const approximateSphericalSpiralNurbsCurve=(center:[number,number,number],radius:number,longitudeTurns:number,latitudeTurns:number,longitudePhaseDegrees:number,latitudePhaseDegrees:number,maxDeviation:number):NurbsHelixApproximation=>
 callNurbsRust('curve_spherical_spiral',{center,radius,longitude_turns:longitudeTurns,latitude_turns:latitudeTurns,longitude_phase_degrees:longitudePhaseDegrees,latitude_phase_degrees:latitudePhaseDegrees,max_deviation:maxDeviation})

export interface NurbsClothoidApproximation {
 curve:NurbsCurve
 report:NurbsHelixApproximation['report'] & {quadratureIntervals:number;quadratureErrorEstimate:number;hermiteErrorEstimate:number}
}
export const approximateClothoidNurbsCurve=(center:[number,number,number],length:number,startCurvature:number,endCurvature:number,phaseDegrees:number,maxDeviation:number):NurbsClothoidApproximation=>
 callNurbsRust('curve_clothoid',{center,length,start_curvature:startCurvature,end_curvature:endCurvature,phase_degrees:phaseDegrees,max_deviation:maxDeviation})

export interface NurbsScrewApproximation {patches:NurbsSurface[];report:NurbsHelixApproximation['report']}
export const approximateScrewNurbsSurfaces=(profile:NurbsCurve,origin:[number,number,number],axis:[number,number,number],height:number,turns:number,phaseDegrees:number,maxDeviation:number):NurbsScrewApproximation=>
 callNurbsRust('surface_screw',{profile,origin,axis,height,turns,phase_degrees:phaseDegrees,max_deviation:maxDeviation})

export interface NurbsCatenoidPatchesApproximation {patches:NurbsSurface[];report:NurbsHelixApproximation["report"]}
export const approximateCatenoidNurbsPatches=(center:[number,number,number],scale:number,startZ:number,endZ:number,maxDeviation:number):NurbsCatenoidPatchesApproximation=>
 callNurbsRust('surface_catenoid_patches',{center,scale,start_z:startZ,end_z:endZ,max_deviation:maxDeviation})

/** Circular authored stations, with sampled frame refinement only. */
export const checkedPipeNurbsSurface=(path:NurbsCurve,radius:number,normal:[number,number,number],sections:number,maxDeviation:number):FramedSweepResult=>
 callNurbsRust('surface_pipe',{path,radius,normal,sections,max_deviation:maxDeviation})

export const checkedVariablePipeNurbsSurface=(path:NurbsCurve,radius:NurbsCurve,normal:[number,number,number],sections:number,maxDeviation:number):FramedSweepResult=>
 callNurbsRust("surface_variable_pipe",{path,radius,normal,sections,max_deviation:maxDeviation})

export interface NurbsHelicoidPatchesApproximation {patches:NurbsSurface[];report:NurbsHelixApproximation["report"]}
export const approximateHelicoidNurbsPatches=(center:[number,number,number],innerRadius:number,outerRadius:number,height:number,turns:number,phaseDegrees:number,maxDeviation:number):NurbsHelicoidPatchesApproximation=>
 callNurbsRust("surface_helicoid_patches",{center,inner_radius:innerRadius,outer_radius:outerRadius,height,turns,phase_degrees:phaseDegrees,max_deviation:maxDeviation})

export const checkedRibbonNurbsSurface=(path:NurbsCurve,width:NurbsCurve,normal:[number,number,number],sections:number,maxDeviation:number):FramedSweepResult=>
 callNurbsRust("surface_ribbon",{path,width,normal,sections,max_deviation:maxDeviation})

export interface NurbsCircleSection {center:[number,number,number];normal:[number,number,number];seam:[number,number,number];radius:number}
export const circleTransitionNurbsSurface=(start:NurbsCircleSection,end:NurbsCircleSection):NurbsSurface=>
 callNurbsRust("surface_circle_transition",{start_center:start.center,start_normal:start.normal,start_seam:start.seam,start_radius:start.radius,end_center:end.center,end_normal:end.normal,end_seam:end.seam,end_radius:end.radius})

export interface NurbsEllipseSection {center:[number,number,number];axisU:[number,number,number];axisV:[number,number,number]}
export const ellipseTransitionNurbsSurface=(start:NurbsEllipseSection,end:NurbsEllipseSection):NurbsSurface=>
 callNurbsRust("surface_ellipse_transition",{start_center:start.center,start_axis_u:start.axisU,start_axis_v:start.axisV,end_center:end.center,end_axis_u:end.axisU,end_axis_v:end.axisV})

export interface NurbsRectangleSection {center:[number,number,number];axisU:[number,number,number];axisV:[number,number,number]}
export const circleRectangleTransitionNurbsPatches=(circle:NurbsCircleSection,rectangle:NurbsRectangleSection):NurbsSurface[]=>
 callNurbsRust("surface_circle_rectangle_transition",{circle_center:circle.center,circle_normal:circle.normal,circle_seam:circle.seam,circle_radius:circle.radius,rectangle_center:rectangle.center,rectangle_axis_u:rectangle.axisU,rectangle_axis_v:rectangle.axisV})

export const controlTangentLoftNurbsCurves = (curves: NurbsCurve[], parameters: number[], startTangents: [number,number,number][], endTangents: [number,number,number][]): NurbsSurface => callNurbsRust('surface_control_tangent_loft', {curves, parameters, start_tangents:startTangents, end_tangents:endTangents})
export const guidedLoftNurbsCurves = (curves: NurbsCurve[], parameters: number[], guides: NurbsCurve[], guideParameters: number[], startTangents?: [number,number,number][], endTangents?: [number,number,number][]): NurbsSurface => callNurbsRust('surface_guided_loft', {curves, parameters, guides, guide_parameters:guideParameters, start_tangents:startTangents, end_tangents:endTangents})

export interface AlignedNurbsLoft {
 surface:NurbsSurface
 guides:NurbsCurve[]
 guide_parameters:number[]
 guide_order:number[]
 reversed:boolean[]
 section_error_upper:number[]
 guide_error_upper:number[]
}
/** Automatic isolated intersections, guide reversal/sorting and piecewise V mapping. */
export const autoGuidedLoftNurbsCurves=(curves:NurbsCurve[],parameters:number[],guides:NurbsCurve[],budget:number):AlignedNurbsLoft=>callNurbsRust('surface_auto_guided_loft',{curves,parameters,guides,budget})
export interface LoftEndConstraint {
 reference:NurbsSurface
 boundary:'uMin'|'uMax'|'vMin'|'vMax'
 order:1|2
 scale:number
 reverse?:boolean
}
export interface MatchedNurbsLoft {
 surface:NurbsSurface
 seams:unknown[]
 section_error_upper:number[]
 guide_error_upper:number[]
}
/** Certified scaled boundary jets with whole-curve section/guide retention bounds. */
export const matchNurbsLoftEnds=(surface:NurbsSurface,curves:NurbsCurve[],parameters:number[],budget:number,start?:LoftEndConstraint,end?:LoftEndConstraint,guides:NurbsCurve[]=[],guideParameters:number[]=[]):MatchedNurbsLoft=>callNurbsRust('surface_loft_match_ends',{surface,curves,parameters,budget,start,end,guides,guide_parameters:guideParameters})
