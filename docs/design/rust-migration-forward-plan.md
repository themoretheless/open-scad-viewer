# Forward plan: B-rep/NURBS completion and full Rust migration

Date: 2026-09-15. Objective: **«цель B-rep завершен полностью, перенести все в rust»**
(NURBS and B-rep fully closed, all logic in Rust).

This plan is the forward-looking companion to
[brep-completion-status.md](brep-completion-status.md), which holds the
authoritative per-slice evidence trail and the requirements table. Nothing in
this plan is completion evidence; completion claims live only in the status
document and its linked qualification records.

## Where we are (2026-09-15, evidence-backed)

Done and evidenced (scoped, none of it a full-completion certificate):

- **Native SemanticProgram admission chain** (requirement row 2): envelope
  object, source descriptor + attestation (sha256, UTF-8/UTF-16 lengths,
  surrogate-safe spans), core literals, capability closure (exact Rust port of
  `deriveSemanticCapabilityClosure`), identity digests
  (`opv1:`/`ambv1:`/`occv1:`/`entity:v2:` with byte-exact canonical JSON and
  ECMAScript number formatting), provenance/tessellationIntents/diagnostics/
  diagnosticTemplates, occurrence-production replay for the brep-1 path, and
  the execution plan. All admission runs natively before any graph node
  executes; the legacy per-node TS executor (`brepSemanticBackend.ts`) is
  deleted; the application scene runs on the native graph executor.
- **Host-geometry migration slices** (top of the status document): scene
  flatten, body transforms/arrangement/patterns, joints, mirror, selection
  transforms, editor split, mesh planes (push/pull, chamfer, fillet, shell),
  surface texture, lattice preparation + decimation + lightening + print
  settings, display-buffer preparation — all in Rust behind transport-only
  TS adapters.
- **Intersection matrix (row 8a, numerical foundation)**: curve/plane,
  curve/finite-segment, general curve/curve, curve/ruled-surface (incl. seam
  unification and overlap parameter correspondence), affine surface/surface,
  ruled sections with trace conversion. Newton terminal confirmation with
  one-sided C0-knot jets and half-open boundary ownership.
- **Analytic SS family COMPLETE for canonical primitives**: all 15 pairs of
  {plane, sphere, cylinder, cone, torus}² in axial/resolvable configurations,
  with exact rational curves (circles/lines/conics) and two-sided UV lifts.
  Strict structural recognizers; non-canonical or quartic configurations
  refuse honestly; tangency is always an explicit unresolved band; no
  numerical fallback anywhere in the family.

## Forward plan, in dependency order

### A. Curved Booleans (row 9c) — the main line

A1. **First curved Boolean: sphere OP sphere** (union/intersect/subtract).
    Consume `intersect_sphere_sphere`; split both spheres' faces along the
    exact UV lift circles; one shared section edge with oriented coedges and
    per-face pcurves; outward-interval material classification; spherical-cap
    volume/area oracles; honest refusal for tangency/unresolved; empty-algebra
    and containment cases per existing conventions (cavity policy decided and
    documented). Route through the native semantic boolean admission so a real
    lowered OpenSCAD program executes it end-to-end.
