import {constructProgressiveMiterOwnedBoundary,progressiveMiterOwnedRequest,reconstructMiterOwnedBoundary,type MiterOwnedBoundaryReport} from '../nurbsMiterOwnedBoundary'
import {inspectMiterProfileSmoothness,type MiterProfileSmoothness} from '../miterProfileSmoothness'
import {type SweepBoundaryCertificate} from '../sweepBoundaryCertificate'
import {planMiterBody,ownedMiterSharpStations,previewMiterWalls} from '../miterBodyLayout'
import {correctMiterSweepSections,type SweepSectionCorrection} from '../nurbsSectionProjection'
import {inspectSweepRetainedCorrespondence,inspectSweepRetainedDecomposition,inspectSweepRetainedCaps,inspectSweepRetainedCapDecomposition,type SweepRetainedCaps,type SweepRetainedCorrespondence} from '../sweepRetainedCorrespondence'
import {inspectSweepEmbedding,DEFAULT_SWEEP_EMBEDDING_BUDGETS,type SweepEmbeddingAudit,inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS,type SweepVolumeAudit} from '../nurbsSweepEmbedding'
import type {DirectSketch} from '../directModeling'
import {callGeometryRust} from './kernel'
import type {NurbsCurve} from '../nurbsCurve'
import type {NurbsSurface} from '../nurbsSurface'
import {normalizePolygonMesh} from './polygon'
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
 audit:{ok:boolean;bodyCount:number;shellCount:number;selfIntersectionPairsCandidate:number;selfIntersectionComplete:boolean;selfIntersectionPairsChecked:number}
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
  audit:{ok:true;bodyCount:number;shellCount:number;selfIntersectionPairsCandidate:number;selfIntersectionComplete:boolean;selfIntersectionPairsChecked:number}
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
 audit:{ok:true;bodyCount:number;shellCount:number;selfIntersectionPairsCandidate:number;selfIntersectionComplete:boolean;selfIntersectionPairsChecked:number;notes:string[]}
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
export const createRuledSketchLoft=(sketches:DirectSketch[],ids:string[]):{brep:NurbsBrep;mesh:PolygonMesh}=>{const r=callGeometryRust<{brep:NurbsBrep;mesh:PolygonMesh}>('cad_ruled_sketch_loft',{sketches,ids});normalizePolygonMesh(r.mesh);return r}
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
  audit:{ok:true;bodyCount:1;shellCount:1;selfIntersectionPairsCandidate:number;selfIntersectionComplete:boolean;selfIntersectionPairsChecked:number;notes:string[]}
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
  audit:{ok:true;bodyCount:number;shellCount:number;selfIntersectionPairsCandidate:number;selfIntersectionComplete:boolean;selfIntersectionPairsChecked:number;notes:string[]}
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
 certificate:{capability:'analytic-multi-edge-fillet/1'|'exact-parallel-frame-sweep/1'|'exact-convex-straight-edge-chamfer/1'|'exact-convex-prism-edge-fillet/1'|'exact-simple-prism-convex-edge-fillet/1'|'exact-annular-circular-edge-fillet/1'|'exact-layered-prism-edge-fillet/1'|'analytic-solid-loft/2'|'exact-parallel-frame-sweep/2'|'exact-variable-radius-fillet/1'|'exact-valence3-corner-blend/1';complete:true;notes:string[]}
 context:{version:number;canonical:string}
 evidenceClaimCount:number
 audit:{ok:true;bodyCount:number;shellCount:number;selfIntersectionPairsCandidate:number;selfIntersectionComplete:boolean;selfIntersectionPairsChecked:number}
 changeSet:RustChangeSet
 namingComplete:true
}
/** Experimental geometry result; it must not enter the audited commit path. */
export interface PartialAnnularPreview {
 model:NurbsBrep
 changeSet:RustChangeSet
 qualification:{status:'preview-only';commitAllowed:false;boundaryIntersectionProof:'unqualified';transitionContinuityProof:'unqualified'}
}
export const partialAnnularPreview=(model:NurbsBrep,edge:number,radius:number):PartialAnnularPreview=>callGeometryRust('brep_nurbs_partial_annular_preview',{model,edge,radius})
export const auditedMultiEdgeFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_multi_edge_fillet',{model,edges,radius})
/** Exact equal-distance chamfer on a connected open/closed convex edge selection. */
export const exactConvexChamfer=(model:NurbsBrep,edges:number[],distance:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_convex_chamfer',{model,edges,distance})
/** Exact cylindrical rounds on selected longitudinal edges of a rigidly placed convex prism. */
export const exactConvexPrismFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_convex_prism_fillet',{model,edges,radius})
export const exactSimplePrismFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_simple_prism_fillet',{model,edges,radius})
/** Exact linear radius law on one vertical cuboid edge; constant-radius pairs refuse. */
export const exactVariableRadiusFillet=(model:NurbsBrep,edges:number[],radii:[number,number][]):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_variable_radius_fillet',{model,edges,radii})
/** Exact equal-radius sphere+cylinder valence-3 blend at any corner of a rigidly placed cuboid. */
export const exactValence3CornerBlend=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_valence3_corner_blend',{model,edges,radius})
export const auditedParallelFrameSweep=(profile:[number,number][],path:[number,number,number][],frameLaw:'fixed'|'rotation-minimizing'|'rmf'='rmf'):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_parallel_frame_sweep',{profile,path,frameLaw})
/** Exact indexed 3..16-section rational ruled/Bezier solid loft successor. */
export const auditedMultiSectionLoft=(sections:[number,number,number][][]):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_multi_section_loft_v2',{sections})
/** Exact open bent degree-1 Bezier path with bounded discrete RMF, twist, and positive scale laws. */
export const auditedBentRmfSweep=(profile:[number,number][],path:[number,number,number][],twistRadians:number[],scales:number[]):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_bent_rmf_sweep_v2',{profile,path,twistRadians,scales})
export const inspectNurbsBrep=(model:NurbsBrep):BrepReport=>callGeometryRust('brep_nurbs_inspect',{model})
export const tessellateNurbsBrep=(model:NurbsBrep,segments=4):BrepMesh=>normalizePolygonMesh(callGeometryRust('brep_nurbs_tessellate',{model,segments}))
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
export const tessellateCertifiedNurbsBrep=(model:NurbsBrep,chordToleranceMm:number,maxTriangles=20000):CertifiedBrepTessellation=>{const r=callGeometryRust<CertifiedBrepTessellation>('brep_nurbs_certified_tessellate',{model,chordToleranceMm,maxTriangles});normalizePolygonMesh(r.tessellation);return r}
export interface CertifiedFreeformBrepTessellation extends Omit<CertifiedBrepTessellation,'capability'> {
 capability:'certified-generic-rational-freeform-tessellation/1'
}
export const tessellateCertifiedFreeformNurbsBrep=(model:NurbsBrep,chordToleranceMm:number,maxTriangles=20000):CertifiedFreeformBrepTessellation=>{const r=callGeometryRust<CertifiedFreeformBrepTessellation>('brep_nurbs_certified_freeform_tessellate',{model,chordToleranceMm,maxTriangles});normalizePolygonMesh(r.tessellation);return r}
export const prepareBrepDisplay=(model:NurbsBrep,segments=4):Pick<BrepMesh,'report'|'faceIds'|'topologyFaceIds'>&{displayVertices:number[];displayIndices:number[];surfaceArea:number}=>callGeometryRust('brep_nurbs_display',{model,segments})
/** Polygon B-rep carriers embed one PolygonMesh per face; box the decoded plain
 * arrays into typed views once, here at the kernel boundary. */
const typedPolygonBrep=(brep:PolygonBrep):PolygonBrep=>{for(const face of brep.faces)normalizePolygonMesh(face.surface.mesh);return brep}
export const nurbsBrepToPolygon=(model:NurbsBrep,segments=4):PolygonBrep=>typedPolygonBrep(callGeometryRust('brep_nurbs_to_polygon',{model,segments}))
export const polygonBrepFromMesh=(mesh:PolygonMesh,faceIds?:number[]):PolygonBrep=>typedPolygonBrep(callGeometryRust('brep_polygon_from_mesh',{mesh,...(faceIds?{faceIds}:{})}))
export const inspectPolygonBrep=(model:PolygonBrep):BrepReport=>callGeometryRust('brep_polygon_inspect',{model})
export const tessellatePolygonBrep=(model:PolygonBrep):BrepMesh=>normalizePolygonMesh(callGeometryRust('brep_polygon_tessellate',{model}))
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

export const exactAnnularFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_annular_fillet',{model,edges,radius})

export const exactLayeredPrismFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_layered_prism_fillet',{model,edges,radius})

export const constantFilletFamily=(model:NurbsBrep,edges:number[]):'annular'|'layered'|'simple'=>callGeometryRust('brep_nurbs_constant_fillet_family',{model,edges})

/** Ordered rational loops per section, with audited planar cap trim regions.
 * Preserves manifold incidence; global embedding/self-intersections are unproven. */
export const createNaturalBrepSectionLoft=(sections:NurbsCurve[][][],parameters:number[]):NurbsBrep=>callGeometryRust('brep_nurbs_natural_section_loft',{sections,parameters})
export const createCappedBrepLoftSurfaces=(start:NurbsCurve[][],end:NurbsCurve[][],sides:NurbsSurface[][]):NurbsBrep=>callGeometryRust('brep_nurbs_capped_loft_surfaces',{start,end,sides})
/** Retains authored nonlinear walls on every span; Solid requires a separate audit. */
export const createBrepSectionLoftSurfaces=(sections:NurbsCurve[][][],sides:NurbsSurface[][],closed=false):NurbsBrep=>callGeometryRust('brep_nurbs_section_loft_surfaces',{sections,sides,closed})
export interface SmoothStationWallCandidate {
 sides:NurbsSurface[][]|null
 wallDisplacementUpper:number|null
 work:number
 reason:string
}
/** Bounded candidate only: no regularity, embedding or Solid certificate. */
export const proposeSmoothStationWalls=(sections:NurbsCurve[][][],sharp:number[],closed:boolean,quantum:number,tolerance:number,maxWork:number):SmoothStationWallCandidate=>callGeometryRust('brep_nurbs_smooth_station_walls',{sections,sharp,closed,quantum,tolerance,maxWork})
import {miterNurbsProfileSections} from '../nurbsConstructors'
import {inspectProgressiveMiterIdealCapDomains,inspectProgressiveMiterCapProjection,inspectProgressiveMiterCapParallelism,inspectProgressiveMiterWalls,streamProgressiveMiterNurbsProfiles,type ProgressiveMiterOptions,type ProgressiveMiterResult} from '../nurbsConstructors'
import {inspectSweepContours,inspectSweepProfileRegularity,type SweepProfileRegularityAudit,type SweepContourAudit} from '../nurbsSweepAudit'
import {inspectSweepRetainedWallCharts,type SweepRetainedChartEvidence} from '../nurbsSweepRetainedCharts'
import {inspectSweepCapContacts,inspectSweepCapPairs,type SweepCapPairEvidence,type SweepCapContactEvidence} from '../nurbsSweepCapContacts'
const correctMiterCaps=(sections:NurbsCurve[][],points:[number,number,number][],closed:boolean,capCorrection:{quantum:number;tolerance:number;maxWork:number;authoredFrame?:boolean},frameAxis?:import('../nurbsConstructors').NurbsVectorLaw):SweepSectionCorrection & {sections:NurbsCurve[][]}=>{
 const result=correctMiterSweepSections(sections,points,{closed,capCorrection,frameAxis})
 if(!result)throw new Error('Miter cap correction result missing')
 return result
}
export interface ProgressiveMiterBrepBody {profileSmoothness:MiterProfileSmoothness;boundaryCertificate:SweepBoundaryCertificate;retainedCapDecomposition:import('../sweepRetainedCorrespondence').SweepRetainedCapDecomposition|null;retainedDecomposition:import('../sweepRetainedCorrespondence').SweepRetainedDecomposition|null;capParallelism:import('../nurbsConstructors').ProgressiveMiterCapParallelism|null;boundaryErrorWithinBudget:boolean|null;boundaryErrorUpper:number|null;filledCapErrorUpper:[number,number]|null;idealCapDomains:import('../nurbsConstructors').ProgressiveMiterIdealCapDomains|null;capProjection:import('../nurbsConstructors').ProgressiveMiterCapProjection|null;retainedWallErrorUpper:number|null;sectionCorrection?:Omit<SweepSectionCorrection,'sections'>;model:NurbsBrep;approximation:ProgressiveMiterResult;wallAudit:import('../nurbsSweepAudit').SweepWallAudit;retainedCorrespondence:SweepRetainedCorrespondence;retainedCaps:SweepRetainedCaps|null;retainedWallCharts:SweepRetainedChartEvidence;capDomains:[SweepContourAudit,SweepContourAudit]|null;capContacts:SweepCapContactEvidence[]|null;capPairs:SweepCapPairEvidence|null;embedding:SweepEmbeddingAudit|null;volume:SweepVolumeAudit;globalEmbeddingCertified:false}
// Route original requests and station metadata by object identity. Rust replays
// and compares actual geometry/certificates before deriving modifier bounds.
export interface CertifiedMiterBoundaryOwner {model:NurbsBrep;boundaryCertificate:SweepBoundaryCertificate}
const miterOriginalRequests=new WeakMap<CertifiedMiterBoundaryOwner,string>()
const miterAffineHistory=new WeakMap<CertifiedMiterBoundaryOwner,string>()
const miterReconstructionReplay=new WeakMap<CertifiedMiterBoundaryOwner,string>()
const miterProofOwners=new WeakMap<CertifiedMiterBoundaryOwner,{sections?:string;sharp?:number[];authoring?:{authoredFramesApplied:boolean;orientationGuideApplied:boolean;affineLawsApplied:boolean}}>()
/** Replayable inputs only; no positive geometry certificate is transported. */
export function readMiterAffineReplayRouting(owner:CertifiedMiterBoundaryOwner) {
 const original=miterOriginalRequests.get(owner),encoded=miterAffineHistory.get(owner)
 if(!original)return null
 const reconstruction=miterReconstructionReplay.get(owner)
 if(!encoded&&!reconstruction)return null
 // Unplaced reconstructions still need original-request replay in a new
 // worker. Identity placement uses the same native final-model binding gate.
 const steps=encoded?JSON.parse(encoded):[{
  matrix:[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]],
  quantum:JSON.parse(reconstruction!).quantum,maxWork:JSON.parse(reconstruction!).maxWork,budget:owner.boundaryCertificate.budget,
 }]
 const [first,...following]=steps as {matrix:number[][];quantum:number;maxWork:number;budget:number|null}[]
 if(!first)return null
 return {source:JSON.parse(original),...first,followingPlacements:following,
  ...(miterReconstructionReplay.has(owner)?{beforeReconstruction:JSON.parse(miterReconstructionReplay.get(owner)!)}:{})}
}
const ownMiterProof=(body:ProgressiveMiterBrepBody,sections:NurbsCurve[][][],edges:number):ProgressiveMiterBrepBody=>{
 const steps=body.approximation.report.steps
 const sharp=ownedMiterSharpStations(sections.length,edges,steps,body.boundaryCertificate.closed)
 const {authoredFramesApplied,orientationGuideApplied,affineLawsApplied}=body.approximation.report
 miterProofOwners.set(body,{sections:JSON.stringify(sections),sharp,authoring:{authoredFramesApplied,orientationGuideApplied,affineLawsApplied}})
 return body
}
/** Reconstruct final corrected sections owned by the source constructor.
 * Original polyline vertices keep their independent one-sided jets. */
