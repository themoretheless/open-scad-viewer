import {transformNurbsCurve} from './nurbsCurve';
import {transformNurbsSurface,transformNurbsSurfacePatches} from './nurbsSurface';
import {circleRectangleTransitionNurbsPatches,ellipseTransitionNurbsSurface,circleTransitionNurbsSurface,checkedRibbonNurbsSurface,approximateHelicoidNurbsPatches,checkedVariablePipeNurbsSurface,checkedPipeNurbsSurface,approximateCatenoidNurbsPatches,approximateScrewNurbsSurfaces,approximateClothoidNurbsCurve,approximateSphericalSpiralNurbsCurve,extrudeNurbsCurvePatches,approximateToroidalSpiralNurbsCurve,approximateTorusKnotNurbsCurve,approximateHelicoidNurbsSurface,approximateCatenoidNurbsSurface,approximateCatenaryNurbsCurve,approximateArchimedeanSpiralNurbsCurve,approximateEpicycloidNurbsCurve,approximateHypocycloidNurbsCurve,approximateTrochoidNurbsCurve,approximateCycloidNurbsCurve,approximateLissajousNurbsCurve,approximateLogarithmicSpiralNurbsCurve,approximateInvoluteNurbsCurve,approximateVariablePitchHelixNurbsCurve,approximateEllipticHelixNurbsCurve,approximateConicalHelixNurbsCurve,approximateHelixNurbsCurve,type NurbsHelixApproximation,hermiteNurbsCurve,naturalSplineNurbsCurve,clampedSplineNurbsCurve,closedSplineNurbsCurve} from './nurbsConstructors';
import {streamProgressiveNurbsProfiles,type ProgressiveSweepPreview,progressiveSweepNurbsProfiles,type ProgressiveSweepReport,checkedProfileSweepNurbsSurface,twistSweepNurbsCurve,twoGuideSweepNurbsCurve,scaledSweepNurbsCurve,framedSweepNurbsCurve,type FramedSweepResult} from './nurbsConstructors';
import {formulaNurbsCurve,formulaNurbsSurface} from './nurbsConstructors';
import {coonsNurbsPatch} from './nurbsConstructors';
import {sphereNurbsSurface,cylinderNurbsSurface,coneNurbsSurface} from './nurbsConstructors';
import { hyperboloidOneSheetNurbsSurface, hyperboloidTwoSheetNurbsSurface, polynomialGraphNurbsSurface, polynomialNurbsCurve, polynomialNurbsSurface, rationalPolynomialNurbsCurve, rationalPolynomialNurbsSurface, parabolaNurbsCurve, hyperbolaNurbsCurve, ellipticCylinderNurbsSurface, coneFrustumNurbsSurface, quadraticNurbsPatch } from './nurbsConstructors';
import { ellipseNurbsArc, ellipsoidNurbsSurface, torusNurbsSurface } from './nurbsConstructors';
import {lineNurbsCurve,polylineNurbsCurve,circleNurbsCurve,circleNurbsArc} from './nurbsConstructors';
import {bezierNurbsCurve,composeNurbsCurves,roundPolylineNurbsCurve,transitionPolylineNurbsCurve} from './nurbsConstructors';
import {planeNurbsPatch,bilinearNurbsPatch,bezierNurbsSurface,hermiteNurbsPatch,gridSplineNurbsSurface} from './nurbsConstructors';
import {createNativeGeometryArtifact} from '../core/nativeGeometry';
import { stringifyMeshJson } from './meshJson'
import {extrudePolygonProfile,revolvePolygonProfile,loftPolygonSections,sweepPolygonProfile,type PolygonProfile} from './geometry/polygon';
import {meshToNurbsBrep,meshToSdf,meshToSubdivision,meshToNurbs,tessellateNurbsPatches,type NurbsSurfaceSet} from './geometry/reconstruction';
import type {SubdivisionCage} from './geometry/subdivision';
import {tessellateSubdivision} from './geometry/subdivision';
import {tessellateSdfGpuAware as tessellateSdf,evaluateSdf,type SdfField} from './geometry/sdf';
import {reconstructCertifiedMiterStations,transformCertifiedMiterBody,type CertifiedMiterBoundaryOwner,createProgressiveMiterBrepProfileBody,streamProgressiveMiterBrepProfileBody,type ProgressiveMiterBrepBody,createMiterBrepProfileBody,type MiterBrepBody,streamProgressiveBrepProfileBody,createNaturalBrepSectionLoft,createCappedBrepLoftSurfaces,createProgressiveBrepProfileBody,transformNurbsBrep,createBrepSphere,createBrepGear,createBrepTorus,createBrepBox,revolveBrepProfile,createBrepCylinder,createBrepFrustum,createBrepTube,extrudeBrepCurves,extrudeBrepPolygon,booleanNurbsBrep,chamferNurbsBrepEdges,filletNurbsBrepEdges,tessellateNurbsBrep,type NurbsBrep} from './geometry/brep';
import { inspectPolygonMesh,booleanPolygonMeshes } from './geometry/polygon';
import { exportMeshFormat, meshExportBase64, type MeshExportFormat } from './meshExportFormats';
import { compileModelGraphNurbs } from './modelGraphNurbsCompiler';
import { validateNurbsCurve, evaluateNurbsCurve, insertNurbsKnot, elevateNurbsCurve, trimNurbsCurve, reverseNurbsCurve, nurbsCurveBounds, type NurbsCurve } from './nurbsCurve';
import { validateNurbsSurface, evaluateNurbsSurface, insertNurbsSurfaceKnot, elevateNurbsSurface, trimNurbsSurface, reverseNurbsSurface, isoNurbsCurve, nurbsSurfaceBounds, type NurbsSurface } from './nurbsSurface';
import {certifyNurbsCurveFoundation, certifyNurbsSurfaceFoundation} from './nurbsFoundation';
import { autoGuidedLoftNurbsCurves,matchNurbsLoftEnds,guidedLoftNurbsCurves,controlTangentLoftNurbsCurves,boundaryFillNurbsSurfaces,triangularNurbsPatch,gordonNurbsSurface,closedLoftNurbsCurves,clampedLoftNurbsCurves,naturalLoftNurbsCurves,loftAlignedNurbsCurves,sweepNurbsCurve,loftNurbsCurves, extrudeNurbsCurve, revolveNurbsCurve } from './nurbsConstructors';
import { tessellateNurbsSurface, thickenNurbsMesh, exportNurbsStl } from './geometry/tessellation';
type Mesh = ReturnType<typeof tessellateNurbsSurface>;
type Value = {kind:'profile';data:PolygonProfile} | {kind:'patches';data:NurbsSurfaceSet} | {kind:'subdivision';data:SubdivisionCage} | {kind:'sdf';data:SdfField} | {kind:'brep';data:NurbsBrep} | {
    kind: 'curve';
    data: NurbsCurve;
} | {
    kind: 'surface';
    data: NurbsSurface;
} | {
    kind: 'mesh';
    data: Mesh;
};
export type OwnNurbsRequest = {
    action: 'build' | 'evaluate' | 'export';
    format?: 'json' | MeshExportFormat;
    /** Viewer-only derived display policy; does not change the authored graph. */
    display?: {segments:number;subdivisionLevels:number};
    evaluations?: Array<{
        node: string;
        u: number;
        v?: number;
    }>;
};
/** An sdf_tessellate job whose input subtree is pure SDF (no mesh inputs). */
export interface SdfTessellationJob {
    field: import('./geometry/sdf').SdfField
    grid: { min: number[]; max: number[]; cells: number[] }
}

/** Collects sdf_tessellate jobs resolvable without mesh evaluation, so the
 * caller can run their grid sampling on the GPU before the synchronous build. */
