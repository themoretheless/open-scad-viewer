# Artifact-pinned real worker boundary suite

Every worker in `tests/mainSolidWorkerRealBoundary.test.ts` now receives the
expected artifact identity. The parent hashes public WASM, compares the
generated identity, and optionally requires CAD_EXPECTED_WASM_SHA256. The
existing worker harness checks its imported identity before delivering queued
requests; the port validates and filters its metadata message. Replacement
workers after cancellation use the same expected artifact.

All 34 tests passed against SHA256
`c5d0f3bb20987d7be380374a82d41cfc959fdef632afb7d4967fe3443309fbbe`.
Log `/tmp/retained-cache-pinned-real-worker-all-20261010.log`; test time 21.03s,
total 23.11s. Vue typecheck passed.

A deliberate all-zero SHA fails at module initialization before any tests or
geometry requests execute (exit one, no tests). This expected negative control
is logged in `/tmp/pinned-real-worker-wrong-sha-20261010.log`. It demonstrates
that a stale expected artifact cannot silently qualify the current publication.
This is test infrastructure and provides no general geometry or full P0 claim.
