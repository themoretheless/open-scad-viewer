import {validCurveOffsetDiagnostics} from './curveOffsetDiagnostics'
import {solidDistanceExpectation,validSolidDistance,type SolidDistanceOptions,type SolidDistanceResult} from './solidDistance'
import {selfIntersectionExpectation,validSelfIntersection,type SelfIntersection} from './solidSelfIntersection'
import {faceContactExpectation,validFaceContacts,type FaceContacts,type FaceContactLimits} from './solidFaceContacts'
import {boundaryExpectation,validBoundaryAgreement,type BoundaryAgreement} from './solidBoundaryAgreement'
import type {SnapGeometry} from './modelingSnaps'
import {validBodySnapGeometry,validSketchSnapGeometry} from './solidSnapProtocol'
import type {prepareSolidFaceSketch} from './solidFaceSketch'
import type {solidBodyEdges} from './solidBodyEdges'
import type {solidTopology} from './directSolidTools'
import {validSolidTopology} from './solidTopologyProtocol'
import type {NurbsCurve,NurbsCurveDistance} from './nurbsCurve'
import type {BrepProfile} from './geometry/brepProfile'
import type {SolidNurbsSurface} from './solidNurbs'
import type {SurfaceBoundaryOptions,SurfaceBoundaryReport} from './solidSurfaceDiagnostics'
import type {NurbsSurface,NurbsSurfaceDistance} from './nurbsSurface'
import type {PointMeasurement,CurveMeasurement,FaceDistanceOptions,FaceDistanceResult,ShellDistanceOptions,ShellDistanceResult} from './solidMeasurements'
import type {SolidPrimitiveOptions} from './solidPrimitive'
import type {DisplayMesh} from './solidDisplayCache'
import type {NurbsBrep} from './geometry/brep'
import type {SolidBrepToolOptions,SolidBrepToolResult} from './solidBrepTool'
import type {SolidNurbsEditOptions} from './solidNurbsEdit'
import type {SolidCurveOffsetOptions,offsetSolidCurve} from './solidCurveOffset'
import type {SolidPointEditOptions} from './solidPointEdit'
import type {SolidSketchEditOptions} from './solidSketchEdit'
import type {SolidBooleanOptions,SolidBooleanResult} from './solidBoolean'
import type {SolidSceneEditOptions} from './solidSceneEdit'
import type {matchSolidCurve} from './solidCurveMatching'
import type {matchSolidSurface,prepareSolidSurfaceSeams} from './solidSurfaceMatching'
import type {SolidSurfaceBuildOptions,buildSolidSurface} from './solidSurfaceConstruction'
import type {SolidNurbsRefitOptions,refitSolidNurbs} from './solidCurveReduction'
import type {prepareSolidProfile} from './solidProfilePreparation'
import type {SolidProfileEditOptions} from './solidProfileEdit'
import type {SolidBodyEditOptions} from './solidBodyEdit'
import type {SolidRevolveOptions} from './solidRevolve'
import type {DirectExtrusionOptions} from './directExtrusion'
import type {inspectSolidIntersections} from './solidDiagnostics'
import type {PolygonMesh} from './geometry/polygon'
import {parseBondedSolidInput,isBondedSolidResult,type BondedSolidResult} from './bondedSolidProtocol'
import type {StructuralSections,BoundaryConnectivity} from './structuralSections'
import type {MeshData} from '../core/mesh'
import type {CadOptions} from './cadWorkbench'
import type {CadPairReport} from './cadInspection'
import type {DirectBody, DirectSketch, DirectDocument} from './directModeling'
import type {MainOperation, MainParameters} from './mainModeling'
import type {PickHit} from './rendererContracts'
import type {TrussInput, TrussResponse} from './trussAnalysis'
import {isNominalLatticeGraph, type LatticeGraphMesh, type NominalLatticeGraph} from './latticeGraphProtocol'
import type {LighteningOptions} from './solidLightening'

export type MainSolidJob =
  | {kind:'sketchSnaps';sketch:DirectSketch}
  | {kind:'bodySnaps';body:DirectBody}
  | {kind:'faceSketch';body:DirectBody;face:number}
  | {kind:'bodyEdges';body:DirectBody}
  | {kind:'topology';mesh:PolygonMesh}
  | {kind:'curveDisplay';curve:NurbsCurve}
  | {kind:'profileDisplay';profile:BrepProfile}
  | {kind:'surfaceMesh';item:SolidNurbsSurface}
  | {kind:'surfaceBoundary';a:NurbsSurface;b:NurbsSurface;options:SurfaceBoundaryOptions}
  | {kind:'selfIntersection';model:NurbsBrep;toleranceUv:number;limits:FaceContactLimits;maxSpans:number}
  | {kind:'faceContacts';model:NurbsBrep;toleranceUv:number;limits:FaceContactLimits}
  | {kind:'boundaryAgreement';model:NurbsBrep;maxCells:number}
  | {kind:'solidDistance';options:SolidDistanceOptions}
  | {kind:'shellDistance';options:ShellDistanceOptions}
  | {kind:'faceDistance';options:FaceDistanceOptions}
  | {kind:'surfaceDistance';a:NurbsSurface;b:NurbsSurface;toleranceMm:number;maxCells:number}
  | {kind:'curveDistance';a:NurbsCurve;b:NurbsCurve;toleranceMm:number;maxCells:number}
  | {kind:'measureVertices';a:DirectBody;indexA:number;b:DirectBody;indexB:number}
  | {kind:'measureEdge';body:DirectBody;edge:number;parameter:number}
  | {kind:'displayMesh';mesh:PolygonMesh;brep?:NurbsBrep;segments:number}
  | {kind:'restoreDocument';text:string}
  | {kind:'primitive';document:DirectDocument;options:SolidPrimitiveOptions}
  | {kind:'modelGraphImport';document:DirectDocument;text:string;group?:string}
  | {kind:'brepTool';document:DirectDocument;options:SolidBrepToolOptions}
  | {kind:'nurbsEdit';document:DirectDocument;options:SolidNurbsEditOptions}
  | {kind:'curveChainInspection';document:DirectDocument;ids:string[];maxPairs:number}
  | {kind:'curveOffset';document:DirectDocument;options:SolidCurveOffsetOptions}
  | {kind:'pointEdit';document:DirectDocument;options:SolidPointEditOptions}
  | {kind:'sketchEdit';document:DirectDocument;options:SolidSketchEditOptions}
  | {kind:'boolean';document:DirectDocument;options:SolidBooleanOptions}
  | {kind:'sceneEdit';document:DirectDocument;options:SolidSceneEditOptions}
  | {kind:'curveMatch';args:Parameters<typeof matchSolidCurve>}
  | {kind:'surfaceMatch';args:Parameters<typeof matchSolidSurface>}
  | {kind:'seamPrepare';args:Parameters<typeof prepareSolidSurfaceSeams>}
  | {kind:'surfaceBuild';document:DirectDocument;options:SolidSurfaceBuildOptions}
  | {kind:'nurbsRefit';document:DirectDocument;options:SolidNurbsRefitOptions}
  | {kind:'profilePrepare';document:DirectDocument;ids:string[];tolerance:number}
  | {kind:'profileEdit';document:DirectDocument;options:SolidProfileEditOptions}
  | {kind:'bodyEdit';document:DirectDocument;options:SolidBodyEditOptions}
  | {kind:'revolve'; document:DirectDocument; options:SolidRevolveOptions}
  | {kind:'extrusion'; document:DirectDocument; options:DirectExtrusionOptions}
  | {kind:'main'; meshes:MeshData[]; selected:number; hit:PickHit|null; operation:MainOperation; parameters:MainParameters}
  | {kind:'cad'; document:DirectDocument; options:CadOptions}
  | {kind:'inspect'; bodies:DirectBody[]}
  | {kind:'meshContacts'; mesh:PolygonMesh;maxWork?:number;maxContacts?:number}
  | {kind:'truss'; model:TrussInput}
  | {kind:'bondedSolid';inputJson:string}
  | {kind:'structuralSections'; mesh:LatticeGraphMesh; axis:'x'|'y'|'z'; stations:number[]}
  | {kind:'latticeGraph'; mesh:LatticeGraphMesh; options:LighteningOptions}