export function reconstructCertifiedMiterStations(source:CertifiedMiterBoundaryOwner,options:{quantum:number;maxWork:number;wallTolerance:number;maxDeviation:number}) {
 const owner=miterProofOwners.get(source)
 if(!owner?.sections||!owner.sharp)throw new Error('Station reconstruction requires constructor-owned retained sections')
 return smoothCertifiedMiterBody(source,JSON.parse(owner.sections) as NurbsCurve[][][],owner.sharp,options)
}
export interface ExactAffineLatticePlacement {model:NurbsBrep|null;operatorNormUpper:number|null;arithmeticErrorUpper:number|null;work:number;reason:string}
export const placeNurbsBrepOnExactAffineLattice=(model:NurbsBrep,matrix:number[][],quantum:number,maxWork:number):ExactAffineLatticePlacement=>callGeometryRust('brep_nurbs_affine_lattice',{model,matrix,quantum,maxWork})
/** Exact placement scales the complete wall/cap Hausdorff bound; actual G2,
 * regularity, material nesting and orientation are audited on the new body. */
export function transformCertifiedMiterBody(source:CertifiedMiterBoundaryOwner,matrix:number[][],options:{quantum:number;maxWork:number;maxDeviation:number}) {
 const owner=miterProofOwners.get(source)
 if(!owner)throw new Error('Affine placement requires constructor-owned boundary evidence')
 const original=miterOriginalRequests.get(source)
 if(!original)throw new Error('Affine placement requires constructor-owned original request')
 const history=JSON.parse(miterAffineHistory.get(source)??'[]') as {matrix:number[][];quantum:number;maxWork:number;budget:number|null}[]
 const steps=[...history,{matrix,quantum:options.quantum,maxWork:options.maxWork,budget:Number.isFinite(options.maxDeviation)?options.maxDeviation:null}]
 const result=callGeometryRust<{placement:ExactAffineLatticePlacement|null;boundaryCertificate:SweepBoundaryCertificate|null;reason:string;retainedWallCharts:SweepRetainedChartEvidence|null;volume:SweepVolumeAudit|null}>('brep_miter_owned_place',{
  source:JSON.parse(original),...steps[0],followingPlacements:steps.slice(1),
  expectedSourceModel:source.model,expectedSourceCertificate:source.boundaryCertificate,
  ...(miterReconstructionReplay.has(source)?{beforeReconstruction:JSON.parse(miterReconstructionReplay.get(source)!)}:{}),
  requireSolid:true,wallCells:100000,volumeBudgets:DEFAULT_SWEEP_VOLUME_BUDGETS,
 })
 const {placement,boundaryCertificate}=result
 if(result.reason==='boundary-budget-unproved')throw new Error('Affine complete boundary error exceeds max_deviation or is unproved')
 if(!placement?.model||!boundaryCertificate)throw new Error(`Affine placement unproved: ${result.reason}`)
 const closed=boundaryCertificate.closed
 const model=placement.model,capFaces=closed?[]:[model.faces.length-2,model.faces.length-1]
 const profileSmoothness=inspectMiterProfileSmoothness(model,capFaces)
 const {retainedWallCharts,volume}=result
 if(!retainedWallCharts||!volume)throw new Error('Native affine material report missing')
 const transformed={model,placement,boundaryCertificate,profileSmoothness,retainedWallCharts,volume,continuousBound:boundaryCertificate.continuousBound,boundaryErrorUpper:boundaryCertificate.errorUpper,boundaryErrorWithinBudget:boundaryCertificate.withinBudget,budget:boundaryCertificate.budget,filledCapErrorUpper:boundaryCertificate.filledCapErrorUpper,retainedWallErrorUpper:boundaryCertificate.wallErrorUpper,wallRegularityCertified:retainedWallCharts.allChartsCertified,profileRegularityCertified:retainedWallCharts.allChartsCertified}
 miterProofOwners.set(transformed,{})
 miterOriginalRequests.set(transformed,original)
 miterAffineHistory.set(transformed,JSON.stringify(steps))
 if(miterReconstructionReplay.has(source))miterReconstructionReplay.set(transformed,miterReconstructionReplay.get(source)!)
 return transformed
}
/** Compose a bounded wall reconstruction with constructor-owned full boundary
 * evidence. Endpoint sections/caps stay identical; every material audit is new. */
