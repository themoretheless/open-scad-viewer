# Bound retained B-rep inspection keys

Direct document validation previously remembered up to 128 inspected B-reps
using their complete JSON strings as keys. The number of entries was bounded,
but their aggregate serialized size was not. Large documents and successive
edited versions could therefore retain many large keys after the corresponding
document snapshots were no longer needed.

`BrepInspectionCache` now owns exact-content keys, their summed character count,
and FIFO eviction in one module. Production limits are 128 entries and
`MAX_DOCUMENT_CHARACTERS` (67,108,864 UTF-16 code units). The character limit
matches one admitted document, so its unique B-reps can fit without new
size-driven thrashing. The existing 128-entry bound still applies: documents
with more unique bodies can already churn for that independent reason.

This is a bound on retained key content, not on total JS heap, process RSS,
WASM memory or temporary JSON allocation. At two bytes per code unit the key
payload would occupy up to 128 MiB, plus implementation-specific overhead;
engines can also store ASCII strings more compactly. No actual heap saving is
claimed without a heap measurement.

## Correctness and ownership

- Every call serializes current content, so clones hit and mutations do not.
- No weak content hash or identity-only cache can bypass validation.
- Failed inspection never inserts or evicts an entry.
- An individually oversized model is inspected but not retained; it does not
  flush the useful working set.
- FIFO hit behavior and the 128-entry bound match the previous implementation.
- Only strings are retained, not mutable B-rep object references.
- Constructor limits are copied and checked for positive safe integers.

The cache is validation memoization, not a geometry authority. Invalid topology
still reaches the native inspector and throws. Document mesh/bounds/group/ID
checks remain outside the cache and run as before.

## Workload check

Run `node --import tsx benchmarks/direct-validation.mts [output-json]`.
The benchmark builds the existing planetary-spinner fixture, normalizes only
random body IDs, and validates the same document repeatedly. Three warmups,
nine measured samples; fixture construction and equality checks are excluded.
The separate first-validation diagnostic includes reference serialization and
must not be compared to the warm samples as if it measured the same interval.

Local Apple M4 Max, Node v22.23.2, macOS arm64:

| Property | Value |
| --- | ---: |
| Bodies / unique B-reps | 20 / 20 |
| Full document characters | 19,852,257 |
| Unique retained B-rep key characters | 14,517,080 |
| Previous warm validation median | 317.20 ms |
| Bounded-cache median / repeat | 315.71 / 317.15 ms |

Performance is effectively unchanged on this working set. All three runs have
the same normalized-document SHA-256. The purpose is bounded retention, not a
speedup. These are in-process synchronous API timings, not browser frame-time
measurements. No build or test ran alongside the timed benchmark.

Evidence: `tmp/performance/direct-validation-before.json`,
`direct-validation-after.json`, `direct-validation-after-repeat.json`. The
repeat report additionally fingerprints the new cache module.

## Verification

96 targeted tests passed, including cache hit/mutation/refusal behavior,
character/count eviction, oversized bypass, invalid limits, document validation,
history and worker cancellation. Strict TypeScript checks passed.
Log: `tmp/performance/brep-cache-tests.log`.

Full regression after both the cache and history changes: 3,216 passed, nine
failed across 332 files (133.25 seconds). The failures are the existing
historical engine-manifest and G0/G1 qualification fingerprint checks; no
historical evidence was rewritten. Log: `tmp/performance/brep-cache-full-tests.log`.

UI and MCP typechecks and Vite production build passed. `verify-dist` still
rejects the aggregate size: 5,934,741 bytes versus the unchanged 5,600,000-byte
budget. Logs: `tmp/performance/brep-cache-vite-build.log` and
`tmp/performance/brep-cache-dist-check.log`. This change is not a release gate
pass and does not resolve bundle-size or qualification debt.