export interface MainSolidResults {curveChainInspection:import('./curveOffsetDiagnostics').CurveOffsetDiagnostics;solidDistance:SolidDistanceResult;selfIntersection:SelfIntersection;faceContacts:FaceContacts;boundaryAgreement:BoundaryAgreement;shellDistance:ShellDistanceResult;faceDistance:FaceDistanceResult;surfaceDistance:NurbsSurfaceDistance;curveDistance:NurbsCurveDistance;sketchSnaps:SnapGeometry;bodySnaps:SnapGeometry;faceSketch:ReturnType<typeof prepareSolidFaceSketch>;bodyEdges:ReturnType<typeof solidBodyEdges>;topology:ReturnType<typeof solidTopology>;curveDisplay:number[][];profileDisplay:[number,number][][];surfaceMesh:PolygonMesh;surfaceBoundary:SurfaceBoundaryReport;measureVertices:PointMeasurement;measureEdge:CurveMeasurement;primitive:DirectDocument;modelGraphImport:DirectDocument;displayMesh:DisplayMesh;restoreDocument:DirectDocument;brepTool:SolidBrepToolResult;nurbsEdit:DirectDocument;curveOffset:ReturnType<typeof offsetSolidCurve>;pointEdit:DirectDocument;sketchEdit:DirectDocument;boolean:SolidBooleanResult;sceneEdit:DirectDocument;curveMatch:ReturnType<typeof matchSolidCurve>;surfaceMatch:ReturnType<typeof matchSolidSurface>;seamPrepare:ReturnType<typeof prepareSolidSurfaceSeams>;surfaceBuild:ReturnType<typeof buildSolidSurface>;nurbsRefit:ReturnType<typeof refitSolidNurbs>;profilePrepare:ReturnType<typeof prepareSolidProfile>;profileEdit:DirectDocument;bodyEdit:DirectDocument;revolve:DirectDocument;extrusion:DirectDocument;meshContacts:ReturnType<typeof inspectSolidIntersections>;bondedSolid:BondedSolidResult; main:DirectDocument; cad:DirectDocument; inspect:CadPairReport[]; truss:TrussResponse; latticeGraph:NominalLatticeGraph; structuralSections:StructuralSections}
export type MainSolidRequest = {version:1; id:number; job:MainSolidJob}
export type MainSolidResponse = {version:1; id:number; kind:MainSolidJob['kind']} & (
  | {ok:true; result:MainSolidResults[keyof MainSolidResults]}
  | {ok:false; error:{name:string; message:string; code?:string}}
)

const finite = (v:unknown): v is number => typeof v==='number' && Number.isFinite(v)
function arrayOf(value:unknown, length:number, check:(v:unknown)=>boolean):boolean {
  if(!Array.isArray(value)||value.length!==length)return false
  for(const item of value)if(!check(item))return false
  return true
}
/** Mesh buffers arrive as typed views after the worker transfer; plain arrays from JSON. */
const isNumericSequence=(v:unknown):v is ArrayLike<number>=>
  Array.isArray(v)||(ArrayBuffer.isView(v)&&!(v instanceof DataView))
function numericSequenceOf(value:unknown, length:number, check:(v:unknown)=>boolean):boolean {
  if(!isNumericSequence(value)||value.length!==length)return false
  for(let i=0;i<length;i++)if(!check(value[i]))return false
  return true
}
const vector = (v:unknown) => arrayOf(v,3,finite)
export type MainSolidExpectation = {kind:'displayMesh';triangles:number}
  | ({kind:'solidDistance'}&ReturnType<typeof solidDistanceExpectation>)
  | ({kind:'selfIntersection'}&ReturnType<typeof selfIntersectionExpectation>)
  | ({kind:'faceContacts'}&ReturnType<typeof faceContactExpectation>)
  | ({kind:'boundaryAgreement'}&ReturnType<typeof boundaryExpectation>)
  | {kind:'shellDistance';domains:[number,number][][][];toleranceMm:number;toleranceUv:number;maxCells:number;maxDomainCells:number}
  | {kind:'faceDistance';domains:[number,number][][];toleranceMm:number;toleranceUv:number;maxCells:number;maxDomainCells:number}
  | {kind:'surfaceDistance';domains:[number,number][][];toleranceMm:number;maxCells:number}
  | {kind:'curveDistance';dimension:number;domains:[number,number][];toleranceMm:number;maxCells:number}
  | {kind:'sketchSnaps';maxPoints:number;maxSegments:number;maxCircles:number}
  | {kind:'bodySnaps';maxPoints:number;maxSegments:number}
  | {kind:'bodyEdges';vertices:number;edges:number;exact:boolean}
  | {kind:'topology';vertices:number;triangles:number}
  | {kind:'curveChainInspection';segments:number;maxPairs:number}
  | {kind:'curveDisplay';dimension:number}
  | {kind:'profileDisplay';counts:number[]}
  | {kind:'surfaceMesh';u:number;v:number}
  | {kind:'surfaceBoundary';samples:number}
  | {kind:'brepTool';id:string;operation:'mass'|'mesh'|'display'}
  | {kind:'meshContacts';triangles:number;maxWork:number;maxContacts:number}
  | {kind:'truss'; nodes:number; members:number}
  | {kind:'structuralSections';axis:'x'|'y'|'z';stations:number[]}
  | {kind:'bondedSolid';nodes:number;tets:number;bonds:number}
  | {kind:Exclude<MainSolidJob['kind'],'curveChainInspection'|'solidDistance'|'selfIntersection'|'faceContacts'|'boundaryAgreement'|'shellDistance'|'faceDistance'|'surfaceDistance'|'curveDistance'|'sketchSnaps'|'bodySnaps'|'bodyEdges'|'topology'|'curveDisplay'|'profileDisplay'|'surfaceMesh'|'surfaceBoundary'|'displayMesh'|'brepTool'|'truss'|'structuralSections'|'bondedSolid'|'meshContacts'>}
