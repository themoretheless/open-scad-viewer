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
import {reconstructCertifiedMiterStations,transformCertifiedMiterBody,type CertifiedMiterBoundaryOwner,createProgressiveMiterBrepProfileBody,streamProgressiveMiterBrepProfileBody,type ProgressiveMiterBrepBody,createMiterBrepProfileBody,type MiterBrepBody,streamProgressiveBrepProfileBody,createNaturalBrepSectionLoft,createCappedBrepLoftWithCaps,createCappedBrepLoftSurfaces,createProgressiveBrepProfileBody,transformNurbsBrep,createBrepSphere,createBrepGear,createBrepTorus,createBrepBox,revolveBrepProfile,createBrepCylinder,createBrepFrustum,createBrepTube,extrudeBrepCurves,extrudeBrepPolygon,booleanNurbsBrep,chamferNurbsBrepEdges,filletNurbsBrepEdges,tessellateNurbsBrep,type NurbsBrep} from './geometry/brep';
import { inspectPolygonMesh,booleanPolygonMeshes } from './geometry/polygon';
import { exportMeshFormat, meshExportBase64, type MeshExportFormat } from './meshExportFormats';
import { compileModelGraphNurbs } from './modelGraphNurbsCompiler';
import { validateNurbsCurve, evaluateNurbsCurve, insertNurbsKnot, elevateNurbsCurve, trimNurbsCurve, reverseNurbsCurve, nurbsCurveBounds, type NurbsCurve } from './nurbsCurve';
import { validateNurbsSurface, evaluateNurbsSurface, insertNurbsSurfaceKnot, elevateNurbsSurface, trimNurbsSurface, reverseNurbsSurface, isoNurbsCurve, nurbsSurfaceBounds, type NurbsSurface } from './nurbsSurface';
import {createMappedNaturalLoft,createCartesianGuidedLoft,createControlTangentCartesianGuidedLoft,createAutoCartesianGuidedLoft,createAutoControlTangentCartesianGuidedLoft,certifyNurbsCurveFoundation, certifyNurbsSurfaceFoundation} from './nurbsFoundation';
import { autoGuidedLoftNurbsCurves,matchNurbsLoftEnds,guidedLoftNurbsCurves,controlTangentLoftNurbsCurves,boundaryFillNurbsSurfaces,triangularNurbsPatch,gordonNurbsSurface,closedLoftNurbsCurves,clampedLoftNurbsCurves,naturalLoftNurbsCurves,loftAlignedNurbsCurves,sweepNurbsCurve,loftNurbsCurves, extrudeNurbsCurve, revolveNurbsCurve } from './nurbsConstructors';
import { tessellateNurbsSurface, thickenNurbsMesh, exportNurbsStl } from './geometry/tessellation';
export { collectSdfJobs, buildOwnNurbs, buildOwnNurbsAsync } from './modelGraphNurbsKernel/collectSdfJobs'
export type Mesh = ReturnType<typeof tessellateNurbsSurface>;
export type Value = {kind:'profile';data:PolygonProfile} | {kind:'patches';data:NurbsSurfaceSet} | {kind:'subdivision';data:SubdivisionCage} | {kind:'sdf';data:SdfField} | {kind:'brep';data:NurbsBrep} | {
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

export type SweepArguments=Parameters<typeof progressiveSweepNurbsProfiles>
export type SweepResolver=(nodeId:string,args:SweepArguments)=>ReturnType<typeof progressiveSweepNurbsProfiles>
export class PendingProgressiveSweep {
 constructor(readonly nodeId:string,readonly args:SweepArguments){}
}
export type BodyArguments=Parameters<typeof createProgressiveBrepProfileBody>
export type BodyResolver=(nodeId:string,args:BodyArguments)=>ReturnType<typeof createProgressiveBrepProfileBody>
export class PendingProgressiveBody {
 constructor(readonly nodeId:string,readonly args:BodyArguments){}
}
export type MiterArguments=Parameters<typeof createProgressiveMiterBrepProfileBody>
export type MiterResolver=(nodeId:string,args:MiterArguments)=>ProgressiveMiterBrepBody
export class PendingProgressiveMiter {constructor(readonly nodeId:string,readonly args:MiterArguments){}}
export interface OwnNurbsBuildControl {
 shouldAbort?:()=>boolean
 onYield?:()=>void
 onSweepPreview?:(nodeId:string,preview:ProgressiveSweepPreview)=>void|Promise<void>
}
