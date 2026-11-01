# Oriented line-image material registration

Implementation: `fba57ef773a339d3ec4da835b0722df7802c29f1` (Rust only).

Original closed material may now bind a full oriented monotone segment image
without requiring the same normalized clock. The exact predicate uses original
coefficients, positive weights, complete chart membership, homogeneous endpoint
identity, collinearity and strict monotone progress. It grants no normalized
parameter-map authority. The retained/coedge/error-bound preparation continues
to require its original normalized identity and rejects the same inputs.

The native source fixture binds ten nonlinear rational UV clocks on a nonplanar
carrier, including an opposite-winding hole, mixed G2/C0 joins and C2 periodic
motion. All global source guarantees are recomputed. Exact work replays; zero
work and a bowed world curve with unchanged endpoints refuse. Native checks
include nonuniform rational carrier weights, transposed UV axes, backtracking
and wrong orientation. [THEOREM.md](THEOREM.md) records the proof and limits.

Frozen validation passed: 322 unique native tests in 32 focused groups and
10 infrastructure tests. Full suites: cad-predicates 122 passed /0 failed;
nurbs-core 2059 passed /0 failed /1 ignored; brep-core 2062 passed /0 failed /5
ignored. Focused/full coverage overlaps and must not be summed. Ignored tests
supply no evidence. All 4932 frozen source files remained unchanged. Commands,
catalogs, logs, hashes and backup receipts are in [verification.json](verification.json).
Preliminary diagnostics and the corrected compiler-only test assertion failure
remain explicitly separate from qualified executions.

[completion-audit.md](completion-audit.md) preserves the full objective and its
remaining gaps. Curved/general image registration is not qualified here. No
retained Solid, WASM, browser, STEP or remote CI result is inferred. Architecture
still has 68 diagnostics with no baseline changes; publication remains held.