export function mainSolidExpectation(job:MainSolidJob):MainSolidExpectation {
  if(job.kind==='solidDistance')return {kind:job.kind,...solidDistanceExpectation(job.options)}
  if(job.kind==='selfIntersection')return {kind:job.kind,...selfIntersectionExpectation(job.model,job.toleranceUv,job.limits,job.maxSpans)}
  if(job.kind==='faceContacts')return {kind:job.kind,...faceContactExpectation(job.model,job.toleranceUv,job.limits)}
  if(job.kind==='boundaryAgreement')return {kind:job.kind,...boundaryExpectation(job.model,job.maxCells)}
  if(job.kind==='shellDistance'){const o=job.options;return {kind:job.kind,domains:[o.a,o.b].map(m=>m.faces.map(({surface:s})=>[[s.knotsU[s.degreeU],s.knotsU[s.controlPoints.length]],[s.knotsV[s.degreeV],s.knotsV[s.controlPoints[0]?.length??0]]])),toleranceMm:o.toleranceMm,toleranceUv:o.toleranceUv,maxCells:o.maxCells,maxDomainCells:o.maxDomainCells}}
  if(job.kind==='faceDistance'){const o=job.options;return {kind:job.kind,domains:[o.a.faces[o.faceA]?.surface,o.b.faces[o.faceB]?.surface].map(s=>s?[[s.knotsU[s.degreeU],s.knotsU[s.controlPoints.length]],[s.knotsV[s.degreeV],s.knotsV[s.controlPoints[0]?.length??0]]]:[]),toleranceMm:o.toleranceMm,toleranceUv:o.toleranceUv,maxCells:o.maxCells,maxDomainCells:o.maxDomainCells}}
  if(job.kind==='surfaceDistance')return {kind:job.kind,domains:[job.a,job.b].map(s=>[[s.knotsU[s.degreeU],s.knotsU[s.controlPoints.length]],[s.knotsV[s.degreeV],s.knotsV[s.controlPoints[0]?.length??0]]]),toleranceMm:job.toleranceMm,maxCells:job.maxCells}
  if(job.kind==='curveDistance')return {kind:job.kind,dimension:job.a.controlPoints[0]?.length??0,domains:[job.a,job.b].map(c=>[c.knots[c.degree],c.knots[c.controlPoints.length]]),toleranceMm:job.toleranceMm,maxCells:job.maxCells}
  if(job.kind==='sketchSnaps'){const s=job.sketch,n=s.retainedProfile?s.retainedProfile.loops.reduce((sum,loop)=>sum+loop.length,0):s.points.length;return {kind:job.kind,maxPoints:s.retainedProfile?n*3:s.analytic?5:n*2+2,maxSegments:s.retainedProfile?s.retainedProfile.loops.flat().reduce((sum,c)=>sum+(c.controlPoints.length-c.degree)*(c.degree===1?1:16),0):s.analytic?0:n,maxCircles:!s.retainedProfile&&s.analytic?1:0}}
  if(job.kind==='bodySnaps'){const edges=job.body.brep?.edges.length??job.body.mesh.indices.length;return {kind:job.kind,maxPoints:edges*4+job.body.mesh.indices.length/3+1,maxSegments:edges*16}}
  if(job.kind==='bodyEdges')return {kind:job.kind,vertices:job.body.brep?.vertices.length??job.body.mesh.positions.length/3,edges:job.body.brep?.edges.length??job.body.mesh.indices.length,exact:!!job.body.brep}
  if(job.kind==='topology')return {kind:job.kind,vertices:job.mesh.positions.length/3,triangles:job.mesh.indices.length/3}
  if(job.kind==='curveChainInspection')return {kind:job.kind,maxPairs:job.maxPairs,segments:job.ids.reduce((n,id)=>n+(job.document.curves?.find(c=>c.id===id)?.curve.controlPoints.length??1)-1,0)}
  if(job.kind==='curveDisplay')return {kind:job.kind,dimension:job.curve.controlPoints[0]?.length??0}
  if(job.kind==='profileDisplay')return {kind:job.kind,counts:job.profile.loops.map(loop=>loop.reduce((n,c)=>n+(c.degree===1?1:24),0))}
  if(job.kind==='surfaceMesh')return {kind:job.kind,u:job.item.segmentsU,v:job.item.segmentsV}
  if(job.kind==='surfaceBoundary')return {kind:job.kind,samples:job.options.samples}
  if(job.kind==='displayMesh')return {kind:job.kind,triangles:job.mesh.indices.length/3}
  if(job.kind==='brepTool')return {kind:job.kind,id:job.options.id,operation:job.options.kind}
  if(job.kind==='meshContacts')return {kind:job.kind,triangles:job.mesh.indices.length/3,maxWork:job.maxWork??200_000,maxContacts:job.maxContacts??10_000}
  if(job.kind==='bondedSolid'){
    const m=parseBondedSolidInput(job.inputJson)
    return {kind:job.kind,nodes:m.nodesMm.length,tets:m.tets.length,bonds:m.bonds.length}
  }
  return job.kind==='truss' ? {kind:job.kind,nodes:job.model.nodesMm.length,members:job.model.members.length} : job.kind==='structuralSections'?{kind:job.kind,axis:job.axis,stations:[...job.stations]}:{kind:job.kind}
}