A2. **Generalize the imprint/split pipeline** extracted from A1: surface
    imprint of an exact UV curve loop on a canonical patch, face splitting,
    trimmed-face assembly. Each new pair family (sphere/cylinder, plane/*
    sections, cylinder/cylinder lines, cone sections) joins the Boolean
    matrix only through this shared pipeline — no per-pair assembly shortcuts.
A3. **UV arrangements (row 8b)**: multiple section curves per face, DCEL/
    arrangement in the UV domain, holes, periodic lifts, pole rules, robust
    cell/material classification with independent coverage and non-overlap
    evidence (including missed-branch mutations).
A4. **Global solid audit**: connected material components, orientation audit,
    watertightness at shared edges, empty/multiple-output regularization —
    required before any "general curved Boolean" claim.

### B. Intersection completeness (rows 8a/8b/R1)

B1. **Tangency certification**: today every tangent contact is an unresolved
    band. Add a certified tangent-contact analysis (resultant/GRöbner-free
    discriminant classification on the exact rational representations) so
    provable tangencies resolve to points/curves with explicit multiplicity
    evidence instead of bands.
B2. **Non-axial quartic configurations** (sphere/cylinder off-axis,
    cylinder/cylinder non-parallel, cone/cone non-coaxial, oblique plane/torus
    incl. Villarceau circles): each is a bounded analytic extension of the
    existing recognizers; implement per pair with exact quartic-root
    classification or honest continued refusal.
B3. **General CS** (curve vs arbitrary supported NURBS surface): lift the
    (t,u,v) system from curve/ruled-surface to tensor patches.
B4. **General NURBS SS** (R1/G6): research spike only after B1–B3; a fitted
    NURBS intersection curve may become a topology boundary only with
    two-sided Hausdorff + parameter-correspondence + tubular/isotopy
    certificates (kernel design §9.1).

### C. Foundations and operations above the Boolean line

C1. **Canonicalization, exact sewing, explicit healing (row 10)**: boundary
    correspondence, orientation repair plans, gap/duplicate handling,
    displacement/error ledger, idempotence, cancellation rollback. Post-hoc
    mesh welding never counts.
C2. **Certified shell-aware tessellation (row 11)**: lifted UV constraints and
    separate certified coverage/incidence/deviation evidence for all
    supported surfaces (the shared-edge sample registry is the substrate).
C3. **Later features**: analytic fillet/chamfer chains and corners, general
    shell/offset with deviation evidence, loft/sweep SectionMatch/FrameLaw,
    transactional direct face edits and surface replacement.
C4. **Persistent naming (N1)**: roles/anchors, parameter rebuilds,
    generated/deleted lineage, symmetric ambiguity, selection transfer without
    nearest-face guesses.

### D. Production surface and qualification (rows 1, 3, 4, 12, 13, 14)

D1. **Browser/MCP activation (row 13)**: permanent B-rep provider with
    admission/queue/quotas/watchdog/restart, immutable executable manifest,
    same-program build/export parity; crash/hang/OOM/spoof/rollback drills.
D2. **Execution hardening (row 12)**: checked leases/chunks/epochs, full
    fault/OOM/cancel behavior, GeometrySceneV2 migration, snapshot/export
    leases, native/WASM topology/status/evidence parity.
D3. **Reproducibility (row 4)**: artifact reproducibility, production
    dependency/provenance evidence, bounded ABI allocation, architecture
    tests.
D4. **G0 closure (row 3)**: the declared clean-run matrix on the frozen
    corpus; no rewriting frozen oracle hashes to make results pass.
D5. **Full qualification and release (row 14)**: freeze and run the actual
    supported matrix with independent oracles/certificate mutations, exact
    toolchain/corpus/artifact hashes, resource/performance/security budgets,
    expected refusals, reproducible release/rollback evidence.
D6. **Exchange (row Exchange)**: canonical versioned snapshots,
    corruption/forward-version/limit handling, then STEP/IGES B-rep exchange
    with roundtrip geometry/topology evidence.

### E. Remaining host-side TypeScript (migration completeness)

E1. Per-body `lighten` orchestration loop in `cadWorkbench.ts` and the panel
    volume-reduction percentage (presentation; decide if in scope).
E2. `directModeling.ts` document parsing/history remnants; periodic
    re-inventory of `src/` against the objective (every audit round lists what
    remains, with file paths).
E3. Manifold backend (`executeSemanticProgram` generic path) — decide
    keep-vs-retire explicitly.

## Working agreements (unchanged)

- Every slice: native implementation → native tests (debug AND release) →
  bridge/TS transport-only plumbing → integration tests through the packed
  WASM → both typechecks → `npm run build` with 68 verified dist artifacts →
  qualification JSON with real sha256 pins and `fullGoalComplete: false`
  unless proven → dense status-doc entry with honest limitations.
- Refusal over guesswork; no tolerance merging; unresolved stays unresolved;
  reports never permit topology changes; no mesh fallback satisfies a B-rep
  requirement; frozen oracle hashes are never rewritten to force a pass.
- Budget changes in `scripts/verify-dist.mjs` require a measured justification
  recorded in the evidence.
