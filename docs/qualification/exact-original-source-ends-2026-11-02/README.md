# Exact direct end contacts on original source graphs

The full seven-stage CAD goal remains active. This step gives the complete transition perimeter exact source-chart bindings, including direct end G2 for the supported graph case. No retained source face or closed solid is created. Prior receipts remain unchanged.

## Exact end geometry and G2

`Patch.source_end_contact` restricts the actual exact U-end boundary to one side of the original source crease V=1/2. It derives the actual source UV line from the canonical world curve and checks EVERY coefficient of its source lift, not only endpoint positions. The original source chart and rational parent interval remain owned by an opaque SourceEndContact.

For the qualified polynomial graph class z=q(X)-h(X)V, the factory evaluates the ORIGINAL h, physical-X first derivative and second derivative exactly at the end. All three must equal zero. In the already-proved regular XY charts this makes the complete second-order surface graph jets equal to q(X), independently of width derivatives. It proves direct positional/G2 contact to both original source halves. Nonvanishing or unproved jets refuse; no tolerance snapping or sampled tangency supplies authority. This special direct-join construction is not a general end-cap surface algorithm.

## Saved topology and stale plans

`Strip.source_end_splits` proposes exact crease splits for actual saved end edges. EndSplitProposal binds the panel Arc, original root Arc, source-end role and current interval. `apply_source_end_split` refuses a changed interval or other stale scope before mutation, then uses the existing transactional exact split. Splitting the crease is explicit; source-end binding refuses an edge crossing both source charts.

`source_end_contacts` compares each actual saved canonical end edge against a fresh exact source half lift. SourceEndBinding retains the geometry scope and invalidates after a relevant edit. No partial collection is published on failure. Planning and binding are read-only; individual applied splits are transactional. Full command/worker/UI generation handling remains separate.

## Evidence

Eleven native tests and seven B-rep tests pass. Native checks include independent rational end positions, exact source second-order conditions, crossing-crease refusal, insufficient budget and a nonflat end refusal. B-rep checks exercise the two crease splits, four source uses, further child restrictions, binding invalidation and refusal of a stale split plan without topology changes.

The final expanded split case has 41 longitudinal plus four end source bindings. A unique-edge set comparison confirms EXACTLY 45/45 open perimeter edges are covered, with no duplicate or missing edge. It retains 20 transition faces, 66 canonical edges, 47 exact vertices, 87 actual face-boundary checks, 19 within-tolerance internal G2 joins and complete-panel curvature upper 8.402466975075457/mm under the unchanged 10/mm cap. Shared charged coefficients: 758001. It is still an open shell: shell.closed=false and bodies.len=0.

Reproduce with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --lib original_graph_transition`. For B-rep add `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0` and use `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining sewing

Construct retained original-source trim loops using these exact UV curves, preserve original source identities and sew their shared edges into the transition topology. Complete original-face fields, exterior/global intersections, radius, thickness, orientation and solid-volume admission are not inferred from perimeter coverage. General rational NURBS/algebraic endpoint and mixed-radius corner/end construction, persistence, coordinate Model integration, WASM/worker/UI, STEP/history, part acceptance, P0 and P2-P3 remain open. Scoped hashes identify listed files in the shared dirty worktree, not a delivery revision.
