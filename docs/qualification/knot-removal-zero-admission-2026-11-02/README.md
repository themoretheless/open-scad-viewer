# Knot-removal zero-budget admission correction

The original precheck reported `removable_at_tolerance=0` when a refit/reinsert control residual was below `1e-9*scale`. A nonzero 1e-10 control perturbation satisfied this test despite an exact third-derivative jump at the removed knot. `before-probe.log`, `probe.rs` and `verify-jump.py` retain the source and independently verify the exact binary-rational jump 56295/8796093022208. A cubic without that interior knot has no such jump, so the zero-tolerance claim was false.

The precheck now always reports the continuous deviation envelope it calculated. The refit/reinsert norm remains a diagnostic. No threshold converts it into exact-identity authority. The paired approximate-removal acceptance is still checked at that envelope, and original limits and useful approximate edits are preserved. Exact degree-one simplification remains a separate source-predicate operation.

Six focused tests pass, including the formerly admitted tiny perturbation; the full default library passes 2044 tests, zero failures, one existing ignored microbenchmark and no filters. This does not qualify the full workspace, frontend or production WASM. The recovery-root frontend run remains on its previous immutable source snapshot until it terminates; this correction is initially feature-worktree-only.
