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
 operation:'persist'|'split'|'merge'|'share'|'decompose'|'recompose'
 entityKind:'vertex'|'edge'|'face'|'loop'|'shell'|'body'
 parents:TopoId[]
 children:TopoId[]
}
export type BrepBodyRole='wire'|'face'|'sheet-shell'|'open-shell'|'solid'|'compound'
export interface MixedBrepPart {role:BrepBodyRole;model:NurbsBrep}
export interface MixedBrepFaceUse {part:number;face:number;reversed:boolean}
export interface MixedBrepEdgeUse extends MixedBrepFaceUse {edge:number}
export interface MixedBrepVertexUse {part:number;face:number;vertex:number}
export interface MixedBrepVertexFan {
 part:number
 vertex:number
 closed:boolean
 uses:MixedBrepVertexUse[]
}
export interface AuditedMixedBrep {
 certificate:{
  capability:'close-topology/1'
  complete:true
  partCount:number
  solidCellCount:number
  sheetCount:number
  openShellCount:number
  sharedFaceCount:number
  nonManifoldEdgeCount:number
  vertexFanCount:number
  boundaryFaceCount:number
  maxRadialValence:number
  namingComplete:true
  notes:string[]
 }
 boundaryFaces:MixedBrepFaceUse[]
 decomposition:MixedBrepPart[]
}
/** Bounded explicit non-manifold/mixed-dimensional audit; solid audit stays manifold-only. */
export const auditMixedDimensionalBrep=(
 parts:MixedBrepPart[],
 sharedFaces:MixedBrepFaceUse[][]=[],
 radialRings:MixedBrepEdgeUse[][]=[],
 vertexFans:MixedBrepVertexFan[]=[],
):AuditedMixedBrep=>callGeometryRust('brep_nurbs_close_topology_v1',{parts,sharedFaces,radialRings,vertexFans})
export type NurbsBrep=BrepModel<NurbsCurve,NurbsSurface,NurbsCurve>
export type PolygonBrep=BrepModel<null,{mesh:PolygonMesh;sourceFaceId:number},null>
export interface BrepReport {topologyValid:boolean;solidGeometryStatus:'not_certified'}
export interface BrepMesh extends PolygonBuild {faceIds:number[];topologyFaceIds?:string[]}
export const createBrepBox=(min:number[],max:number[]):NurbsBrep=>callGeometryRust('brep_nurbs_box',{min,max})
export const createFreeformBrepCuboid=(min:number[],max:number[]):NurbsBrep=>callGeometryRust('brep_nurbs_freeform_cuboid',{min,max})
export const createFreeformBrepCuboidBump=(min:number[],max:number[]):NurbsBrep=>callGeometryRust('brep_nurbs_freeform_cuboid_bump',{min,max})
/** Exact rational B-reps; mesh detail does not change their authored geometry. */
export const revolveBrepProfile=(profile:[number,number][],angleDegrees=360):NurbsBrep=>callGeometryRust('brep_nurbs_revolve',{profile,angleDegrees})
export const createBrepCylinder=(radius:number,height:number):NurbsBrep=>callGeometryRust('brep_nurbs_cylinder',{radius,height})
export const createBrepFrustum=(bottomRadius:number,topRadius:number,height:number):NurbsBrep=>callGeometryRust('brep_nurbs_frustum',{bottomRadius,topRadius,height})
export const createBrepSphere=(radius:number):NurbsBrep=>callGeometryRust('brep_nurbs_sphere',{radius})
export const createBrepTorus=(majorRadius:number,minorRadius:number):NurbsBrep=>callGeometryRust('brep_nurbs_torus',{majorRadius,minorRadius})
/** Involute gear as a trimmed NURBS solid: spur, helical or herringbone, external with a bore or internal with a rim. */
export interface BrepGearSpec{module:number;teeth:number;height:number;pressureAngle?:number;helixAngle?:number;herringbone?:boolean;bore?:number;internal?:boolean;rimWidth?:number;clearance?:number;backlash?:number}
/**
 * Faces of a gear body before it is built: six outline curves per tooth, the
 * bore or the rim circle as one arc per two teeth (4..64), twice for a
 * herringbone, plus two caps. Mirrors `circle_arcs` in gear.rs, so display
 * detail can be chosen against the triangle budget without building first.
 */
