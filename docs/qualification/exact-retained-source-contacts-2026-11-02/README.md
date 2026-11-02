# Exact transition-to-retained-source contact isolation

The full seven-stage CAD goal remains active. This qualifies the 40 transition/retained-source pairs on the existing exact polynomial graph shell. The remaining original-face pairs are explicit; global embedding and solid admission remain false. Earlier receipts remain unchanged.

## Whole footprint, with actual source ownership

`Patch.inspect_retained_projection` freshly lifts the selected original source contact. It checks every exact transition tensor coefficient against X=U and Y=(2V-1)w(U), after exact width degree elevation, with every original width coefficient strictly between zero and one. The complete footprint lies between the two source contact graphs, and only the V=0/1 rims reach their respective graphs. The original support graph identities and immutable native Patch constructor remain prerequisites; this is not a generic rational-NURBS proof.

`RetainedRegion` already owns a simple complementary original-chart notch: exact closed UV/world loops, monotone physical U contact chain, strict 0<width<1, positive UV area, no holes and exact source-owner bindings. `ClosedShell.inspect_retained_pairs` first rechecks every saved topology/UV/world lift through the existing hull/transverse audit, then binds each newly admitted pair to the actual immutable retained region and actual transition Arc.

For every longitudinal contact segment, face index, side, source equality, native panel Arc, exact world-edge root Arc and root interval must agree. Actual transition pcurves must be on the correct V rim and their physical-U intervals must agree with the original contact intervals. Together these intervals partition the entire transition U domain exactly once. Graph-notch domain separation excludes any additional interior contact.

At the global ends of the notch, the retained region has an additional vertical boundary. The corresponding source-bound end owners must belong to the same actual panel and side, and actual saved end-edge intervals must partition the required V half ([0,1/2] or [1/2,1]) exactly once. No end owner may appear at an internal panel end. The complete expected edge set must equal the actual shared-edge set. Thus the proof includes the finite end-half contacts as well as all longitudinal source-contact splits; topological adjacency alone is never accepted.

All arithmetic in the native footprint proof shares the existing exact coefficient/bit budget. Unsupported footprints remain unresolved; incomplete work is an error.

## Terminal evidence

24 focused NURBS tests and 13 B-rep integration tests pass. The new native regression changes a non-rim interior Y coefficient by 2^-100 while preserving the source contact rims: the footprint proof rejects it. Both original sides, invalid side selection and exhausted work are tested. The complete B-rep test keeps the independent volume formula, non-dyadic shared-root splits, all-pair accounting and chart injectivity checks, and now verifies all four transition/retained pairs on the two-panel control including global end halves. Insufficient geometry/pair budgets are rejected.

The expanded 20-panel shell remains 27 faces, 82 edges, 57 exact vertices, zero open edges and no published bodies. Of all 351 face pairs, 277 are strictly hull-separated, 19 meet only on the admitted transverse seam, and 40 meet only on the admitted original-source contact boundary. Fifteen original-source pairs remain unresolved and none are omitted. The explicit indices are in unresolved-pairs.json.

Whole-face chart injectivity remains proven for all 27 supports, nineteen internal G2 joins remain valid and whole-support curvature upper remains 9.867331764661289/mm under the unchanged 10/mm cap. Geometry, oriented flux and source document are unchanged. Shared charged coefficient work after this complete diagnostic sequence is 2207603.

## Reproduce

Use `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core original_graph_transition --lib`. For B-rep add `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0` and run `test -p brep-core --test exact_graph_strip` or `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining scope

The fifteen remaining contacts are between the two retained original roof regions and original walls/floor/endcaps. Prove contact isolation from the actual original supports and trimmed domains; possible supporting-plane certificates must also prove complete shared-edge coverage, including the two retained intervals of the original crease. Existing shared incidence and positive algebraic flux do not substitute for that proof.

Then finish material/volume admission, actual fillet-radius qualification and thickness. General rational/multi-span source trimming, arbitrary branches, unequal-radius complex nodes and 5+ corners remain open. WASM/worker/UI, STEP/independent measurements/history, three-part acceptance, P0 and P2-P3 remain open. Hashes refer to selected source files in the shared dirty checkout, not an isolated delivery revision.
