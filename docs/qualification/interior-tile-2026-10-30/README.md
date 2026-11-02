# Closed composite central tile, 2026-10-30

This advances the central corner construction to an actual composite B-rep.
It does **not** qualify a material-preserving fillet, arbitrary NURBS edge
fillets, end transitions, interior radius laws or global embedding.

## Construction and fresh admission scope

`region_replacement::propose_interior_tile` extracts a bounded degree-6..12
rectangular child support and retains its first/last three control layers on
both axes. One interior pole is displaced. The complete closed rectangle must
first be certified Inside the source trimmed face, including any holes.

Four world curves are generated from the original support. Their endpoints
bind to explicit original UV-corner roles, never by proximity. Both position
and geometric G2 are reverified after that binding. Four complete seam proofs
share cell, exact-predicate and optional correlated-coefficient budgets.

Assembly uses the existing `face_window::replace_with_exact_limit` transaction:
the original support/outer loop are retained, an owned clockwise hole is
inserted, and the child face shares four edges and four vertices. The new
entry point preserves `replace`'s default API while allowing a caller to pass
a remaining exact budget, including zero. Split lineage and persistent naming
are mandatory. A fresh whole-model continuous closure proof must also pass.
No partial model is returned when ownership, G2 or closure work is exhausted.

Exact graph tensor extraction now contracts the two axes separately and skips
literal zero products. A dense rational reference verifies identical binary
interval endpoints and fewer charged operations. This is an exact arithmetic
reassociation, not a tolerance change, sampled fit or normal-equation shortcut.

## Actual unequal-radius control body

The source is the saved boundary-only trihedral fixture reconstructed on the
10 mm cube with radius requests 0.1, 0.2 and 0.35 mm. Reconstruction inherits
no original-rim G2 receipt. The current operation preserves that source's six
outer rim definitions literally; its new certificates concern the four new
internal seams. The three distant finite stops remain planar sharp stops.

The actual composite candidate has **14 faces, 37 edges, 26 vertices**, one
shell and one body. Accounting for the one hole gives Euler characteristic 2.
All trim regions pass; native closure is `WithinTolerance`. All original
vertices, edges and loops, their identities, and unaffected face identities
are preserved. The retained original surface is byte-for-byte unchanged.

All four complete new seams pass the original limits of 1e-7 mm position,
1e-5 normal difference and 1e-4 /mm world shape-tensor difference. The current
correlated proof used 20,636,444 coefficient-work units. Its largest normal
upper bound is approximately 4.40e-12 and tensor upper bound 1.33e-8 /mm.
Closure remains tolerance-qualified, not an exact analytic-equality claim.

The previous material witness at UV
(0.9430759112081446, 0.4836167378919158) changes from Y approximately
-1.03955653212e-5 mm to +1e-6 mm. Its fresh native point enclosure and an
Inside trim classification confirm that this particular witness is repaired.

## Independent STEP and the remaining rejection

OCCT 7.7.2 imports one root, one solid and one shell with exactly the expected
14/37/26 face/edge/vertex counts. Its basic B-rep analyzer passes. The exported
surface point is in the authored tile's UV interior and matches Rust with
error 5.55e-17 mm. The volume value is only an independent numerical estimate.

**Whole material containment is still Outside on both the tile and retained
face.** Three of the four fixed new corner vertices remain outside the
original cube allowance. Additional independent tiny windows with those
boundaries frozen therefore cannot establish the requested global result.

The independent point curvature diagnostic also shows a substantial increase:
the source principal radii at the witness were approximately 0.390 and
0.114 mm; the new local point has radii approximately 0.0147 and 0.00465 mm.
These are sampled principal-curvature radii, not a qualified fillet radius
law. Neither the interior curvature nor the complete body's material,
thickness, overlaps or self-intersections are admitted.

The next construction must coordinate neighbouring subpatches and their
internal curves while enforcing material and curvature constraints, followed
by whole-body admission. Passing the four local G2 seams or one repaired
point cannot replace that work. No live document was changed.

## Focused validation and reproduction

25 unique focused Rust tests passed: 12 composition/exact-trace controls,
4 face-window transactions, 4 interior-tile cases, 2 high-degree composition
cases and 3 original-span collar cases. The window cases include an explicit
zero exact budget; the tile cases include a rotated source, shared topology,
identities, full trim audits and late exhaustion with immutable input. This
is not the full workspace, WASM, worker, UI or P0 command matrix.

Build `central_graph_diagnostics` and `interior_lift_diagnostics` to reproduce
the saved source and ownership audit as described in the preceding
[interior-lift qualification](../interior-lift-2026-10-30/README.md). Then:

```sh
cargo build --offline --manifest-path crates/Cargo.toml -p brep-core --example interior_tile_diagnostics
interior_tile_diagnostics SOURCE.graft.json SOURCE.local-lift-audit.json OUTPUT_DIRECTORY
python scripts/verify-central-graft-step-occt.py OUTPUT_DIRECTORY/candidate.step --faces 14 --edges 37 --vertices 26 --tile-report OUTPUT_DIRECTORY/report.json
```

The Python command requires the recorded OCCT environment. Explicit expected
counts extend the verifier without accepting arbitrary imported topology;
its defaults remain the previous 13/33/22 control body. The optional witness
check compares the actual exported surface, not native flags.

- [recipe.json](recipe.json): bounded construction parameters and source hash.
- [native-summary.json](native-summary.json): current complete local proofs,
  identity preservation and remaining material rejection.
- [step-occt-summary.json](step-occt-summary.json): independent current STEP.
- [source-curvature.occt.json](source-curvature.occt.json): original point curvature.
- [source-hashes.json](source-hashes.json): relevant source snapshot.
- [superseded-run.json](superseded-run.json): explicitly retired older binary;
  it is not a passing, failing or pending qualification result.
