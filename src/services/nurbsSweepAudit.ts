import {callNurbsRust} from './geometry/nurbs'
import type {NurbsSurface} from './nurbsSurface'
import type {NurbsCurve} from './nurbsCurve'
// Shared orchestration budget; every individual native predicate remains
// bounded to one million operations.
export const MAX_SWEEP_SEAM_WORK=2000000

export interface SweepExactStripJetAudit {
  certified:boolean
  exactIdentity:boolean
  regularityCertified:boolean
  work:number
  reason:string
}
export interface SweepProjectiveStripJetAudit extends SweepExactStripJetAudit {
  method:'constant-projective-strip-jets'
  certifiedOrder:1|2|null
}
/** Sufficient geometric G1/G2 with constant transverse reparameterization.
 * A refusal does not disprove general G1/G2. Regularity is independently checked. */
export function inspectSweepProjectiveStripJets(reference:NurbsSurface,edited:NurbsSurface,
  referenceBoundary:'uMin'|'uMax'|'vMin'|'vMax',editedBoundary:'uMin'|'uMax'|'vMin'|'vMax',
  order:1|2,normalScale:number,maxWork:number):SweepProjectiveStripJetAudit {
  return callNurbsRust('surface_projective_strip_jets_audit',{reference,edited,referenceBoundary,editedBoundary,order,normalScale,maxWork})
}
/** Sufficient exact C1/C2 for Bezier cross strips with a common along-seam
 * clamped B-spline basis. Internal knots must support the requested smoothness; seam
 * regularity is checked independently. Scale uses normalized cross parameters. */
export function inspectSweepExactStripJets(reference:NurbsSurface,edited:NurbsSurface,
  referenceBoundary:'uMin'|'uMax'|'vMin'|'vMax',editedBoundary:'uMin'|'uMax'|'vMin'|'vMax',
  order:1|2,normalScale:number,maxWork:number):SweepExactStripJetAudit {
  return callNurbsRust('surface_exact_strip_jets_audit',{reference,edited,referenceBoundary,editedBoundary,order,normalScale,maxWork})
}

export interface SweepProfileRegularityAudit {
  spanwiseRegular:boolean
  cells:number
  unresolvedProfiles:number[]
  continuityCertified:false
}
/** Nonzero one-sided tangents over every knot span, with a shared cell budget.
 * This does not certify G1/G2 at authored corners or periodic seams. */
export function inspectSweepProfileRegularity(profiles:NurbsCurve[],maxCells:number):SweepProfileRegularityAudit {
  return callNurbsRust('sweep_profile_regularity_audit',{profiles,maxCells})
}

export interface SweepSeamDeclaration {
  patches: [number, number]
  boundaries: ['uMin'|'uMax'|'vMin'|'vMax', 'uMin'|'uMax'|'vMin'|'vMax']
  order: 1|2
  normalScale: number
  jetTolerance: number
}
export interface SweepExactSeamAudit {
  /** Sufficient exact C1/C2 implies the requested G1/G2 at regular seams. */
  exactG1G2Certified:boolean
  /** Minimum requested order across the certified set; null on any refusal. */
  certifiedOrder:1|2|null
  exactWork:number
  unresolvedSeams:number[]
  seams:SweepExactStripJetAudit[]
}
export interface SweepProjectiveSeamAudit {
  method:'constant-projective-strip-jets'
  exactG1G2Certified:boolean
  certifiedOrder:1|2|null
  exactWork:number
  unresolvedSeams:number[]
  seams:SweepProjectiveStripJetAudit[]
}
/** Certifies the complete explicitly declared seam set with one shared budget.
 * Callers must include closure and all intended joins; no adjacency is inferred. */