export function brepGearFaceCount(spec: Pick<BrepGearSpec, 'teeth' | 'herringbone' | 'bore' | 'internal'>): number {
  const arcs = Math.min(64, Math.max(4, Math.floor(spec.teeth / 2)))
  const circle = spec.internal || (spec.bore ?? 0) > 0 ? arcs : 0
  return (spec.teeth * 6 + circle) * (spec.herringbone ? 2 : 1) + 2
}
/**
 * Gears are built once per specification and then copied: the eighteen planets of
 * a spinner share one involute fit and one helical sweep, and only their placement
 * differs. The key normalises defaults so equal gears written differently still hit.
 */
const GEAR_CACHE_LIMIT = 16
const gearBodies = new Map<string, NurbsBrep>()
const gearMeshes = new Map<string, BrepMesh>()
function gearKey(spec: BrepGearSpec): string {
  return JSON.stringify([spec.module, spec.teeth, spec.height, spec.pressureAngle ?? 20, spec.helixAngle ?? 0, spec.herringbone === true, spec.bore ?? 0, spec.internal === true, spec.rimWidth ?? 2, spec.clearance ?? 0.25, spec.backlash ?? 0])
}
function remember<T>(cache: Map<string, T>, key: string, build: () => T): T {
  const hit = cache.get(key)
  if (hit !== undefined) return hit
  const value = build()
  if (cache.size >= GEAR_CACHE_LIMIT) cache.delete(cache.keys().next().value!)
  cache.set(key, value)
  return value
}
const buildBrepGear=(spec:BrepGearSpec):NurbsBrep=>callGeometryRust('brep_nurbs_gear',{...spec})
/** A fresh copy of the gear body for this specification; the kernel builds it once. */
export const createBrepGear=(spec:BrepGearSpec):NurbsBrep=>structuredClone(remember(gearBodies,gearKey(spec),()=>buildBrepGear(spec)))
/** A fresh copy of the gear's display mesh at `segments`; tessellated once per (gear, detail). */
export const tessellateBrepGear=(spec:BrepGearSpec,segments:number):BrepMesh=>structuredClone(remember(gearMeshes,gearKey(spec)+':'+segments,()=>tessellateNurbsBrep(remember(gearBodies,gearKey(spec),()=>buildBrepGear(spec)),segments)))
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
 capability:'certified-mass-properties/2'
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
 composition:{componentCount:number;cavityCount:number;signedShellComposition:true}
 proof:'closed_form_analytic_with_outward_rounded_binary64_enclosures'
}
export interface CertifiedFreeformBrepMassProperties extends Omit<CertifiedBrepMassProperties,'capability'|'proof'> {
 capability:'certified-generic-rational-freeform-mass-quadrature/1'
 proof:'closed_form_planar_freeform_bezier_cuboid_enclosures'
}
export type AuthorizedHealOperation =
  | {kind:'endpointSnap';vertex:number;to:[number,number,number]}
  | {kind:'rationalRefit';edge:number;replacement:NurbsCurve}
