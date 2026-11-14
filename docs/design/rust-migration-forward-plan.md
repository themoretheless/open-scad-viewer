# Forward plan: B-rep/NURBS completion and full Rust migration

Date: 2026-09-15. Objective: **«цель B-rep завершен полностью, перенести все в rust»**
(NURBS and B-rep fully closed, all logic in Rust).

This plan is the forward-looking companion to
[brep-completion-status.md](brep-completion-status.md), which holds the
authoritative per-slice evidence trail and the requirements table. Nothing in
this plan is completion evidence; completion claims live only in the status
document and its linked qualification records.

Updated 2026-11-13: removed 10 completed items (A1–A4, B3, C3, C4, D1, D5,
D6) and the "Where we are" list of done work; B2, B4, C1, C2 and D2 were
narrowed to their open parts. Evidence for removed items: native
sphere/sphere Boolean (`crates/brep-core/src/boolean/sphere_boolean.rs`), shared
imprint pipeline and UV arrangements (`crates/brep-core/src/boolean/{imprint_pipeline,uv_arrangement}`),
global solid audit (`crates/brep-core/src/body_analysis/solid_audit.rs`), and the
Closed rows 8b, 9c, 13, 14, N1, R1/G6, later features and exchange in
[brep-completion-status.md](brep-completion-status.md). Freeform×freeform
Boolean and off-axis quartic contacts are also tracked in
[nurbs-brep-bvh-roadmap-status.md](nurbs-brep-bvh-roadmap-status.md).

## Remaining work, in dependency order

### B. Intersection completeness (rows 8a/8b/R1)

B1. **Tangency certification**: tangent contacts are still reported as
    unresolved (`UnresolvedReason::TangencyOrMultipleRoot` in
    `crates/nurbs-intersect/src/queries/curve_plane.rs`). Add a certified
    tangent-contact analysis (resultant/GRöbner-free discriminant
    classification on the exact rational representations) so provable
    tangencies resolve to points/curves with explicit multiplicity evidence
    instead of bands.
    Status (curve/plane slice done): `queries/curve_plane_tangency.rs`
    re-decides curve/plane refusals with exact floating-point expansions of
    the span's homogeneous Bernstein coefficients (Rolle bound on the first
    one-signed derivative plus endpoint multiplicity and sign parity).
    Tangencies at binary64-dyadic parameters return points with
    `Contact::Tangent { multiplicity }`, near tangency returns 0 or 2 proved
    simple roots, in-plane spans are overlaps. Open: tangency at non-dyadic
    parameters (still `TangencyOrMultipleRoot`), curve/surface beyond the
    plane reduction, surface/surface tangency and degenerate branch tracing.
B2. **Non-axial quartic configurations** (sphere/cylinder off-axis,
    cylinder/cylinder non-parallel, cone/cone non-coaxial, oblique plane/torus
    incl. Villarceau circles, sphere/torus off-axis contact): coaxial pairs and
    provable misses are done; true off-axis contact still refuses with
    `UnsupportedSurface`. Implement per pair with exact quartic-root
    classification or honest continued refusal.
B4. **Freeform×freeform Boolean**: general NURBS SS is qualified only for the
    finite `nurbs-ss/1` and `nurbs-boolean/1` (graph × affine cutter) cells;
    freeform×freeform Boolean remains ExplicitRefuse
    (`freeform-x-freeform-boolean`).

### C. Foundations and operations above the Boolean line

C1. **Explicit healing (row 10)**: boundary correspondence, exact sew and
    global solid audit are qualified; `authorized-heal-gap-le1/1` remains
    Unavailable. Positive-gap bridging with displacement/error ledger,
    idempotence and cancellation rollback is still missing. Post-hoc mesh
    welding never counts.
C2. **Certified tessellation beyond the closed cells (row 11)**: analytic and
    equal-weight Bezier ≤3 freeform tessellation are certified; varying-weight,
    loft and bent-sweep surfaces remain ExplicitRefuse.

### D. Production surface and qualification (rows 3, 4, 12)

D2. **Execution hardening (row 12)**: the closed-matrix provider and
    lease/cancel/LKG drills are closed; broader OOM/chunk behavior and
    GeometrySceneV2 migration remain operational hardening.
D3. **Reproducibility (row 4)**: artifact reproducibility, production
    dependency/provenance evidence, bounded ABI allocation, architecture
    tests.
D4. **G0 closure (row 3)**: G0.14 is still `partial` in
    `docs/qualification/g0-contract-pack-status-v1.json`; the declared
    clean-run matrix on the frozen corpus is missing. No rewriting frozen
    oracle hashes to make results pass.

### E. Remaining host-side TypeScript (migration completeness)

E1. Per-body `lighten` orchestration loop in `src/kernel/cadWorkbench.ts`
    (still a host loop over `lightenSolid`) and the panel volume-reduction
    percentage (presentation; decide if in scope).
E2. `src/app/directModeling.ts` document parsing/history remnants; periodic
    re-inventory of `src/` against the objective (every audit round lists what
    remains, with file paths).
E3. Manifold backend (`executeSemanticProgram` generic path in
    `src/language/semantic/semanticProgramExecutor/`) — decide
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
