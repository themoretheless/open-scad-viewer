# Owned B-rep decoding: 2026-09-19

## Change

`Model::from_value` receives an owned `value_codec::Value`, but previously
borrowed its object and cloned the entire object tree before removing
`topologyIds`. It now destructures `Value::Object` and consumes the existing map.
No validation branch, error message, field policy, identity migration or format
changed. Non-object input still returns `Expected B-rep object`.

This removes one full recursive clone. It is **not zero-copy decoding**:
`geometry-bridge::field` still clones its borrowed field, and downstream typed
deserializers have their own allocation/copy work. Peak process memory was not
measured, so no quantitative memory-reduction claim is made.

## Native benchmark

```sh
npm run bench:brep-decode
```

Canonical-ID herringbone gear fixtures, three warmups and nine samples each.
Input tree cloning is outside the timed interval. The timer covers owned
`from_value::<Model>` only, excluding fixture construction, JSON/binary parsing,
result validation, serialization and result destruction. Every result is compared
to the complete canonical serialization outside the timer. Reports include its
SHA-256 and byte length.

| Teeth | Faces | Canonical bytes | Before median | After median | Reduction |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 12 | 158 | 1,225,421 | 10.757 ms | 8.913 ms | 17.1% |
| 32 | 418 | 3,273,606 | 29.189 ms | 23.496 ms | 19.5% |
| 60 | 542 | 3,762,582 | 36.400 ms | 29.611 ms | 18.7% |

All three canonical hashes match. Raw samples:
`tmp/performance/brep-decode-before.jsonl` and
`tmp/performance/brep-decode-after.jsonl`. Both runs used the same release profile
and rustc 1.100.0-nightly (`923c95cdf`, 2026-09-16) on this machine. CPU affinity
and frequency were not controlled. These numbers do not imply equivalent
end-to-end browser gains or gains for the legacy identity-generation path.

## Native verification

938 tests passed across brep-core, nurbs-core and geometry-bridge. Existing tests
cover canonical and legacy IDs, change-set compatibility, malformed rational
definitions and partial identity tables. The new regression test pins the
non-object error for null, boolean, number, string and array values.

The preceding surface-validation optimization and its independent benchmark are
documented in [validated face sampler reuse](brep-validation-2026-09-19.md).

## Browser and delivery

Baseline: `tmp/performance/brep-validation-browser-repeat/report.json`.
New runs: `tmp/performance/brep-decode-browser-after/report.json` and
`tmp/performance/brep-decode-browser-repeat/report.json`.

The 20-body spinner took 7,509.8 and 7,505.9 ms versus the preceding 7,641.2 ms.
Largest main-thread timer gaps were 388.7 and 383.0 ms versus 402.4 ms. This is a
small local difference, not evidence of 17-20% overall application acceleration.
No profiler, build or test suite ran alongside these browser measurements.
Source and geometry hashes match the baseline; emitted JS hashes match between
the two new runs. Hull refusal, cancellation, recovery, group replacement and
scene/editor preservation all passed in the real browser check.

- Rebuilt kernel: 7,530,983 bytes, SHA-256
  `076ded32f962ce32b043c20e7fd863b8cac15a0986bec2d445ba4e9bca5691d3`.
- Vite output: 5,933,130 bytes, down 70 bytes. Packed payload identity and
  individual artifact checks pass; the total 5,600,000-byte gate still fails.
- Full Vitest with this WASM: 3192 passed, 14 failed in 330 files. Failure titles
  exactly match the preceding surface-sampler run. No qualification evidence or
  test expectation was rewritten to hide those failures.
- `git diff --check` passed.
