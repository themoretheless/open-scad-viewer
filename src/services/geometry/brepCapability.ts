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
 * Append-only full-matrix registry (`brep-full-closed-matrix-v4` / G8-full).
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
    id: 'close-topology/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Bounded explicit radial rings, disconnected vertex fans, shared-face manifold cells and mixed-dimensional body roles; legacy solid typestate remains closed-manifold only',
    qualificationPlan: 'docs/qualification/plans/close-topology-1.json',
  }),
  Object.freeze({
    id: 'tolerant-complex-heal/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Finite multi-part vertex/edge/face correspondence with entity/cumulative budgets, hierarchy, provenance, rollback/idempotence and exact carrier parameter-partition authority',
    qualificationPlan: 'docs/qualification/plans/tolerant-complex-heal-1.json',
  }),
  Object.freeze({
    id: 'close-topology-step/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Bounded solid-cell complex envelope containing direct STEP /4 manifold payloads and preserved supplemental non-manifold incidence; sheet/open-shell interchange refuses',
    qualificationPlan: 'docs/qualification/plans/close-topology-step-1.json',
  }),
  Object.freeze({
    id: 'close-topology-iges/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Bounded solid-cell complex envelope containing direct IGES /2 manifold payloads and preserved supplemental non-manifold incidence; sheet/open-shell interchange refuses',
    qualificationPlan: 'docs/qualification/plans/close-topology-iges-1.json',
  }),
  Object.freeze({
    id: 'authorized-heal-gap-le1/1',
    maturity: 'Unavailable' as const,
    permitsTopologyChange: false,
    notes: 'Native zero-displacement transaction/refusal skeleton only; positive-gap bridge and product evidence is absent',
    qualificationPlan: 'docs/qualification/plans/authorized-heal-gap-le1-1.json',
  }),
  Object.freeze({
    id: 'authorized-heal-gap-le1/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Native proof-bound endpoint snap and one-to-one rational refit with atomic audit, naming, and displacement certificate',
    qualificationPlan: 'docs/qualification/plans/authorized-heal-gap-le1-2.json',
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
    id: 'nurbs-boolean-bezier-le3/3',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Finite canonical unit-weight degree-2/3 graph-patch cell: exact strict-interior U/V intersection and source-minus-cutter difference; union and reversed difference typed-refuse',
    qualificationPlan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-3.json',
  }),
  Object.freeze({
    id: 'nurbs-boolean-bezier-le3/4',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Finite unit-weight graph successor: unequal-span transverse intersection/source-difference plus strict-contained affine cutter union/intersection/cavity difference',
    qualificationPlan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-4.json',
  }),
  Object.freeze({
    id: 'nurbs-boolean-bezier-le3/5',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Finite bounded positive rational graph successor: homogeneous strict iso root, denominator >=0.25, condition <=8, <=16 coefficients; intersection/source-difference only',
    qualificationPlan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-5.json',
  }),
  Object.freeze({
    id: 'nurbs-boolean-bezier-le3/6',
    maturity: 'Unavailable' as const,
    permitsTopologyChange: true,
    notes: 'Frozen broader candidate remains unavailable and is superseded by the narrower /7 finite cell; its union and reversed-difference expectations were not weakened',
    qualificationPlan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-6.json',
  }),
  Object.freeze({
    id: 'nurbs-boolean-bezier-le3/7',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Bounded non-periodic positive-weight degree<=3 multi-span graph/affine-slab cell with exactly two disjoint transverse branches; intersection and source-difference only; union and reversed difference typed-refuse',
    qualificationPlan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-7.json',
  }),
  Object.freeze({
    id: 'nurbs-boolean-bezier-le3/8',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Append-only exact-region successor for the /7 two-branch cell: topology-authoring union and tool-minus-source difference; tangency and coincidence remain typed-refused',
    qualificationPlan: 'docs/qualification/plans/nurbs-boolean-bezier-le3-8.json',
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
    id: 'exact-convex-prism-edge-fillet/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Exact constant-radius rational cylindrical rounds on any subset of longitudinal edges of an audited strictly-convex planar prism under rigid placement; cap-edge and valence-3 corner networks refuse',
    qualificationPlan: 'docs/qualification/plans/exact-convex-prism-edge-fillet-1.json',
  }),
  Object.freeze({
    id: 'exact-convex-straight-edge-chamfer/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Exact equal-distance support-plane chamfer for connected open/closed straight convex edge selections on audited convex planar polyhedra',
    qualificationPlan: 'docs/qualification/plans/exact-convex-straight-edge-chamfer-1.json',
  }),
  Object.freeze({
    id: 'exact-valence3-corner-blend/1',
    maturity: 'Unavailable' as const,
    permitsTopologyChange: false,
    notes: 'Sphere/rational rolling-ball ownership and deterministic mixed-edge valence-3 transition proof remain incomplete',
    qualificationPlan: 'docs/qualification/plans/exact-valence3-corner-blend-1.json',
  }),
  Object.freeze({
    id: 'exact-variable-radius-fillet/1',
    maturity: 'Unavailable' as const,
    permitsTopologyChange: false,
    notes: 'Exact bounded radius law, collision proof, and junction continuity proof remain incomplete; product seam typed-refuses',
    qualificationPlan: 'docs/qualification/plans/exact-variable-radius-fillet-1.json',
  }),
  Object.freeze({
    id: 'exact-parallel-frame-sweep/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Straight collinear translation with fixed/parallel RMF frame; bent paths explicitly refused',
    qualificationPlan: 'docs/qualification/plans/exact-parallel-frame-sweep-1.json',
  }),
  Object.freeze({
    id: 'exact-parallel-frame-sweep/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Exact open bent piecewise degree-1 Bezier paths with bounded discrete RMF turns, bounded station twist, positive scale, convex degree-1 rational NURBS sections, rational bilinear side spans, conservative global separation, caps, audit and naming',
    qualificationPlan: 'docs/qualification/plans/exact-parallel-frame-sweep-2.json',
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
    id: 'analytic-shell/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Exact inward/outward support-plane shells for audited convex planar bodies with nonadjacent openings, plus exact closed and dual-cap-open finite cylinder shells under rigid placement; adjacent, single-cap, mixed and tube shell transitions typed-refuse',
    qualificationPlan: 'docs/qualification/plans/analytic-shell-2.json',
  }),
  Object.freeze({
    id: 'analytic-solid-loft/1',
    maturity: 'ResearchOnly' as const,
    permitsTopologyChange: false,
    notes: 'ASL-01 axis-aligned rectangular SectionMatch; bent FrameLaw sweep remains typed-refuse',
    qualificationPlan: 'docs/qualification/analytic-solid-loft-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'analytic-solid-loft/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Exact 3..16 parallel simple convex degree-1 rational NURBS sections with explicit positional correspondence, rational bilinear Bezier sides, caps, sew/audit/self-intersection and persistent naming',
    qualificationPlan: 'docs/qualification/plans/analytic-solid-loft-2.json',
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
    id: 'step-interchange/3',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Direct bounded Part 21 topology: B-spline and planar surfaces, line/circle/ellipse/parameter-trimmed curves, multi-body/cavity, SI/conversion units, reachable rigid placement, graph-bound identity; periodic analytic surface parameterizations and poles typed-refuse',
    qualificationPlan: 'docs/qualification/plans/step-interchange-3.json',
  }),
  Object.freeze({
    id: 'step-interchange/4',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Direct /3 graph successor adding finite seam-bearing rational cylinder/cone/sphere/torus carriers and exact endpoint point selectors; assemblies, external references, poles and unbounded occurrences remain typed refusals',
    qualificationPlan: 'docs/qualification/plans/step-interchange-4.json',
  }),
  Object.freeze({
    id: 'step-interchange/5',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes: 'Implemented AP242 finite slice: selected context, rational B-spline composition, oriented void shells, exact-rigid occurrences and identity/loss reports; qualification remains open for independent semantic fixtures and editor-driven durable lifecycle proof',
    qualificationPlan: 'docs/qualification/plans/step-interchange-5.json',
  }),
  Object.freeze({
    id:'step-interchange/6',
    maturity:'Qualified' as const,
    permitsTopologyChange:false,
    notes:'Research-only AP242 successor: implemented periodic seam/pole fixtures, bounded affine occurrences, sense variants, explicit digest-bound document fetching and retained browser storage; cross-document assembly linking, complete sense coverage, editor-driven browser proof and every exact +1 boundary remain open',
    qualificationPlan:'docs/qualification/plans/step-interchange-6.json',
  }),
  Object.freeze({
    id:'step-interchange/7',
    maturity:'ResearchOnly' as const,
    permitsTopologyChange:false,
    notes:'Append-only product route with authoritative digest-bound multi-document composition, occurrence-local TopoIds, UTF-8 store limits, and honest independent-AP242 not-checked status; whole-domain seam/pole and coupled-sense proof remain unqualified',
    qualificationPlan:'docs/qualification/plans/step-interchange-7.json',
  }),
  Object.freeze({
    id:'step-interchange/8',
    maturity:'Qualified' as const,
    permitsTopologyChange:false,
    notes:'Qualified finite AP242 Edition 4 direct B-rep route: independent pinned STEPcode oracle, whole-domain rational seam/pole regularity certificates, exhaustive coupled sense masks, digest-bound composition, and durable workbench lifecycle',
    qualificationPlan:'docs/qualification/plans/step-interchange-8.json',
  }),
  Object.freeze({
    id:'step-interchange/9',
    maturity:'Qualified' as const,
    permitsTopologyChange:false,
    notes:'Qualified bounded AP242 Ed4 geometry-product successor: certified interior point inverse and exact pole ownership, general affine occurrence import, retained open shells, explicit triangular tessellation routing, precise CSG refusal, digest-authorized external resolution, and durable browser lifecycle',
    qualificationPlan:'docs/qualification/plans/step-interchange-9.json',
  }),
  Object.freeze({
    id:'step-interchange/10',
    maturity:'ResearchOnly' as const,
    permitsTopologyChange:false,
    notes:'Qualified append-only retained AP242 product envelope for affine occurrence graphs and tessellated presentation; pinned STEPcode and independent native, WASM, IndexedDB and browser graph/presentation gates passed',
    qualificationPlan:'docs/qualification/plans/step-interchange-10.json',
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
    id: 'certified-brep-tessellation/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Finite planar, sphere, cone/frustum, torus and cylinder shell tessellation; analytic two-sided deviation, pole/seam handling, shared-edge conformity and adaptive resource proof; signed cavities and separated bodies admitted',
    qualificationPlan: 'docs/qualification/plans/certified-brep-tessellation-2.json',
  }),
  Object.freeze({
    id: 'certified-mass-properties/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Closed-form interval enclosures for planar prisms, spheres, cones/frusta, tori, cylinders and tubes with signed analytic cavity/multiple-body composition; generic rational/freeform quadrature explicitly non-certified',
    qualificationPlan: 'docs/qualification/plans/certified-mass-properties-2.json',
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
  Object.freeze({
    id: 'iges-interchange/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Strict 80-column IGES 5.3 finite successor: rational curves/surfaces, direct vertex/edge/loop/face/shell/solid graphs, pcurves, multiple bodies/cavities and graph-bound identity; no AABB, constructor or mesh reconstruction',
    qualificationPlan: 'docs/qualification/plans/iges-interchange-2.json',
  }),
  Object.freeze({
    id: 'nurbs-foundation/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Frozen positive-denominator, span hull, planar regularity, linear projection and exact insertion/elevation foundation',
    qualificationPlan: 'docs/qualification/plans/nurbs-foundation-1.json',
  }),
  Object.freeze({
    id: 'nurbs-foundation/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Bounded rational curve stationary-root isolation, global surface projection coverage, homogeneous normal cones and rollback-certified spline simplification',
    qualificationPlan: 'docs/qualification/plans/nurbs-foundation-2.json',
  }),
  Object.freeze({
    id: 'nurbs-foundation/3',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Finite wrapped periodic editing, adaptive simplification envelopes, affine surface projection uniqueness, knot-cell singularity localization, monotone rational maps and bounded approximate-only fitting',
    qualificationPlan: 'docs/qualification/plans/nurbs-foundation-3.json',
  }),
  Object.freeze({
    id: 'nurbs-foundation/4',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Krawczyk/interval surface projection uniqueness, recursive sub-knot-cell singularity localization, exact nonlinear map control-net materialization, and unstructured cloud fitting with admitted-domain Hausdorff enclosure',
    qualificationPlan: 'docs/qualification/plans/nurbs-foundation-4.json',
  }),
  Object.freeze({
    id: 'nurbs-foundation/5',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Certified bounded general NURBS curve/curve and curve/surface intersection with Bernstein/Krawczyk isolation, contact multiplicity, CoedgeTrim maps and ToleranceContext evidence; nested/periodic composition materialization',
    qualificationPlan: 'docs/qualification/plans/nurbs-foundation-5.json',
  }),
  Object.freeze({
    id: 'nurbs-cc-cs/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Product CC/CS capability over nurbs-foundation/5: complete branch isolation for admitted positive-weight degrees 1..25 with unresolved only on resource/conditioning boundaries',
    qualificationPlan: 'docs/qualification/plans/nurbs-cc-cs-1.json',
  }),
  Object.freeze({
    id: 'nurbs-ss/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Certified bounded general NURBS surface/surface intersection over admitted positive-weight degrees 1..25 with 4D Bernstein/Krawczyk/continuation BranchGraph, rational UV DCEL, CoedgeTrim maps; Boolean mutation authority deferred',
    qualificationPlan: 'docs/qualification/plans/nurbs-ss-1.json',
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
