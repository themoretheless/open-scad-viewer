import {repairNurbsCircleSweepSections} from '../nurbsCircleSectionRepair'
import {inspectMiterProfileSmoothness,type MiterProfileSmoothness} from '../miterProfileSmoothness'
import {composeSweepBoundaryCertificate,type SweepBoundaryCertificate} from '../sweepBoundaryCertificate'
import {certifiedSweepBoundaryErrorUpper,filledMiterCapErrorUpper} from '../nurbsFilledCapError'
import {addCertifiedErrorUpper,multiplyCertifiedErrorUpper} from '../nurbsErrorComposition'
import {projectSweepSections,type SweepSectionCorrection} from '../nurbsSectionProjection'
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
export const createRuledSketchLoft=(sketches:DirectSketch[],ids:string[]):{brep:NurbsBrep;mesh:PolygonMesh}=>{if(sketches.some(s=>ids.includes(s.id)&&s.retainedProfile))throw Error('Ruled sketch loft does not yet support retained curve profiles.');const r=callGeometryRust<{brep:NurbsBrep;mesh:PolygonMesh}>('cad_ruled_sketch_loft',{sketches,ids});normalizePolygonMesh(r.mesh);return r}
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
export const auditedMultiEdgeFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_audited_multi_edge_fillet',{model,edges,radius})
/** Exact equal-distance chamfer on a connected open/closed convex edge selection. */
export const exactConvexChamfer=(model:NurbsBrep,edges:number[],distance:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_convex_chamfer',{model,edges,distance})
/** Exact cylindrical rounds on selected longitudinal edges of a rigidly placed convex prism. */
export const exactConvexPrismFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_convex_prism_fillet',{model,edges,radius})
export const exactSimplePrismFillet=(model:NurbsBrep,edges:number[],radius:number):AuditedBrepFeature=>callGeometryRust('brep_nurbs_exact_simple_prism_fillet',{model,edges,radius})
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
import {inspectProgressiveMiterIdealCapDomains,inspectProgressiveMiterCapProjection,inspectProgressiveMiterCapParallelism,inspectProgressiveMiterWalls,streamProgressiveMiterNurbsProfiles,progressiveMiterNurbsProfiles,type ProgressiveMiterOptions,type ProgressiveMiterResult} from '../nurbsConstructors'
import {inspectSweepContours,inspectSweepProfileRegularity,type SweepProfileRegularityAudit,type SweepContourAudit} from '../nurbsSweepAudit'
import {inspectSweepRetainedWallCharts,type SweepRetainedChartEvidence} from '../nurbsSweepRetainedCharts'
import {inspectSweepCapContacts,inspectSweepCapPairs,type SweepCapPairEvidence,type SweepCapContactEvidence} from '../nurbsSweepCapContacts'
const correctMiterCaps=(sections:NurbsCurve[][],points:[number,number,number][],closed:boolean,capCorrection:{quantum:number;tolerance:number;maxWork:number;authoredFrame?:boolean},frameAxis?:import('../nurbsConstructors').NurbsVectorLaw):SweepSectionCorrection & {sections:NurbsCurve[][]}=>{
 if(closed)throw new Error('Closed miter has no caps to correct')
 if(capCorrection.authoredFrame){
  if(!frameAxis)throw new Error('Authored cap correction requires frame_axis')
  const axis:NurbsCurve={degree:frameAxis.degree,knots:frameAxis.knots,controlPoints:frameAxis.values,weights:frameAxis.weights,periodic:false}
  const result=projectSweepSections(sections,[
   {section:0,frameAxis:axis,traversal:0,quantum:capCorrection.quantum,tolerance:capCorrection.tolerance},
   {section:sections.length-1,frameAxis:axis,traversal:1,quantum:capCorrection.quantum,tolerance:capCorrection.tolerance},
  ],capCorrection.maxWork)
  if(!result.sections||result.wallDisplacementUpper===null)throw new Error(`Miter authored cap correction unproved: ${result.reason}`)
  return {...result,sections:result.sections}
 }

  const endpointPlane=(end:boolean)=>{
   const at=end?points.length-1:0,from=end?points.length-2:0,to=end?points.length-1:1
   const direction=points[to]!.map((x,k)=>x-points[from]![k])
   const axis=([0,1,2] as const).reduce((a,b)=>Math.abs(direction[a]!)>=Math.abs(direction[b]!)?a:b)
   if(!Number.isFinite(direction[axis])||direction[axis]===0)throw new Error('Cap correction needs a nonzero endpoint direction')
   const free=([0,1,2] as const).filter(k=>k!==axis)
   const coefficients=free.map(k=>-direction[k]!/direction[axis]!) as [number,number]
   const offset=points[at]![axis]-coefficients[0]*points[at]![free[0]!]-coefficients[1]*points[at]![free[1]!]
   return {axis,coefficients,offset}
  }
  const result=projectSweepSections(sections,[
   {section:0,plane:endpointPlane(false),quantum:capCorrection.quantum,tolerance:capCorrection.tolerance},
   {section:sections.length-1,plane:endpointPlane(true),quantum:capCorrection.quantum,tolerance:capCorrection.tolerance},
  ],capCorrection.maxWork)
  if(!result.sections||result.wallDisplacementUpper===null)throw new Error(`Miter cap correction unproved: ${result.reason}`)
  return {...result,sections:result.sections}
}
const correctMiterSections=(sections:NurbsCurve[][],points:[number,number,number][],options:ProgressiveMiterOptions,checkAbort=()=>{}):(SweepSectionCorrection & {sections:NurbsCurve[][]})|undefined=>{
 if(!options.circleCorrection)return options.capCorrection?correctMiterCaps(sections,points,options.closed??false,options.capCorrection,options.frameAxis):undefined
 const circle=repairNurbsCircleSweepSections(sections,options.circleCorrection,checkAbort)
 if(!circle.sections||circle.wallDisplacementUpper===null)throw new Error(`Miter circle section correction unproved: ${circle.reason}`)
 const cap=options.capCorrection?correctMiterCaps(circle.sections,points,options.closed??false,options.capCorrection,options.frameAxis):undefined
 const upper=addCertifiedErrorUpper(cap?.wallDisplacementUpper??0,circle.wallDisplacementUpper)
 if(upper===null)throw new Error('Miter combined section correction displacement unproved')
 // Project caps last; their exact-plane evidence then belongs to the final sections.
 // Projection may disturb profile jets, which are revalidated on the rebuilt body.
 return {sections:cap?.sections??circle.sections,wallDisplacementUpper:upper,exactPlanarSections:cap?.exactPlanarSections??[],work:(cap?.work??0)+circle.work,reason:'bounded-circle-section-interpolation'}
}
export interface ProgressiveMiterBrepBody {profileSmoothness:MiterProfileSmoothness;boundaryCertificate:SweepBoundaryCertificate;retainedCapDecomposition:import('../sweepRetainedCorrespondence').SweepRetainedCapDecomposition|null;retainedDecomposition:import('../sweepRetainedCorrespondence').SweepRetainedDecomposition|null;capParallelism:import('../nurbsConstructors').ProgressiveMiterCapParallelism|null;boundaryErrorWithinBudget:boolean|null;boundaryErrorUpper:number|null;filledCapErrorUpper:[number,number]|null;idealCapDomains:import('../nurbsConstructors').ProgressiveMiterIdealCapDomains|null;capProjection:import('../nurbsConstructors').ProgressiveMiterCapProjection|null;retainedWallErrorUpper:number|null;sectionCorrection?:Omit<SweepSectionCorrection,'sections'>;model:NurbsBrep;approximation:ProgressiveMiterResult;wallAudit:import('../nurbsSweepAudit').SweepWallAudit;retainedCorrespondence:SweepRetainedCorrespondence;retainedCaps:SweepRetainedCaps|null;retainedWallCharts:SweepRetainedChartEvidence;capDomains:[SweepContourAudit,SweepContourAudit]|null;capContacts:SweepCapContactEvidence[]|null;capPairs:SweepCapPairEvidence|null;embedding:SweepEmbeddingAudit|null;volume:SweepVolumeAudit;globalEmbeddingCertified:false}
// Bind source certificates to the exact constructor-owned geometry. Mutation,
// JSON copies or caller-authored evidence cannot transfer an old boundary bound.
export interface CertifiedMiterBoundaryOwner {model:NurbsBrep;boundaryCertificate:SweepBoundaryCertificate}
const miterProofOwners=new WeakMap<CertifiedMiterBoundaryOwner,{model:string;certificate:string;sections?:string;sharp?:number[];authoring?:{authoredFramesApplied:boolean;orientationGuideApplied:boolean;affineLawsApplied:boolean}}>()
const ownMiterProof=(body:ProgressiveMiterBrepBody,sections:NurbsCurve[][][],edges:number):ProgressiveMiterBrepBody=>{
 const steps=body.approximation.report.steps
 if(sections.length!==edges*steps+1)throw new Error('Miter station ownership requires complete uniform span coverage')
 const sharp=Array.from({length:body.boundaryCertificate.closed?edges:Math.max(0,edges-1)},(_,i)=>(i+(body.boundaryCertificate.closed?0:1))*steps)
 const {authoredFramesApplied,orientationGuideApplied,affineLawsApplied}=body.approximation.report
 miterProofOwners.set(body,{model:JSON.stringify(body.model),certificate:JSON.stringify(body.boundaryCertificate),sections:JSON.stringify(sections),sharp,authoring:{authoredFramesApplied,orientationGuideApplied,affineLawsApplied}})
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
 if(!owner||owner.model!==JSON.stringify(source.model)||owner.certificate!==JSON.stringify(source.boundaryCertificate))throw new Error('Affine placement requires unchanged constructor-owned boundary evidence')
 if(!source.boundaryCertificate.continuousBound||!source.boundaryCertificate.withinBudget)throw new Error('Affine source complete boundary bound unproved')
 const placement=placeNurbsBrepOnExactAffineLattice(source.model,matrix,options.quantum,options.maxWork)
 if(!placement.model||placement.operatorNormUpper===null||placement.arithmeticErrorUpper!==0)throw new Error(`Exact affine placement unproved: ${placement.reason}`)
 const norm=placement.operatorNormUpper
 const scale=(upper:number|null)=>upper===null?null:multiplyCertifiedErrorUpper(norm,upper)
 const caps=source.boundaryCertificate.filledCapErrorUpper?.map(scale)
 const closed=source.boundaryCertificate.closed
 const boundaryCertificate=composeSweepBoundaryCertificate(scale(source.boundaryCertificate.wallErrorUpper),caps&&caps.every((x):x is number=>x!==null)?caps as [number,number]:null,closed,options.maxDeviation)
 if(!boundaryCertificate.continuousBound||!boundaryCertificate.withinBudget)throw new Error('Affine complete boundary error exceeds max_deviation or is unproved')
 const model=placement.model,capFaces=closed?[]:[model.faces.length-2,model.faces.length-1]
 const profileSmoothness=inspectMiterProfileSmoothness(model,capFaces)
 const retainedWallCharts=inspectSweepRetainedWallCharts(model,capFaces,100000)
 const volume=inspectSweepVolume(model,capFaces,DEFAULT_SWEEP_VOLUME_BUDGETS)
 if(!retainedWallCharts.allChartsCertified||!volume.solidGeometryCertified)throw new Error('Affine transformed Solid geometry unproved')
 const transformed={model,placement,boundaryCertificate,profileSmoothness,retainedWallCharts,volume,continuousBound:boundaryCertificate.continuousBound,boundaryErrorUpper:boundaryCertificate.errorUpper,boundaryErrorWithinBudget:boundaryCertificate.withinBudget,budget:boundaryCertificate.budget,filledCapErrorUpper:boundaryCertificate.filledCapErrorUpper,retainedWallErrorUpper:boundaryCertificate.wallErrorUpper,wallRegularityCertified:retainedWallCharts.allChartsCertified,profileRegularityCertified:retainedWallCharts.allChartsCertified}
 miterProofOwners.set(transformed,{model:JSON.stringify(model),certificate:JSON.stringify(boundaryCertificate)})
 return transformed
}
/** Compose a bounded wall reconstruction with constructor-owned full boundary
 * evidence. Endpoint sections/caps stay identical; every material audit is new. */
export function smoothCertifiedMiterBody(source:CertifiedMiterBoundaryOwner,sections:NurbsCurve[][][],sharp:number[],options:{quantum:number;maxWork:number;wallTolerance:number;maxDeviation:number}) {
 const owner=miterProofOwners.get(source)
 if(!owner||owner.model!==JSON.stringify(source.model)||owner.certificate!==JSON.stringify(source.boundaryCertificate))throw new Error('Station smoothing requires unchanged constructor-owned boundary evidence')
 const certificate=source.boundaryCertificate
 if(!certificate.continuousBound||!certificate.withinBudget||certificate.wallErrorUpper===null)throw new Error('Station smoothing source complete boundary bound unproved')
 const baseline=certificate.closed?createPeriodicBrepSectionLoft(sections):createRationalBrepSectionLoft(sections)
 if(JSON.stringify(baseline)!==owner.model)throw new Error('Station smoothing sections do not reproduce the certified source body')
 const candidate=proposeSmoothStationWalls(sections,sharp,certificate.closed,options.quantum,options.wallTolerance,options.maxWork)
 if(!candidate.sides||candidate.wallDisplacementUpper===null)throw new Error(`Station smoothing candidate unproved: ${candidate.reason}`)
 const wallUpper=addCertifiedErrorUpper(certificate.wallErrorUpper,candidate.wallDisplacementUpper)
 const boundaryCertificate=composeSweepBoundaryCertificate(wallUpper,certificate.filledCapErrorUpper,certificate.closed,options.maxDeviation)
 if(!boundaryCertificate.continuousBound||!boundaryCertificate.withinBudget)throw new Error('Smoothed complete boundary error exceeds max_deviation or is unproved')
 const model=createBrepSectionLoftSurfaces(sections,candidate.sides,certificate.closed)
 const capFaces=certificate.closed?[]:[model.faces.length-2,model.faces.length-1]
 for(const face of capFaces) {
  if(JSON.stringify(model.faces[face])!==JSON.stringify(baseline.faces[face]))throw new Error('Station smoothing changed a filled cap')
  for(const loop of [model.faces[face]!.outer,...model.faces[face]!.holes]) {
   if(JSON.stringify(model.loops[loop])!==JSON.stringify(baseline.loops[loop]))throw new Error('Station smoothing changed cap trims')
   for(const use of model.loops[loop]!.coedges) {
    const edge=model.edges[use.edge]!
    if(JSON.stringify(edge)!==JSON.stringify(baseline.edges[use.edge]))throw new Error('Station smoothing changed a cap boundary edge')
    for(const vertex of edge.vertices)if(JSON.stringify(model.vertices[vertex])!==JSON.stringify(baseline.vertices[vertex]))throw new Error('Station smoothing changed a cap boundary vertex')
   }
  }
 }
 const profileSmoothness=inspectMiterProfileSmoothness(model,capFaces)
 const retainedWallCharts=inspectSweepRetainedWallCharts(model,capFaces,100000)
 const volume=inspectSweepVolume(model,capFaces,DEFAULT_SWEEP_VOLUME_BUDGETS)
 if(!retainedWallCharts.allChartsCertified||!volume.solidGeometryCertified)throw new Error('Smoothed Solid geometry unproved')
 const smoothed={...owner.authoring,method:'bounded-miter-station-reconstruction' as const,model,candidate,sharpStationIndices:[...sharp],boundaryCertificate,profileSmoothness,retainedWallCharts,volume,
  continuousBound:boundaryCertificate.continuousBound,boundaryErrorUpper:boundaryCertificate.errorUpper,
  boundaryErrorWithinBudget:boundaryCertificate.withinBudget,filledCapErrorUpper:boundaryCertificate.filledCapErrorUpper,
  retainedWallErrorUpper:boundaryCertificate.wallErrorUpper,budget:boundaryCertificate.budget,
  wallRegularityCertified:retainedWallCharts.allChartsCertified,profileRegularityCertified:retainedWallCharts.allChartsCertified,globalEmbeddingCertified:false as const}
 miterProofOwners.set(smoothed,{model:JSON.stringify(model),certificate:JSON.stringify(boundaryCertificate)})
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

export const createProgressiveMiterBrepProfileBody=(loops:NurbsCurve[][],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions):ProgressiveMiterBrepBody=>{
 if(loops.length<1||loops.length>16||loops.some(loop=>!loop.length))throw new Error('Miter body needs 1..16 nonempty loops')
 const profiles=loops.flat(),edges=points.length-(options.closed?0:1)
 const spans=profiles.reduce((count,curve)=>count+decomposeNurbsCurve(curve).length,0)
 if(edges<1||spans<1||spans>64)throw new Error('Progressive miter body exceeds its site/span budget')
 const maximum=Math.min(options.maxSteps??64,Math.floor(1024/edges),Math.floor((1024-(options.closed?0:2))/(edges*spans)))
 if(maximum<(options.initialSteps??1))throw new Error('Progressive miter initial steps exceed face budget')
 const approximation=progressiveMiterNurbsProfiles(profiles,points,scale,twist,{...options,maxSteps:maximum})
 if(approximation.report.frameTransportCertified===false)throw new Error('Progressive miter frame transport could not be proved; review the path, normal and miter limit')
    if(approximation.report.certifiedErrorUpper===null)throw new Error(`Progressive miter wall error bound could not be proved: ${approximation.report.errorCertificateReason??'unresolved certificate'}`)
    if(approximation.report.profileRegularityCertified===false)throw new Error('Progressive miter profile tangent regularity could not be proved')
    if(approximation.report.wallRegularityCertified===false)throw new Error('Progressive miter retained wall Jacobian regularity could not be proved')
 if(!approximation.report.accepted||!approximation.sections)throw new Error('Progressive miter refinement/phase budget not met within the body face budget')
 const correction=correctMiterSections(approximation.sections,points,options)
 const retainedSections=correction?.sections??approximation.sections
 const sectionCorrection=correction?(({sections,...evidence})=>evidence)(correction):undefined
 const wallAudit=inspectProgressiveMiterWalls(profiles,points,scale,twist,{...options,maxSteps:maximum},retainedSections,options.wallAuditBudgets??defaultMiterWallAuditBudgets,loops.map(loop=>loop.length))
 const sections=retainedSections.map(section=>{let offset=0;return loops.map(loop=>{const wire=section.slice(offset,offset+loop.length);offset+=loop.length;return wire})})
 const capDomains=auditMiterCapDomains(sections,options)
 const model=options.closed?createPeriodicBrepSectionLoft(sections):createRationalBrepSectionLoft(sections)
 const retainedCorrespondence=inspectSweepRetainedCorrespondence(model,sections,options.closed??false)
 const retainedDecomposition=retainedCorrespondence.exact?null:inspectSweepRetainedDecomposition(model,sections,options.closed??false,options.retainedDecompositionBudgets?.maxProducts??100000,options.retainedDecompositionBudgets?.maxFaces??1024)
 const decompositionError=retainedCorrespondence.exact?0:retainedDecomposition?.wallErrorUpper??null
 if(sectionCorrection&&decompositionError===null)throw new Error('Corrected progressive miter retained wall correspondence unproved')
 const baseWallError=addCertifiedErrorUpper(approximation.report.certifiedErrorUpper!,sectionCorrection?.wallDisplacementUpper??0)
 const retainedWallErrorUpper=baseWallError===null||decompositionError===null?null:addCertifiedErrorUpper(baseWallError,decompositionError)
 if(sectionCorrection&&(retainedWallErrorUpper===null||retainedWallErrorUpper>options.maxDeviation))throw new Error('Corrected progressive miter retained wall error exceeds max_deviation or is unproved')
 const retainedCaps=options.closed?null:inspectSweepRetainedCaps(model,[sections[0]!,sections.at(-1)!],(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets)
 const retainedCapDecomposition=options.closed||retainedCaps?.exact?null:inspectSweepRetainedCapDecomposition(model,[sections[0]!,sections.at(-1)!],(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets,options.retainedDecompositionBudgets?.maxProducts??100000)
 if(sectionCorrection&&!options.closed&&!retainedCaps?.exact&&!retainedCapDecomposition?.certified)throw new Error('Corrected progressive miter filled retained cap regions unproved')
 const idealCapDomains=options.closed?null:inspectProgressiveMiterIdealCapDomains(profiles,points,scale,twist,options,loops.map(loop=>loop.length),options.capDomainBudgets)
 const capProjection=options.closed?null:inspectProgressiveMiterCapProjection(profiles,points,scale,twist,options,[model.faces.at(-2)!.surface,model.faces.at(-1)!.surface],options.capProjectionBudgets?.maxCells??10000,options.capProjectionBudgets?.maxExactWork??1000000)
 const capParallelism=options.closed?null:inspectProgressiveMiterCapParallelism(profiles,points,scale,twist,options,[model.faces.at(-2)!.surface,model.faces.at(-1)!.surface],options.capProjectionBudgets?.maxCells??10000,options.capProjectionBudgets?.maxExactWork??1000000)
 const filledCapErrorUpper=options.closed?null:filledMiterCapErrorUpper({parallelPlanesCertified:capParallelism?.parallel??null,idealCapDomainsCertified:idealCapDomains?.idealCapDomainsCertified===true,retainedCapRegionsExact:retainedCaps?.exact===true||retainedCapDecomposition?.certified===true,decompositionErrorUpper:retainedCaps?.exact?[0,0]:retainedCapDecomposition?.capErrorUpper??null,projectionNormalDots:capProjection?.normalDots??null,endpointContourErrorUpper:approximation.report.endpointContourErrorUpper,correctionDisplacementUpper:sectionCorrection?sectionCorrection.wallDisplacementUpper:0})
 const boundaryErrorUpper=certifiedSweepBoundaryErrorUpper(retainedWallErrorUpper,filledCapErrorUpper,options.closed??false)
 const boundaryErrorWithinBudget=boundaryErrorUpper===null?null:boundaryErrorUpper<=options.maxDeviation
 const boundaryCertificate=composeSweepBoundaryCertificate(retainedWallErrorUpper,filledCapErrorUpper,options.closed??false,options.maxDeviation)
 if(boundaryCertificate.withinBudget===false)throw new Error('Progressive miter complete boundary error exceeds max_deviation')
 const retainedWallCharts=inspectSweepRetainedWallCharts(model,options.closed?[]:[model.faces.length-2,model.faces.length-1],options.retainedWallMaxInjectivityCells??1000)
 if(sectionCorrection&&!retainedWallCharts.allChartsCertified)throw new Error('Corrected progressive miter retained wall regularity unproved')
 const capPairs=options.closed?null:inspectSweepCapPairs(model,[model.faces.length-2,model.faces.length-1],options.capPairAuditBudgets??{clearance:0,distanceTolerance:.001,maxInjectivityCells:1000,maxPairs:1000,maxPairCells:1000})
 const capContacts=options.closed?null:inspectSweepCapContacts(model,[model.faces.length-2,model.faces.length-1],options.capWallMaxWalls??1024)
 const embedding=options.closed?null:inspectSweepEmbedding(model,[model.faces.length-2,model.faces.length-1],options.embeddingBudgets??DEFAULT_SWEEP_EMBEDDING_BUDGETS)
 const volume=inspectSweepVolume(model,options.closed?[]:[model.faces.length-2,model.faces.length-1],options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS)
 return ownMiterProof({model,approximation,profileSmoothness:inspectMiterProfileSmoothness(model,options.closed?[]:[model.faces.length-2,model.faces.length-1]),boundaryCertificate,retainedCapDecomposition,retainedDecomposition,capParallelism,boundaryErrorWithinBudget,boundaryErrorUpper,filledCapErrorUpper,idealCapDomains,capProjection,sectionCorrection,retainedWallErrorUpper,wallAudit,retainedCorrespondence,retainedCaps,retainedWallCharts,capDomains,capContacts,capPairs,embedding,volume,globalEmbeddingCertified:false},sections,edges)
}
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
export interface ProgressiveBrepBody {model:NurbsBrep;approximation:ProgressiveMultiSweepResult;globalEmbeddingCertified:false}
/** Open-path caps or closed-path periodic shells, constrained by the shared B-rep face budget. Twist values are degrees. */
export const createProgressiveBrepProfileBody=(loops:NurbsCurve[][],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveGuidedSurfaceSweepOptions):ProgressiveBrepBody=>
 callGeometryRust('brep_nurbs_progressive_profile_body',{loops,path,...sweepAffineLawPayload(options),...sweepFrameLawPayload(options),...sweepGuidePayload(options),
  scale:{degree:scale.degree,knots:scale.knots,controlPoints:scale.values.map(r=>[r,0,0]),weights:scale.weights,periodic:false},
  twist:{degree:twist.degree,knots:twist.knots,controlPoints:twist.values.map(a=>[a*Math.PI/180,0,0]),weights:twist.weights,periodic:false},
  normal:options.normal,orientation:options.orientation??'rmf',spacing:options.spacing??'parameter',initial_sections:options.initialSections??5,max_sections:options.maxSections??257,max_deviation:options.maxDeviation,length_tolerance:options.lengthTolerance??0.001,length_max_cells:options.lengthMaxCells??100000})

import {sweepAffineLawPayload,sweepFrameLawPayload,sweepGuidePayload} from '../nurbsConstructors'

/** Closed contour shells with an identical repeated endpoint section; no caps. */
export const createPeriodicBrepSectionLoft=(sections:NurbsCurve[][][]):NurbsBrep=>callGeometryRust('brep_nurbs_periodic_section_loft',{sections})


/** Streams side-wall previews on the body face budget, then constructs audited
 * caps/seams. A preview level never contains an authoritative B-rep body.
 */
export async function* streamProgressiveBrepProfileBody(
 loops:NurbsCurve[][],path:NurbsCurve,scale:NurbsScaleLaw,twist:NurbsScaleLaw,
 options:ProgressiveGuidedSurfaceSweepOptions,control:import('../nurbsConstructors').ProgressiveSweepStreamOptions={},
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
    if(!level.value.report.accepted)throw new Error('Progressive body sampled refinement exceeds budget')
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
 if(!loops.length||loops.length>16||loops.some(loop=>!loop.length))throw new Error('Miter body needs 1..16 nonempty loops')
 const profiles=loops.flat(),edges=points.length-(options.closed?0:1)
 const spans=profiles.reduce((n,c)=>n+decomposeNurbsCurve(c).length,0)
 if(edges<1||!spans||spans>64)throw new Error('Progressive miter body exceeds its site/span budget')
 const maximum=Math.min(options.maxSteps??64,Math.floor(1024/edges),Math.floor((1024-(options.closed?0:2))/(edges*spans)))
 if(maximum<(options.initialSteps??1))throw new Error('Progressive miter initial steps exceed face budget')
 const stream=streamProgressiveMiterNurbsProfiles(profiles,points,scale,twist,{...options,maxSteps:maximum},control)
 try{
  for(;;){
   const level=await stream.next();checkAbort()
   if(level.done){
    const approximation=level.value
    if(approximation.report.frameTransportCertified===false)throw new Error('Progressive miter frame transport could not be proved; review the path, normal and miter limit')
    if(approximation.report.certifiedErrorUpper===null)throw new Error(`Progressive miter wall error bound could not be proved: ${approximation.report.errorCertificateReason??'unresolved certificate'}`)
    if(approximation.report.profileRegularityCertified===false)throw new Error('Progressive miter profile tangent regularity could not be proved')
    if(approximation.report.wallRegularityCertified===false)throw new Error('Progressive miter retained wall Jacobian regularity could not be proved')
    if(!approximation.report.accepted||!approximation.sections)throw new Error('Progressive miter refinement/phase budget not met within the body face budget')
    await new Promise<void>(resolve=>setTimeout(resolve,0));checkAbort()
    const correction=correctMiterSections(approximation.sections,points,options,checkAbort)
    const retainedSections=correction?.sections??approximation.sections
    const sectionCorrection=correction?(({sections,...evidence})=>evidence)(correction):undefined
    const wallAudit=inspectProgressiveMiterWalls(profiles,points,scale,twist,{...options,maxSteps:maximum},retainedSections,options.wallAuditBudgets??defaultMiterWallAuditBudgets,loops.map(loop=>loop.length))
    checkAbort()
    const sections=retainedSections.map(row=>{let offset=0;return loops.map(loop=>{const wire=row.slice(offset,offset+loop.length);offset+=loop.length;return wire})})
    const capDomains=auditMiterCapDomains(sections,options,checkAbort)
    const model=options.closed?createPeriodicBrepSectionLoft(sections):createRationalBrepSectionLoft(sections)
    checkAbort()
    const retainedCorrespondence=inspectSweepRetainedCorrespondence(model,sections,options.closed??false)
 const retainedDecomposition=retainedCorrespondence.exact?null:inspectSweepRetainedDecomposition(model,sections,options.closed??false,options.retainedDecompositionBudgets?.maxProducts??100000,options.retainedDecompositionBudgets?.maxFaces??1024)
 const decompositionError=retainedCorrespondence.exact?0:retainedDecomposition?.wallErrorUpper??null
 if(sectionCorrection&&decompositionError===null)throw new Error('Corrected progressive miter retained wall correspondence unproved')
 const baseWallError=addCertifiedErrorUpper(approximation.report.certifiedErrorUpper!,sectionCorrection?.wallDisplacementUpper??0)
 const retainedWallErrorUpper=baseWallError===null||decompositionError===null?null:addCertifiedErrorUpper(baseWallError,decompositionError)
 if(sectionCorrection&&(retainedWallErrorUpper===null||retainedWallErrorUpper>options.maxDeviation))throw new Error('Corrected progressive miter retained wall error exceeds max_deviation or is unproved')
 const retainedCaps=options.closed?null:inspectSweepRetainedCaps(model,[sections[0]!,sections.at(-1)!],(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets)
 const retainedCapDecomposition=options.closed||retainedCaps?.exact?null:inspectSweepRetainedCapDecomposition(model,[sections[0]!,sections.at(-1)!],(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets,options.retainedDecompositionBudgets?.maxProducts??100000)
 if(sectionCorrection&&!options.closed&&!retainedCaps?.exact&&!retainedCapDecomposition?.certified)throw new Error('Corrected progressive miter filled retained cap regions unproved')
 const idealCapDomains=options.closed?null:inspectProgressiveMiterIdealCapDomains(profiles,points,scale,twist,options,loops.map(loop=>loop.length),options.capDomainBudgets)
 const capProjection=options.closed?null:inspectProgressiveMiterCapProjection(profiles,points,scale,twist,options,[model.faces.at(-2)!.surface,model.faces.at(-1)!.surface],options.capProjectionBudgets?.maxCells??10000,options.capProjectionBudgets?.maxExactWork??1000000)
 const capParallelism=options.closed?null:inspectProgressiveMiterCapParallelism(profiles,points,scale,twist,options,[model.faces.at(-2)!.surface,model.faces.at(-1)!.surface],options.capProjectionBudgets?.maxCells??10000,options.capProjectionBudgets?.maxExactWork??1000000)
 const filledCapErrorUpper=options.closed?null:filledMiterCapErrorUpper({parallelPlanesCertified:capParallelism?.parallel??null,idealCapDomainsCertified:idealCapDomains?.idealCapDomainsCertified===true,retainedCapRegionsExact:retainedCaps?.exact===true||retainedCapDecomposition?.certified===true,decompositionErrorUpper:retainedCaps?.exact?[0,0]:retainedCapDecomposition?.capErrorUpper??null,projectionNormalDots:capProjection?.normalDots??null,endpointContourErrorUpper:approximation.report.endpointContourErrorUpper,correctionDisplacementUpper:sectionCorrection?sectionCorrection.wallDisplacementUpper:0})
 const boundaryErrorUpper=certifiedSweepBoundaryErrorUpper(retainedWallErrorUpper,filledCapErrorUpper,options.closed??false)
 const boundaryErrorWithinBudget=boundaryErrorUpper===null?null:boundaryErrorUpper<=options.maxDeviation
 const boundaryCertificate=composeSweepBoundaryCertificate(retainedWallErrorUpper,filledCapErrorUpper,options.closed??false,options.maxDeviation)
 if(boundaryCertificate.withinBudget===false)throw new Error('Progressive miter complete boundary error exceeds max_deviation')
 const retainedWallCharts=inspectSweepRetainedWallCharts(model,options.closed?[]:[model.faces.length-2,model.faces.length-1],options.retainedWallMaxInjectivityCells??1000,checkAbort)
    if(sectionCorrection&&!retainedWallCharts.allChartsCertified)throw new Error('Corrected progressive miter retained wall regularity unproved')
 const capPairs=options.closed?null:inspectSweepCapPairs(model,[model.faces.length-2,model.faces.length-1],options.capPairAuditBudgets??{clearance:0,distanceTolerance:.001,maxInjectivityCells:1000,maxPairs:1000,maxPairCells:1000},checkAbort)
    const capContacts=options.closed?null:inspectSweepCapContacts(model,[model.faces.length-2,model.faces.length-1],options.capWallMaxWalls??1024,checkAbort)
    checkAbort()
    const embedding=options.closed?null:inspectSweepEmbedding(model,[model.faces.length-2,model.faces.length-1],options.embeddingBudgets??DEFAULT_SWEEP_EMBEDDING_BUDGETS)
    checkAbort()
    const volume=inspectSweepVolume(model,options.closed?[]:[model.faces.length-2,model.faces.length-1],options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS)
    checkAbort();return ownMiterProof({model,approximation,profileSmoothness:inspectMiterProfileSmoothness(model,options.closed?[]:[model.faces.length-2,model.faces.length-1],undefined,checkAbort),boundaryCertificate,retainedCapDecomposition,retainedDecomposition,capParallelism,boundaryErrorWithinBudget,boundaryErrorUpper,filledCapErrorUpper,idealCapDomains,capProjection,sectionCorrection,retainedWallErrorUpper,wallAudit,retainedCorrespondence,retainedCaps,retainedWallCharts,capDomains,capContacts,capPairs,embedding,volume,globalEmbeddingCertified:false},sections,edges)
   }
   const rows=level.value.sections.map(row=>row.map(c=>decomposeNurbsCurve(c).map(span=>span.curve)))
   const patches:NurbsSurface[]=[],profilePatchRanges:[number,number][]=[]
   for(let p=0;p<profiles.length;p++){
    const start=patches.length
    for(let station=0;station<rows.length-1;station++)for(let span=0;span<rows[station]![p]!.length;span++){
     const a=rows[station]![p]![span]!,b=rows[station+1]![p]![span]!
     patches.push({degreeU:a.degree,degreeV:1,knotsU:a.knots,knotsV:[0,0,1,1],
      controlPoints:a.controlPoints.map((point,i)=>[point,b.controlPoints[i]!]),weights:a.weights.map((w,i)=>[w,b.weights[i]!]),periodicU:false,periodicV:false})
    }
    profilePatchRanges.push([start,patches.length])
   }
   checkAbort();yield {preview:true,patches,profilePatchRanges,report:level.value.report};checkAbort()
  }
 }finally{await stream.return(undefined as never)}
}
