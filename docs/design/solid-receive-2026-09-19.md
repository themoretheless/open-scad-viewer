# Cooperative Solid document validation: 2026-09-19

Follow-up: [validated face sampler reuse](brep-validation-2026-09-19.md) removes
repeated Rust surface checks within an individual B-rep inspection. The numbers
below remain the preceding scheduling-only baseline.

## Evidence and scope

The exact-solid worker removed compilation and exact construction from the main
thread, but receiving the 20-body planetary spinner still caused a 1,295.3 ms
timer gap. A main-thread Chrome CPU profile was captured before this change:
`tmp/performance/solid-receive-profile/main-thread.cpuprofile`.

Across the isolated browser workloads, the profile attributes approximately
806 ms of self time to geometry WASM, 251 ms to the binary codec, and 141 ms to
direct-modeling JavaScript, with 153 ms of GC and 9.1 seconds idle. These are
sampled totals across all fixtures, including receiving-realm initialization,
not exclusive timings for the spinner's final receive stage. Benchmark hashing
is also within the profile. Profiled wall times are not used as the baseline.

The evidence supports interrupting the consecutive validation work between
bodies before attempting a serialization rewrite or bypassing topology checks.

## Implementation

`directDocumentValidation` is the single generator implementing the existing
document checks. `parseDirectDocument` drains it synchronously for history and
imports. `parseDirectDocumentAsync` drains the same generator cooperatively for
worker results, returning to the event loop between objects after at least 8 ms
of work. This is an opportunistic budget, **not an 8 ms upper bound**: a single
kernel call, initial JSON parse and final normalization clone are synchronous.

The receiving client owns a separate AbortController for validation. Every
terminal outcome aborts that work; a user abort or timeout during a yield cannot
leave validation running and later publish a stale document. No partially checked
document is returned. All existing mesh, identity, group, NURBS, topology and
mesh/B-rep correspondence checks remain shared by both adapters. JSON normalization
and independent ownership are unchanged.

## Browser measurement

Same source files, three fixtures, actual production bundle, fresh worker per
request, local headless Chrome Dev. Baseline:
`tmp/performance/solid-receive-before/report.json`; after:
`tmp/performance/solid-receive-after/report.json`.

| Fixture | Before wall time | After wall time | Before largest timer gap | After largest timer gap |
| --- | ---: | ---: | ---: | ---: |
| Box | 251.6 ms | 249.8 ms | 65.7 ms | 63.3 ms |
| Two boxes | 140.0 ms | 139.2 ms | 18.3 ms | 17.8 ms |
| 20-body spinner | 9,937.4 ms | 10,037.8 ms | 1,295.3 ms | 530.9 ms |

The spinner's largest observed timer gap decreased 59%. End-to-end time did not
improve; the ~1% difference is not a throughput claim. This is a local observation,
not a statistical distribution or a frame-rate guarantee. All three geometry
hashes are identical. The browser also checked named hull refusal, cancellation,
recovery, real App body creation, group replacement and preservation of the scene
and editor on cancellation.

A second unprofiled run with identical JS artifact hashes measured 9,964.5 ms
wall time and a 516.8 ms largest gap for the spinner. All three geometry hashes
matched again. Its report is
`tmp/performance/solid-receive-after-repeat/report.json`.

Run a non-profiled check with:

```sh
EXACT_SOLID_BENCH_OUT=tmp/performance/solid-receive-after \
CHROMIUM_EXECUTABLE='/Applications/Google Chrome Dev.app/Contents/MacOS/Google Chrome Dev' \
  node benchmarks/exact-solid-browser.mjs
```

Set `EXACT_SOLID_PROFILE=1` and use a different output directory for a diagnostic
CPU profile. Do not compare profiled timings to an unprofiled run. Reports include
source and emitted-JavaScript SHA-256 identities, browser version and platform.

## Remaining work

- A ~0.53-second single gap is still visible; inspect individual large-body
  validation and the binary encoding boundary before further scheduling changes.
- JSON parsing/cloning and synchronous history publication remain whole-document
  operations. No immutable geometry ownership redesign has been completed here.
- The inspection cache retains full JSON keys, bounded by 128 entries rather than
  byte weight. A byte bound needs a realistic working-set measurement to avoid
  turning repeated large-document validation into cache thrashing.
- Normal dist is 5,932,985 bytes, 500 bytes larger than the preceding worker build.
  Per-artifact checks pass; the 5,600,000-byte total budget still fails.

## Verification

- 51 targeted async-validation, direct-modeling, worker and UI tests passed.
- Full Vitest: 3192 passed, 14 failed in 330 files. The failure titles match the
  preceding worker-transfer run exactly; no qualification archives were rewritten.
- App/MCP typechecks and a separate strict check of the new test files passed.
- Vite build and `git diff --check` passed. The total dist size gate remains red.
