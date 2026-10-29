# Original graph-body union/intersection envelopes · 2026-10-30

`ss_face_membership::assemble_closed_graph_envelope` constructs actual union or intersection boundaries from the complete original closed SS material partition of two admitted rectangular graph bodies. Union retains roof sectors outside the opposite solid; intersection retains sectors inside. Each result consists of one outer roof with its SS hole, the opposite inner roof disk, and the five unchanged floor/wall faces of the operand selected by the outer sector. Both original bodies share the exact footprint and floor, so those original walls already bound the appropriate maximum/minimum roof envelope at the footprint boundary.

The source material owner is freshly replayed before selection. Actual SS edge IDs define one key namespace; literal original floor/wall edge IDs define a separate namespace. Reversed SS records are returned to their original complete world-curve orientation. Non-SS edges must equal the selected original source edges. Neither endpoint proximity nor a tessellated approximation assigns a shared identity.

The shared-boundary classifier now recognizes the complete common SS cycle in an outer or hole wire. Independent joint exact lift, simple-region and whole-chart injectivity proofs still precede it. A simple common world cycle lying on the unique complete original ellipse must cover that ellipse. No other contact exists in the full original SS census, so the two retained surface regions can meet only on this actual shared boundary. A hole's negative winding does not change this contact theorem. Multiple/incomplete common cycles and retained coedge clocks remain explicit refusals.

`ClosedGraphEnvelope` owns closed incidence, not automatic volume authority. Result exact lifts, trim regions, injectivity, every face pair and outwardness are freshly audited before `trimmed_shell_recipe::QualifiedVolume` is obtained. Structural work is bounded by the explicit shared operations allowance; every one of the seven fragments has explicit face-audit limits, and the result volume audit has separate caller-supplied limits. Existing maxima, tolerances, violation pins and the 1000-work contact-pair limit are unchanged.

The regression covers union/intersection under operand swap, 7 faces / 16 edges / 12 vertices, unchanged original floor/wall supports and literal source world curves, independent interior/exterior point parity, exact structural/coefficient work cutoffs and a one-ULP deformation of the actual hole pcurve that tolerant Model validation accepts but exact admission rejects. Inputs are retained unchanged.

This extends the previously admitted inner difference lens. The annular difference still requires wall strips between the two roofs. General rational/multispan/periodic Boolean, the central sewn G2 corner, continuous offsets and self-intersection cleanup, general exact simplification, full minimal unit-test qualification, WASM/Rush/UI, edited-body STEP and distribution gates remain open. This is not completion of the five-direction plan.

## Verification on final Rust sources

- Four native libraries: **4220 passed, 0 failed, 6 ignored** (brep 2011/5, nurbs 2001/1, predicates 100/0, intersection 108/0). The full run includes all four envelope/operator-order variants, all source-curve and floor/wall checks, point parity, hole deformation and budget regressions.
- Node architecture/native-partition/required-check contracts: **10/10**. API map **239 modules / 200 catalog entries**. Architecture audit: **70 existing violations**, unchanged limits and pins.
- Explicit-file rustfmt check and `git diff --check` pass. Rust source hashes agree before/after the full run.

Logs are under `/Users/themoretheless/.codex/tmp/nurbs-plan-recovery-commands-20261030`: `graph-envelope-full.log`, `graph-envelope-focused.log`, `graph-envelope-node.log`, `graph-envelope-architecture.log`. The focused log predates the added original-curve/hole-deformation assertions; the full log covers the final test source.

The full minimal unit-test target remains unqualified (previously 412 compile errors, not remeasured here). No new WASM/Rush/UI/STEP runtime or distribution qualification is claimed.

Minimal native library: `cargo check --manifest-path crates/Cargo.toml -p brep-core --offline --locked --no-default-features --lib` passed on the same final Rust sources (`graph-envelope-minimal-check.log`). This does not qualify the full minimal unit-test target.
