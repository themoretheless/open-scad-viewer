# Worker failure lifecycle evidence

The expression inspection checker now persists its failure artifact only after disposing the client and awaiting every worker exit. It includes completed phase timings and workersJoined, so a saved timeout report proves the observed workers have exited rather than merely recording that termination was requested.

The deliberate 1 ms timeout control exited with the expected code 1 and CAD_TIMEOUT in budget-refusal. One worker started and one worker joined; published WASM remained unchanged. Evidence: `worker-timeout-joined-2026-10-10/expression-worker-failure.json` and `control-result.json`. This is a diagnostic checker improvement, not a production geometry or latency qualification.
