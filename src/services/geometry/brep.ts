import type {DirectSketch} from '../directModeling'
import {callGeometryRust} from './kernel'
import type {NurbsCurve} from '../nurbsCurve'
import type {NurbsSurface} from '../nurbsSurface'
import type {PolygonMesh,PolygonBuild} from './polygon'
import type {RustChangeSet,TopoId} from '../../core/topologyLineage'
export interface BrepModel<C,S,P> {
 vertices:{point:[number,number,number]}[]
 edges:{vertices:[number,number];curve:C;degenerate?:boolean}[]
 loops:{coedges:{edge:number;reversed:boolean;pcurve:P}[]}[]
 faces:{surface:S;outer:number;holes:number[]}[]
 shells:{faces:{face:number;reversed:boolean}[];closed:boolean}[]
 bodies:{outerShell:number;innerShells:number[]}[]
 toleranceMm:number
 topologyIds?:BrepTopologyIds
}
export interface BrepTopologyIds {
 vertices:TopoId[]
 edges:TopoId[]
 loops:TopoId[]
 faces:TopoId[]
 shells:TopoId[]
 bodies:TopoId[]
 lineage?:BrepTopologyLineageRecord[]
 changeSet?:RustChangeSet
}
export interface BrepTopologyLineageRecord {
 operation:'persist'|'split'|'merge'
 entityKind:'vertex'|'edge'|'face'|'loop'|'shell'|'body'
 parents:TopoId[]
 children:TopoId[]
}
export type NurbsBrep=BrepModel<NurbsCurve,NurbsSurface,NurbsCurve>
export type PolygonBrep=BrepModel<null,{mesh:PolygonMesh;sourceFaceId:number},null>
export interface BrepReport {topologyValid:boolean;solidGeometryStatus:'not_certified'}
export interface BrepMesh extends PolygonBuild {faceIds:number[];topologyFaceIds?:string[]}
export const createBrepBox=(min:number[],max:number[]):NurbsBrep=>callGeometryRust('brep_nurbs_box',{min,max})
/** Exact rational B-reps; mesh detail does not change their authored geometry. */
export const revolveBrepProfile=(profile:[number,number][],angleDegrees=360):NurbsBrep=>callGeometryRust('brep_nurbs_revolve',{profile,angleDegrees})
export const createBrepCylinder=(radius:number,height:number):NurbsBrep=>callGeometryRust('brep_nurbs_cylinder',{radius,height})
export const createBrepFrustum=(bottomRadius:number,topRadius:number,height:number):NurbsBrep=>callGeometryRust('brep_nurbs_frustum',{bottomRadius,topRadius,height})
export const createBrepSphere=(radius:number):NurbsBrep=>callGeometryRust('brep_nurbs_sphere',{radius})
export const createBrepTorus=(majorRadius:number,minorRadius:number):NurbsBrep=>callGeometryRust('brep_nurbs_torus',{majorRadius,minorRadius})
export interface BrepMassProperties {
 surfaceAreaMm2:number
 signedVolumeMm3:number
 centroid:[number,number,number]
 inertiaMm5:[[number,number,number],[number,number,number],[number,number,number]]
 conservativeBounds:[[number,number,number],[number,number,number]]
 areaErrorEstimateMm2:number
 volumeErrorEstimateMm3:number
 evaluations:number
 status:'converged_estimate'
 solidGeometryStatus:'not_certified'
}
export interface CertifiedInterval {lower:number;upper:number}
export interface CertifiedBrepMassProperties {
 capability:'certified-mass-properties/1'
 status:'certified_enclosure'
 surfaceAreaMm2:CertifiedInterval
 volumeMm3:CertifiedInterval
 centroid:[CertifiedInterval,CertifiedInterval,CertifiedInterval]
 inertiaMm5:[[CertifiedInterval,CertifiedInterval,CertifiedInterval],[CertifiedInterval,CertifiedInterval,CertifiedInterval],[CertifiedInterval,CertifiedInterval,CertifiedInterval]]
 context:{version:number;canonical:string}
 evidenceClaimCount:number
 audit:{ok:boolean;bodyCount:number;shellCount:number;selfIntersectionPairsChecked:number}
 changeSet:RustChangeSet
 namingComplete:true
}
/** Integrates authored NURBS and trim curves; independent of display tessellation. */
export const pushNurbsBrepFace=(model:NurbsBrep,face:number,distance:number):NurbsBrep=>callGeometryRust('brep_nurbs_push_face',{model,face,distance})
export const shellNurbsBrep=(model:NurbsBrep,openings:number[],thickness:number):NurbsBrep=>callGeometryRust('brep_nurbs_shell',{model,openings,thickness})
export const splitNurbsBrep=(model:NurbsBrep,normal:[number,number,number],offset:number):[NurbsBrep,NurbsBrep]=>callGeometryRust('brep_nurbs_split',{model,normal,offset})
export const analyzeNurbsBrep=(model:NurbsBrep,relativeTolerance=1e-7,maxEvaluations=300000):BrepMassProperties=>callGeometryRust('brep_nurbs_mass_properties',{model,relativeTolerance,maxEvaluations})
export const analyzeCertifiedNurbsBrep=(model:NurbsBrep):CertifiedBrepMassProperties=>callGeometryRust('brep_nurbs_certified_mass_properties',{model})
export const createBrepTube=(outerRadius:number,innerRadius:number,height:number):NurbsBrep=>callGeometryRust('brep_nurbs_tube',{outerRadius,innerRadius,height})
/** Exact extrusion of oriented 2D NURBS line/circular-arc loops; outer CCW, holes CW. */
export const extrudeBrepCurves=(loops:NurbsCurve[][],zMin:number,zMax:number):NurbsBrep=>callGeometryRust('brep_nurbs_extrude_curves',{loops,zMin,zMax})
export const extrudeBrepPolygon=(profile:[number,number][],zMin:number,zMax:number,holes:[number,number][][]=[]):NurbsBrep=>callGeometryRust('brep_nurbs_extrude_polygon',{profile,holes,zMin,zMax})
/** Planar-triangulated construction, not a smooth NURBS loft. */
/** Native bilinear side patches between admitted parallel convex sections. */
export const createRuledSketchLoft=(sketches:DirectSketch[],ids:string[]):{brep:NurbsBrep;mesh:PolygonMesh}=>callGeometryRust('cad_ruled_sketch_loft',{sketches,ids})
export const createRuledBrepLoft=(sections:[number,number,number][][]):NurbsBrep=>callGeometryRust('brep_nurbs_ruled_loft',{sections})
export const createFacetedBrepLoft=(sections:[number,number,number][][]):NurbsBrep=>callGeometryRust('brep_nurbs_faceted_loft',{sections})
/** Planar-triangulated polyline sweep, not an analytic pipe. */
export const createFacetedBrepSweep=(profile:[number,number][],path:[number,number,number][],up:[number,number,number]):NurbsBrep=>callGeometryRust('brep_nurbs_faceted_sweep',{profile,path,up})
/** Full planar-faceted revolve around local Z, not an analytic surface of revolution. */
export const createFacetedBrepRevolve=(profile:[number,number][],segments=48):NurbsBrep=>callGeometryRust('brep_nurbs_faceted_revolve',{profile,segments})
/** Planar-faced B-rep approximation, not an analytic cylinder. */
export const createFacetedBrepCylinder=(radius:number,height:number,segments=48):NurbsBrep=>callGeometryRust('brep_nurbs_faceted_cylinder',{radius,height,segments})
/** Planar-faced B-rep approximation, not an analytic sphere. */
export const createFacetedBrepSphere=(radius:number,radialSegments=16,latitudeSegments=8):NurbsBrep=>callGeometryRust('brep_nurbs_faceted_sphere',{radius,radialSegments,latitudeSegments})
export type BrepBooleanOperation='union'|'difference'|'intersection'|'xor'
export const booleanNurbsBrep=(a:NurbsBrep,b:NurbsBrep,operation:BrepBooleanOperation):NurbsBrep=>callGeometryRust('brep_nurbs_boolean',{a,b,operation})
export const chamferNurbsBrep=(model:NurbsBrep,edge:number,size:number):NurbsBrep=>callGeometryRust('brep_nurbs_chamfer',{model,edge,size})
export const chamferNurbsBrepEdges=(model:NurbsBrep,edges:number[],size:number):NurbsBrep=>callGeometryRust('brep_nurbs_chamfer_edges',{model,edges,size})
export const filletNurbsBrep=(model:NurbsBrep,edge:number,radius:number,segments=12):NurbsBrep=>callGeometryRust('brep_nurbs_fillet',{model,edge,radius,segments})
export const filletNurbsBrepEdges=(model:NurbsBrep,edges:number[],radius:number,segments=12):NurbsBrep=>callGeometryRust('brep_nurbs_fillet_edges',{model,edges,radius,segments})
export interface AuditedBrepFeature {
 model:NurbsBrep
 certificate:{capability:'analytic-multi-edge-fillet/1'|'exact-parallel-frame-sweep/1';complete:true;notes:string[]}
 context:{version:number;canonical:string}
 evidenceClaimCount:number
 audit:{ok:true;bodyCount:number;shellCount:number}
 changeSet:RustChangeSet
 namingComplete:true
}
export const auditedMultiEdgeFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_multi_edge_fillet',{model,edges,radius})
export const auditedParallelFrameSweep=(profile:[number,number][],path:[number,number,number][],frameLaw:'fixed'|'rotation-minimizing'|'rmf'='rmf'):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_parallel_frame_sweep',{profile,path,frameLaw})
export const inspectNurbsBrep=(model:NurbsBrep):BrepReport=>callGeometryRust('brep_nurbs_inspect',{model})
export const tessellateNurbsBrep=(model:NurbsBrep,segments=4):BrepMesh=>callGeometryRust('brep_nurbs_tessellate',{model,segments})
export interface CertifiedBrepTessellation {
 capability:'certified-brep-tessellation/1'
 tessellation:BrepMesh
 context:{version:number;canonical:string}
 surfaceToMeshDeviationMm:number
 meshToSurfaceDeviationMm:number
 coverage:{sharedEdgeIdentity:true;orientation:true;noTJunctions:true}
 audit:{ok:boolean;bodyCount:number;shellCount:number}
 evidenceClaimCount:number
 changeSet:RustChangeSet
 namingComplete:true
}
export const tessellateCertifiedNurbsBrep=(model:NurbsBrep,chordToleranceMm:number,maxTriangles=20000):CertifiedBrepTessellation=>callGeometryRust('brep_nurbs_certified_tessellate',{model,chordToleranceMm,maxTriangles})
export const prepareBrepDisplay=(model:NurbsBrep,segments=4):Pick<BrepMesh,'report'|'faceIds'|'topologyFaceIds'>&{displayVertices:number[];displayIndices:number[];surfaceArea:number}=>callGeometryRust('brep_nurbs_display',{model,segments})
export const nurbsBrepToPolygon=(model:NurbsBrep,segments=4):PolygonBrep=>callGeometryRust('brep_nurbs_to_polygon',{model,segments})
export const polygonBrepFromMesh=(mesh:PolygonMesh,faceIds?:number[]):PolygonBrep=>callGeometryRust('brep_polygon_from_mesh',{mesh,...(faceIds?{faceIds}:{})})
export const inspectPolygonBrep=(model:PolygonBrep):BrepReport=>callGeometryRust('brep_polygon_inspect',{model})
export const tessellatePolygonBrep=(model:PolygonBrep):BrepMesh=>callGeometryRust('brep_polygon_tessellate',{model})
/** Affine edit of authored carriers. Reflections also reverse shell face uses. */
export const transformNurbsBrep=(model:NurbsBrep,matrix:number[][]):NurbsBrep=>callGeometryRust('brep_nurbs_transform',{model,matrix})
export const placeNurbsBrep=(model:NurbsBrep,origin:number[],u:number[],v:number[],offset:number[]):NurbsBrep=>callGeometryRust('brep_nurbs_workplane',{model,origin,u,v,offset})