export function smoothCertifiedMiterBody(source:CertifiedMiterBoundaryOwner,sections:NurbsCurve[][][],sharp:number[],options:{quantum:number;maxWork:number;wallTolerance:number;maxDeviation:number}) {
 const owner=miterProofOwners.get(source)
 if(!owner)throw new Error('Station smoothing requires constructor-owned boundary evidence')
 const original=miterOriginalRequests.get(source)
 if(!original)throw new Error('Station smoothing requires constructor-owned original request')
 const reconstruction=reconstructMiterOwnedBoundary(JSON.parse(original),{
  quantum:options.quantum,tolerance:options.wallTolerance,maxWork:options.maxWork,budget:options.maxDeviation,
  requireSolid:true,wallCells:100000,volumeBudgets:DEFAULT_SWEEP_VOLUME_BUDGETS,
  expectedSourceModel:source.model,expectedSourceCertificate:source.boundaryCertificate,expectedSections:sections,expectedSharp:sharp,
 })
 const {model,candidate,boundaryCertificate}=reconstruction
 if(reconstruction.reason==='boundary-budget-unproved')throw new Error('Smoothed complete boundary error exceeds max_deviation or is unproved')
 if(!model||!candidate?.sides||candidate.wallDisplacementUpper===null||!boundaryCertificate)throw new Error(`Station smoothing candidate unproved: ${reconstruction.reason}`)
 const capFaces=boundaryCertificate.closed?[]:[model.faces.length-2,model.faces.length-1]
 const profileSmoothness=inspectMiterProfileSmoothness(model,capFaces)
 const {retainedWallCharts,volume}=reconstruction
 if(!retainedWallCharts||!volume)throw new Error('Native reconstruction material report missing')
 const smoothed={...owner.authoring,method:'bounded-miter-station-reconstruction' as const,model,candidate,sharpStationIndices:[...sharp],boundaryCertificate,profileSmoothness,retainedWallCharts,volume,
  continuousBound:boundaryCertificate.continuousBound,boundaryErrorUpper:boundaryCertificate.errorUpper,
  boundaryErrorWithinBudget:boundaryCertificate.withinBudget,filledCapErrorUpper:boundaryCertificate.filledCapErrorUpper,
  retainedWallErrorUpper:boundaryCertificate.wallErrorUpper,budget:boundaryCertificate.budget,
  wallRegularityCertified:retainedWallCharts.allChartsCertified,profileRegularityCertified:retainedWallCharts.allChartsCertified,globalEmbeddingCertified:false as const}
 miterProofOwners.set(smoothed,{})
 miterOriginalRequests.set(smoothed,original)
 miterReconstructionReplay.set(smoothed,JSON.stringify({quantum:options.quantum,tolerance:options.wallTolerance,maxWork:options.maxWork,budget:options.maxDeviation}))
 return smoothed
}
const auditMiterCapDomains=(sections:NurbsCurve[][][],options:ProgressiveMiterOptions,checkAbort=()=>{}):[SweepContourAudit,SweepContourAudit]|null=>{
 if(options.closed)return null
 const budgets=options.contourAuditBudgets??{tolerance:.001,maxPairs:1000,maxCells:1000}
 checkAbort();const start=inspectSweepContours(sections[0]!,budgets)
 checkAbort();const end=inspectSweepContours(sections.at(-1)!,budgets)
 checkAbort();return [start,end]
}
const defaultMiterWallAuditBudgets={clearance:0,distanceTolerance:.001,maxInjectivityCells:1000,maxPairs:1000,maxPairCells:1000}

