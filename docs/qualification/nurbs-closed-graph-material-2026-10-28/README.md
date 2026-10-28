# Closed SS sectors bound to original graph-body material · 2026-10-28

`ss_face_membership::qualify_closed_graph_material` binds the complete audited closed SS partition to two actual `graph_body::QualifiedGraphBody` owners. The original membership Model reference and roof face must match each owner exactly. Equal geometry in another Model is insufficient. Both original bodies must have the same exact XY footprint and floor height.

Each body is freshly replayed through its original coefficient, boundary, embedding, contact and volume gates. The complete closed-graph partition and coefficient height-side proof are replayed as well. Over the common rectangle, both original roofs lie strictly above the common floor. Thus the relative interior of an authored roof sector is strictly inside the other body precisely when below the other roof. This derives a material role from actual solid column semantics. No sampled point or serialized result assigns the role. All sector boundary edges are excluded from this strict-interior statement, including original outer boundary edges and the shared SS contour.

The immutable owner retains both actual bodies and the original partition. Exported face records are snapshots; altering a snapshot cannot change authority. Reverification regenerates all records and work counters. The shared structural/coefficient budget includes the partition and both graph-body counts, with native embedding/contact/volume work still constrained by each original body's separate unchanged phase limits. No limits or violation pins increased.

Coverage includes four actual roof sectors, exact operation cutoff, wrong source order, equal-but-independent Model substitution, tampered cached material roles, source immutability and rejection of different original floor heights even when the support-relative SS roles are available.

This does not sew a Boolean result. Contact/coincidence roles on original walls/floors, source-edge identity assembly, result embedding and result volume must still be proved. General multispan/periodic/rational cases, central G2 fill, continuous offset and self-intersection cleanup, general simplification, WASM/Rush/UI, edited-body STEP and distribution gates remain open.

## Verification on final Rust sources

- Four native libraries: **4217 passed, 0 failed, 6 ignored** (brep 2009/5, nurbs 2000/1, predicates 100/0, intersection 108/0). Both new material tests pass in this full run.
- API map: 239 public modules / 200 catalog entries.
- Architecture audit: 70 existing violations, unchanged limits and pins.
- Whitespace audit: `git diff --check` passes.

Logs are under `/Users/themoretheless/.codex/tmp/nurbs-plan-recovery-commands-20261028`: `graph-material-full.log`, `graph-material-focused-r2.log`, `graph-material-architecture.log`. The focused log predates the additional floor-mismatch test; the full log covers final sources.

The full minimal unit-test target remains unqualified (previously 412 compile errors). This milestone does not qualify WASM, Rush, UI, edited-body STEP or release distribution. The five-direction plan remains active.

Minimal native library: `cargo check --manifest-path crates/Cargo.toml -p brep-core --offline --locked --no-default-features --lib` passed on final Rust sources. See `graph-material-minimal-check.log`. This does not imply the full minimal unit-test target passes.
