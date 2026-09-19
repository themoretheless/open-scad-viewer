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
# CI and contention follow-up

The downloaded `provider-readiness-node-22` artifact from CI run 35470505256
(commit 2dbec2f833dc1bfe32743e2d079a41219a218164) reports fresh-process
readiness at 241.15, 229.97 and 222.42 ms on Linux x64 / AMD EPYC 7763,
Node 22.23.2. Warm checks were 0.031-0.032 ms. All three sequential cold
samples were available, but the margin to the unchanged 250 ms deadline was
only 9-28 ms. That same run's test log contains repeated readiness-timeout
failures. The macOS MCP smoke also failed its first analysis, but its printed
assertion does not expose the failure cause; do not assume the same cause.

The benchmark now supports `READINESS_CONCURRENCY=1..16` (default 1), with
three sequential waves and concurrent fresh children within each wave.
`Promise.allSettled` joins all children before surfacing any error; each child
retains the 60-second bound and eventual-availability assertion. Cold timeout
remains an observed outcome, not a benchmark failure that discards the report.

Local control (same current generated kernel, no concurrent build/test):
three serial samples 86-87 ms; 24 samples in eight-child waves 99-108 ms.
All cold checks succeeded. Thus local contention increases startup cost but
does not reproduce the CI failure. These absolute values are not comparable
to CI as a speedup: CPU, artifact and host conditions differ.

The check matrix now collects both sequential and four-child-wave reports
in the existing readiness artifact. This workflow edit has not yet run in CI.
No production readiness deadline, provider selection, fallback policy, or
qualification expectation is relaxed by this diagnostic change.

Evidence downloaded locally to
`/private/tmp/osv-readiness-ci-35470505256/provider-readiness.json` and
`/private/tmp/osv-ci-failed.log`; new local measurements are
`/private/tmp/osv-readiness-serial.json` and
`/private/tmp/osv-readiness-concurrent.json`.

## CI contention reproduced on fd03ebcf

Run 35475827899, Node 22 job 105984846087, now supplies both reports. This
supersedes the earlier note that the concurrent workflow had not yet run.
Node v22.23.2, Linux x64, Intel Xeon Platinum 8573C:

| Workload | Samples | Cold available | Cold response ms | Warmup settled ms | Warm response ms |
| --- | ---: | ---: | --- | --- | --- |
| Sequential fresh processes | 3 | 3/3 | 196.852-219.658 | 196.858-219.663 | 0.027-0.043 |
| Four concurrent fresh processes, three waves | 12 | 0/12 | 251.408-341.990 | 355.973-501.295 | 0.099-0.262 |

Both mesh and B-rep are unavailable with `readiness-timeout` in every concurrent
cold sample, and available after the original warmup settles. The benchmark exits
successfully because eventual availability is asserted; that success is not a
claim that cold admission meets 250 ms. Timer callbacks themselves can be delayed
under load, so an observed timeout response may exceed the nominal deadline.
These results isolate within-run contention; they are not a speedup comparison
against the earlier AMD runner.

The same Node 22 job reports 3260 passed, 17 failed and five skipped tests.
There are remaining readiness failures in `geometryWorkerAssertion.test.ts` and
`mcpGeometryService.test.ts` (the latter directly calls the in-process
`HeadlessGeometryService`), plus unavailable capability expectations in
`geometryBuildEngine.test.ts`. The disposable-worker startup fix therefore does
not cover every cold initialization path. Oracle/HTTP test timeouts and MCP stdio
shutdown failure are separate observations; do not attribute them to readiness
without a causal trace. The Manifold shadow test's printed failed-build result
also does not identify its underlying failure cause.

Next lifecycle work must cover browser workers and in-process service startup
with explicit initialization, cancellation and deadline ownership. Do not hide
these failures by globally increasing 250 ms or reducing CI concurrency. The
qualification/fingerprint failures remain independent problems.

Downloaded reports:
`/private/tmp/osv-readiness-ci-35475827899-node22/provider-readiness.json` and
`/private/tmp/osv-readiness-ci-35475827899-node22/provider-readiness-concurrent.json`.
Completed-job log: `/private/tmp/osv-ci-35475827899-node22-job.log`. The CLI run-log
command waits for the entire run; the completed job's REST logs endpoint provides
this evidence while other jobs are still running.

Node 20 confirmation from the same run: v20.19.6 on AMD EPYC 9V74 passed all
three sequential cold probes (216.962-218.887 ms), but all 12 four-process cold
probes timed out (250.765-370.287 ms). Original warmup settled in
436.765-532.312 ms; subsequent warm probes took 0.089-0.150 ms and were available.
Reports are under `/private/tmp/osv-readiness-ci-35475827899-node20/`.
Both runtime versions therefore reproduce contention locally within their own
runner; the different CPU hosts do not support a Node-version speed comparison.

Subsequent fixes are documented in
[browser worker startup](browser-worker-cold-start-2026-09-19.md) and
[in-process startup](inprocess-cold-start-2026-09-19.md). They are not present in
the CI run whose measurements are reported above.
