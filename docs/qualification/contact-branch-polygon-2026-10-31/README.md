# Contact branch ownership and polygonal central graphs

## Selected-edge binding

`fillet_contact::branch::inspect` freshly inspects actual source geometry and actual contacts. It binds a pair of contacts to one original named edge through two injective original-support chart collars:

- Original edge lifts on both supports are checked over the complete curves.
- A face must have one literal rectangular trim, without holes; subdivisions of its sides are permitted.
- The selected edge must be a straight chart-side segment. Each contact must be a positive rational Bezier trace parallel to that side, with strictly ordered free-coordinate controls and the same extent and traversal as the original edge.
- Each collar stays strictly inside the selected half of the face. No other trim boundary enters its interior.
- Full original-support injectivity is rechecked. No coordinate sorting or sampled proximity gives ownership.
- Both actual contact lifts are checked against the original support and actual proposed skin; supplied `positions` receipts are not trusted.
- Work exhaustion, unsupported trim/trace shapes and failed geometry remain `Unresolved`.

This is ownership by an embedded chart ribbon, **not a proof of unique rolling-ball continuation**. It does not prove radius, G2, self-intersection freedom, thickness or solid closure. A chamfer can have valid edge ownership while failing G2.

`topology::propose_bound_edge` connects automatic support discovery, the existing G2-section proposal builder and this ownership inspection. It never silently promotes a numerical surface-pair proposal to a bound/accepted solid. Geometry cell/exact work is shared across the original-edge, proposal and ownership stages. The bounded topology planning calls retain their individual `max_visits` limit.

## Central construction

The existing Hermite graph constructor, central face trimming and whole-seam inspection now accept **3..16 boundary curves**, replacing their hard-coded six-boundary restriction. The six entries in a graph two-jet remain position/gradient/Hessian components; they are not boundary counts. Seam proof budgets divide by the actual number of remaining boundaries.

This supports central graph candidates in a common graph frame. It does not solve non-graph configurations, incompatible boundary jets, generate all missing contact/end boundaries, or automatically construct a complete high-valence fillet.

New controls exercise five-, seven- and eight-sided curved rings against an independently authored quadratic support. Their whole-seam G2 and actual trimmed-face curvature <=10/mm are separate gates. These rings are interpolation controls, **not closed five/eight-edge unequal-radius fillet bodies**. Passing those controls must not be described as full high-valence body qualification.

## Still open

- Arbitrary trimmed/rational contact ribbons, periodic charts, independent branch continuation and automatic retry on another offset branch.
- Automatic contact/root generation and common endpoint splitting for a full corner.
- Joint construction of real 5+ edge unequal-radius transition strips and a compatible central surface.
- Sewing those new transitions into an admitted closed original-source body, with complete G2, curvature <=10/mm, thickness and global intersection checks.
- STEP/WASM/worker/UI qualification of new cases.

Final executed tests and source hashes accompany this document. No G2 tolerance or 10/mm curvature limit is relaxed.

## Graph-to-body assembly connection

`region_replacement::trim_and_replace_graph_contacts` derives replacement pcurves from the generated graph and sends the source-owned prepared contact cuts into the atomic trim/sew kernel. It supports 3..16 original support cuts (up to 256 contact segments), keeps original support definitions, and rejects missing/changed contact geometry. It ignores sampled fitting residuals. Its receipt is C0 closure only; G2, <=10/mm and global embedding are independently rechecked on the saved body.

The rational-prism regression now goes through this connection and retains its complete final-body G2/curvature/global tests. This tests assembly on that existing control, not on a new high-valence unequal-radius body.

## Proof-coordinate correction

The correlated graph inspector previously used only the exact integer chart `(x-y, x+y-2z, x+y+z)`. An otherwise accurate Cartesian graph candidate could therefore produce broad regularity corrections and exhaust its proof budget. The inspector now also supports the exact Cartesian chart `(x,y,z)` and selects a chart by a proposal-only conditioning score. Original poles are projected in exact rational arithmetic, and the shape-operator reconstruction uses the corresponding exact dual basis. Both paths still require complete outward interval proofs. No original geometry is rounded into a substitute rotated support.

## Executed validation

- 29 targeted B-rep integration tests passed.
- 5 graph-trace kernel tests passed, including the original diagonal frame, exact Cartesian curved graphs, changed curvature and no-work rejection.
- The three curved polygon controls passed all 5/7/8 seams. Conservative full trimmed-face curvature upper bounds were 0.790586484889 / 0.809180237325 / 0.790586484889 per mm, below the unchanged 10/mm limit.
- Correlated coefficient work was 2,546,535 / 3,565,261 / 4,074,584, within the shared 100,000,000 budget for each ring.
- Source-stock branch tests include a rational curved prism and a nonlinear rational contact clock. Negative controls cover a different selected edge, opposite-side contacts, folded traces, changed skin and stale supplied position receipts.

Commands use `CARGO_TARGET_DIR=output/qualification/central-graph-dev-target`, the pinned nightly, `--offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2`. Integration targets: `-p brep-core --test contact_branch --test central_graph --test contact_topology --test four_edge_corner --test trim_contact_replacement`. Kernel target: `-p nurbs-core --lib graph_trace::tests`.

Earlier failed diagnostic runs were investigated, not counted as successful qualification. The enlarged-budget diagnostic was explicitly interrupted after the one-cell strategy was chosen; the final successful checks use the original 100M coefficient bound. These results do not qualify all kernel tests or the complete application.
