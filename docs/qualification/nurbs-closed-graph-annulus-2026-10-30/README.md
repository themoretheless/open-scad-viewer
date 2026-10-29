# Annular graph differences and one native Boolean entry · 2026-10-30

`assemble_closed_graph_annulus` constructs the complementary outer difference component of two admitted original rectangular graph bodies. Both original outer roof regions, including their complete SS hole, are retained. The subtrahend roof is reversed. Four new planar ruled strips join the two original roof rails; the common original floor is not retained because it belongs to both operands.

Strip pairing uses the original wall-plane/roof-edge incidence from each qualified graph body, not endpoint proximity. Literal XY endpoint equality selects curve orientation. Every input rail control and every output strip control must stay on the exact original footprint plane. Both original complete roof curves remain the authored shared seam definitions. Roof edge keys include the operand identity; SS keys retain the original intersection-edge identity; vertical seams are keyed by the original minuend roof vertices. The shell recipe rechecks complete curve and endpoint identities and two opposite uses. The result has 6 faces, 16 edges and 12 vertices with two annular roof faces (one genus-one boundary).

`assemble_closed_graph_boolean` is the common Rust entry for union, intersection and either operand order of difference. For difference, the sealed original inner material role selects the lens or annulus constructor. The selected constructor freshly replays the original bodies, SS partition and material roles; selection alone grants no authority. Existing direct constructors remain available.

`ClosedBoundary::audit_volume_with_linear` exposes the existing independently bounded monotonicity admission route while preserving the legacy default. This allows planar strips with varying height, which do not admit the legacy global contraction, to obtain a fresh injectivity proof. The test keeps the old total chart-work cap of 1000: 100 contraction spans plus at most 900 monotonicity cells. The observed combined count must stay within 1000. Contact-pair work remains 1000; tolerances, mathematical maxima and architecture pins are unchanged. No automatic retry or hidden larger budget is used.

The common entry is tested across all eight operation/order variants: union, intersection, inner difference and annular difference under source swap. Acceptance checks independent point parity, literal original roof-curve retention, actual topology, exact construction-work cutoff and source immutability. A one-cell monotonicity budget cannot obtain `QualifiedVolume`. A folded strip with unchanged boundaries passes tolerant Model validation and exact lift identity but fails fresh chart injectivity and volume admission; closed incidence and exact edges therefore do not bypass geometric validity.

The source body class and complete contained SS ellipse requirements remain. General rational/multispan/periodic Boolean, source boundary-crossing branches and other contact strata remain open. So do the central G2 fill and sewn corner, continuous offsets and self-intersection cleanup, general exact simplification, the full minimal unit-test target, WASM/Rush/UI, edited-body STEP and distribution gates. This is not completion of the five-direction plan.

## Verification on final Rust sources

- Four native libraries: **4221 passed, 0 failed, 6 ignored** (brep 2012/5, nurbs 2001/1, predicates 100/0, intersection 108/0).
- The final full run includes all common-entry variants, the small-budget refusal and the boundary-preserving interior fold refusal.
- Node architecture/native-partition/required-check contracts: **10/10**. API map **239 modules / 200 catalog entries**. Architecture audit: **70 existing violations**.
- Explicit-file rustfmt and `git diff --check` pass. Recorded Rust source hashes agree before/after qualification.

Logs: `/Users/themoretheless/.codex/tmp/nurbs-plan-recovery-commands-20261030/graph-annulus-full.log`, `graph-annulus-focused-r3.log`, `graph-annulus-node.log`, `graph-annulus-architecture.log`. The earlier `graph-annulus-focused.log` records the real missing-admission regression (correct sewn boundary, legacy wall injectivity unproven); `graph-annulus-focused-r2.log` proves the annulus via the explicit monotonicity route before adding the common entry. Final tests keep the full volume assertion.

Full minimal unit tests remain unqualified (previously 412 compile errors, not remeasured here). No new WASM/Rush/UI/STEP runtime or release-distribution qualification is claimed.

Minimal native library also passes `cargo check --manifest-path crates/Cargo.toml -p brep-core --offline --locked --no-default-features --lib` on final Rust sources (`graph-annulus-minimal-check.log`). This is separate from the still-open full minimal unit-test target.