export function inspectSweepProjectiveSeams(patches:NurbsSurface[],seams:SweepSeamDeclaration[],maxWork:number,checkAbort?:()=>void):SweepProjectiveSeamAudit {
  checkAbort?.()
  if(!Number.isSafeInteger(maxWork)||maxWork<0||maxWork>MAX_SWEEP_SEAM_WORK)throw new Error('Invalid exact seam work budget')
  for(const seam of seams){
    if(seam.patches.some(index=>!Number.isSafeInteger(index)||index<0||index>=patches.length))throw new Error('Invalid exact seam patch index')
    if(seam.order!==1&&seam.order!==2)throw new Error('Invalid exact seam order')
    if(!Number.isFinite(seam.normalScale)||seam.normalScale<=0)throw new Error('Invalid exact seam normal scale')
    if(seam.boundaries.some(boundary=>!['uMin','uMax','vMin','vMax'].includes(boundary)))throw new Error('Invalid exact seam boundary')
  }
  let exactWork=0
  const reports=seams.map(seam=>{
    checkAbort?.()
    const report=inspectSweepProjectiveStripJets(patches[seam.patches[0]]!,patches[seam.patches[1]]!,seam.boundaries[0],seam.boundaries[1],seam.order,seam.normalScale,Math.min(1000000,maxWork-exactWork))
    if(!Number.isSafeInteger(report.work)||report.work<0||report.work>maxWork-exactWork)throw new Error('Invalid native exact seam work accounting')
    exactWork+=report.work
    return report
  })
  checkAbort?.()
  const unresolvedSeams=reports.flatMap((report,index)=>report.certified?[]:[index])
  const exactG1G2Certified=reports.length>0&&unresolvedSeams.length===0
  return {method:'constant-projective-strip-jets',exactG1G2Certified,
    certifiedOrder:exactG1G2Certified?(seams.some(seam=>seam.order===1)?1:2):null,
    exactWork,unresolvedSeams,seams:reports}
}
/** Whole declared seam set with one shared exact-work budget. An empty set
 * carries no positive smoothness claim. Unsupported bases remain unresolved. */
export function inspectSweepExactSeams(patches:NurbsSurface[],seams:SweepSeamDeclaration[],maxWork:number):SweepExactSeamAudit {
  if(!Number.isSafeInteger(maxWork)||maxWork<0||maxWork>1000000)throw new Error('Invalid exact seam work budget')
  for(const seam of seams){
    if(seam.patches.some(index=>!Number.isSafeInteger(index)||index<0||index>=patches.length))throw new Error('Invalid exact seam patch index')
    if(seam.order!==1&&seam.order!==2)throw new Error('Invalid exact seam order')
    if(!Number.isFinite(seam.normalScale)||seam.normalScale<=0)throw new Error('Invalid exact seam normal scale')
    if(seam.boundaries.some(boundary=>!['uMin','uMax','vMin','vMax'].includes(boundary)))throw new Error('Invalid exact seam boundary')
  }
  let exactWork=0
  const reports=seams.map(seam=>{
    const report=inspectSweepExactStripJets(patches[seam.patches[0]]!,patches[seam.patches[1]]!,seam.boundaries[0],seam.boundaries[1],seam.order,seam.normalScale,maxWork-exactWork)
    if(!Number.isSafeInteger(report.work)||report.work<0||report.work>maxWork-exactWork)throw new Error('Invalid native exact seam work accounting')
    exactWork+=report.work
    return report
  })
  const unresolvedSeams=reports.flatMap((report,index)=>report.certified?[]:[index])
  const exactG1G2Certified=reports.length>0&&unresolvedSeams.length===0
  const certifiedOrder=exactG1G2Certified?(seams.some(seam=>seam.order===1)?1:2):null
  return {exactG1G2Certified,certifiedOrder,exactWork,unresolvedSeams,seams:reports}
}
export interface SweepSeamAudit {
  allWithinJetBudget: boolean
  inspectedSeams: number
  exactG1G2Certified: false
  seams: {
    withinJetBudget: boolean
    regularityCertified: boolean
    tangentialSmoothnessCertified: boolean
    errorUpper: number|null
    reason: string
  }[]
}
/** Read-only bounded jet qualification; a tolerance is not exact G1/G2. */
export function inspectSweepSeams(patches: NurbsSurface[], seams: SweepSeamDeclaration[], maxSeams = seams.length): SweepSeamAudit {
  return callNurbsRust('sweep_seam_audit', {patches, seams, maxSeams})
}

export interface SweepWallAuditOptions {
  sharedBoundaries: [number, number][]
  clearance: number
  distanceTolerance: number
  maxInjectivityCells: number
  maxPairs: number
  maxPairCells: number
}
export interface SweepWallAudit {
  chartsAndPairsCertified: boolean
  /** Caps and shell containment are separate, still unproved obligations. */
  globalEmbeddingCertified: false
  injectivityCells: number
  unresolvedCharts: number[]
  charts: {
    certified: boolean
    cells: number
    projection: [[number, number], [number, number]]|null
    reason: string|null
  }[]
  declaredBoundariesC0: boolean
  c0Boundaries: [number, number][]
  unresolvedBoundaries: [number, number][]
  pairs: {
    allPairsSeparated: boolean
    allPairsCompatible: boolean
    boundaryOnlyPairs: [number, number][]
    separatedPairs: number
    pairs: number
    cells: number
    unresolved: {patches: [number, number]; reason: string}[]
  }
}
/** Read-only whole-chart/pair evidence with three independent explicit budgets. */
export function inspectSweepWalls(walls: NurbsSurface[], options: SweepWallAuditOptions): SweepWallAudit {
  return callNurbsRust('sweep_wall_audit', {walls, ...options})
}