/** Native root construction supplies geometry and boundary proof; the remaining
 * diagnostics are bounded native audits, preserving the public body report. */
const completeOwnedMiterBody=(
 owned:MiterOwnedBoundaryReport,loops:NurbsCurve[][],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,checkAbort=()=>{},
):ProgressiveMiterBrepBody=>{
 const profiles=loops.flat(),model=owned.model,sections=owned.sections
 const sourceOptions={...options,maxSteps:owned.maxSteps}
 const approximation:ProgressiveMiterResult={sections:owned.sourceSections,levels:owned.levels,report:owned.levels.at(-1)!}
 const sectionCorrection=owned.sectionCorrection??undefined
 const retainedSections=sections.map(section=>section.flat())
 checkAbort()
 const wallAudit=inspectProgressiveMiterWalls(profiles,points,scale,twist,sourceOptions,retainedSections,options.wallAuditBudgets??defaultMiterWallAuditBudgets,loops.map(loop=>loop.length))
 checkAbort()
 const capDomains=auditMiterCapDomains(sections,options,checkAbort)
 checkAbort()
 const retainedCorrespondence=inspectSweepRetainedCorrespondence(model,sections,owned.closed)
 checkAbort()
 const retainedDecomposition=retainedCorrespondence.exact?null:inspectSweepRetainedDecomposition(model,sections,owned.closed,options.retainedDecompositionBudgets?.maxProducts??100000,options.retainedDecompositionBudgets?.maxFaces??1024)
 checkAbort()
 const retainedCaps=owned.closed?null:inspectSweepRetainedCaps(model,[sections[0]!,sections.at(-1)!],(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets)
 checkAbort()
 const retainedCapDecomposition=owned.closed||retainedCaps?.exact?null:inspectSweepRetainedCapDecomposition(model,[sections[0]!,sections.at(-1)!],(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets,options.retainedDecompositionBudgets?.maxProducts??100000)
 checkAbort()
 const idealCapDomains=owned.closed?null:inspectProgressiveMiterIdealCapDomains(profiles,points,scale,twist,sourceOptions,loops.map(loop=>loop.length),options.capDomainBudgets)
 checkAbort()
 const capProjection=owned.closed?null:inspectProgressiveMiterCapProjection(profiles,points,scale,twist,sourceOptions,[model.faces.at(-2)!.surface,model.faces.at(-1)!.surface],options.capProjectionBudgets?.maxCells??10000,options.capProjectionBudgets?.maxExactWork??1000000)
 checkAbort()
 const capParallelism=owned.closed?null:inspectProgressiveMiterCapParallelism(profiles,points,scale,twist,sourceOptions,[model.faces.at(-2)!.surface,model.faces.at(-1)!.surface],options.capProjectionBudgets?.maxCells??10000,options.capProjectionBudgets?.maxExactWork??1000000)
 checkAbort()
 const boundaryCertificate=owned.boundaryCertificate
 const {errorUpper:boundaryErrorUpper,withinBudget:boundaryErrorWithinBudget,filledCapErrorUpper,wallErrorUpper:retainedWallErrorUpper}=boundaryCertificate
 const retainedWallCharts=owned.retainedWallCharts
 const capFaces=owned.closed?[]:[model.faces.length-2,model.faces.length-1]
 const capPairs=owned.closed?null:inspectSweepCapPairs(model,capFaces,options.capPairAuditBudgets??{clearance:0,distanceTolerance:.001,maxInjectivityCells:1000,maxPairs:1000,maxPairCells:1000},checkAbort)
 checkAbort()
 const capContacts=owned.closed?null:inspectSweepCapContacts(model,capFaces,options.capWallMaxWalls??1024,checkAbort)
 checkAbort()
 const embedding=owned.closed?null:inspectSweepEmbedding(model,capFaces,options.embeddingBudgets??DEFAULT_SWEEP_EMBEDDING_BUDGETS)
 checkAbort()
 const volume=inspectSweepVolume(model,capFaces,options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS)
 checkAbort()
 const profileSmoothness=inspectMiterProfileSmoothness(model,capFaces,undefined,checkAbort)
 checkAbort()
 const body=ownMiterProof({model,approximation,profileSmoothness,boundaryCertificate,retainedCapDecomposition,retainedDecomposition,capParallelism,boundaryErrorWithinBudget,boundaryErrorUpper,filledCapErrorUpper,idealCapDomains,capProjection,sectionCorrection,retainedWallErrorUpper,wallAudit,retainedCorrespondence,retainedCaps,retainedWallCharts,capDomains,capContacts,capPairs,embedding,volume,globalEmbeddingCertified:false},sections,owned.edges)
 miterOriginalRequests.set(body,JSON.stringify(progressiveMiterOwnedRequest(loops,points,scale,twist,{...options,maxSteps:owned.maxSteps})))
 return body
}
export const createProgressiveMiterBrepProfileBody=(loops:NurbsCurve[][],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions):ProgressiveMiterBrepBody=>
 completeOwnedMiterBody(constructProgressiveMiterOwnedBoundary(loops,points,scale,twist,options),loops,points,scale,twist,options)
export interface MiterBrepBody {model:NurbsBrep;report:{profileSmoothness:MiterProfileSmoothness;method:'polyline-miter-sections';sections:number;closedPath:boolean;globalEmbeddingCertified:false;roundingCertified:false;continuousBound:false;profileRegularityCertified:boolean;profileRegularity:SweepProfileRegularityAudit;wallRegularityCertified:boolean;retainedWallCharts:SweepRetainedChartEvidence;volume:SweepVolumeAudit;sectionCorrection?:Omit<SweepSectionCorrection,'sections'>;retainedCorrespondence?:SweepRetainedCorrespondence}}
export const createMiterBrepProfileBody=(loops:NurbsCurve[][],points:[number,number,number][],normal:[number,number,number],miterLimit=4,closed=false,capCorrection?:{quantum:number;tolerance:number;maxWork:number}):MiterBrepBody=>{
 if(loops.length<1||loops.length>16||loops.some(loop=>loop.length===0))throw new Error('Miter body needs 1..16 nonempty loops')
 let sections=miterNurbsProfileSections(loops.flat(),points,normal,miterLimit,closed)
 let sectionCorrection:Omit<SweepSectionCorrection,'sections'>|undefined
 if(capCorrection){
  if(closed)throw new Error('Closed miter has no caps to correct')
  const result=correctMiterCaps(sections,points,closed,capCorrection)
  const {sections:corrected,...evidence}=result
  sections=corrected;sectionCorrection=evidence
 }
 const nested=sections.map(section=>{let offset=0;return loops.map(loop=>{const wire=section.slice(offset,offset+loop.length);offset+=loop.length;return wire})})
 const model=closed?createPeriodicBrepSectionLoft(nested):createRationalBrepSectionLoft(nested)
 const retainedCorrespondence=sectionCorrection?inspectSweepRetainedCorrespondence(model,nested,closed):undefined
 if(sectionCorrection&&!retainedCorrespondence?.exact)throw new Error('Corrected miter retained wall correspondence unproved')
 const volume=inspectSweepVolume(model,closed?[]:[model.faces.length-2,model.faces.length-1],DEFAULT_SWEEP_VOLUME_BUDGETS)
 const profileRegularity=inspectSweepProfileRegularity(loops.flat(),10000)
 // A positive projected symmetric Jacobian has rank two everywhere on each
 // actual retained chart, and therefore proves wall regularity independently
 // of cap contacts, material orientation and authored approximation error.
 const retainedWallCharts=inspectSweepRetainedWallCharts(model,closed?[]:[model.faces.length-2,model.faces.length-1],DEFAULT_SWEEP_VOLUME_BUDGETS.maxLinearCells)
 return {model,report:{profileSmoothness:inspectMiterProfileSmoothness(model,closed?[]:[model.faces.length-2,model.faces.length-1]),method:'polyline-miter-sections',sections:sections.length,closedPath:closed,globalEmbeddingCertified:false,roundingCertified:false,continuousBound:false,profileRegularityCertified:profileRegularity.spanwiseRegular,profileRegularity,wallRegularityCertified:retainedWallCharts.allChartsCertified,retainedWallCharts,volume,...(sectionCorrection?{sectionCorrection,retainedCorrespondence}:{})}}
}
export const createRationalBrepSectionLoft=(sections:NurbsCurve[][][]):NurbsBrep=>callGeometryRust('brep_nurbs_rational_section_loft',{sections})
import type {NurbsScaleLaw,ProgressiveGuidedSurfaceSweepOptions,ProgressiveMultiSweepResult} from '../nurbsConstructors'
export type ProgressiveBrepSweepOptions=ProgressiveGuidedSurfaceSweepOptions & {capCorrection?:{quantum:number;tolerance:number;maxWork:number}}
export interface ProgressiveBrepBody {model:NurbsBrep;approximation:ProgressiveMultiSweepResult;globalEmbeddingCertified:false;
 bodyDecompositionErrorUpper?:number|null;bodyDecompositionProducts?:number
 capProjection?:{idealCapDomainsCertified:boolean;normalDots:[[number,number],[number,number]]|null;reversesOrientation:[boolean,boolean]|null;
  cells:number;exactWork:number;reason:string|null;scope:'constructor-owned-endpoint-plane-projection'}|null
 capCorrectionErrorUpper?:number|null
 filledCapErrorUpper?:[number,number]|null;boundaryContinuousBound?:boolean;boundaryErrorUpper?:number|null
 boundaryErrorWithinBudget?:boolean|null
 boundaryErrorScope?:'constructor-owned-retained-wall-and-cap-union'
 retainedWalls?:{certified:boolean;faceCoverageCertified:boolean;coefficientFamilyCertified:boolean;inspectedFaces:number;exactWork:number;reason:string}
 retainedCaps?:{exact:boolean;capErrorUpper:0|null;exactWork:number;inspectedEdges:number;reason:string;
  scope:'constructor-owned-retained-endpoint-regions';continuousBound:false;globalEmbeddingCertified:false}|null}
export type ProgressiveBodyBoundaryEvidence=Omit<ProgressiveBrepBody,'model'|'approximation'> & {budget:number;closedPath:boolean}
/** Open-path caps or closed-path periodic shells, constrained by the shared B-rep face budget. Twist values are degrees. */
export const createProgressiveBrepProfileBody=(loops:NurbsCurve[][],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveBrepSweepOptions):ProgressiveBrepBody=>
 callGeometryRust('brep_nurbs_progressive_profile_body',{loops,path,...(options.capCorrection?{cap_correction_quantum:options.capCorrection.quantum,cap_correction_tolerance:options.capCorrection.tolerance,cap_correction_max_work:options.capCorrection.maxWork}:{}),...sweepAffineLawPayload(options),...sweepFrameLawPayload(options),...sweepGuidePayload(options),
  scale:{degree:scale.degree,knots:scale.knots,controlPoints:scale.values.map(r=>[r,0,0]),weights:scale.weights,periodic:false},
  twist:{degree:twist.degree,knots:twist.knots,controlPoints:twist.values.map(a=>[a*Math.PI/180,0,0]),weights:twist.weights,periodic:false},
  ...(options.rmfTransportSteps===undefined?{}:{rmf_transport_steps:options.rmfTransportSteps}),...(options.errorMaxCells===undefined?{}:{error_max_cells:options.errorMaxCells}),...(options.errorMaxProducts===undefined?{}:{error_max_products:options.errorMaxProducts}),normal:options.normal,orientation:options.orientation??'rmf',spacing:options.spacing??'parameter',initial_sections:options.initialSections??5,max_sections:options.maxSections??257,max_deviation:options.maxDeviation,length_tolerance:options.lengthTolerance??0.001,length_max_cells:options.lengthMaxCells??100000})

import {sweepAffineLawPayload,sweepFrameLawPayload,sweepGuidePayload} from '../nurbsConstructors'

/** Closed contour shells with an identical repeated endpoint section; no caps. */
export const createPeriodicBrepSectionLoft=(sections:NurbsCurve[][][]):NurbsBrep=>callGeometryRust('brep_nurbs_periodic_section_loft',{sections})


/** Streams side-wall previews on the body face budget, then constructs audited
 * caps/seams. A preview level never contains an authoritative B-rep body.
 */
export async function* streamProgressiveBrepProfileBody(
 loops:NurbsCurve[][],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveBrepSweepOptions,control:import('../nurbsConstructors').ProgressiveSweepStreamOptions={},
):AsyncGenerator<import('../nurbsConstructors').ProgressiveSweepPreview,ProgressiveBrepBody,void>{
 const checkAbort=()=>{
  control.signal?.throwIfAborted()
  if(control.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')
 }
 checkAbort()
 // Retain the same original request through all post-preview certificates.
 ;({loops,path,scale,twist,options}=structuredClone({loops,path,scale,twist,options}))
 if(!loops.length||loops.length>16||loops.some(loop=>!loop.length)||(options.maxSections??257)>1025)
  throw new Error('Progressive body needs 1..16 nonempty loops and at most1025 sections')
 const profiles=loops.flat()
 const spans=profiles.reduce((count,curve)=>count+decomposeNurbsCurve(curve).length,0)
 if(!spans||spans>64)throw new Error('Progressive body exceeds64 section spans')
 const first=previewProgressiveNurbsProfiles(profiles,path,scale,twist,options,options.initialSections??5)
 // brep-core MAX_FACES is 1024; an open path reserves its two cap faces.
 const maximum=Math.min(options.maxSections??257,Math.floor((1024-(first.report.closedPath?0:2))/spans)+1)
 const bounded={...options,maxSections:maximum}
 if((options.initialSections??5)>maximum)throw new Error('Progressive body initial sections exceed face budget')
 const stream=streamProgressiveNurbsProfiles(profiles,path,scale,twist,bounded,control)
 try{
  for(;;){
   const level=await stream.next()
   checkAbort()
   if(level.done){
    if(!level.value.report.accepted)throw new Error(options.rmfTransportSteps===undefined
     ?'Progressive body sampled refinement exceeds budget'
     :'Progressive body continuous retained-patch error is unproved or exceeds budget')
    // Let a queued worker cancel run before final topology/cap construction.
    await new Promise<void>(resolve=>setTimeout(resolve,0))
    checkAbort()
    const body=createProgressiveBrepProfileBody(loops,path,scale,twist,bounded)
    checkAbort()
    return body
   }
   yield level.value
  }
 }finally{await stream.return(undefined as never)}
}
import {decomposeNurbsCurve} from '../nurbsCurve'
import {previewProgressiveNurbsProfiles,streamProgressiveNurbsProfiles} from '../nurbsConstructors'

/** Stream retained miter walls; build topology only after acceptance and a cancellation boundary. */
export async function* streamProgressiveMiterBrepProfileBody(
 loops:NurbsCurve[][],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveMiterOptions,control:import('../nurbsConstructors').ProgressiveSweepStreamOptions={},
):AsyncGenerator<import('../nurbsConstructors').ProgressiveSweepPreview,ProgressiveMiterBrepBody,void>{
 const checkAbort=()=>{control.signal?.throwIfAborted();if(control.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')}
 checkAbort()
 // Retain the same original request through all post-preview certificates.
 ;({loops,points,scale,twist,options}=structuredClone({loops,points,scale,twist,options}))
 const profiles=loops.flat(),{maxSteps:maximum}=planMiterBody(loops,points.length,options.closed??false,options.initialSteps??1,options.maxSteps??64)
 const stream=streamProgressiveMiterNurbsProfiles(profiles,points,scale,twist,{...options,maxSteps:maximum},control)
 try{
  for(;;){
   const level=await stream.next();checkAbort()
   if(level.done){
    await new Promise<void>(resolve=>setTimeout(resolve,0));checkAbort()
    const owned=constructProgressiveMiterOwnedBoundary(loops,points,scale,twist,{...options,maxSteps:maximum})
    checkAbort()
    return completeOwnedMiterBody(owned,loops,points,scale,twist,options,checkAbort)
   }
   const {patches,profilePatchRanges}=previewMiterWalls(level.value.sections)
   checkAbort();yield {preview:true,patches,profilePatchRanges,report:level.value.report};checkAbort()
  }
 }finally{await stream.return(undefined as never)}
}

export interface NurbsLoftCap { surface:NurbsSurface; trims:NurbsCurve[][] }
export interface LoftEmbeddingLimits {
 exactWork:number; trimPairs:number; trimCells:number; trimDomainCells:number; spans:number
 facePairs:number; faceCells:number; faceDomainCells:number; faceCellsPerPair:number; faceDomainCellsPerPair:number
}
const defaultLoftEmbeddingLimits:LoftEmbeddingLimits={exactWork:1000000,trimPairs:10000,trimCells:100000,
 trimDomainCells:1000000,spans:10000,facePairs:10000,faceCells:1000000,faceDomainCells:1000000,
 faceCellsPerPair:10000,faceDomainCellsPerPair:10000}
/** Authored curved caps; refuses unless whole-boundary embedding is proven. */
export const createCappedBrepLoftWithCaps=(start:NurbsCurve[][],end:NurbsCurve[][],sides:NurbsSurface[][],
 caps:[NurbsLoftCap,NurbsLoftCap],embeddingLimits:LoftEmbeddingLimits=defaultLoftEmbeddingLimits,toleranceUv=1e-9):NurbsBrep=>
 callGeometryRust('brep_nurbs_capped_loft_with_caps',{start,end,sides,caps,embeddingLimits,toleranceUv})
