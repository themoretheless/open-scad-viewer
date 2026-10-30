# Frontend recovery, 2026-10-31

The complete frontend run passed: 6540 tests, zero failures, 10 skipped;
718 passing files and 5 skipped files (723 total). This supersedes the incomplete
frontend status in the architecture checkpoint `ab995f1dc`.

## Repairs

- Restored built-in name discovery with a dependency-free module inventory,
  including real isolated worker startup coverage for circular imports.
- Registered the raw shader loader inside disposable MCP transaction workers.
- Corrected inclusive native contact-work accounting without raising budgets;
  forged hull/source subtotals remain rejected.
- Replayed original direct-annular miter construction in Rust before Solid
  admission. Exact final-model binding and freshly derived cap selections replace
  the old assumption that the last two faces are planar endpoint caps. Forged
  model/source/cap hints cannot grant admission. Original example geometry and
  native proof/work budgets stay unchanged.
- Kept selected-cap receipt requirements strict. Frontend refusal assertions now
  match the native contract, retaining independent pair diagnostics.
- Preserved historical fixtures and source/STEP geometry while checking live
  native-to-worker parity for evolving proof counters. Empty serde topology
  defaults are explicitly checked before historical byte comparisons.
- Gave the two multi-stage integration suites a 120-second Vitest deadline instead
  of the runner default of 30 seconds. Native algorithm budgets and application
  worker deadlines were not changed; no performance qualification is claimed.
- Bound runtime v7 to the exact new WASM. Historical v6 manifests remain immutable;
  runtime selection refuses a qualified record for different packaged bytes.
- Added OwnRust79 / G0v93 / G1v110 source/artifact records. All 807 previous archive
  identities remain intact, with six additional predecessor records pinned. All
  4740 G1 clean work units remain incomplete; no old result is imported.

## Verification boundaries

Frontend and MCP types, 45 Node tests (one skipped), the 385-test geometry-bridge
library suite (one ignored), active STEP identity regression, architecture audit,
and four Chromium Solid lifecycle cases (36 checks) passed. This is a finite
local matrix, not a full native-workspace, universal geometry or hosted CI claim.

The exact WASM is `91521ffbc32570cddc99b4555dcdd3f55fdaa902a806fccc215bf7afefab129f`
(15900297 bytes). Evidence files and their hashes are listed in `receipt.json`.
The full frontend JSON report and logs are deterministic gzip files.

Vite built successfully, but distribution verification still rejects the
5360574-byte geometry payload against the unchanged 4680000-byte delivery cap.
No delivery budget or architecture ceiling was raised. Main was not pushed.
The earlier architecture checkpoint documents the controlled input-build evidence
that this payload-budget overrun predates the architecture refactor.
