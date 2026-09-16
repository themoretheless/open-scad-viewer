/** Honest B-rep capability maturity for WASM / MCP / SemanticProgram seams. */

export type BrepCapabilityMaturity =
  | 'Unavailable'
  | 'ResearchOnly'
  | 'NumericUncertified'
  | 'AnalyticComplete'
  | 'Qualified'

export interface BrepCapabilityDescriptor {
  readonly id: string
  readonly maturity: BrepCapabilityMaturity
  readonly permitsTopologyChange: boolean
  readonly notes: string
  readonly qualificationPlan?: string
}

/**
 * Full-matrix registry (`brep-full-closed-matrix-v1` / G8-full).
 * Maturity bumps only after walking-slice evidence is green.
 */
export const BREP_CAPABILITY_MATRIX: readonly BrepCapabilityDescriptor[] = Object.freeze([
  Object.freeze({
    id: 'planar-csg/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Orthogonal / convex / bounded planar CSG',
    qualificationPlan: 'docs/qualification/brep-native-workbench-boolean-v1.json',
  }),
  Object.freeze({
    id: 'analytic-constructors/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes: 'Cylinder/sphere/torus/cone/revolve constructors with identity matrix',
    qualificationPlan: 'docs/qualification/brep-full-closed-matrix-v1.json',
  }),
  Object.freeze({
    id: 'analytic-boolean/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes:
      'Frozen analytic matrix: exact cylinder wall + sphere imprint; disjoint tube/frustum and cylinder/sphere; non-empty mixed and cone/torus refuse; no prism/mesh/Manifold authorship',
    qualificationPlan: 'docs/qualification/plans/analytic-boolean-1-g8.json',
  }),
  Object.freeze({
    id: 'intersection-queries/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes: 'Affine + listed analytic SS may be Complete; general NURBS Incomplete until G6 expand',
    qualificationPlan: 'docs/qualification/brep-full-closed-matrix-v1.json',
  }),
  Object.freeze({
    id: 'nurbs-ss-bezier-le3/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes:
      'G6 Phase B: non-rational Bezier deg≤3×≤3 elevated to bicubic; Complete-empty/line/curve when certified; supersedes transverse-bicubic/2',
    qualificationPlan: 'docs/qualification/plans/nurbs-ss-bezier-le3-1.json',
  }),
  Object.freeze({
    id: 'nurbs-boolean-bezier-le3/1',
    maturity: 'Unavailable' as const,
    permitsTopologyChange: false,
    notes:
      'Frozen after audit: SS queries remain scoped, but contacting-solid face-split topology authorship is not qualified',
    qualificationPlan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-1.json',
  }),
  Object.freeze({
    id: 'analytic-fillet/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: true,
    notes: 'AF-01 + multi-edge chain remapping by durable XY; curved edges refuse',
    qualificationPlan: 'docs/qualification/analytic-fillet-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'analytic-chamfer/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: true,
    notes: 'AF-01-like cuboid vertical edge chamfer; mesh bevel must not be relabeled',
    qualificationPlan: 'docs/qualification/plans/analytic-chamfer-1-g8.json',
  }),
  Object.freeze({
    id: 'analytic-shell/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: true,
    notes: 'AS-01 closed cuboid offset + cylinder wall offset',
    qualificationPlan: 'docs/qualification/analytic-shell-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'analytic-solid-loft/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: true,
    notes: 'ASL-01 ruled SectionMatch; FrameLaw Frenet/RMF/fixed production sweep',
    qualificationPlan: 'docs/qualification/analytic-solid-loft-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'step-interchange/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes: 'AP214/AP242 graph-only + product seam: CIRCLE rings, cuboid 12-edge, tube FACE_BOUND, apex/oriented tube; cadAnalyticStep beside faceted cadStep; OSCAD_SOLID/AABB/silent-height removed',
    qualificationPlan: 'docs/design/qualification/step-interchange-1.md',
  }),
  Object.freeze({
    id: 'nurbs-step-bicubic-face/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes:
      'Freeform STEP: single untrimmed bicubic open face (B_SPLINE + OPEN_SHELL); cadNurbsStep peer; solids/trims/rational refuse',
    qualificationPlan: 'docs/design/qualification/nurbs-step-bicubic-face-1.md',
  }),
  Object.freeze({
    id: 'nurbs-step-trimmed-bicubic/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes:
      'Freeform STEP: trimmed bicubic open face (FACE_OUTER_BOUND + FACE_BOUND + OPEN_SHELL); solids refuse',
    qualificationPlan: 'docs/qualification/plans/nurbs-step-trimmed-bicubic-1.json',
  }),
  Object.freeze({
    id: 'nurbs-step-solid/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes:
      'Freeform STEP solid: elevated bicubic cuboid / bump / cavity via MANIFOLD_SOLID_BREP + B_SPLINE; no OSCAD_SOLID/AABB',
    qualificationPlan: 'docs/qualification/plans/nurbs-step-solid-1.json',
  }),
  Object.freeze({
    id: 'iges-interchange/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes: 'IGES 110/116/128/190 plus solid 186/514/510/144; STL/OBJ refuse',
    qualificationPlan: 'docs/qualification/plans/iges-interchange-1-g8.json',
  }),
])

export function brepCapability(id: string): BrepCapabilityDescriptor | undefined {
  return BREP_CAPABILITY_MATRIX.find(entry => entry.id === id)
}

export function assertBrepCapabilityAllowsTopology(id: string): void {
  const entry = brepCapability(id)
  if (!entry) throw new Error(`Unknown B-rep capability: ${id}`)
  if (entry.maturity === 'Unavailable' || entry.maturity === 'ResearchOnly') {
    throw new Error(`B-rep capability ${id} is ${entry.maturity}; refuse topology change`)
  }
  if (!entry.permitsTopologyChange) {
    throw new Error(`B-rep capability ${id} does not permit topology change`)
  }
}

/** Rollback drill: revoked / stale LKG must not fall through to Manifold. */
export function assertNoManifoldFallbackOnBrepFailure(capabilityId: string): void {
  const entry = brepCapability(capabilityId)
  if (!entry) throw new Error(`Unknown B-rep capability: ${capabilityId}`)
  if (entry.maturity === 'Unavailable') {
    throw new Error(`B-rep capability ${capabilityId} unavailable; refuse Manifold cross-route`)
  }
}