export function collectSdfJobs(document: unknown): SdfTessellationJob[] {
    type SdfField = import('./geometry/sdf').SdfField
    const compiled = compileModelGraphNurbs(document)
    const nodes = new Map(compiled.resolved_document.nodes.map(n => [n.id, n] as const))
    const memo = new Map<string, SdfField | null>()
    const pure = (key: string): SdfField | null => {
        const hit = memo.get(key)
        if (hit !== undefined) return hit
        const n = nodes.get(key) as any
        const field: SdfField | null = !n ? null
            : n.op === 'sdf_sphere' ? { kind: 'sphere', center: n.center, radius: n.radius }
            : n.op === 'sdf_box' ? { kind: 'box', center: n.center, half_size: n.half_size }
            : n.op === 'sdf_torus' ? { kind: 'torus', center: n.center, major_radius: n.major_radius, minor_radius: n.minor_radius }
            : n.op === 'sdf_union' || n.op === 'sdf_intersection' || n.op === 'sdf_difference'
                ? (() => { const a = pure(n.inputs[0]); const b = pure(n.inputs[1]); return a && b
                    ? { kind: n.op === 'sdf_union' ? 'union' : n.op === 'sdf_intersection' ? 'intersection' : 'difference', a, b } : null })()
            : n.op === 'sdf_smooth_union'
                ? (() => { const a = pure(n.inputs[0]); const b = pure(n.inputs[1]); return a && b ? { kind: 'smooth_union', a, b, radius: n.radius } : null })()
            : n.op === 'sdf_offset' ? (() => { const input = pure(n.input); return input ? { kind: 'offset', input, distance: n.distance } : null })()
            : n.op === 'sdf_translate' ? (() => { const input = pure(n.input); return input ? { kind: 'translate', input, vector: n.vector } : null })()
            : null
        memo.set(key, field)
        return field
    }
    const jobs: SdfTessellationJob[] = []
    for (const n of compiled.resolved_document.nodes) {
        if ((n as any).op !== 'sdf_tessellate') continue
        const node = n as any
        const field = pure(node.input)
        if (field) jobs.push({ field, grid: { min: node.min, max: node.max, cells: node.cells } })
    }
    return jobs
}

