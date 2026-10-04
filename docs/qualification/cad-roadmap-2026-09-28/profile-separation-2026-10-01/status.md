# Early curve separation proof — 2026-10-01

Native changes verified; new WASM/runtime qualification pending.

`curve_distance::prove_separation` reuses the original global interval distance search but stops when its lower bound is strictly positive. Initialization still covers every original knot-span pair. The original `distance` endpoint retains its full tolerance contract. Early separation reports `reason=separated` and `converged=false`; it never claims precise distance convergence. Contact, crossing, work/precision limits preserve explicit unproven results.

Trim-loop simplicity, trim-region auditing and retained-profile pair separation now use this mode because their acceptance requires a positive lower bound rather than a precise minimum distance. No proof condition is weakened and the source curves are immutable.

Tests include a positive-weight rational arch and line with one-cell separation, compared against the same full-distance search requiring more cells. Initial coverage overflow, a late-span endpoint contact, crossing and exact interior tangency never receive a separation proof. There is no measured UI latency or universal speedup claim.

Validation: 230 NURBS library tests passed; two focused tests including the added tangency passed; three profile-region tests passed; 682 full B-rep tests passed, three existing roadmap cases ignored. Logs are adjacent. `git diff --check` passed.

Live WASM build: exec session 59023; log `/private/tmp/cad-profile-separation-wasm.log`. Poll this same handle; do not restart a quiet optimizer. After terminal success, re-run profile/real-worker and full CAD tests, distribution checks, and the current general-profile browser scenarios. No commit/push or browser qualification for this stage yet. The complete P0–P3 goal remains active.

## Final qualification

Build session 59023 completed. Native, 835 CAD, mouse/keyboard browser and independent STEP checks passed. See `final-status.md`; it supersedes the pending handoff above. Primary idle comparison: 1195.987 ms → 730.259 ms on the fixed Node/WASM fixture. The complete goal remains open.