/** Admit the result for this request, not merely any object with a result field. */
export function mainSolidResult(job:MainSolidExpectation, value:unknown): boolean {
  if (!value || typeof value!=='object') return false
  if(job.kind==='solidDistance')return validSolidDistance(job,value)
  if(job.kind==='selfIntersection')return validSelfIntersection(job,value)
  if(job.kind==='faceContacts')return validFaceContacts(job,value)
  if(job.kind==='boundaryAgreement')return validBoundaryAgreement(job,value)
  if(job.kind==='shellDistance'){
    const v=value as ShellDistanceResult
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    if(v.method!=='interval-trimmed-face-pairs'||v.scope!=='boundary-shells-bounded-joins'||v.containment!=='not-classified'
      ||v.toleranceMm!==job.toleranceMm||v.toleranceUv!==job.toleranceUv||v.maxCells!==job.maxCells||v.maxDomainCells!==job.maxDomainCells
      ||v.pairs!==job.domains[0].length*job.domains[1].length||!Number.isInteger(v.evaluatedPairs)||v.evaluatedPairs<0||v.evaluatedPairs>v.pairs
      ||!Number.isInteger(v.cells)||v.cells<0||v.cells>job.maxCells||!Number.isInteger(v.domainCells)||v.domainCells<0||v.domainCells>job.maxDomainCells
      ||!Array.isArray(v.distanceIntervalMm)||v.distanceIntervalMm.length!==2||!finite(v.distanceIntervalMm[0])||v.distanceIntervalMm[0]<0||typeof v.converged!=='boolean')return false
    const [lo,hi]=v.distanceIntervalMm
    if(hi===null){if(v.faces!==null||v.parameters!==null||v.points!==null||v.pointEnclosures!==null||v.converged)return false}
    else if(!finite(hi)||hi<lo||!arrayOf(v.faces,2,f=>Number.isInteger(f)&&Number(f)>=0)
      ||!v.faces!.every((f,i)=>!!job.domains[i][f])
      ||!arrayOf(v.parameters,2,uv=>arrayOf(uv,2,finite))
      ||!v.parameters!.every((uv,i)=>uv.every((t,k)=>t>=job.domains[i][v.faces![i]][k][0]&&t<=job.domains[i][v.faces![i]][k][1]))
      ||!arrayOf(v.points,2,vector)||!arrayOf(v.pointEnclosures,2,p=>arrayOf(p,3,interval))||v.evaluatedPairs===0||v.cells===0||v.domainCells===0)return false
    return v.converged?v.reason==='tolerance'&&hi!==null&&hi-lo<=job.toleranceMm
      :['work-limit','domain-work-limit','pair-resolution-limit','empty-domain'].includes(v.reason)&&(hi===null||hi-lo>job.toleranceMm)&&(v.reason!=='empty-domain'||hi===null)
  }
  if(job.kind==='faceDistance'){
    const v=value as FaceDistanceResult
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    if(v.method!=='interval-trimmed-surface-subdivision'||v.scope!=='trimmed-surfaces-bounded-joins'
      ||v.toleranceMm!==job.toleranceMm||v.toleranceUv!==job.toleranceUv||v.maxCells!==job.maxCells||v.maxDomainCells!==job.maxDomainCells
      ||!Number.isInteger(v.cells)||v.cells<1||v.cells>job.maxCells||!Number.isInteger(v.domainCells)||v.domainCells<1||v.domainCells>job.maxDomainCells
      ||!Array.isArray(v.distanceIntervalMm)||v.distanceIntervalMm.length!==2||!finite(v.distanceIntervalMm[0])||v.distanceIntervalMm[0]<0||typeof v.converged!=='boolean')return false
    const [lo,hi]=v.distanceIntervalMm
    if(hi===null){if(v.parameters!==null||v.points!==null||v.pointEnclosures!==null||v.converged)return false}
    else if(!finite(hi)||hi<lo||!arrayOf(v.parameters,2,uv=>arrayOf(uv,2,finite))
      ||!v.parameters!.every((uv,i)=>uv.every((t,k)=>!!job.domains[i][k]&&t>=job.domains[i][k][0]&&t<=job.domains[i][k][1]))
      ||!arrayOf(v.points,2,vector)||!arrayOf(v.pointEnclosures,2,p=>arrayOf(p,3,interval)))return false
    return v.converged?v.reason==='tolerance'&&hi!==null&&hi-lo<=job.toleranceMm
      :['work-limit','domain-work-limit','precision-limit','empty-domain'].includes(v.reason)&&(hi===null||hi-lo>job.toleranceMm)&&(v.reason!=='empty-domain'||hi===null)
  }
  if(job.kind==='surfaceDistance'){
    const v=value as NurbsSurfaceDistance
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    return v.method==='interval-tensor-de-boor-pair-subdivision'&&v.scope==='untrimmed-surfaces'
      &&v.toleranceMm===job.toleranceMm&&v.maxCells===job.maxCells
      &&Number.isInteger(v.cells)&&v.cells>=1&&v.cells<=job.maxCells
      &&interval(v.distanceIntervalMm)&&v.distanceIntervalMm[0]>=0
      &&arrayOf(v.parameters,2,uv=>arrayOf(uv,2,finite))&&v.parameters.every((uv,i)=>uv.every((t,k)=>t>=job.domains[i][k][0]&&t<=job.domains[i][k][1]))
      &&arrayOf(v.points,2,vector)&&arrayOf(v.pointEnclosures,2,p=>arrayOf(p,3,interval))
      &&typeof v.converged==='boolean'
      &&(v.converged?v.reason==='tolerance'&&v.distanceIntervalMm[1]-v.distanceIntervalMm[0]<=job.toleranceMm
        :['work-limit','precision-limit'].includes(v.reason)&&v.distanceIntervalMm[1]-v.distanceIntervalMm[0]>job.toleranceMm)
  }
  if(job.kind==='curveDistance'){
    const v=value as NurbsCurveDistance
    const interval=(r:unknown)=>arrayOf(r,2,finite)&&(r as number[])[0]<=(r as number[])[1]
    return [2,3].includes(job.dimension)&&v.method==='interval-de-boor-pair-subdivision'
      &&v.toleranceMm===job.toleranceMm&&v.maxCells===job.maxCells
      &&Number.isInteger(v.cells)&&v.cells>=1&&v.cells<=job.maxCells
      &&interval(v.distanceIntervalMm)&&v.distanceIntervalMm[0]>=0
      &&arrayOf(v.parameters,2,finite)&&v.parameters.every((t,i)=>t>=job.domains[i][0]&&t<=job.domains[i][1])
      &&arrayOf(v.points,2,p=>arrayOf(p,job.dimension,finite))
      &&arrayOf(v.pointEnclosures,2,p=>arrayOf(p,job.dimension,interval))
      &&typeof v.converged==='boolean'
      &&(v.converged?v.reason==='tolerance'&&v.distanceIntervalMm[1]-v.distanceIntervalMm[0]<=job.toleranceMm
        :['work-limit','precision-limit'].includes(v.reason)&&v.distanceIntervalMm[1]-v.distanceIntervalMm[0]>job.toleranceMm)
  }
  if(job.kind==='sketchSnaps')return validSketchSnapGeometry(value,job.maxPoints,job.maxSegments,job.maxCircles)
  if(job.kind==='bodySnaps')return validBodySnapGeometry(value,job.maxPoints,job.maxSegments)
  if(job.kind==='faceSketch'){const v=value as MainSolidResults['faceSketch'];return !!v.plane&&vector(v.plane.origin)&&vector(v.plane.u)&&vector(v.plane.v)&&vector(v.normal)&&Array.isArray(v.outline)&&v.outline.length>0&&v.outline.every(loop=>Array.isArray(loop)&&loop.length>=3&&loop.every(p=>arrayOf(p,2,finite)))}
  if(job.kind==='bodyEdges')return Array.isArray(value)&&(job.exact?value.length===job.edges:value.length<=job.edges)&&value.every((e,i)=>e&&e.index===i&&typeof e.id==='string'&&e.id.length>0&&[e.a,e.b].every(n=>Number.isSafeInteger(n)&&n>=0&&n<job.vertices)&&Array.isArray(e.points)&&e.points.length>=2&&e.points.every(vector))
  if(job.kind==='topology')return validSolidTopology(value,job.vertices,job.triangles)
  if(job.kind==='curveChainInspection')return validCurveOffsetDiagnostics(value,job.segments)&&value.checks<=job.maxPairs
  if(job.kind==='curveDisplay')return [2,3].includes(job.dimension)&&Array.isArray(value)&&value.length===49&&value.every(p=>Array.isArray(p)&&p.length===job.dimension&&p.every(finite))
  if(job.kind==='profileDisplay')return Array.isArray(value)&&value.length===job.counts.length&&value.every((loop,i)=>Array.isArray(loop)&&loop.length===job.counts[i]&&loop.every(p=>Array.isArray(p)&&p.length===2&&p.every(finite)))
  if(job.kind==='surfaceMesh'){
    const m=value as PolygonMesh
    if(![job.u,job.v].every(n=>Number.isInteger(n)&&n>=2&&n<=64)||!isNumericSequence(m.positions)||!isNumericSequence(m.indices))return false
    // The native mesher triangulates each untrimmed grid cell around its center.
    const vertices=m.positions.length/3,triangles=m.indices.length/3
    return Number.isInteger(vertices)&&vertices>0&&vertices<=(job.u+1)*(job.v+1)+job.u*job.v&&Number.isInteger(triangles)&&triangles<=job.u*job.v*4
      &&numericSequenceOf(m.positions,m.positions.length,finite)&&numericSequenceOf(m.indices,m.indices.length,i=>Number.isSafeInteger(i)&&Number(i)>=0&&Number(i)<vertices)
  }
  if(job.kind==='surfaceBoundary'){
    const v=value as SurfaceBoundaryReport,angle=(a:unknown)=>a===null||finite(a)&&a>=0&&a<=90
    return Number.isInteger(job.samples)&&job.samples>=2&&job.samples<=257&&v.samplingOnly===true
      &&finite(v.maxGapMm)&&v.maxGapMm>=0&&angle(v.maxTangentPlaneAngleDeg)
      &&Number.isInteger(v.undefinedNormals)&&v.undefinedNormals>=0&&v.undefinedNormals<=job.samples
      &&Number.isInteger(v.worstGapSample)&&v.worstGapSample>=0&&v.worstGapSample<job.samples&&typeof v.sampledWithinTolerance==='boolean'
      &&arrayOf(v.samples,job.samples,item=>{const s=item as SurfaceBoundaryReport['samples'][number];return !!s&&finite(s.t)&&s.t>=0&&s.t<=1&&vector(s.a)&&vector(s.b)&&finite(s.gapMm)&&s.gapMm>=0&&angle(s.tangentPlaneAngleDeg)})
  }
  if(job.kind==='measureVertices'){
    const v=value as PointMeasurement
    return vector(v.a)&&vector(v.b)&&vector(v.deltaMm)&&finite(v.distanceMm)&&v.distanceMm>=0
  }
  if(job.kind==='measureEdge'){
    const v=value as CurveMeasurement
    return vector(v.point)&&finite(v.parameter)&&v.parameter>=0&&v.parameter<=1&&finite(v.curvaturePerMm)&&v.curvaturePerMm>=0
      &&(v.radiusMm===null||finite(v.radiusMm)&&v.radiusMm>0)
  }
  if(job.kind==='displayMesh'){
    const v=value as DisplayMesh,m=v.mesh
    if(!m||!isNumericSequence(m.positions)||!isNumericSequence(m.indices)||m.positions.length%3||m.indices.length%3)return false
    const count=m.indices.length/3,vertices=m.positions.length/3
    return numericSequenceOf(m.positions,m.positions.length,finite)
      &&numericSequenceOf(m.indices,m.indices.length,i=>Number.isSafeInteger(i)&&Number(i)>=0&&Number(i)<vertices)
      &&(m.uv===undefined||numericSequenceOf(m.uv,vertices*2,finite))
      &&arrayOf(v.normals,count,vector)
      &&(v.closed==null||arrayOf(v.closed,count,x=>typeof x==='boolean'))
      &&(v.workClosed==null||arrayOf(v.workClosed,job.triangles,x=>typeof x==='boolean'))
      &&(v.map===null?count===job.triangles:count<=4000&&arrayOf(v.map,count,i=>Number.isSafeInteger(i)&&Number(i)>=0&&Number(i)<job.triangles))
  }
  if(job.kind==='brepTool'){
    const v=value as SolidBrepToolResult
    if(v.id!==job.id||v.kind!==job.operation)return false
    if(v.kind==='display'){
      const d=v.diagnostics
      return !!d&&!!d.report&&['boundaryEdges','nonManifoldEdges','orientationConflicts','degenerateTriangles'].every(k=>Number.isSafeInteger(d.report[k as keyof typeof d.report])&&Number(d.report[k as keyof typeof d.report])>=0)
        &&Array.isArray(d.defectLines)&&d.defectLines.every(l=>typeof l.kind==='string'&&Array.isArray(l.points)&&l.points.every(vector))
        &&!!d.section&&vector(d.section.normal)&&finite(d.section.offsetMm)&&Array.isArray(d.section.contours)
        &&d.section.contours.every(c=>Array.isArray(c.points)&&c.points.every(vector)&&Array.isArray(c.sourceTriangles))
        &&Array.isArray(d.section.collapsedSegmentTriangles)&&typeof d.sectionError==='string'&&typeof d.boundaryError==='string'
    }
    if(v.kind==='mesh')return mainSolidResult({kind:'extrusion'},v.document)&&v.document.bodies.some(b=>b.id===v.id)
    if(v.kind!=='mass'||!v.mass)return false
    const m=v.mass
    return finite(m.surfaceAreaMm2)&&m.surfaceAreaMm2>=0&&finite(m.signedVolumeMm3)&&vector(m.centroid)
      &&arrayOf(m.inertiaMm5,3,vector)&&arrayOf(m.conservativeBounds,2,vector)
      &&finite(m.areaErrorEstimateMm2)&&m.areaErrorEstimateMm2>=0&&finite(m.volumeErrorEstimateMm3)&&m.volumeErrorEstimateMm3>=0
      &&Number.isSafeInteger(m.evaluations)&&m.evaluations>=0&&m.status==='converged_estimate'&&m.solidGeometryStatus==='not_certified'
  }
  if(job.kind==='boolean') {
    const v=value as SolidBooleanResult
    return mainSolidResult({kind:'extrusion'},v.document)&&typeof v.exact==='boolean'&&finite(v.toleranceMm)&&v.toleranceMm>=0
      &&(v.resultId===null||typeof v.resultId==='string'&&v.document.bodies.some(b=>b.id===v.resultId))
  }
  if(job.kind==='curveMatch'||job.kind==='surfaceMatch'||job.kind==='seamPrepare') {
    const v=value as MainSolidResults['curveMatch'|'surfaceMatch'|'seamPrepare'],r=v.report
    if(!mainSolidResult({kind:'extrusion'},v.document)||!r||typeof r.accepted!=='boolean'||typeof r.reason!=='string'||r.accepted!==(r.reason==='accepted'))return false
    const nonnegative=(n:unknown)=>finite(n)&&n>=0
    if(job.kind==='curveMatch'){
      const c=(v as MainSolidResults['curveMatch']).report
      return c.method==='outward-endpoint-handle-wedge'&&nonnegative(c.positionErrorUpper)&&nonnegative(c.maxAngleDegrees)
        &&(c.sineAngleUpper===null||nonnegative(c.sineAngleUpper))&&(c.angleDegreesUpper===null||nonnegative(c.angleDegreesUpper))
        &&typeof c.regularityCertified==='boolean'&&typeof c.orientationCertified==='boolean'
        &&(!c.accepted||c.regularityCertified&&c.orientationCertified&&c.angleDegreesUpper!==null&&c.angleDegreesUpper<=c.maxAngleDegrees)
    }
    if(job.kind==='surfaceMatch'){
      const c=(v as MainSolidResults['surfaceMatch']).report,b=c.errorBounds
      return [1,2].includes(c.continuityOrder)&&finite(c.normalScale)&&c.normalScale>0&&nonnegative(c.errorUpper)&&nonnegative(c.maxError)
        &&c.method==='homogeneous-normalized-boundary-jets'&&c.seamBasis==='exact-affine-knot-correspondence'&&!!b&&b.wholeSeam===true&&b.normalizedParameters===true
        &&nonnegative(b.positionUpper)&&nonnegative(b.firstDerivativeUpper)&&(b.secondDerivativeUpper===null||nonnegative(b.secondDerivativeUpper))&&(b.mixedDerivativeUpper===null||nonnegative(b.mixedDerivativeUpper))
        &&(!c.accepted||c.regularityCertified&&c.tangentialSmoothnessCertified&&c.errorUpper<=c.maxError)
    }
    const c=(v as MainSolidResults['seamPrepare']).report,b=(v as MainSolidResults['seamPrepare']).basis
    return nonnegative(c.budget)&&!!b&&Number.isInteger(b.degree)&&b.degree>=1&&Number.isInteger(b.controlCount)&&b.controlCount>b.degree
      &&b.normalizedSeam===true&&typeof b.editedReversed==='boolean'&&arrayOf(b.referenceDomain,2,finite)&&arrayOf(b.editedDomain,2,finite)
      &&(!c.accepted||c.wholeSurface===true&&nonnegative(c.referenceErrorUpper)&&nonnegative(c.editedErrorUpper)&&c.referenceErrorUpper!<=c.budget&&c.editedErrorUpper!<=c.budget)
  }
  if(job.kind==='curveOffset') {
    const nonnegative=(n:unknown):n is number=>finite(n)&&n>=0
    const v=value as MainSolidResults['curveOffset'],r=v.report
    return mainSolidResult({kind:'extrusion'},v.document)&&!!r&&r.accepted===true
      &&(r.method==='outward-rational-jets-chord-bound/1'?r.wholeCurve===true:r.method==='outward-source-offset-bevel-wire/1'&&r.wholeCurve===false&&r.wholeWire===true&&r.regionTrimmed===false)&&typeof r.closed==='boolean'
      &&r.regionTopologyCertified===false&&r.offsetRegularityCertified===false
      &&nonnegative(r.errorUpperMm)&&finite(r.toleranceMm)&&r.toleranceMm>0&&r.errorUpperMm<=r.toleranceMm
      &&Array.isArray(r.cells)&&r.cells.length<=65536&&r.cells.every(c=>!!c&&arrayOf(c.domain,2,finite)
        &&c.domain[0]<c.domain[1]&&nonnegative(c.errorUpperMm)&&c.errorUpperMm<=r.errorUpperMm)
      &&(r.wholeWire!==true||r.cells.length>0&&r.cells.every((c,i)=>{
        const role=c.source
        return c.domain[0]===i&&c.domain[1]===i+1&&!!role&&(role.kind==='source-offset'
          ?arrayOf(role.domain,2,finite)&&role.domain[0]<role.domain[1]
          :role.kind==='bevel'&&finite(role.sourceKnot))
      }))
      &&(r.cells.length?validCurveOffsetDiagnostics(r.chainDiagnostics,r.cells.length):r.chainDiagnostics===null)
  }
  if(job.kind==='surfaceBuild') {
    const v=value as MainSolidResults['surfaceBuild'],r=v.report
    return typeof v.error==='string'&&(v.document===null?v.error.length>0:v.error===''&&mainSolidResult({kind:'extrusion'},v.document))
      &&(r===null||!!r&&typeof r.accepted==='boolean'&&finite(r.sampledControlDeviation)&&r.sampledControlDeviation>=0&&finite(r.budget)&&r.budget>=0
        &&Number.isInteger(r.stations)&&r.stations>0&&Number.isInteger(r.sections)&&r.sections>0&&r.continuousBound===false&&r.method==='double-reflection-fourfold-section-refinement'
        &&r.accepted===(r.sampledControlDeviation<=r.budget)&&(!v.document||r.accepted))
  }
  if(job.kind==='nurbsRefit') {
    const v=value as MainSolidResults['nurbsRefit'],c=v.certificate
    return mainSolidResult({kind:'extrusion'},v.document)&&!!c&&/^nurbs-foundation\/[1-5]$/.test(c.version)
      &&typeof c.accepted==='boolean'&&typeof c.rolledBack==='boolean'&&!(c.accepted&&c.rolledBack)
      &&['geometryErrorUpper','hausdorffErrorUpper','dataSiteErrorUpper','budget'].every(key=>{const n=c[key as keyof typeof c];return n===undefined||finite(n)&&n>=0})
      &&!!c.evidence&&!!c.evidence.toleranceIdentity&&typeof c.evidence.toleranceIdentity.canonical==='string'
  }
  if(job.kind==='profilePrepare') {
    const v=value as MainSolidResults['profilePrepare'],r=v.report
    const point=(p:unknown)=>arrayOf(p,2,finite)
    return mainSolidResult({kind:'extrusion'},v.document)&&typeof v.id==='string'&&!!v.plane&&vector(v.plane.origin)&&vector(v.plane.u)&&vector(v.plane.v)
      &&!!r&&typeof r.accepted==='boolean'&&['accepted','endpoint-topology','disconnected','invalid-contour'].includes(r.reason)
      &&r.accepted===(r.reason==='accepted')&&Array.isArray(r.points)&&r.points.every(point)
      &&Array.isArray(r.defects)&&r.defects.every(d=>d&&Number.isInteger(d.chain)&&d.chain>=0&&['start','end'].includes(d.end)&&point(d.point)&&['gap','ambiguous'].includes(d.kind)&&Array.isArray(d.candidates)&&d.candidates.every(i=>Number.isInteger(i)&&i>=0))
      &&Array.isArray(r.connectors)&&r.connectors.every(c=>c&&point(c.a)&&point(c.b))
      &&(!r.segmentDefect||(['intersection','overlap','degenerate','unproven'].includes(r.segmentDefect.kind)&&Array.isArray(r.segmentDefect.segments)&&r.segmentDefect.segments.every(s=>Number.isInteger(s.index)&&s.index>=0&&point(s.a)&&point(s.b))))
  }
  if(job.kind==='meshContacts') {
    const v=value as MainSolidResults['meshContacts']
    if(v.scope!=='display-mesh-all-contacts'||v.relativeTolerance!==1e-9||typeof v.complete!=='boolean'
      ||v.maxWork!==job.maxWork||v.maxContacts!==job.maxContacts||!Number.isInteger(v.work)||v.work<0||v.work>job.maxWork
      ||(v.complete?v.stopReason!==null:!['work-limit','contact-limit'].includes(v.stopReason??''))
      ||!Array.isArray(v.contacts)||v.contacts.length>job.maxContacts||!Array.isArray(v.triangleIds)||!Array.isArray(v.lines))return false
    if(!v.contacts.every((c,i)=>!!c&&arrayOf(c.triangles,2,t=>Number.isInteger(t)&&Number(t)>=0&&Number(t)<job.triangles)
      &&c.triangles[0]<c.triangles[1]&&vector(c.point)&&finite(c.toleranceMm)&&c.toleranceMm>0
      &&Number.isInteger(c.sharedVertices)&&c.sharedVertices>=0&&c.sharedVertices<=3
      &&(i===0||v.contacts[i-1].triangles[0]<c.triangles[0]||v.contacts[i-1].triangles[0]===c.triangles[0]&&v.contacts[i-1].triangles[1]<c.triangles[1])))return false
    const ids=[...new Set(v.contacts.flatMap(c=>c.triangles))].sort((a,b)=>a-b)
    return JSON.stringify(v.contact)===JSON.stringify(v.contacts[0]??null)
      &&v.triangleIds.length===ids.length&&v.triangleIds.every((id,i)=>id===ids[i])
      &&arrayOf(v.lines,ids.length,line=>arrayOf(line,4,vector))
  }
  if (job.kind==='bondedSolid')return isBondedSolidResult(value,job.nodes,job.tets,job.bonds)
  if (job.kind==='latticeGraph') return isNominalLatticeGraph(value)
  if (job.kind==='structuralSections') {
    const v=value as StructuralSections
    const planeAxes={x:[1,2],y:[2,0],z:[0,1]}[job.axis]
    return v.modelKind==='finished-mesh-sections-v1'&&v.axis===job.axis
      &&arrayOf(v.planeAxes,2,n=>Number.isInteger(n)&&Number(n)>=0&&Number(n)<3)
      &&v.planeAxes.every((axis,i)=>axis===planeAxes[i])
      &&(v.selfIntersections==='not-checked'||v.selfIntersections==='checked-at-tolerance')
      &&!!v.sourceMesh&&isNumericSequence(v.sourceMesh.positions)&&v.sourceMesh.positions.length<=900000
      &&v.sourceMesh.positions.length%3===0&&numericSequenceOf(v.sourceMesh.positions,v.sourceMesh.positions.length,finite)
      &&isNumericSequence(v.sourceMesh.indices)&&v.sourceMesh.indices.length===v.triangleCount*3
      &&numericSequenceOf(v.sourceMesh.indices,v.sourceMesh.indices.length,n=>Number.isInteger(n)&&Number(n)>=0&&Number(n)<v.sourceMesh.positions.length/3)
      &&finite(v.volumeMm3)&&v.volumeMm3>0&&Number.isInteger(v.triangleCount)&&v.triangleCount>0&&v.triangleCount<=100000
      &&isBoundaryConnectivity(v.connectivity,v.triangleCount,v.sourceMesh.positions.length/3)
      &&isMaterialAudit(v)
      &&Array.isArray(v.sections)&&v.sections.length===job.stations.length&&v.sections.length>0&&v.sections.length<=64
      &&Array.from(v.sections).every((s,i)=>s&&s.positionMm===job.stations[i]&&finite(s.positionMm)&&typeof s.material==='boolean'&&Array.isArray(s.contours)
        &&(s.material?!!s.properties&&finite(s.properties.areaMm2)&&s.properties.areaMm2>0
          &&arrayOf(s.properties.centroidMm,2,finite)&&finite(s.properties.iuuMm4)&&s.properties.iuuMm4>0
          &&finite(s.properties.ivvMm4)&&s.properties.ivvMm4>0&&finite(s.properties.iuvMm4):s.properties===null))
  }
  if (job.kind==='truss') {
    const v=value as TrussResponse, {nodes,members}=job
    return nodes>0 && nodes<=125 && members>0 && members<=400
      && arrayOf(v.displacementsMm,nodes,vector)
      && arrayOf(v.reactionsN,nodes,vector)
      && arrayOf(v.axialForcesN,members,finite)
      && arrayOf(v.axialStressesMpa,members,finite)
      && finite(v.maxDeflectionMm) && v.maxDeflectionMm>=0
      && finite(v.maxRelativeResidual) && v.maxRelativeResidual>=0 && v.maxRelativeResidual<=1e-9
      && Number.isInteger(v.freeDofs) && v.freeDofs>=0 && v.freeDofs<=3*nodes
  }
  if (job.kind==='inspect') return Array.isArray(value) && value.every(v => v
    && typeof v.a==='string' && typeof v.b==='string' && finite(v.overlapMm3) && finite(v.gapMm)
    && (v.closestPoints===undefined || v.closestPoints===null || arrayOf(v.closestPoints,2,vector))
    && (v.displayMeshOnly===undefined || v.displayMeshOnly===true))
  const v=value as DirectDocument
  return v.version===1 && Array.isArray(v.sketches) && Array.isArray(v.bodies)
}

