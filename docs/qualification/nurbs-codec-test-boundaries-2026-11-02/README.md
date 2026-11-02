# Codec test boundaries and complete border proof dispatch

The original no-codec library test harness failed to compile with 79 errors.
Production native geometry was already available; unguarded tests referenced
optional JSON methods or codec/transport-only adapters. This repair changes the
feature boundaries of tests, without adding any unconditional codec dependency.

All native portions of mixed tests remain active. Scalar sweep input snapshots
use typed curve clones instead of JSON strings. Only JSON assertions in the
three offset/contact tests, the Cartesian cell test, tangent-audit test and
invalid scalar-wrapper requests are conditional on `codec`.

Eleven tests of APIs absent in no-codec builds receive matching feature guards:

- Two JSON retained-wall `inspect` tests.
- Three JSON intersection-report adapter tests.
- One guided-contact `transport::dispatch` regression (`transport` guard).
- One checked-profile JSON certificate regression.
- Two scalar checked-profile JSON adapter regressions.
- Two checked Cartesian Gordon JSON adapter regressions.

All 2224 test names in the original default test executable are retained. The
full first repaired runs compiled but exposed an existing exact-boundary
regression: both modes returned `Indeterminate(MissingProof)` where the rounded
reversal fixture requires `Different`. The identical failure is reproduced on
the pre-repair native234 default executable, whose hash was checked before use.
Independent Fraction cell identities show a nonzero exact complement residual
of `1/36028797018963968`; the dyadic reversal control remains equal.

The production correction places the complete clamped-border owner before the
periodic/nonclamped trace owner. The latter can decline distinct normalized
bases; it must not preempt a complete proof already supported by the former.
This chooses a proof before evaluation rather than restarting a spent ledger.
Existing admissions, limits and expected `Different` assertions are unchanged.

Qualification is source-bound to the accompanying receipt/logs. Historical
native234 receipts and its original failed no-codec harness remain unchanged.
Library qualification is distinct from WASM, full workspace, frontend, editor,
STEP and delivery to main.