type SweepArguments=Parameters<typeof progressiveSweepNurbsProfiles>
type SweepResolver=(nodeId:string,args:SweepArguments)=>ReturnType<typeof progressiveSweepNurbsProfiles>
class PendingProgressiveSweep {
 constructor(readonly nodeId:string,readonly args:SweepArguments){}
}
type BodyArguments=Parameters<typeof createProgressiveBrepProfileBody>
type BodyResolver=(nodeId:string,args:BodyArguments)=>ReturnType<typeof createProgressiveBrepProfileBody>
class PendingProgressiveBody {
 constructor(readonly nodeId:string,readonly args:BodyArguments){}
}
type MiterArguments=Parameters<typeof createProgressiveMiterBrepProfileBody>
type MiterResolver=(nodeId:string,args:MiterArguments)=>ProgressiveMiterBrepBody
class PendingProgressiveMiter {constructor(readonly nodeId:string,readonly args:MiterArguments){}}
export interface OwnNurbsBuildControl {
 shouldAbort?:()=>boolean
 onYield?:()=>void
 onSweepPreview?:(nodeId:string,preview:ProgressiveSweepPreview)=>void|Promise<void>
}
export function buildOwnNurbs(document:unknown,request:OwnNurbsRequest){
 return buildOwnNurbsResolved(document,request)
}
/** Keeps graph evaluation synchronous except for bounded progressive sweep levels. */
export async function buildOwnNurbsAsync(document:unknown,request:OwnNurbsRequest,control:OwnNurbsBuildControl={}){
 const completed=new Map<string,ReturnType<typeof progressiveSweepNurbsProfiles>>()
 const bodies=new Map<string,ReturnType<typeof createProgressiveBrepProfileBody>>()
 const miters=new Map<string,ProgressiveMiterBrepBody>()
 const checkAbort=()=>{if(control.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')}
 async function consume<T>(nodeId:string,stream:AsyncGenerator<ProgressiveSweepPreview,T,void>):Promise<T>{
  try{
   for(;;){
    const level=await stream.next()
    checkAbort()
    control.onYield?.()
    if(level.done)return level.value
    await control.onSweepPreview?.(nodeId,level.value)
   }
  }finally{await stream.return(undefined as never)}
 }
 for(;;){
  checkAbort()
  try{
   return buildOwnNurbsResolved(document,request,(nodeId,args)=>{
    const ready=completed.get(nodeId)
    if(ready)return ready
    throw new PendingProgressiveSweep(nodeId,args)
   },(nodeId,args)=>{
    const ready=bodies.get(nodeId)
    if(ready)return ready
    throw new PendingProgressiveBody(nodeId,args)
   },(nodeId,args)=>{const ready=miters.get(nodeId);if(ready)return ready;throw new PendingProgressiveMiter(nodeId,args)})
  }catch(error){
   if(error instanceof PendingProgressiveSweep){
    completed.set(error.nodeId,await consume(error.nodeId,
     streamProgressiveNurbsProfiles(...error.args,{shouldAbort:control.shouldAbort})))
   }else if(error instanceof PendingProgressiveBody){
    bodies.set(error.nodeId,await consume(error.nodeId,
     streamProgressiveBrepProfileBody(...error.args,{shouldAbort:control.shouldAbort})))
   }else if(error instanceof PendingProgressiveMiter){
    miters.set(error.nodeId,await consume(error.nodeId,streamProgressiveMiterBrepProfileBody(...error.args,{shouldAbort:control.shouldAbort})))
   }else throw error
  }
 }
}
function buildOwnNurbsResolved(document: unknown, request: OwnNurbsRequest,resolveSweep?:SweepResolver,resolveBody?:BodyResolver,resolveMiter?:MiterResolver) {
    if (!['build', 'evaluate', 'export'].includes(request.action) || (request.evaluations?.length ?? 0) > 64)
        throw new Error('Invalid NURBS request or evaluation budget exceeded.');
    const compiled = compileModelGraphNurbs(document), nodes = new Map(compiled.resolved_document.nodes.map(n => [n.id, n])), cache = new Map<string, Value>();
    const needCurve = (key: string) => { const result = get(key); if (result.kind !== 'curve')
        throw new Error(`Expected curve at ${key}.`); return result.data; };
    const needSurface = (key: string) => { const result = get(key); if (result.kind !== 'surface')
        throw new Error(`Expected surface at ${key}.`); return result.data; };
    const needMesh = (key: string) => { const result = get(key); if (result.kind !== 'mesh')
        throw new Error(`Expected tessellated mesh at ${key}.`); return result.data; };
    const needBrep=(key:string):NurbsBrep=>{const v=get(key);if(v.kind!=='brep')throw new Error('Expected a B-rep body');return v.data;};
    const needSdf=(key:string):SdfField=>{const v=get(key);if(v.kind!=='sdf')throw new Error('Expected SDF field');return v.data;};
    const reconstructionReports:Record<string,unknown>={};
    const miterSources=new Map<string,CertifiedMiterBoundaryOwner>()
    const exactMiterPlacements=new Map<string,{body:CertifiedMiterBoundaryOwner;quantum:number;maxWork:number}>()
    const constructionReports:Record<string,(Omit<ProgressiveMiterBrepBody["approximation"]["report"],"continuousBound"> & {continuousBound:boolean;profileSmoothness:ProgressiveMiterBrepBody["profileSmoothness"];boundaryCertificate:ProgressiveMiterBrepBody["boundaryCertificate"];retainedCapDecomposition:ProgressiveMiterBrepBody["retainedCapDecomposition"];retainedDecomposition:ProgressiveMiterBrepBody["retainedDecomposition"];capParallelism:ProgressiveMiterBrepBody["capParallelism"];levels:ProgressiveMiterBrepBody["approximation"]["levels"];boundaryErrorWithinBudget:ProgressiveMiterBrepBody["boundaryErrorWithinBudget"];boundaryErrorUpper:ProgressiveMiterBrepBody["boundaryErrorUpper"];filledCapErrorUpper:ProgressiveMiterBrepBody["filledCapErrorUpper"];idealCapDomains:ProgressiveMiterBrepBody["idealCapDomains"];capProjection:ProgressiveMiterBrepBody["capProjection"];retainedWallErrorUpper:ProgressiveMiterBrepBody["retainedWallErrorUpper"];sectionCorrection?:ProgressiveMiterBrepBody["sectionCorrection"];wallAudit:ProgressiveMiterBrepBody["wallAudit"];retainedCorrespondence:ProgressiveMiterBrepBody["retainedCorrespondence"];retainedCaps:ProgressiveMiterBrepBody["retainedCaps"];retainedWallCharts:ProgressiveMiterBrepBody["retainedWallCharts"];capDomains:ProgressiveMiterBrepBody["capDomains"];capContacts:ProgressiveMiterBrepBody["capContacts"];capPairs:ProgressiveMiterBrepBody["capPairs"];embedding:ProgressiveMiterBrepBody["embedding"];volume:ProgressiveMiterBrepBody["volume"];globalEmbeddingCertified:false})|ReturnType<typeof reconstructCertifiedMiterStations>|ReturnType<typeof transformCertifiedMiterBody>|MiterBrepBody["report"]|{method:"automatic-loft-alignment"|"loft-end-jets";guideParameters?:number[];guideOrder?:number[];reversed?:boolean[];seams?:unknown[];sectionErrorUpper:number[];guideErrorUpper:number[]}|FramedSweepResult["report"]|NurbsHelixApproximation["report"]|(ProgressiveSweepReport & {levels:ProgressiveSweepReport[];profilePatchRanges:[number,number][]|null;globalEmbeddingCertified?:false})>={};
    let meshTriangles = 0;
    function get(key: string): Value {
        const cached = cache.get(key);
        if (cached)
            return cached;
        const n = nodes.get(key);
        if (!n)
            throw new Error('Unknown node ' + key);
        let result: Value;
        try {
            switch (n.op) {
                case 'parabola_curve': result={kind:'curve',data:parabolaNurbsCurve(n.center,n.axis_u,n.axis_v,n.start,n.end)};break;
                case 'hyperbola_curve': result={kind:'curve',data:hyperbolaNurbsCurve(n.center,n.axis_u,n.axis_v,n.start,n.end)};break;
                case 'elliptic_cylinder_surface': result={kind:'surface',data:ellipticCylinderNurbsSurface(n.center,n.radius_x,n.radius_y,n.height)};break;
                case 'cone_frustum_surface': result={kind:'surface',data:coneFrustumNurbsSurface(n.center,n.bottom_radius,n.top_radius,n.height)};break;
                case 'hyperboloid_one_sheet': result={kind:'surface',data:hyperboloidOneSheetNurbsSurface(n.center,n.radii,n.start,n.end)};break;
                case 'line_curve': result={kind:'curve',data:lineNurbsCurve(n.start,n.end)};break;
                case 'bezier_curve': result={kind:'curve',data:bezierNurbsCurve(n.points,n.weights)};break;
                case 'sphere_surface': result={kind:'surface',data:sphereNurbsSurface(n.center,n.radius)};break;
                case 'cylinder_surface': result={kind:'surface',data:cylinderNurbsSurface(n.center,n.radius,n.height)};break;
                case 'cone_surface': result={kind:'surface',data:coneNurbsSurface(n.center,n.radius,n.height)};break;
                case 'plane_patch': result={kind:'surface',data:planeNurbsPatch(n.origin,n.axis_u,n.axis_v)};break;
                case 'grid_spline_surface': result={kind:'surface',data:gridSplineNurbsSurface(n.points,n.parameters_u,n.parameters_v)};break;
                case 'hermite_patch': result={kind:'surface',data:hermiteNurbsPatch(n.corners,n.tangent_u,n.tangent_v,n.twist)};break;
                case 'bilinear_patch': result={kind:'surface',data:bilinearNurbsPatch(n.corners)};break;
                case 'bezier_surface': result={kind:'surface',data:bezierNurbsSurface(n.points,n.weights)};break;
                case 'transition_polyline_curve': result={kind:'curve',data:transitionPolylineNurbsCurve(n.points,n.setback,n.closed)};break;
                case 'round_polyline_curve': result={kind:'curve',data:roundPolylineNurbsCurve(n.points,n.radius,n.closed)};break;
                case 'curve_compose': result={kind:'curve',data:composeNurbsCurves(n.inputs.map(needCurve))};break;
                case 'polyline_curve': result={kind:'curve',data:polylineNurbsCurve(n.points,n.closed)};break;
                case 'circle_curve': result={kind:'curve',data:circleNurbsCurve(n.center,n.normal,n.radius)};break;
                case 'circle_arc': result={kind:'curve',data:circleNurbsArc(n.center,n.normal,n.radius,n.start_degrees,n.sweep_degrees)};break;
                case 'hyperboloid_two_sheet': result={kind:'surface',data:hyperboloidTwoSheetNurbsSurface(n.center,n.radii,n.start,n.end,n.lower)};break;
                case 'polynomial_graph': result={kind:'surface',data:polynomialGraphNurbsSurface(n.bounds,n.coefficients)};break;
                case 'polynomial_curve': result={kind:'curve',data:polynomialNurbsCurve(n.domain,n.coefficients)};break;
                case 'polynomial_surface': result={kind:'surface',data:polynomialNurbsSurface(n.domain,n.coefficients)};break;
                case 'rational_polynomial_curve': result={kind:'curve',data:rationalPolynomialNurbsCurve(n.domain,n.coefficients)};break;
                case 'rational_polynomial_surface': result={kind:'surface',data:rationalPolynomialNurbsSurface(n.domain,n.coefficients)};break;
                case 'quadratic_patch': result={kind:'surface',data:quadraticNurbsPatch(n.bounds,n.coefficients)};break;
                case 'ellipse_arc': result={kind:'curve',data:ellipseNurbsArc(n.center,n.axis_u,n.axis_v,n.start_degrees,n.sweep_degrees)};break;
                case 'ellipsoid_surface': result={kind:'surface',data:ellipsoidNurbsSurface(n.center,n.radii)};break;
                case 'torus_surface': result={kind:'surface',data:torusNurbsSurface(n.center,n.major_radius,n.radial_radius,n.axial_radius)};break;
                case 'curve': {
                    const c: NurbsCurve = { degree: n.degree, knots: n.knots, controlPoints: n.control_points, weights: n.weights, periodic: n.periodic };
                    validateNurbsCurve(c);
                    result = { kind: 'curve', data: c };
                    break;
                }
                case 'surface': {
                    const s: NurbsSurface = { degreeU: n.degree_u, degreeV: n.degree_v, knotsU: n.knots_u, knotsV: n.knots_v, controlPoints: n.control_points, weights: n.weights, periodicU: n.periodic_u, periodicV: n.periodic_v };
                    validateNurbsSurface(s);
                    result = { kind: 'surface', data: s };
                    break;
                }
                case 'curve_edit': {
                    let c = needCurve(n.input);
                    for (const knot of n.insert_knots ?? [])
                        c = insertNurbsKnot(c, knot.value, knot.count);
                    if (n.degree !== undefined)
                        c = elevateNurbsCurve(c, n.degree);
                    if (n.trim)
                        c = trimNurbsCurve(c, ...n.trim);
                    if (n.reverse)
                        c = reverseNurbsCurve(c);
                    result = { kind: 'curve', data: c };
                    break;
                }
                case 'surface_edit': {
                    let s = needSurface(n.input);
                    for (const k of n.insert_knots_u ?? [])
                        s = insertNurbsSurfaceKnot(s, 'u', k.value, k.count);
                    for (const k of n.insert_knots_v ?? [])
                        s = insertNurbsSurfaceKnot(s, 'v', k.value, k.count);
                    if (n.degree_u !== undefined)
                        s = elevateNurbsSurface(s, 'u', n.degree_u);
                    if (n.degree_v !== undefined)
                        s = elevateNurbsSurface(s, 'v', n.degree_v);
                    if (n.trim)
                        s = trimNurbsSurface(s, ...n.trim);
                    if (n.reverse_u)
                        s = reverseNurbsSurface(s, 'u');
                    if (n.reverse_v)
                        s = reverseNurbsSurface(s, 'v');
                    result = { kind: 'surface', data: s };
                    break;
                }
                case 'iso_curve':
                    result = { kind: 'curve', data: isoNurbsCurve(needSurface(n.input), n.direction, n.parameter) };
                    break;
                case 'closed_spline_curve': result={kind:'curve',data:closedSplineNurbsCurve(n.points,n.parameters)};break;
                case 'clamped_spline_curve': result={kind:'curve',data:clampedSplineNurbsCurve(n.points,n.parameters,n.start_tangent,n.end_tangent)};break;
                case 'natural_spline_curve': result={kind:'curve',data:naturalSplineNurbsCurve(n.points,n.parameters)};break;
                case 'hermite_curve': result={kind:'curve',data:hermiteNurbsCurve(n.points,n.tangents,n.parameters)};break;
                case 'clothoid_curve': {
                    const approximation=approximateClothoidNurbsCurve(n.center,n.length,n.start_curvature,n.end_curvature,n.phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'spherical_spiral_curve': {
                    const approximation=approximateSphericalSpiralNurbsCurve(n.center,n.radius,n.longitude_turns,n.latitude_turns,n.longitude_phase_degrees,n.latitude_phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'toroidal_spiral_curve': {
                    const approximation=approximateToroidalSpiralNurbsCurve(n.center,n.major_radius,n.minor_radius,n.major_turns,n.minor_turns,n.major_phase_degrees,n.minor_phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'torus_knot_curve': {
                    const approximation=approximateTorusKnotNurbsCurve(n.center,n.major_radius,n.minor_radius,n.p,n.q,n.major_phase_degrees,n.minor_phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'helicoid_patches': {
                    const approximation=approximateHelicoidNurbsPatches(n.center,n.inner_radius,n.outer_radius,n.height,n.turns,n.phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'patches',data:{patches:approximation.patches,faceIds:approximation.patches.map((_,i)=>i)}};break;
                }
                case 'circle_rectangle_transition': {const patches=circleRectangleTransitionNurbsPatches({center:n.circle_center,normal:n.circle_normal,seam:n.circle_seam,radius:n.circle_radius},{center:n.rectangle_center,axisU:n.rectangle_axis_u,axisV:n.rectangle_axis_v});result={kind:'patches',data:{patches,faceIds:patches.map((_,i)=>i)}};break;}
                case 'ellipse_transition_surface': result={kind:'surface',data:ellipseTransitionNurbsSurface({center:n.start_center,axisU:n.start_axis_u,axisV:n.start_axis_v},{center:n.end_center,axisU:n.end_axis_u,axisV:n.end_axis_v})};break;
                case 'circle_transition_surface': result={kind:'surface',data:circleTransitionNurbsSurface({center:n.start_center,normal:n.start_normal,seam:n.start_seam,radius:n.start_radius},{center:n.end_center,normal:n.end_normal,seam:n.end_seam,radius:n.end_radius})};break;
                case 'helicoid_surface': {
                    const approximation=approximateHelicoidNurbsSurface(n.center,n.inner_radius,n.outer_radius,n.height,n.turns,n.phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'surface',data:approximation.surface};break;
                }
                case 'catenoid_patches': {
                    const approximation=approximateCatenoidNurbsPatches(n.center,n.scale,n.start_z,n.end_z,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'patches',data:{patches:approximation.patches,faceIds:approximation.patches.map((_,i)=>i)}};break;
                }
                case 'catenoid_surface': {
                    const approximation=approximateCatenoidNurbsSurface(n.center,n.scale,n.start_z,n.end_z,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'surface',data:approximation.surface};break;
                }
                case 'catenary_curve': {
                    const approximation=approximateCatenaryNurbsCurve(n.center,n.scale,n.start_x,n.end_x,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'archimedean_spiral_curve': {
                    const approximation=approximateArchimedeanSpiralNurbsCurve(n.center,n.start_radius,n.end_radius,n.start_degrees,n.end_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'epicycloid_curve': {
                    const approximation=approximateEpicycloidNurbsCurve(n.center,n.fixed_radius,n.rolling_radius,n.start_degrees*Math.PI/180,n.end_degrees*Math.PI/180,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'hypocycloid_curve': {
                    const approximation=approximateHypocycloidNurbsCurve(n.center,n.fixed_radius,n.rolling_radius,n.start_degrees*Math.PI/180,n.end_degrees*Math.PI/180,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'trochoid_curve': {
                    const approximation=approximateTrochoidNurbsCurve(n.center,n.rolling_radius,n.tracing_radius,n.start_degrees*Math.PI/180,n.end_degrees*Math.PI/180,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'cycloid_curve': {
                    const approximation=approximateCycloidNurbsCurve(n.center,n.radius,n.start_degrees*Math.PI/180,n.end_degrees*Math.PI/180,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'lissajous_curve': {
                    const approximation=approximateLissajousNurbsCurve(n.center,n.amplitudes,n.frequencies,n.phases_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'logarithmic_spiral_curve': {
                    const approximation=approximateLogarithmicSpiralNurbsCurve(n.center,n.radius,n.growth,n.start_degrees*Math.PI/180,n.end_degrees*Math.PI/180,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'involute_curve': {
                    const approximation=approximateInvoluteNurbsCurve(n.center,n.radius,n.start_degrees*Math.PI/180,n.end_degrees*Math.PI/180,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'helix_curve': {
                    const approximation=approximateHelixNurbsCurve(n.center,n.radius,n.height,n.turns,n.phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'elliptic_helix_curve': {
                    const approximation=approximateEllipticHelixNurbsCurve(n.center,n.radius_x,n.radius_y,n.height,n.turns,n.phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'variable_pitch_helix_curve': {
                    const approximation=approximateVariablePitchHelixNurbsCurve(n.center,n.radius,n.height,n.turns,n.start_pitch,n.end_pitch,n.phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'conical_helix_curve': {
                    const approximation=approximateConicalHelixNurbsCurve(n.center,n.start_radius,n.end_radius,n.height,n.turns,n.phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'curve',data:approximation.curve};break;
                }
                case 'formula_surface': result={kind:'surface',data:formulaNurbsSurface(n.domain,n.expressions)};break;
                case 'formula_curve': result={kind:'curve',data:formulaNurbsCurve(n.domain,n.expressions)};break;
                case 'coons_patch': result={kind:'surface',data:coonsNurbsPatch(n.inputs.map(needCurve))};break;
                case 'ruled_surface':
                    result = { kind: 'surface', data: loftNurbsCurves(n.inputs.map(needCurve)) };
                    break;
                case 'polygon_profile': result={kind:'profile',data:{outer:n.outer,holes:n.holes}};break;
                case 'extrude': {
                    const v=get(n.input);const vector=[0,0,n.height];
                    if(v.kind==='curve')result={kind:'surface',data:extrudeNurbsCurve(v.data,vector)};
                    else if(v.kind==='profile')result={kind:'mesh',data:extrudePolygonProfile(v.data,vector)};
                    else throw new Error('Extrude expects a NURBS curve or polygon profile');break;
                }
                case 'revolve': {
                    const v=get(n.input);
                    if(v.kind==='curve')result={kind:'surface',data:revolveNurbsCurve(v.data,[0,0,0],[0,0,1],n.angle)};
                    else if(v.kind==='profile'){if(v.data.holes?.length)throw new Error('Revolve with profile holes is not supported');result={kind:'mesh',data:revolvePolygonProfile(v.data.outer,n.angle,n.segments)};}
                    else throw new Error('Revolve expects a NURBS curve or polygon profile');break;
                }
                case 'polygon_extrude': {const v=get(n.input);if(v.kind!=='profile')throw new Error('Expected polygon profile');result={kind:'mesh',data:extrudePolygonProfile(v.data,n.vector)};break;}
                case 'polygon_sweep': {const v=get(n.input);if(v.kind!=='profile')throw new Error('Expected polygon profile');if(v.data.holes?.length)throw new Error('Sweep with profile holes is not supported');result={kind:'mesh',data:sweepPolygonProfile(v.data.outer,n.path,n.up)};break;}
                case 'polygon_loft': result={kind:'mesh',data:loftPolygonSections(n.sections)};break;
                case 'surface_sweep': result={kind:'surface',data:sweepNurbsCurve(needCurve(n.inputs[0]),needCurve(n.inputs[1]))};break;
                case 'twist_sweep': result={kind:'surface',data:twistSweepNurbsCurve(needCurve(n.inputs[0]),needCurve(n.inputs[1]),n.origin,n.axis,n.start_degrees,n.sweep_degrees)};break;
                case 'two_guide_sweep': result={kind:'surface',data:twoGuideSweepNurbsCurve(needCurve(n.inputs[0]),needCurve(n.inputs[1]),needCurve(n.inputs[2]),n.width,n.axis_y,n.axis_z)};break;
                case 'scaled_sweep': result={kind:'surface',data:scaledSweepNurbsCurve(needCurve(n.inputs[0]),needCurve(n.inputs[1]),n.scale,n.origin)};break;
                case 'progressive_sweep': {
                    if(n.orientation==='authored'&&(!n.frame_axis||!n.frame_normal))throw new Error('Authored progressive sweep requires frame_axis and frame_normal');
                    if((n.contact_parameter!==undefined&&!n.orientation_guide)||(n.contact_profile!==undefined&&n.contact_parameter===undefined))throw new Error('Contact anchor requires orientation_guide and contact_parameter');
                    const frameOptions=n.orientation==='authored'?{orientation:'authored' as const,frameAxis:n.frame_axis!,frameNormal:n.frame_normal!}:{orientation:n.orientation};
                    const sweep=(...args:SweepArguments)=>resolveSweep?resolveSweep(key,args):progressiveSweepNurbsProfiles(...args);
                    const builtSweep=sweep(n.inputs.slice(0,-1).map(needCurve),needCurve(n.inputs[n.inputs.length-1]!),n.scale,n.twist,{...(n.orientation_guide?{orientationGuide:needCurve(n.orientation_guide),...(n.contact_parameter!==undefined?{contactAnchor:{profileIndex:n.contact_profile??0,parameter:n.contact_parameter}}:{})}:{}),axisScale:n.axis_scale,centerLaw:n.center_law,normal:n.normal,...frameOptions,spacing:n.spacing,initialSections:n.initial_sections,maxSections:n.max_sections,maxDeviation:n.max_deviation,lengthTolerance:n.length_tolerance,lengthMaxCells:n.length_max_cells});
                    if(!builtSweep.report.accepted||!builtSweep.patches){
                        const report=builtSweep.report;
                        const error=report.knownProfileErrorUpper!=null&&report.knownProfileErrorUpper>report.budget
                            ?`continuous retained-patch error ${report.knownProfileErrorUpper}`
                            :`sampled refinement ${report.sampledControlDeviation}`;
                        throw new Error(`Progressive sweep ${error}mm exceeds ${report.budget}mm at ${report.sections} sections`);
                    }
                    constructionReports[key]={...builtSweep.report,levels:builtSweep.levels,profilePatchRanges:builtSweep.profilePatchRanges};
                    result={kind:'patches',data:{patches:builtSweep.patches,faceIds:builtSweep.patches.map((_,i)=>i)}};break;
                }
                case 'profile_sweep': {
                    const sweep=checkedProfileSweepNurbsSurface(needCurve(n.inputs[0]),needCurve(n.inputs[1]),n.scale,n.normal,n.sections,n.max_deviation);
                    if(!sweep.report.accepted||!sweep.surface)throw new Error(`Profile sweep sampled refinement ${sweep.report.sampledControlDeviation}mm exceeds ${sweep.report.budget}mm; refine sections or split the path`);
                    constructionReports[key]=sweep.report;
                    result={kind:'surface',data:sweep.surface};break;
                }
                case 'framed_sweep': {
                    const sweep=framedSweepNurbsCurve(needCurve(n.inputs[0]),needCurve(n.inputs[1]),n.normal,n.sections,n.max_deviation);
                    if(!sweep.report.accepted||!sweep.surface)throw new Error(`Framed sweep sampled refinement ${sweep.report.sampledControlDeviation}mm exceeds ${sweep.report.budget}mm; refine sections or split the path`);
                    constructionReports[key]=sweep.report;
                    result={kind:'surface',data:sweep.surface};break;
                }
                case 'control_tangent_loft_surface': result={kind:'surface',data:controlTangentLoftNurbsCurves(n.inputs.map(needCurve),n.parameters,n.start_tangents,n.end_tangents)};break;
                case 'auto_guided_loft_surface': {
                    const built=autoGuidedLoftNurbsCurves(n.inputs.map(needCurve),n.parameters,n.guides.map(needCurve),n.budget);
                    constructionReports[key]={method:"automatic-loft-alignment",guideParameters:built.guide_parameters,guideOrder:built.guide_order,reversed:built.reversed,sectionErrorUpper:built.section_error_upper,guideErrorUpper:built.guide_error_upper};
                    result={kind:'surface',data:built.surface};break;
                }
                case 'loft_match_surface': {
                    const start=n.start_reference?{reference:needSurface(n.start_reference),boundary:n.start_boundary,order:n.order as 1|2,scale:n.start_scale,reverse:n.start_reverse}:undefined;
                    const end=n.end_reference?{reference:needSurface(n.end_reference),boundary:n.end_boundary,order:n.order as 1|2,scale:n.end_scale,reverse:n.end_reverse}:undefined;
                    const built=matchNurbsLoftEnds(needSurface(n.input),n.sections.map(needCurve),n.parameters,n.budget,start,end,n.guides.map(needCurve),n.guide_parameters);
                    constructionReports[key]={method:"loft-end-jets",seams:built.seams,sectionErrorUpper:built.section_error_upper,guideErrorUpper:built.guide_error_upper};
                    result={kind:'surface',data:built.surface};break;
                }
                case 'brep_progressive_miter_sweep': {
                    if(n.cap_correction_tolerance===undefined&&(n.cap_correction_quantum!==undefined||n.cap_correction_max_work!==undefined||n.cap_correction_authored_frame===true))throw new Error('Miter cap correction requires a displacement tolerance');
                    if(n.circle_correction_tolerance===undefined&&(n.circle_correction_quantum!==undefined||n.circle_correction_max_work!==undefined))throw new Error('Miter circle correction requires a displacement tolerance');
                    const args:MiterArguments=[n.loops.map(wire=>wire.map(needCurve)),n.points,n.scale,n.twist,{...(n.circle_correction_tolerance===undefined?{}:{circleCorrection:{tolerance:n.circle_correction_tolerance,quantum:n.circle_correction_quantum??2**-40,maxWork:n.circle_correction_max_work??1000000}}),retainedWallMaxInjectivityCells:n.retained_wall_max_injectivity_cells,...(n.cap_correction_tolerance===undefined?{}:{capCorrection:{tolerance:n.cap_correction_tolerance,quantum:n.cap_correction_quantum??2**-40,maxWork:n.cap_correction_max_work??1000000,authoredFrame:n.cap_correction_authored_frame}}),...(n.orientation_guide?{orientationGuide:needCurve(n.orientation_guide)}:{}),frameAxis:n.frame_axis,frameNormal:n.frame_normal,axisScale:n.axis_scale,centerLaw:n.center_law,normal:n.normal,closed:n.closed,miterLimit:n.miter_limit,initialSteps:n.initial_steps,maxSteps:n.max_steps,maxDeviation:n.max_deviation}];
                    const body=resolveMiter?resolveMiter(key,args):createProgressiveMiterBrepProfileBody(...args);
                    miterSources.set(key,body);
                    constructionReports[key]={...body.approximation.report,continuousBound:body.boundaryCertificate.continuousBound,profileSmoothness:body.profileSmoothness,boundaryCertificate:body.boundaryCertificate,wallRegularityCertified:body.retainedWallCharts.allChartsCertified,levels:body.approximation.levels,sectionCorrection:body.sectionCorrection,retainedWallErrorUpper:body.retainedWallErrorUpper,capProjection:body.capProjection,capParallelism:body.capParallelism,retainedCapDecomposition:body.retainedCapDecomposition,retainedDecomposition:body.retainedDecomposition,idealCapDomains:body.idealCapDomains,filledCapErrorUpper:body.filledCapErrorUpper,boundaryErrorUpper:body.boundaryErrorUpper,boundaryErrorWithinBudget:body.boundaryErrorWithinBudget,wallAudit:body.wallAudit,retainedCorrespondence:body.retainedCorrespondence,retainedCaps:body.retainedCaps,retainedWallCharts:body.retainedWallCharts,capDomains:body.capDomains,capContacts:body.capContacts,capPairs:body.capPairs,embedding:body.embedding,volume:body.volume,globalEmbeddingCertified:false};
                    if(n.circle_correction_tolerance!==undefined)exactMiterPlacements.set(key,{body,quantum:n.circle_correction_quantum??2**-40,maxWork:n.circle_correction_max_work??1000000});
                    result={kind:'brep',data:body.model};break;
                }
                case 'brep_smooth_miter_stations': {
                    needBrep(n.input);
                    const source=miterSources.get(n.input);
                    if(!source)throw new Error('Station reconstruction requires a progressive miter source with owned retained sections');
                    const body=reconstructCertifiedMiterStations(source,{quantum:n.quantum,maxWork:n.max_work,wallTolerance:n.wall_tolerance,maxDeviation:n.max_deviation});
                    constructionReports[key]=body;
                    exactMiterPlacements.set(key,{body,quantum:n.quantum,maxWork:n.max_work});
                    result={kind:'brep',data:body.model};break;
                }
                case 'brep_miter_sweep': {if(n.cap_correction_tolerance===undefined&&(n.cap_correction_quantum!==undefined||n.cap_correction_max_work!==undefined))throw new Error('Miter cap correction requires a displacement tolerance');const body=createMiterBrepProfileBody(n.loops.map(wire=>wire.map(needCurve)),n.points,n.normal,n.miter_limit,n.closed,n.cap_correction_tolerance===undefined?undefined:{tolerance:n.cap_correction_tolerance,quantum:n.cap_correction_quantum??2**-40,maxWork:n.cap_correction_max_work??1000000});constructionReports[key]=body.report;result={kind:'brep',data:body.model};break;}
                case 'brep_natural_loft': result={kind:'brep',data:createNaturalBrepSectionLoft(n.sections.map(s=>s.map(r=>r.map(needCurve))),n.parameters)};break;
                case 'brep_capped_loft': result={kind:'brep',data:createCappedBrepLoftSurfaces(n.start.map(r=>r.map(needCurve)),n.end.map(r=>r.map(needCurve)),n.sides.map(r=>r.map(needSurface)))};break;
                case 'guided_loft_surface': result={kind:'surface',data:guidedLoftNurbsCurves(n.inputs.map(needCurve),n.parameters,n.guides.map(needCurve),n.guide_parameters,n.start_tangents,n.end_tangents)};break;
                case 'clamped_loft_surface': result={kind:'surface',data:clampedLoftNurbsCurves(n.inputs.map(needCurve),n.parameters,n.start_tangent,n.end_tangent)};break;
                case 'ribbon_surface': {
                    const pipe=checkedRibbonNurbsSurface(needCurve(n.input),needCurve(n.width_law),n.normal,n.sections,n.max_deviation);
                    if(!pipe.report.accepted||!pipe.surface)throw new Error(`Ribbon sampled refinement ${pipe.report.sampledControlDeviation}mm exceeds ${pipe.report.budget}mm; refine sections or split the path`);
                    constructionReports[key]=pipe.report;
                    result={kind:'surface',data:pipe.surface};break;
                }
                case 'variable_pipe_surface': {
                    const pipe=checkedVariablePipeNurbsSurface(needCurve(n.input),needCurve(n.radius_law),n.normal,n.sections,n.max_deviation);
                    if(!pipe.report.accepted||!pipe.surface)throw new Error(`Variable pipe sampled refinement ${pipe.report.sampledControlDeviation}mm exceeds ${pipe.report.budget}mm; refine sections or split the path`);
                    constructionReports[key]=pipe.report;
                    result={kind:'surface',data:pipe.surface};break;
                }
                case 'pipe_surface': {
                    const pipe=checkedPipeNurbsSurface(needCurve(n.input),n.radius,n.normal,n.sections,n.max_deviation);
                    if(!pipe.report.accepted||!pipe.surface)throw new Error(`Pipe sampled refinement ${pipe.report.sampledControlDeviation}mm exceeds ${pipe.report.budget}mm; refine sections or split the path`);
                    constructionReports[key]=pipe.report;
                    result={kind:'surface',data:pipe.surface};break;
                }
                case 'screw_surface': {
                    const approximation=approximateScrewNurbsSurfaces(needCurve(n.input),n.origin,n.axis,n.height,n.turns,n.phase_degrees,n.max_deviation);
                    constructionReports[key]=approximation.report;
                    result={kind:'patches',data:{patches:approximation.patches,faceIds:approximation.patches.map((_,i)=>i)}};break;
                }
                case 'surface_extrude_patches': {const patches=extrudeNurbsCurvePatches(needCurve(n.input),n.vector);result={kind:'patches',data:{patches,faceIds:patches.map((_,i)=>i)}};break;}
                case 'boundary_fill': {const patches=boundaryFillNurbsSurfaces(n.inputs.map(needCurve),n.center);result={kind:'patches',data:{patches,faceIds:patches.map((_,i)=>i)}};break;}
                case 'triangular_patch': result={kind:'surface',data:triangularNurbsPatch(needCurve(n.inputs[0]),needCurve(n.inputs[1]),needCurve(n.inputs[2]))};break;
                case 'gordon_surface': result={kind:'surface',data:gordonNurbsSurface(n.u_curves.map(needCurve),n.v_curves.map(needCurve),n.parameters_u,n.parameters_v)};break;
                case 'closed_loft_surface': result={kind:'surface',data:closedLoftNurbsCurves(n.inputs.map(needCurve),n.parameters)};break;
                case 'natural_loft_surface': result={kind:'surface',data:naturalLoftNurbsCurves(n.inputs.map(needCurve),n.parameters)};break;
                case 'surface_loft': result={kind:'surface',data:loftAlignedNurbsCurves(n.inputs.map(needCurve))};break;
                case 'triangle_mesh': {
                    const mesh={positions:Float64Array.from(n.vertices.flat()),indices:Uint32Array.from(n.triangles.flat())};
                    result={kind:'mesh',data:{...mesh,report:inspectPolygonMesh(mesh)}};break;
                }
                case 'mesh_to_nurbs_brep': result={kind:'brep',data:meshToNurbsBrep(needMesh(n.input))};break;
                case 'mesh_to_sdf': result={kind:'sdf',data:meshToSdf(needMesh(n.input))};break;
                case 'mesh_to_subdivision': {const fitted=meshToSubdivision(needMesh(n.input),n.iterations);const {cage,...report}=fitted;reconstructionReports[key]=report;result={kind:'subdivision',data:cage};break;}
                case 'subdivision_tessellate': {
                    const v=get(n.input);if(v.kind!=='subdivision')throw new Error('Expected subdivision cage');result={kind:'mesh',data:tessellateSubdivision(v.data,n.levels)};break;
                }
                case 'mesh_to_nurbs': result={kind:'patches',data:meshToNurbs(needMesh(n.input))};break;
                case 'mesh_fit_nurbs': result={kind:'patches',data:meshToNurbs(needMesh(n.input),'point_normal',n.max_deviation)};break;
                case 'nurbs_patches_tessellate': {
                    const v=get(n.input);if(v.kind!=='patches')throw new Error('Expected NURBS patch set');result={kind:'mesh',data:tessellateNurbsPatches(v.data,n.segments)};break;
                }
                case 'subdivision':
                    result={kind:'mesh',data:tessellateSubdivision({vertices:n.vertices,faces:n.faces},n.levels)};break;
                case 'sdf_sphere': result={kind:'sdf',data:{kind:'sphere',center:n.center,radius:n.radius}};break;
                case 'sdf_box': result={kind:'sdf',data:{kind:'box',center:n.center,half_size:n.half_size}};break;
                case 'sdf_torus': result={kind:'sdf',data:{kind:'torus',center:n.center,major_radius:n.major_radius,minor_radius:n.minor_radius}};break;
                case 'sdf_union': case 'sdf_intersection': case 'sdf_difference':
                    result={kind:'sdf',data:{kind:n.op==='sdf_union'?'union':n.op==='sdf_intersection'?'intersection':'difference',a:needSdf(n.inputs[0]),b:needSdf(n.inputs[1])}};break;
                case 'sdf_smooth_union': result={kind:'sdf',data:{kind:'smooth_union',a:needSdf(n.inputs[0]),b:needSdf(n.inputs[1]),radius:n.radius}};break;
                case 'sdf_offset': result={kind:'sdf',data:{kind:'offset',input:needSdf(n.input),distance:n.distance}};break;
                case 'sdf_translate': result={kind:'sdf',data:{kind:'translate',input:needSdf(n.input),vector:n.vector}};break;
                case 'sdf_tessellate': result={kind:'mesh',data:tessellateSdf(needSdf(n.input),{min:n.min,max:n.max,cells:n.cells})};break;
                case 'brep_box':
                    result={kind:'brep',data:createBrepBox(n.min,n.max)};
                    break;
                case 'brep_sphere': result={kind:'brep',data:createBrepSphere(n.radius)};break;
                case 'brep_gear': result={kind:'brep',data:createBrepGear({module:n.module,teeth:n.teeth,height:n.height,pressureAngle:n.pressure_angle,helixAngle:n.helix_angle,herringbone:n.herringbone,bore:n.bore,internal:n.internal,rimWidth:n.rim_width,clearance:n.clearance,backlash:n.backlash})};break;
                case 'brep_torus': result={kind:'brep',data:createBrepTorus(n.major_radius,n.minor_radius)};break;
                case 'brep_cylinder': result={kind:'brep',data:createBrepCylinder(n.radius,n.height)};break;
                case 'brep_frustum': result={kind:'brep',data:createBrepFrustum(n.bottom_radius,n.top_radius,n.height)};break;
                case 'brep_tube': result={kind:'brep',data:createBrepTube(n.outer_radius,n.inner_radius,n.height)};break;
                case 'brep_revolve': {
                    const profile=get(n.input);if(profile.kind!=='profile'||profile.data.holes?.length)throw new Error('Exact revolve requires one polygon profile without holes');
                    result={kind:'brep',data:revolveBrepProfile(profile.data.outer as [number,number][],n.angle)};break;
                }
                case 'brep_extrude': {
                    const profile=get(n.input);if(profile.kind!=='profile')throw new Error('Expected a polygon profile');
                    result={kind:'brep',data:extrudeBrepPolygon(profile.data.outer as [number,number][],Math.min(0,n.height),Math.max(0,n.height),profile.data.holes as [number,number][][])};break;
                }
                case 'brep_progressive_sweep': {
                    if(n.orientation==='authored'&&(!n.frame_axis||!n.frame_normal))throw new Error('Authored progressive body requires frame_axis and frame_normal');
                    if((n.contact_parameter!==undefined&&!n.orientation_guide)||(n.contact_profile!==undefined&&n.contact_parameter===undefined))throw new Error('Contact anchor requires orientation_guide and contact_parameter');
                    const frameOptions=n.orientation==='authored'?{orientation:'authored' as const,frameAxis:n.frame_axis!,frameNormal:n.frame_normal!}:{orientation:n.orientation};
                    const makeBody=(...args:BodyArguments)=>resolveBody?resolveBody(key,args):createProgressiveBrepProfileBody(...args);
                    const body=makeBody(n.loops.map(wire=>wire.map(needCurve)),needCurve(n.inputs[0]),n.scale,n.twist,{...(n.orientation_guide?{orientationGuide:needCurve(n.orientation_guide),...(n.contact_parameter!==undefined?{contactAnchor:{profileIndex:n.contact_profile??0,parameter:n.contact_parameter}}:{})}:{}),axisScale:n.axis_scale,centerLaw:n.center_law,normal:n.normal,...frameOptions,spacing:n.spacing,initialSections:n.initial_sections,maxSections:n.max_sections,maxDeviation:n.max_deviation,lengthTolerance:n.length_tolerance,lengthMaxCells:n.length_max_cells});
                    constructionReports[key]={...body.approximation.report,levels:body.approximation.levels,profilePatchRanges:body.approximation.profilePatchRanges,globalEmbeddingCertified:body.globalEmbeddingCertified};
                    result={kind:'brep',data:body.model};break;
                }
                case 'brep_extrude_curves':
                    result={kind:'brep',data:extrudeBrepCurves(n.loops.map(wire=>wire.map(needCurve)),n.z_min,n.z_max)};break;
                case 'brep_boolean': result={kind:'brep',data:booleanNurbsBrep(needBrep(n.inputs[0]),needBrep(n.inputs[1]),n.operation)};break;
                case 'brep_chamfer': result={kind:'brep',data:chamferNurbsBrepEdges(needBrep(n.input),n.edges,n.size)};break;
                case 'brep_fillet': result={kind:'brep',data:filletNurbsBrepEdges(needBrep(n.input),n.edges,n.radius,n.segments)};break;
                case 'brep_tessellate': {
                    const input=get(n.input);if(input.kind!=='brep')throw new Error('Expected a B-rep model');
                    result={kind:'mesh',data:tessellateNurbsBrep(input.data,n.segments)};
                    break;
                }
                case 'surface_extrude':
                    result = { kind: 'surface', data: extrudeNurbsCurve(needCurve(n.input), n.vector) };
                    break;
                case 'surface_revolve':
                    result = { kind: 'surface', data: revolveNurbsCurve(needCurve(n.input), n.origin, n.axis, n.angle) };
                    break;
                case 'transform': {
                    const m = n.matrix;
                    const v = get(n.input);
                    if(v.kind==='curve')result={kind:'curve',data:transformNurbsCurve(v.data,m)};
                    else if(v.kind==='surface')result={kind:'surface',data:transformNurbsSurface(v.data,m)};
                    else if(v.kind==='patches')result={kind:'patches',data:{patches:transformNurbsSurfacePatches(v.data.patches,m),faceIds:[...v.data.faceIds]}};
                    else if(v.kind==='brep'){
                        const source=exactMiterPlacements.get(n.input)
                        if(source){
                            const placed=transformCertifiedMiterBody(source.body,m,{quantum:source.quantum,maxWork:source.maxWork,maxDeviation:source.body.boundaryCertificate.budget})
                            constructionReports[key]=placed
                            exactMiterPlacements.set(key,{...source,body:placed})
                            result={kind:'brep',data:placed.model}
                        }else result={kind:'brep',data:transformNurbsBrep(v.data,m)}
                    }
                    else
                        throw new Error('Transform spline control data before tessellation.');
                    break;
                }
                case 'tessellate': {
                    if (n.trim && n.trim_curves)
                        throw new Error('Use only one trim representation.');
                    const sample = (id: string) => { const c = needCurve(id); if (c.controlPoints[0].length !== 2)
                        throw new Error('UV trim requires a 2D curve.'); const a = c.knots[c.degree], b = c.knots[c.controlPoints.length]; const first = evaluateNurbsCurve(c, a).point, last = evaluateNurbsCurve(c, b).point; if (Math.hypot(...first.map((v, i) => v - last[i])) > 1e-9)
                        throw new Error('UV trim curve must be closed.'); return Array.from({ length: n.trim_segments }, (_, i) => evaluateNurbsCurve(c, a + (b - a) * i / n.trim_segments).point); };
                    const trim = n.trim_curves ? { outer: sample(n.trim_curves.outer), holes: n.trim_curves.holes.map(sample) } : n.trim;
                    result = { kind: 'mesh', data: tessellateNurbsSurface(needSurface(n.input), { segmentsU: n.segments_u, segmentsV: n.segments_v, ...(trim ? { trim } : {}) }) };
                    break;
                }
                case 'mesh_boolean':
                    result = { kind: 'mesh', data: booleanPolygonMeshes(needMesh(n.inputs[0]), needMesh(n.inputs[1]), n.operation, n.relative_tolerance === undefined ? {} : { relativeTolerance: n.relative_tolerance }) };
                    break;
                case 'thicken':
                    result = { kind: 'mesh', data: thickenNurbsMesh(needMesh(n.input), n.vector) };
                    break;
            }
        }
        catch (error) {
            if(error instanceof PendingProgressiveSweep||error instanceof PendingProgressiveBody||error instanceof PendingProgressiveMiter)throw error;
            throw new Error(`Node ${key}: ${error instanceof Error ? error.message : 'NURBS operation failed.'}`);
        }
        if(result.kind==='sdf')evaluateSdf(result.data,[0,0,0]);
        if (result.kind === 'mesh' && (meshTriangles += result.data.indices.length / 3) > 60000)
            throw new Error('NURBS graph mesh work exceeds 60000 triangles.');
        cache.set(key, result);
        return result;
    }
    const root = get(compiled.document.root);
    const definitions = Object.fromEntries([...cache].filter(([, v]) => v.kind !== 'mesh').map(([id, v]) => [id, { kind: v.kind, ...v.data }]));
    const meshBounds = root.kind === 'mesh' && root.data.positions.length > 0 ? { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] } : null;
    if (root.kind === 'mesh' && meshBounds)
        for (let i = 0; i < root.data.positions.length; i++) {
            const axis = i % 3, v = root.data.positions[i];
            meshBounds.min[axis] = Math.min(meshBounds.min[axis], v);
            meshBounds.max[axis] = Math.max(meshBounds.max[axis], v);
        }
    const patchBounds=root.kind==='patches'&&root.data.patches.length>0?{min:[Infinity,Infinity,Infinity],max:[-Infinity,-Infinity,-Infinity]}:null;
    if(root.kind==='patches'&&patchBounds){
        for(const patch of root.data.patches){
            const bounds=nurbsSurfaceBounds(patch);
            for(let axis=0;axis<3;axis++){
                patchBounds.min[axis]=Math.min(patchBounds.min[axis],bounds.min[axis]);
                patchBounds.max[axis]=Math.max(patchBounds.max[axis],bounds.max[axis]);
            }
        }
    }
    const extendedGeometry=compiled.document.nodes.some(n=>n.op.startsWith('polygon_')||n.op==='subdivision'||n.op==='triangle_mesh'||n.op.startsWith('mesh_to_')||n.op==='mesh_fit_nurbs'||n.op.startsWith('sdf_'));
    const foundationCertificate = root.kind === 'curve' ? certifyNurbsCurveFoundation(root.data)
        : root.kind === 'surface' ? certifyNurbsSurfaceFoundation(root.data) : null;
    const report = { ...(Object.keys(constructionReports).length?{construction:constructionReports}:{}), ...(Object.keys(reconstructionReports).length?{reconstruction:reconstructionReports}:{}), bounds_scope: foundationCertificate ? 'outward-rounded rational span hulls' : root.kind === 'mesh' ? 'derived mesh vertices' : 'conservative control hull', kernel: extendedGeometry?'own-rust-geometry':'own-rust-nurbs', polygon_core: 'own-rust-polygons', geometry_authority: extendedGeometry?'source geometry definitions':'rational control data', root: compiled.document.root, root_kind: root.kind, bounds: root.kind === 'curve' ? nurbsCurveBounds(root.data) : root.kind === 'surface' ? nurbsSurfaceBounds(root.data) : root.kind === 'patches' ? patchBounds : meshBounds, foundation_certificate: foundationCertificate, foundation_successor: foundationCertificate ? {version:'nurbs-ss/1',curve_projection:'Bernstein stationary isolation',surface_projection:'2d outward hull subdivision with Krawczyk/interval uniqueness',normal_regularity:'recursive sub-knot-cell homogeneous cone localization',simplification:'adaptive Lipschitz rollback certified',periodic_editing:'wrapped cyclic collocation with C0/C1/C2 seam evidence',reparameterization:'piecewise positive rational monotone with nested/periodic exact control-net materialization',fitting:'ranked bounded approximate-only with cloud Hausdorff enclosure',intersection:'certified general CC/CS/SS with Bernstein/Krawczyk isolation, BranchGraph, CoedgeTrim maps; Boolean mutation deferred'} : null, definitions, mesh: root.kind === 'mesh' ? root.data.report : null, printability: 'unknown', error_bound_certified: foundationCertificate !== null && Object.keys(constructionReports).length === 0 };
    const evaluations = (request.evaluations ?? []).map(q => { const v = get(q.node); if (v.kind === 'curve')
        return { node: q.node, ...evaluateNurbsCurve(v.data, q.u) }; if (v.kind === 'surface' && q.v !== undefined)
        return { node: q.node, ...evaluateNurbsSurface(v.data, q.u, q.v) }; throw new Error('Evaluation requires a curve or surface, and v for a surface.'); });
    const base = { ok: true, document_sha256: compiled.document_sha256, execution_target: 'own-nurbs', automatic_fallback: false, report, evaluations };
    if (request.action === 'export') {
        if (request.format === 'json')
            return { ...base, artifact: { format: 'json', mime_type: 'application/json', text: stringifyMeshJson(compiled.document, 2) } };
        if (!request.format || root.kind !== 'mesh')
            throw new Error('STL export requires a closed tessellated mesh. Export JSON to retain native NURBS definitions.');
        if (request.format !== 'stl') { const artifact = exportMeshFormat(root.data, request.format); return {...base, artifact: {format: request.format, mime_type: artifact.mimeType, extension: artifact.extension, base64: meshExportBase64(artifact.data)}}; }
        const text = exportNurbsStl(root.data);
        if (text.length > 4 * 1024 * 1024)
            throw new Error('STL export exceeds 4 MiB.');
        return { ...base, artifact: { format: 'stl', mime_type: 'model/stl', text } };
    }
    if(request.action==='build' && request.display) {
        const {segments,subdivisionLevels}=request.display;
        if(!Number.isInteger(segments)||segments<1||segments>32||!Number.isInteger(subdivisionLevels)||subdivisionLevels<0||subdivisionLevels>4)throw new Error('Invalid display tessellation policy');
        let mesh:Mesh|undefined=root.kind==='mesh'?root.data:undefined;
        if(root.kind==='surface')mesh=tessellateNurbsSurface(root.data,{segmentsU:segments,segmentsV:segments});
        if(root.kind==='brep')mesh=tessellateNurbsBrep(root.data,segments);
        if(root.kind==='subdivision')mesh=tessellateSubdivision(root.data,subdivisionLevels);
        if(root.kind==='patches')mesh=tessellateNurbsPatches(root.data,segments);
        let sourceNode=compiled.document.root,sourceValue=root;
        const rootNode=nodes.get(sourceNode)!;
        // Only known display conversions preserve source face correspondence.
        // Boolean/thicken/edit results own their mesh; do not invent CAD lineage.
        if(['tessellate','brep_tessellate','subdivision_tessellate','sdf_tessellate','nurbs_patches_tessellate'].includes(rootNode.op)&&'input' in rootNode) {
            sourceNode=rootNode.input;sourceValue=get(sourceNode);
        }
        if(rootNode.op==='subdivision')sourceValue={kind:'subdivision',data:{vertices:rootNode.vertices,faces:rootNode.faces}};
        const boundary=rootNode.op==='tessellate'&&(rootNode.trim||rootNode.trim_curves)?{trim:rootNode.trim??null,trimCurves:rootNode.trim_curves?{outer:needCurve(rootNode.trim_curves.outer),holes:rootNode.trim_curves.holes.map(needCurve)}:null}:null;
        const sourceConstruction=constructionReports[sourceNode];
        // Evidence is snapshot-local and belongs only to this exact source
        // node. Derived/edited nodes do not inherit another body's certificate.
        const sweepEvidence=sourceConstruction && 'volume' in sourceConstruction ? {
            volume:sourceConstruction.volume,
            profileSmoothness:'profileSmoothness' in sourceConstruction?sourceConstruction.profileSmoothness:null,
            boundaryCertificate:'boundaryCertificate' in sourceConstruction?sourceConstruction.boundaryCertificate:null,
            certifiedErrorUpper:'certifiedErrorUpper' in sourceConstruction?sourceConstruction.certifiedErrorUpper:null,
            retainedCapDecomposition:'retainedCapDecomposition' in sourceConstruction?sourceConstruction.retainedCapDecomposition:null,
            retainedDecomposition:'retainedDecomposition' in sourceConstruction?sourceConstruction.retainedDecomposition:null,
            retainedWallErrorUpper:'retainedWallErrorUpper' in sourceConstruction?sourceConstruction.retainedWallErrorUpper:null,
            capParallelism:'capParallelism' in sourceConstruction?sourceConstruction.capParallelism:null,
            capProjection:'capProjection' in sourceConstruction?sourceConstruction.capProjection:null,
            idealCapDomains:'idealCapDomains' in sourceConstruction?sourceConstruction.idealCapDomains:null,
            filledCapErrorUpper:'filledCapErrorUpper' in sourceConstruction?sourceConstruction.filledCapErrorUpper:null,
            boundaryErrorUpper:'boundaryErrorUpper' in sourceConstruction?sourceConstruction.boundaryErrorUpper:null,
            boundaryErrorWithinBudget:'boundaryErrorWithinBudget' in sourceConstruction?sourceConstruction.boundaryErrorWithinBudget:null,
            boundaryErrorBudget:'boundaryErrorWithinBudget' in sourceConstruction&&'budget' in sourceConstruction?sourceConstruction.budget:null,
            sectionCorrection:'sectionCorrection' in sourceConstruction?sourceConstruction.sectionCorrection:null,
            endpointContourErrorUpper:'endpointContourErrorUpper' in sourceConstruction?sourceConstruction.endpointContourErrorUpper:null,
            authoredFramesApplied:'authoredFramesApplied' in sourceConstruction?sourceConstruction.authoredFramesApplied:false,
            orientationGuideApplied:'orientationGuideApplied' in sourceConstruction?sourceConstruction.orientationGuideApplied:false,
            affineLawsApplied:'affineLawsApplied' in sourceConstruction?sourceConstruction.affineLawsApplied:false,
            profileRegularityCertified:'profileRegularityCertified' in sourceConstruction?sourceConstruction.profileRegularityCertified:false,
            wallRegularityCertified:'wallRegularityCertified' in sourceConstruction?sourceConstruction.wallRegularityCertified:false,
            continuousBound:sourceConstruction.continuousBound,
        }:undefined;
        const nativeGeometry=createNativeGeometryArtifact(sourceNode,sourceValue.kind,{geometry:sourceValue.data,boundary,...(sweepEvidence?{sweepEvidence}: {})},compiled.document);
        return {...base,...(mesh?{mesh}:{}),nativeGeometry};
    }
    return { ...base, ...(request.action === 'build' && root.kind === 'mesh' ? { mesh: root.data } : {}) };
}