export interface AuthorizedHealResult {
 model:NurbsBrep
 certificate:{
  capability:'authorized-heal-gap-le1/2'
  status:'Complete'
  context:{version:number;canonical:string}
  displacementLedger:{operation:number;actualMm:number;cumulativeMm:number}[]
  cumulativeDisplacementMm:number
  sew:{matched:number;complete:true;displacementBudgetOk:true}
  audit:{ok:true;bodyCount:number;shellCount:number;selfIntersectionPairsChecked:number}
  changeSet:RustChangeSet
  namingComplete:true
 }
}
/** Integrates authored NURBS and trim curves; independent of display tessellation. */
export const pushNurbsBrepFace=(model:NurbsBrep,face:number,distance:number):NurbsBrep=>callGeometryRust('brep_nurbs_push_face',{model,face,distance})
export const shellNurbsBrep=(model:NurbsBrep,openings:number[],thickness:number):NurbsBrep=>callGeometryRust('brep_nurbs_shell',{model,openings,thickness})
export interface ExactAnalyticShellResult {
 model:NurbsBrep
 certificate:{
  capability:'analytic-shell/2'
  complete:true
  notes:string[]
 }
 context:{version:number;canonical:string}
 evidenceClaimCount:number
 audit:{ok:true;bodyCount:number;shellCount:number;selfIntersectionPairsChecked:number;notes:string[]}
 changeSet:RustChangeSet
 namingComplete:true
}
/** Exact support-plane or rational radial/axial shell; unsupported transitions refuse atomically. */
export const exactAnalyticShell=(
 model:NurbsBrep,
 openings:number[],
 thickness:number,
 direction:'inward'|'outward',
):ExactAnalyticShellResult=>callGeometryRust('brep_nurbs_exact_analytic_shell',{model,openings,thickness,direction})
export const splitNurbsBrep=(model:NurbsBrep,normal:[number,number,number],offset:number):[NurbsBrep,NurbsBrep]=>callGeometryRust('brep_nurbs_split',{model,normal,offset})
export const analyzeNurbsBrep=(model:NurbsBrep,relativeTolerance=1e-7,maxEvaluations=300000):BrepMassProperties=>callGeometryRust('brep_nurbs_mass_properties',{model,relativeTolerance,maxEvaluations})
export const analyzeCertifiedNurbsBrep=(model:NurbsBrep):CertifiedBrepMassProperties=>callGeometryRust('brep_nurbs_certified_mass_properties',{model})
export const analyzeCertifiedFreeformNurbsBrep=(model:NurbsBrep):CertifiedFreeformBrepMassProperties=>callGeometryRust('brep_nurbs_certified_freeform_mass_properties',{model})
/** Strict native transaction: correspondence and topology evidence are derived in Rust. */
export const authorizedHealNurbsBrep=(model:NurbsBrep,operation:AuthorizedHealOperation):AuthorizedHealResult=>callGeometryRust('brep_nurbs_authorized_heal_v2',{model,operation})
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
export interface CertifiedCurvedGraphBoolean {
 model:NurbsBrep
 certificate:{
  capability:'nurbs-boolean-bezier-le3/3'
  status:'Complete'
  operation:'intersection'|'difference'
  axis:'U'|'V'
  fixedParameter:number
  lineage:{sourceFace:string;retainedFace:string;deletedRegion:string;generatedIntersectionEdge:string}
  tensorCells:number
  exactCorrespondence:true
  sew:{matched:number;complete:true;displacementBudgetOk:true}
  audit:{ok:true;bodyCount:1;shellCount:1;selfIntersectionPairsChecked:number;notes:string[]}
  changeSet:RustChangeSet
  namingComplete:true
  noFallback:true
  separationProof:true
 }
}
export const createCanonicalBezierGraphSolid=(degreeU:2|3,degreeV:2|3):NurbsBrep=>callGeometryRust('brep_nurbs_canonical_graph_solid_v3',{degreeU,degreeV})
export const certifiedCurvedGraphBoolean=(a:NurbsBrep,b:NurbsBrep,operation:'intersection'|'difference'):CertifiedCurvedGraphBoolean=>callGeometryRust('brep_nurbs_boolean_bezier_le3_v3',{a,b,operation})
export interface CertifiedSuccessorGraphBoolean extends Omit<CertifiedCurvedGraphBoolean,'certificate'>{
 certificate:Omit<CertifiedCurvedGraphBoolean['certificate'],'capability'> & {
  capability:'nurbs-boolean-bezier-le3/4'|'nurbs-boolean-bezier-le3/5'
  homogeneousRootProof:boolean
  denominatorLowerBound:number
  weightConditionNumber:number
  resourceBound:number
 }
}
export interface CertifiedContainedGraphBoolean {
 model:NurbsBrep
 certificate:{
  capability:'nurbs-boolean-bezier-le3/4'
  status:'Complete'
  operation:'union'|'intersection'|'difference'
  relation:'graph-strictly-contains-affine-cutter'
  strictUvMargin:number
  floorClearance:number
  roofClearance:number
  cavityProof:boolean
  separationProof:true
  audit:{ok:true;bodyCount:number;shellCount:number;notes:string[]}
  changeSet:RustChangeSet
  namingComplete:true
  noFallback:true
 }
}
export const createCanonicalRationalGraphSolid=(degreeU:2|3,degreeV:2|3):NurbsBrep=>callGeometryRust('brep_nurbs_canonical_rational_graph_solid_v5',{degreeU,degreeV})
export const certifiedUnequalSpanGraphBoolean=(a:NurbsBrep,b:NurbsBrep,operation:'intersection'|'difference'):CertifiedSuccessorGraphBoolean=>callGeometryRust('brep_nurbs_boolean_bezier_le3_v4_unequal',{a,b,operation})
export const certifiedContainedGraphBoolean=(graph:NurbsBrep,cutter:NurbsBrep,operation:'union'|'intersection'|'difference'):CertifiedContainedGraphBoolean=>callGeometryRust('brep_nurbs_boolean_bezier_le3_v4_containment',{graph,cutter,operation})
export const certifiedRationalGraphBoolean=(a:NurbsBrep,b:NurbsBrep,operation:'intersection'|'difference'):CertifiedSuccessorGraphBoolean=>callGeometryRust('brep_nurbs_boolean_bezier_le3_v5_rational',{a,b,operation})
export interface GeneralNurbsBooleanResult {
 model:NurbsBrep
 certificate:{
  capability:'nurbs-boolean/1'
  authority:'author-general-nurbs-boolean'
  status:'Complete'
  operation:'union'|'intersection'|'difference'
  operandOrder:'source-tool'|'tool-source'
  exactRegionMembership:true
  partitionCells:number
  cavityCount:number
  branchGraph:{components:number;fragments:number;candidateSpanPairs:number;sourceSpanCount:[number,number];denominatorLowerBound:number;complete:true}
  uv:{tensorCells:number;branches:number;materialCells:number;holeCells:number;complete:true}
  exactCurvePcurveCount:number
  ssReportsComplete:true
  ssFacePairs:number
  sew:{matched:number;complete:true;displacementBudgetOk:true}
  audit:{ok:true;bodyCount:number;shellCount:number;selfIntersectionPairsChecked:number;notes:string[]}
  changeSet:RustChangeSet
  naming:{split:number;retained:number;deleted:number;generated:number;operationStable:true}
  resultComponents:number
  resultFaces:number
  noFallback:true
 }
}
export const createCanonicalMultispanGraphSolid=(spansU:1|2,spansV:1|2):NurbsBrep=>callGeometryRust('brep_nurbs_canonical_multispan_graph_solid',{spansU,spansV})
/** Strict certificate-bearing product operation; never crosses to mesh/Manifold. */
export const generalNurbsBoolean=(a:NurbsBrep,b:NurbsBrep,operation:'union'|'intersection'|'difference'):GeneralNurbsBooleanResult=>callGeometryRust('brep_nurbs_boolean_general',{a,b,operation})
/** Model-only compatibility projection delegates to the strict product operation. */
export const generalNurbsBooleanModel=(a:NurbsBrep,b:NurbsBrep,operation:'union'|'intersection'|'difference'):NurbsBrep=>generalNurbsBoolean(a,b,operation).model
export const booleanNurbsBrep=(a:NurbsBrep,b:NurbsBrep,operation:BrepBooleanOperation):NurbsBrep=>callGeometryRust('brep_nurbs_boolean',{a,b,operation})
export const chamferNurbsBrep=(model:NurbsBrep,edge:number,size:number):NurbsBrep=>callGeometryRust('brep_nurbs_chamfer',{model,edge,size})
export const chamferNurbsBrepEdges=(model:NurbsBrep,edges:number[],size:number):NurbsBrep=>callGeometryRust('brep_nurbs_chamfer_edges',{model,edges,size})
export const filletNurbsBrep=(model:NurbsBrep,edge:number,radius:number,segments=12):NurbsBrep=>callGeometryRust('brep_nurbs_fillet',{model,edge,radius,segments})
export const filletNurbsBrepEdges=(model:NurbsBrep,edges:number[],radius:number,segments=12):NurbsBrep=>callGeometryRust('brep_nurbs_fillet_edges',{model,edges,radius,segments})
export interface AuditedBrepFeature {
 model:NurbsBrep
 certificate:{capability:'analytic-multi-edge-fillet/1'|'exact-parallel-frame-sweep/1'|'exact-convex-straight-edge-chamfer/1'|'exact-convex-prism-edge-fillet/1'|'analytic-solid-loft/2'|'exact-parallel-frame-sweep/2'|'exact-variable-radius-fillet/1'|'exact-valence3-corner-blend/1';complete:true;notes:string[]}
 context:{version:number;canonical:string}
 evidenceClaimCount:number
 audit:{ok:true;bodyCount:number;shellCount:number;selfIntersectionPairsChecked:number}
 changeSet:RustChangeSet
 namingComplete:true
}
export const auditedMultiEdgeFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_multi_edge_fillet',{model,edges,radius})
/** Exact equal-distance chamfer on a connected open/closed convex edge selection. */
export const exactConvexChamfer=(model:NurbsBrep,edges:number[],distance:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_convex_chamfer',{model,edges,distance})
/** Exact cylindrical rounds on selected longitudinal edges of a rigidly placed convex prism. */
export const exactConvexPrismFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_convex_prism_fillet',{model,edges,radius})
/** Exact linear radius law on one vertical cuboid edge; constant-radius pairs refuse. */
export const exactVariableRadiusFillet=(model:NurbsBrep,edges:number[],radii:[number,number][]):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_variable_radius_fillet',{model,edges,radii})
/** Exact equal-radius sphere+cylinder valence-3 blend at the AA cuboid max corner. */
export const exactValence3CornerBlend=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_valence3_corner_blend',{model,edges,radius})
export const auditedParallelFrameSweep=(profile:[number,number][],path:[number,number,number][],frameLaw:'fixed'|'rotation-minimizing'|'rmf'='rmf'):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_parallel_frame_sweep',{profile,path,frameLaw})
/** Exact indexed 3..16-section rational ruled/Bezier solid loft successor. */
export const auditedMultiSectionLoft=(sections:[number,number,number][][]):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_multi_section_loft_v2',{sections})
/** Exact open bent degree-1 Bezier path with bounded discrete RMF, twist, and positive scale laws. */
export const auditedBentRmfSweep=(profile:[number,number][],path:[number,number,number][],twistRadians:number[],scales:number[]):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_bent_rmf_sweep_v2',{profile,path,twistRadians,scales})
export const inspectNurbsBrep=(model:NurbsBrep):BrepReport=>callGeometryRust('brep_nurbs_inspect',{model})
export const tessellateNurbsBrep=(model:NurbsBrep,segments=4):BrepMesh=>callGeometryRust('brep_nurbs_tessellate',{model,segments})
export interface CertifiedBrepTessellation {
 capability:'certified-brep-tessellation/2'
 tessellation:BrepMesh
 context:{version:number;canonical:string}
 surfaceToMeshDeviationMm:number
 meshToSurfaceDeviationMm:number
 coverage:{sharedEdgeIdentity:true;orientation:true;normalConsistency:true;noTJunctions:true;noCracks:true;poleDegeneracyHandled:true;periodicSeamsHandled:true}
 audit:{ok:boolean;bodyCount:number;shellCount:number}
 evidenceClaimCount:number
 changeSet:RustChangeSet
 namingComplete:true
 resourceProof:{triangleBudget:number;subdivisionsPerPatch:number;adaptiveSelection:true}
}
export const tessellateCertifiedNurbsBrep=(model:NurbsBrep,chordToleranceMm:number,maxTriangles=20000):CertifiedBrepTessellation=>callGeometryRust('brep_nurbs_certified_tessellate',{model,chordToleranceMm,maxTriangles})
export interface CertifiedFreeformBrepTessellation extends Omit<CertifiedBrepTessellation,'capability'> {
 capability:'certified-generic-rational-freeform-tessellation/1'
}
export const tessellateCertifiedFreeformNurbsBrep=(model:NurbsBrep,chordToleranceMm:number,maxTriangles=20000):CertifiedFreeformBrepTessellation=>callGeometryRust('brep_nurbs_certified_freeform_tessellate',{model,chordToleranceMm,maxTriangles})
export const prepareBrepDisplay=(model:NurbsBrep,segments=4):Pick<BrepMesh,'report'|'faceIds'|'topologyFaceIds'>&{displayVertices:number[];displayIndices:number[];surfaceArea:number}=>callGeometryRust('brep_nurbs_display',{model,segments})
export const nurbsBrepToPolygon=(model:NurbsBrep,segments=4):PolygonBrep=>callGeometryRust('brep_nurbs_to_polygon',{model,segments})
export const polygonBrepFromMesh=(mesh:PolygonMesh,faceIds?:number[]):PolygonBrep=>callGeometryRust('brep_polygon_from_mesh',{mesh,...(faceIds?{faceIds}:{})})
export const inspectPolygonBrep=(model:PolygonBrep):BrepReport=>callGeometryRust('brep_polygon_inspect',{model})
export const tessellatePolygonBrep=(model:PolygonBrep):BrepMesh=>callGeometryRust('brep_polygon_tessellate',{model})
/** Affine edit of authored carriers. Reflections also reverse shell face uses. */
export const transformNurbsBrep=(model:NurbsBrep,matrix:number[][]):NurbsBrep=>callGeometryRust('brep_nurbs_transform',{model,matrix})
/** `/7` document composition assigns fresh occurrence-local TopoIds in Rust. */
export const composeStepV7Occurrences=(models:NurbsBrep[]):NurbsBrep=>
  callGeometryRust('brep_nurbs_compose_step_v7',{models})
/** Qualified `/8` composition retains whole-domain topology certification. */
export const composeStepV8Occurrences=(models:NurbsBrep[]):NurbsBrep=>
  callGeometryRust('brep_nurbs_compose_step_v8',{models})
/** `/9` composition assigns occurrence-local identities to solids and sheets. */
export const composeStepV9Occurrences=(models:NurbsBrep[]):NurbsBrep=>
  callGeometryRust('brep_nurbs_compose_step_v9',{models})
export const placeNurbsBrep=(model:NurbsBrep,origin:number[],u:number[],v:number[],offset:number[]):NurbsBrep=>callGeometryRust('brep_nurbs_workplane',{model,origin,u,v,offset})
