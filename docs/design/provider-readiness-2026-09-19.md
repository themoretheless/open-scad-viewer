# Cold provider readiness

Node 22 CI job 105961446609 reports multiple provider readiness failures after
250 ms. This is not simply the previously recorded historical fingerprint drift.
`GeometryBuildEngine.ensureProviderReady` races one shared warmup per provider
against that deadline. A timed-out attempt stays in flight; subsequent requests
return unavailable immediately until it settles. Successful late completion
restores availability. Failed initialization stays unavailable for that engine.
No fallback to another geometry class is permitted.

Warmup includes unpacking the geometry WASM, asynchronous compilation and
instantiation, and CAD wrapper setup. Runtime module imports occur before the
engine's admission timer in this benchmark. A timeout is a readiness admission
failure, not cancellation of compilation. The existing deadline is unchanged.

## Measurement

Run `npm run bench:readiness`. Three sequential fresh Node processes report
module-import time separately from cold readiness, time until warmup settles,
and warm-query time. The report includes runtime/platform/CPU, packed kernel
source SHA-256 and both providers' availability/reasons. It does not reuse a
previous child's WASM cache or realm, or assert that cold admission must pass.
It asserts eventual availability and bounds each child process to 60 seconds.

First local run: macOS arm64, Node 22.23.2, cold readiness 117.0-117.9 ms,
warm query 0.019-0.022 ms; both providers available in all three samples.
Current repeat including identity metadata:
`tmp/performance/provider-readiness-local.json`. This does not reproduce the CI
failure or prove that 250 ms is a sufficient cold-start budget on other CPUs.
It is a small startup sample, not a p95/p99 or contention benchmark.

The CI check matrix records this benchmark after its kernel build and uploads
`provider-readiness-node-<version>` even if later checks fail. The probes run in
separate processes and do not prewarm the test workers. That workflow change is
local and has not executed on GitHub yet. Compare this idle baseline with suite
failures before attributing failures to CPU contention or changing admission
policy.

All 19 GeometryBuildEngine tests pass, including a new fake-timer test proving
timeout, immediate refusal while pending, late recovery, single warmup, and
timer cleanup. Existing hung-provider refusal remains intact. No production
timeout, retry policy or fingerprint assertions were weakened.
