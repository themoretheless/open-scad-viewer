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
    qualificationPlan: 'docs/qualification/plans/planar-csg-1.json',
  }),
  Object.freeze({
    id: 'numeric-evidence-curved-brep/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Finite context-bound positional, root, tangent/normal, correspondence, and topology-preservation evidence',
    qualificationPlan: 'docs/qualification/plans/numeric-evidence-curved-brep-1.json',
  }),
  Object.freeze({
    id: 'boundary-correspondence/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Exact authored rational-curve identity, parameter, orientation, ownership, and context correspondence',
    qualificationPlan: 'docs/qualification/plans/boundary-correspondence-1.json',
  }),
  Object.freeze({
    id: 'exact-sew/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Bounded exact sew from complete correspondence proofs; endpoint-only matching remains refused',
    qualificationPlan: 'docs/qualification/plans/exact-sew-1.json',
  }),
  Object.freeze({
    id: 'global-solid-audit/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Finite whole-model shell ownership, orientation, cavity, separation, sew, and self-intersection audit',
    qualificationPlan: 'docs/qualification/plans/global-solid-audit-1.json',
  }),
  Object.freeze({
    id: 'persistent-naming/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Opaque 128-bit topology identity and validated persist/split/merge ChangeSet lineage',
    qualificationPlan: 'docs/qualification/plans/persistent-naming-1.json',
  }),
  Object.freeze({
    id: 'authorized-heal-gap-le1/1',
    maturity: 'Unavailable' as const,
    permitsTopologyChange: false,
    notes: 'Native zero-displacement transaction/refusal skeleton only; positive-gap bridge and product evidence is absent',
    qualificationPlan: 'docs/qualification/plans/authorized-heal-gap-le1-1.json',
  }),
  Object.freeze({
    id: 'analytic-constructors/1',
    maturity: 'ResearchOnly' as const,
    permitsTopologyChange: false,
    notes: 'Constructor corpus is implemented; standalone schema-strict qualification record remains open',
    qualificationPlan: 'docs/qualification/brep-full-closed-matrix-v1.json',
  }),
  Object.freeze({
    id: 'analytic-boolean/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes:
      'Frozen analytic matrix: exact cylinder wall + sphere imprint; disjoint tube/frustum and cylinder/sphere; non-empty mixed and cone/torus refuse; no prism/mesh/Manifold authorship',
    qualificationPlan: 'docs/qualification/plans/analytic-boolean-1.json',
  }),
  Object.freeze({
    id: 'intersection-queries/1',
    maturity: 'ResearchOnly' as const,
    permitsTopologyChange: false,
    notes: 'Affine and listed analytic SS are implemented; standalone qualification record remains open',
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
    id: 'nurbs-boolean-bezier-le3/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Strict partial contact between affine-planar Bezier profile prisms; union/difference/intersection only; no containment, tangency, healing, or fallback',
    qualificationPlan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-2.json',
  }),
  Object.freeze({
    id: 'analytic-fillet/1',
    maturity: 'ResearchOnly' as const,
    permitsTopologyChange: false,
    notes: 'AF-01 single/vertical-corner walking slice; multi-edge transition qualification remains open',
    qualificationPlan: 'docs/qualification/analytic-fillet-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'analytic-multi-edge-fillet/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'One or more vertical edges of an audited axis-aligned cuboid with exact constant-radius arcs',
    qualificationPlan: 'docs/qualification/plans/analytic-multi-edge-fillet-1.json',
  }),
  Object.freeze({
    id: 'exact-parallel-frame-sweep/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Straight collinear translation with fixed/parallel RMF frame; bent paths explicitly refused',
    qualificationPlan: 'docs/qualification/plans/exact-parallel-frame-sweep-1.json',
  }),
  Object.freeze({
    id: 'analytic-chamfer/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: true,
    notes: 'AF-01-like cuboid vertical edge chamfer; mesh bevel must not be relabeled',
    qualificationPlan: 'docs/qualification/plans/analytic-chamfer-1.json',
  }),
  Object.freeze({
    id: 'analytic-shell/1',
    maturity: 'ResearchOnly' as const,
    permitsTopologyChange: false,
    notes: 'AS-01 inward cuboid/cylinder walking slice; general offset and openings remain open',
    qualificationPlan: 'docs/qualification/analytic-shell-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'analytic-solid-loft/1',
    maturity: 'ResearchOnly' as const,
    permitsTopologyChange: false,
    notes: 'ASL-01 axis-aligned rectangular SectionMatch; bent FrameLaw sweep remains typed-refuse',
    qualificationPlan: 'docs/qualification/analytic-solid-loft-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'step-interchange/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes: 'AP214/AP242 graph-only + product seam: CIRCLE rings, cuboid 12-edge, tube FACE_BOUND, apex/oriented tube; cadAnalyticStep beside faceted cadStep; OSCAD_SOLID/AABB/silent-height removed',
    qualificationPlan: 'docs/qualification/plans/step-interchange-1.json',
  }),
  Object.freeze({
    id: 'step-interchange/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Finite AP214/AP242 successor with explicit SI context, rigid placement, multiple bounded bodies, one cavity per body, and opaque identity preservation/loss reporting',
    qualificationPlan: 'docs/qualification/plans/step-interchange-2.json',
  }),
  Object.freeze({
    id: 'certified-brep-tessellation/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Finite planar exact-profile and rational-cylinder tessellation with two-sided deviation and shared-edge incidence certificates',
    qualificationPlan: 'docs/qualification/plans/certified-brep-tessellation-1.json',
  }),
  Object.freeze({
    id: 'certified-mass-properties/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Finite exact box, cylinder, tube, and planar-profile prism area, volume, centroid, and inertia enclosures',
    qualificationPlan: 'docs/qualification/plans/certified-mass-properties-1.json',
  }),
  Object.freeze({
    id: 'nurbs-step-bicubic-face/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes:
      'Freeform STEP: single untrimmed bicubic open face (B_SPLINE + OPEN_SHELL); cadNurbsStep peer; solids/trims/rational refuse',
    qualificationPlan: 'docs/qualification/plans/nurbs-step-bicubic-face-1.json',
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
    maturity: 'ResearchOnly' as const,
    permitsTopologyChange: false,
    notes: 'Scoped IGES entity walking slice; arbitrary linked solid topology is not qualified',
    qualificationPlan: 'docs/qualification/plans/iges-interchange-1.json',
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
