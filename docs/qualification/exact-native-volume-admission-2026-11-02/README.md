# Exact native volume and body-role admission

The full seven-stage CAD goal remains active. This converts the exact embedded control shell into a geometrically admitted native volume with one explicit outer-body role. It does not admit complete fillet quality or an application/STEP result. Earlier receipts remain unchanged.

## Admission owner

`VolumeBounds` validates explicit exact nonnegative lower and positive upper bounds, minimum<=maximum, and coefficient bit limits. Equal positive bounds are allowed for an exact-value constraint. There is no implicit geometric or manufacturing tolerance.

`ClosedShell.qualify_volume` requires one unassigned closed shell, recomputes its exact oriented z-flux on the actual saved geometry, rejects nonpositive flux and any requested-bound violation, and freshly runs the complete global embedding audit. Native topology validation already requires closed oriented connected face components and connected vertex fans. Whole-face injectivity and all face-pair contact proofs establish embedding. For such a connected embedded closed oriented manifold, positive flux of (0,0,z) gives positive bounded material volume and outward orientation. The sign is never repaired with abs(), auto-flipping or a fabricated lower bound.

The admission additionally checks all new transition G2 joins, transition field decision Within with no unresolved regions and upper<=10/mm, and both retained/imported original whole-support field bounds<=10/mm. Existing original sharp edges are preserved; they are not claimed G2.

`ClosedShell.admit_volume` consumes the immutable original shell and its source ownership, recomputes qualification for the supplied bounds, clones the exact native topology, adds one `Body { outer_shell:0, inner_shells:[] }`, and freshly validates topology/geometry again. `NativeVolume` retains the actual shell, exact flux, embedding proof and requested bounds. New topology shares the actual immutable surface/root owners, with no rounded preview or approximate coordinate Model substitution. A report from another geometry/bounds cannot be supplied in place of qualification. The original document and unassigned shell stay unchanged; persistent application naming/serialization of the derived body is still open.

## Evidence

Fourteen B-rep integration tests pass on the final sources. The independent constant-width volume formula remains exact. The test admits with minimum=maximum=that independently derived value, rejects too-small maximum and too-large minimum, refuses insufficient geometry/pair budgets, confirms the pre-admission shell still has zero bodies and the source Model is unchanged, then verifies the admitted role and exact volume. Additional bound tests reject negative minimum, reversed/zero intervals and an oversized integer, while accepting an exact positive equality interval.

The prior 27 NURBS unit proofs and isolated-touch regression were not needlessly rerun: their listed source hashes were checked against the preceding successful receipt and are unchanged. The current B-rep run exercises those proof paths through the full native admission. The full-project suite is not claimed here.

The expanded control finishes with 27 faces, 82 edges, 57 vertices, zero open edges and ONE body with outer shell 0 and no inner shells. Exact volume is in result.json, approximately 34.01287162678737 mm^3. The explicitly configured CONTROL interval [0,40] mm^3 is derived from the original full-support control-hull box; it is not a user-specified volume tolerance. Global embedding has zero unresolved pairs among 351, all 27 charts are injective, nineteen internal G2 joins pass and whole-support curvature upper remains 9.867331764661289/mm under the unchanged 10/mm cap. Shared charged coefficient work for the complete diagnostic/admission sequence is 3390382.

## Reproduce

Use `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --test exact_graph_strip`, or the same flags with `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining full goal

This is geometric/material volume admission of the exact native polynomial control, NOT a completed fillet command. Actual fillet radius and wall-thickness qualification, requested part-specific volume acceptance, general rational/multi-span trim/contact support, unequal-radius complex nodes and 5+ corners remain open. Exact source-bound transport into WASM/worker/UI and STEP must preserve the proof/identity boundaries; rounded display poles cannot inherit native admission. Independent measurements/history/reload and three-part acceptance, P0 and P2-P3 remain open. Listed hashes cover selected files in the shared dirty checkout, not an isolated delivery revision.
