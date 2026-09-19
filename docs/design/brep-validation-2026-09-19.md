# Reuse validated face samplers: 2026-09-19

## Finding

`brep_core::Model::validate` validated each face surface, then called
`Surface::evaluate` for nine parameter samples on every coedge. `evaluate`
validates the entire control net again, including constructing and checking
axis curves. Large helical gear faces therefore repeated the same immutable
surface validation many times during one model inspection.

The implementation now uses the existing `SurfaceSampler` once per face. Its
constructor performs the same validation and owns an immutable snapshot;
subsequent samples use the same evaluator without repeating the complete net
validation. Snapshot lifetime ends with that face. No persistent cache, unsafe
code, new evaluator, altered sample count, relaxed tolerance or ABI change was
introduced. Curve and pcurve checks remain unchanged.

This preserves sampled geometric agreement, not a proof of exact solid validity:
the report still says `sampled_with_tolerance` and `not_certified`.

## Native benchmark

```sh
npm run bench:brep-validation
```

The example constructs each fixture outside the timing, performs three warmups
and records nine complete validation calls. Same herringbone gear definitions,
compiler and release settings on both sides. Compiler: rustc 1.100.0-nightly
`923c95cdf`, 2026-09-16. CPU was not isolated or frequency-locked.

| Teeth | Faces | Before median | After median | Reduction |
| ---: | ---: | ---: | ---: | ---: |
| 12 | 158 | 49.37 ms | 20.41 ms | 58.7% |
| 32 | 418 | 133.01 ms | 54.54 ms | 59.0% |
| 60 | 542 | 157.12 ms | 69.74 ms | 55.6% |

Raw samples: `tmp/performance/brep-validation-before.jsonl` and
`tmp/performance/brep-validation-after.jsonl`. These measurements exclude host
encoding, decoding, worker startup and Solid history publication.

## Browser

The rebuilt production WASM was exercised with the existing exact-solid browser
check. Baseline: `tmp/performance/solid-receive-after-repeat/report.json`.
After: `tmp/performance/brep-validation-browser-after/report.json`.

| Fixture | Before wall time | After wall time | Before largest timer gap | After largest timer gap |
| --- | ---: | ---: | ---: | ---: |
| Box | 260.0 ms | 252.4 ms | 56.9 ms | 66.8 ms |
| Two boxes | 138.8 ms | 139.9 ms | 18.0 ms | 17.8 ms |
| 20-body spinner | 9,964.5 ms | 7,628.6 ms | 516.8 ms | 414.7 ms |

Spinner wall time decreased 23.4% in this local run; the largest callback gap
decreased 19.8%. Small fixtures are dominated by startup and show no convincing
improvement. This is not a statistical browser benchmark or a frame-rate claim.
All input and geometry SHA-256 values match. Named hull refusal, cancellation,
recovery, real App group replacement and preservation of scene/editor on cancel
also passed. Remaining single gaps around 0.4 seconds are still user-visible.

A second unprofiled run on identical JS artifacts measured 7,641.2 ms wall time
and a 402.4 ms largest gap for the spinner. Geometry hashes again matched.
Report: `tmp/performance/brep-validation-browser-repeat/report.json`.

## Artifacts and checks

- Native B-rep/NURBS suite: 672 passed. New tests compare every coedge sample on a
  gear with checked surface evaluation, reject invalid weights and changed surface
  geometry, and retain sphere-pole and closed-cylinder validity.
- Native geometry-bridge integration suite: 265 passed (937 native tests total).
- Full Vitest with the rebuilt WASM: 3192 passed, 14 failed in 330 files. The
  failure titles exactly match the preceding cooperative-validation run. Historical
  qualification fingerprints and refusal/MCP expectation failures remain unresolved.
- App and MCP typechecks, Vite build and `git diff --check` passed.
- Production kernel: 7,530,995 bytes; SHA-256
  `6d91fc2242d7d975503a08a2401d468a62de9f08e0b9c00656aba85c5a7bbbc9`.
- Vite output: 5,933,200 bytes, up 215 bytes. Individual payload/identity checks
  pass; the 5,600,000-byte total gate still fails. No budget was increased.

The existing receive-side scheduling improvement is described in
[cooperative validation](solid-receive-2026-09-19.md). Remaining candidates include
binary request encoding and repeated value-tree cloning at the Rust boundary;
neither was changed in this step.
