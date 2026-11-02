# Expanded contact runout - constructive control

The previous R1 control stopped at two source-crease stations with incompatible endpoint normals. This proposal expands the source cut to x=0 and x=2, where the original source 2-jets coincide, while preserving the central contact-center R1 law on x=0.5..1.5. End regions use a different, variable-width runout; they are NOT covered by a constant-R1 certificate.

## Actual construction

On the unchanged degree-eight source supports, scalar width sites are proposed from the original ball-contact equation. A clamped cubic interpolates the central width. Positive-control quartic runouts match its endpoint value and first two derivatives and reach positive width 0.25 at the zero-angle stations. Exact rational Bernstein products construct a polynomial tensor surface from q, h and width; conversion to binary64 is followed by fresh original-support proofs. Twenty spans fit the existing surface row cap. One batch C2 basis refit changes the entire surface by at most 3.613146311043305e-13 mm. No source surface is replaced or approximated.

Full original-support contacts, the complete central R1 contact-center law, source-root interval ownership and material partitions all pass. The expanded source endpoints are 0 and 2; their necessary normal-fan obstruction is absent. This is not a positive G2 certificate by itself.

The whole candidate surface has a certified curvature bound of 9.99789825936381/mm, below 10/mm, using 5243 cover cells and 8962 shape cells. The regression gives this phase 10000 cells under the existing API maximum of 50000. The first 4000-cell diagnostic was inconclusive. No curvature tolerance or engine row limit was raised. An experimental adaptive split policy was discarded; the existing curvature algorithm remains in use.

## Body admission still open

The native local sew transaction reaches its fresh saved-edge G2 gate, then refuses. Edge 17 between faces 0 and 7 has proven positions but unresolved G2 after exhausting the shared 100000-cell budget. The opposite-orientation normal upper bound is 0.002217199039015875 (limit 1e-5); the tensor upper bound is 0.0048283924284581795 (limit 1e-4). These are inconclusive upper bounds, not certified violations. Later declared seams were not visited. The error now reports bounded edge/face locations, decisions, position status and consumed work, rather than losing all details. A regression verifies that real G2 refusals retain these locations.

No new closed body is admitted, no global embedding or STEP result is claimed, and no WASM/UI path is exposed. Arbitrary rational supports, unequal-radius corners, thickness, full part/history/P0 acceptance and P2-P3 remain open.

## Next proof work

The first unresolved contact is a multispan degree-12 spline. `surface_g2_pcurves::inspect_owned_graph` has correlated graph-jet bounds, but `analysis/curve_surface_composition/exact_trace.rs::pcurve` currently requires a Bezier pcurve. It already contains an exact original-basis extraction matrix. A future extension should process complete original spline spans without rounded knot insertion/refitting, preserve original traversal and charge every coefficient operation. This is a suggested next step, not an implemented certificate. Any graph fallback needs an explicit coefficient ledger and saved-coedge ownership; an independent successful diagnostic must not be promoted to body admission.

## Reproduce

Example command (put options after the Cargo subcommand):

```sh
CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo run --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --example expanded_contact_runout -- --field-only
```

Use the same environment, manifest and profile options for the commands below.

- `run -p brep-core --example expanded_contact_runout -- --field-only`: checks the radius scope, source cuts/end normals and full candidate curvature; zero exit does NOT mean a closed body.
- `run -p brep-core --example expanded_contact_runout`: performs strict local sew/G2/curvature and subsequent global admission; currently exits 2 at G2.
- `test -p brep-core --test expanded_contact_runout --test face_curvature -- --nocapture`: 8 tests.
- `test -p brep-core --test convex_contact_branch --test radius_driven_contact --test compatible_contact_body -- --nocapture`: 20 tests.

The test verifies original model immutability. Runouts are a constructive polynomial-control fixture and are not general arbitrary-B-rep support. Compressed logs and numerical results are included.
