# Original NURBS intersection trace coverage — 2026-11-02

Status: partial progress toward intersecting NURBS Boolean. This change does
not authorize a new standard Boolean family.

## Implemented contract

`nurbs_core::ss_intersection::qualify_fiber_trace_atlas` accepts a verified
`QualifiedFiberComponents` and an ordered partition of world trace receipts
for EVERY coverage chart. It returns a private, replayable
`QualifiedFiberTraceAtlas` only when:

- the parameter coverage is the complete natural product of the two original
  surfaces, not merely a local query box;
- original exclusion/coverage/component receipts replay;
- every chart has a nonempty, exact, ordered partition of its full free interval;
- every piece binds both original surfaces and the same free coordinate;
- unknown-coordinate boxes equal the parent chart box, so local uniqueness
  identifies the parent's root rather than an unrelated root;
- each original root/world trace receipt independently replays and satisfies
  the requested world error;
- no more than 4096 charts/pieces are supplied.

Completeness follows by composition: original-product coverage assigns every
root to a chart; interval partitions cover each chart's complete root graph;
verified world traces enclose its approximation error. Overlapping charts and
their traces remain explicit. The receipt records the maximum verified error.
It does not merge nearby fitted endpoints or infer physical branch identity
from approximation proximity.

## Still open

This certificate does not prove a physical branch bijection, exact agreement
of overlapping fitted traces, trim-domain membership, arrangement of trim
curves on a face, retained-material labels, seam ownership, or topology of an
authored ordinary Model. General tangencies, singular strata, coincident
surfaces and periodic quotient handling remain separate obligations.

In particular, source-wide coverage is not a proof of completeness restricted
to arbitrary B-rep trims: those trims require a certified intersection-domain
arrangement. Standard Boolean must continue to refuse unproved crossings.
No WASM, browser or distribution qualification is claimed here.

## Validation

Command:

```sh
CARGO_TARGET_DIR=/private/tmp/osv-trace-atlas-20261102 \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --manifest-path crates/Cargo.toml -p nurbs-core \
  --test ss_trace_atlas --test ss_construct_trace \
  --test ss_construct_trace_budget --test ss_fiber_components \
  --test ss_fiber_world_trace --test ss_coverage -- --nocapture
```

The new regression uses a nonlinear curved intersection covered by three
charts with different free coordinates. Negative cases omit a chart, omit
pieces, duplicate pieces, substitute a different chart/clock, supply an invalid
tolerance, or attempt to promote local product coverage to entire surfaces.
Native validation passed 23 tests across 6 test executables, with zero failures. See `tests.log`. Source hashes identify the new implementation
and test; the shared checkout contains concurrent unrelated work and this is
not an immutable full-workspace qualification.

API ownership remains `queries::ss_intersection`; the existing public-module
map passes `node scripts/map-nurbs-api.mjs --check`.
