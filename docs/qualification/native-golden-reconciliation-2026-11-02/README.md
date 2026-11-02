# Five reviewed native Linux golden changes

The existing exact 43-case test passes on the unchanged current executable on Debian bookworm and Ubuntu 24.04/26.04 after changing exactly five fixture lines. No case, assertion or tolerance in the test was changed. Runtime glibc differences do not explain the five old mismatches. This does not establish compiler-host reproducibility, hosted CI, WASM or qualification of the separate 214-capability worktree.

- Surface intersection: all 1452 old unresolved obligations and all geometric values remain identical. One full-domain unresolved obligation is added. Metadata withdraws unsupported topology proof claims.
- Surface distance: only the lower interval endpoint moves two ULP downward; upper endpoint, witnesses, enclosures, parameters and search counters are identical.
- Two offset jets: independent 90-digit Decimal evaluation of the exact nominal binary64 input net checks position, normal and first offset derivatives. All 72 baseline/current sample components differ from the reference by at most 3.243e-15. The 5e-13 check in the auxiliary oracle is a diagnostic bound, not a relaxed golden assertion.
- Sketch: independent residual recomputation from the returned geometry verifies every constraint within the requested 1e-7 tolerance and verifies reported residuals within 4e-15. The underconstrained status, three DoF, three iterations, constraints and lack of a convergence certificate remain unchanged.

The historical baseline reproduced all 43 old hashes first; see the separate baseline evidence. The original failed current run and original fixture are preserved here. All three successful replays use the same executable hash recorded in receipt.json. This is a test-executable replay with a runtime-read fixture, not a fresh three-host build.

To repeat the independent payload checks, copy this directory to a temporary directory, extract canonical-payloads.tar.gz there, then run `python3 verify-offset-oracle.py` followed by `python3 verify-five-goldens.py`. The structured JSON preserves exact binary64 bits. The five fixture changes and original fixture allow independent hash comparison.