/** Validate bounded provenance before accepting a worker report. No mechanics in TS. */
export function isBoundaryConnectivity(value:unknown,triangles:number,vertices:number):value is BoundaryConnectivity {
  if(!value||typeof value!=='object')return false
  const v=value as BoundaryConnectivity
  if(v.modelKind!=='indexed-edge-boundary-components-v1'||(v.materialConnectivity!=='not-established'&&v.materialConnectivity!=='classified-at-tolerance')
    ||!Array.isArray(v.components)||!v.components.length||v.components.length>triangles
    ||!Array.isArray(v.sharedVertices)||v.sharedVertices.length>vertices)return false
  const seen=new Set<number>()
  for(let i=0;i<v.components.length;i++){
    const c=v.components[i]
    if(!c||c.id!==i||!Number.isFinite(c.signedVolumeMm3)||!Array.isArray(c.sourceTriangles)
      ||!c.sourceTriangles.length||c.sourceTriangles.length>triangles
      ||!Array.isArray(c.boundsMm)||c.boundsMm.length!==2
      ||!c.boundsMm.every(p=>Array.isArray(p)&&p.length===3&&p.every(Number.isFinite))
      ||c.boundsMm[0].some((x,k)=>x>c.boundsMm[1][k]))return false
    for(const t of c.sourceTriangles){
      if(!Number.isInteger(t)||t<0||t>=triangles||seen.has(t))return false
      seen.add(t)
    }
  }
  if(seen.size!==triangles)return false
  const shared=new Set<number>()
  for(const p of v.sharedVertices){
    if(!p||!Number.isInteger(p.sourceVertex)||p.sourceVertex<0||p.sourceVertex>=vertices||shared.has(p.sourceVertex)
      ||!Array.isArray(p.components)||p.components.length<2||p.components.length>v.components.length
      ||!p.components.every((c,i)=>Number.isInteger(c)&&c>=0&&c<v.components.length&&(i===0||c>p.components[i-1])))return false
    shared.add(p.sourceVertex)
  }
  return true
}