export interface SweepContourAudit {
  capDomainCertified: boolean
  globalEmbeddingCertified: false
  capGeometryCertified: false
  planeAxis: number|null
  pairs: number
  cells: number
  reason: string|null
}
/** Continuous planar domain ownership; loop zero is outer, remaining loops holes. */
export function inspectSweepContours(
 loops: import('./nurbsCurve').NurbsCurve[][],
 options: {tolerance:number;maxPairs:number;maxCells:number},
):SweepContourAudit {
 return callNurbsRust('sweep_contour_audit',{loops,...options})
}

export interface SweepCapBoundaryAudit {
 withinBudget:boolean
 errorUpper:number|null
 products:number
 reason:string|null
 capGeometryCertified:false
 globalEmbeddingCertified:false
}
/** Whole-curve deviation for the stored bilinear cap composed with a UV curve. */
export function inspectSweepCapBoundary(
 surface:NurbsSurface,world:import('./nurbsCurve').NurbsCurve,uv:import('./nurbsCurve').NurbsCurve,
 options:{tolerance:number;maxProducts:number},
):SweepCapBoundaryAudit {
 return callNurbsRust('sweep_cap_boundary_audit',{surface,world,uv,...options})
}

export interface SweepCapWallAudit {
 allWallInteriorsExcluded:boolean
 planeAxis:number|null
 inspectedWalls:number
 separatedWalls:number[]
 boundaryRestrictedWalls:number[]
 unresolvedWalls:number[]
 reason:string|null
 boundaryOwnershipCertified:false
 globalEmbeddingCertified:false
}
/** Interior exclusion alone does not establish allowed boundary ownership. */
export function inspectSweepCapWalls(
 cap:NurbsSurface,walls:NurbsSurface[],boundaries:('uMin'|'uMax'|'vMin'|'vMax'|null)[],maxWalls:number,
):SweepCapWallAudit {
 return callNurbsRust('sweep_cap_wall_audit',{cap,walls,boundaries,maxWalls})
}

export interface SweepCoedgeAgreementAudit {
 withinTolerance:boolean
 status:'within-tolerance'|'mismatch'|'unresolved'
 cells:number
 witness:number|null
 witnessDistance:[number,number]|null
 exactIdentityCertified:false
 globalEmbeddingCertified:false
}
/** Native whole-domain composition bound; exhausted cells leave it unresolved. */
export function inspectSweepCoedgeAgreement(
 surface:NurbsSurface,world:import('./nurbsCurve').NurbsCurve,uv:import('./nurbsCurve').NurbsCurve,
 options:{reversed:boolean;tolerance:number;maxCells:number},
):SweepCoedgeAgreementAudit {
 return callNurbsRust('sweep_coedge_agreement_audit',{surface,world,uv,...options})
}

export interface SweepCoedgeExactAudit {
 status:'equal'|'different'|'unresolved'|'unsupported'
 work:number
 exactIdentityCertified:boolean
 globalEmbeddingCertified:false
}
/** Exact polynomial identity of the stored binary64 geometry, without tolerance. */
export function inspectSweepCoedgeExact(
 surface:NurbsSurface,world:import('./nurbsCurve').NurbsCurve,uv:import('./nurbsCurve').NurbsCurve,
 options:{reversed:boolean;maxWork:number},
):SweepCoedgeExactAudit {
 return callNurbsRust('sweep_coedge_exact_audit',{surface,world,uv,...options})
}

export interface SweepBoundaryCoverageAudit {
 wholeBoundaryCovered:boolean
 injectivityCertified:false
 boundaryOwnershipCertified:false
 globalEmbeddingCertified:false
}
/** Complete boundary image; backtracking and trim ownership are separate. */
export function inspectSweepBoundaryCoverage(surface:NurbsSurface,uv:import('./nurbsCurve').NurbsCurve,boundary:'uMin'|'uMax'|'vMin'|'vMax'):SweepBoundaryCoverageAudit {
 return callNurbsRust('sweep_boundary_coverage_audit',{surface,uv,boundary})
}

export interface SurfaceLinearInjectivityAudit {
 certified:boolean
 cells:number
 projection:[[number,number,number],[number,number,number]]
 reason:string|null
 globalEmbeddingCertified:false
}
/** One fixed integer projection, certified over every original knot rectangle. */
export function inspectSurfaceLinearInjectivity(surface:NurbsSurface,maxCells:number,projection?:[[number,number,number],[number,number,number]]):SurfaceLinearInjectivityAudit {
 return callNurbsRust('surface_linear_injectivity_audit',{surface,maxCells,...(projection?{projection}:{})})
}
