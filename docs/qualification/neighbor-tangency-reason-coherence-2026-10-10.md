# Neighbor tangency response coherence

Transport validation now accepts a qualified neighbor tangency certificate only with the native qualified reason. Conversely, an unqualified response cannot carry the qualified reason. Native geometry remains Rust-owned; this rejects contradictory response authority before display.

Regression: altered native admitted responses carrying budget-exhausted, angle-unproven or unknown reasons are rejected. An unproven response carrying the qualified reason is rejected. The positive numeric certificate control still passes, and incomplete/nonfinite/over-tolerance/over-budget bounds are independently rejected.

Validation: four transport/controller/panel files passed 72 tests; focused updated controls passed 5 tests; vue-tsc exited 0; git diff --check passed. Logs `/tmp/neighbor-tangency-reason-coherence-20261010.log`, `/tmp/neighbor-tangency-reason-controls-20261010.log`, `/tmp/neighbor-tangency-reason-typecheck-20261010.log`. No general tangency or complete command matrix qualification follows.

Full expression-inspection worker reruns for admitted and zero-budget tangency both terminated with CAD_TIMEOUT at 120 seconds under concurrent native load. They are failed timing gates, not tangency qualifications. Logs `/tmp/neighbor-reason-public-worker-20261010.log` and `/tmp/neighbor-reason-unproven-public-worker-20261010.log`. The separate restore-worker checker now supports the same optional native neighbor-tangency request to isolate transport compatibility; it does not replace the full diagnostic gate.

Restore-only admitted tangency worker qualification passed on published SHA dded25909e53fca0021abbc78b00a89e28be5869ff51a8517f9d0b6a98031adf: native budget refusal, dispatched cancellation, terminated worker, fresh-worker retry, closed incidence (29 faces), and actual-neighbor tangency passed. Report: `neighbor-reason-restore-worker-2026-10-10/expression-worker-report.json`. The two full-inspection timing failures remain open.

Both worker checker scripts now preserve a structured failure artifact with request phase, elapsed time, WASM identity and native/client error code before joining workers and rethrowing. TypeScript syntax transformation passed; failure artifact runtime path is not yet exercised. The admitted restore run predates this instrumentation. Zero-budget tangency restore is running separately.

Zero-budget tangency restore worker gate also terminated successfully on the same dded2590… publication: native refusal, cancellation, replacement and Retry preserved qualified=false. Report: `neighbor-reason-unproven-restore-worker-2026-10-10/expression-worker-report.json`. This run used phase/failure instrumentation, but the failure path itself was not exercised.

The same status/reason coherence checks now cover original-support tangency and each section/rail join. Qualified section/rail reasons cannot be interchanged or replaced by regularity/work-limit reasons; unqualified reports cannot carry the qualified report reason. Added altered-native-response regression cases. The four-file transport/controller/panel gate terminated successfully; log `/tmp/support-tangency-reason-coherence-20261010.log`.

The failure-artifact runtime path was exercised with an explicit 1 ms qualification timeout: checker exited 1, recorded phase=budget-refusal, code=CAD_TIMEOUT, unchanged WASM identity, and joined the worker before exit. Expected-failure control assertions passed; evidence `worker-timeout-failure-control-2026-10-10.json`. Default qualification timeout remains 120000 ms. This control neither passes the full inspection gate nor qualifies latency.

The single phased full-inspection rerun terminated successfully with the unchanged 120000 ms per-request limit and published WASM identity. Report: `neighbor-reason-phased-inspection-worker-2026-10-10/expression-inspection-worker-report.json`. This is successful evidence for the same full diagnostic control, not a removal of the earlier timing failures or proof of universal latency.