function isMaterialAudit(v:StructuralSections):boolean {
  const a=v.materialAudit
  if(!a||typeof a!=='object')return false
  if(a.status==='unresolved')return v.selfIntersections==='not-checked'
    &&v.connectivity.materialConnectivity==='not-established'
    &&typeof a.code==='string'&&a.code.length>0&&a.code.length<=128
    &&typeof a.message==='string'&&a.message.length>0&&a.message.length<=4096
  if(a.status!=='classified'||v.selfIntersections!=='checked-at-tolerance'
    ||v.connectivity.materialConnectivity!=='classified-at-tolerance'||v.connectivity.sharedVertices.length!==0
    ||!finite(a.toleranceMm)||a.toleranceMm<=0||!Number.isInteger(a.materialRegions)||a.materialRegions<=0
    ||!Array.isArray(a.shells)||a.shells.length!==v.connectivity.components.length||a.shells.length>128)return false
  if(!a.shells.every((s,i)=>s&&s.id===i&&s.firstTriangle===v.connectivity.components[i].sourceTriangles[0]
    &&Number.isInteger(s.depth)&&s.depth>=0&&s.depth<a.shells.length
    &&s.kind===(s.depth%2===0?'material':'cavity')
    &&(s.parent===null?s.depth===0:Number.isInteger(s.parent)&&s.parent>=0&&s.parent<a.shells.length&&s.parent!==i)))return false
  return a.shells.every(s=>s.parent===null||a.shells[s.parent].depth+1===s.depth)
    &&a.materialRegions===a.shells.filter(s=>s.kind==='material').length
}
