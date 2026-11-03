# Current-source WASM delivery investigation

The catalogue remains248/1000 (79 integrated,169 native). This investigation
adds no capability or application qualification credit. The current kernel must
be delivered with its full functionality; an older reviewed kernel must not be
substituted merely to clear manifest checks. The active1000 goal remains open.

All current Rust compiler inputs were snapshotted from262378052 and checked
after the stock build. The only source experiment was a WASM-only inline-never
attribute on bounded rational Work::op. Its patch and build inputs are retained;
the attribute was removed after the frozen frontend run and all stock compiler
input hashes were verified again. No arithmetic, work, geometry, stream or asset
budget was increased. Installed assets were not updated.

| Variant | Optimized WASM bytes | Decoded16MiB limit |
|---|---:|---|
| Stock | 16,794,155 | Exceeds by16,939 |
| WASM-only outlining | 16,778,513 | Exceeds by1,297 |
| Stock + additional strict Binaryen passes | 16,779,903 | Exceeds by2,687 |
| Outlining + additional strict Binaryen passes | 16,764,253 | Fits by12,963 |

The last variant passed102 raw ABI cases and33 exact primitive checks against
fresh native transport output. Five independent exact primitive oracles verify
all11 actual returned definitions and eight negative controls. These are scoped
raw-module checks, not Worker/editor/installed-package or performance claims.

Its production-packer payload was measured in an isolated actual Vite build.
Decoded WASM identity and installed-file preservation were verified. The emitted
geometry chunk is5,928,455 bytes (limit5,370,000); total counted assets are
10,697,401 bytes (limit10,080,000). This candidate is refused for production.
Alternative section/prefix/whole-file compression retained existing stream
bounds and exact Node round trips, but did not solve the payload budget. The
whole-file variants cannot fit even the current sept118 carrier's theoretical
49-bits-per7-digits lower bound. No budget pin was added for a failing candidate.

The full configured frontend run used Node22.23.4, two workers and the committed
30s test/hook deadlines:6203 passed,137 failed,6 pending across6346 tests.
There are29 files with failed assertions and39 further files that failed
collection before assertions ran:68 failed result files in total. Vitest's suite
count also includes nested describe groups.89 assertion failures report an
unqualified mesh runtime manifest.
The observed installed set is inconsistent: tracked public WASM is00f62e02,
while ignored generated identity/kernel data names e86a0bd9. The historical
OwnRust86 reviewed artifact is baf58e70, a third identity/source epoch. Existing
qualification records must not be relabelled as proof of current source.

The full-run snapshot covered tracked inputs (including public WASM), and those
inputs were unchanged. Ignored generated artifacts were not in its whole-run
before/after inventory. Subsequent checks captured their identities; the isolated
Vite run independently checked its protected installed files before/after. A
repeat qualification must explicitly freeze all generated geometry/language/
photogrammetry and decoder artifacts as well. No green result is inferred here.

Two failing larger cancellation bodies were saved as exact model/provenance
requests (514 and578 faces). Their input bytes were retained unchanged through
each raw ABI call. The installed generated kernel refuses both at boundary
embedding. Both current-source stock and outlined/merged kernels independently
prove solid geometry, boundary embedding, all face injectivity, all pair
classification and shell roles with the original budgets. This establishes a
delivery mismatch for these two cases; it does not resolve all137 failures.
The saved requests, result summaries, hashes, source/recipe inventories, failed
frontend report and experiment commands are archived beside the receipt.

Next work is delivery architecture: investigate a binary compressed payload
without ASCII carrier overhead. Bounded loading and ESM initialization could
preserve synchronous host calls after import, but this is unimplemented. It must
prove browser/Node separation, cold calls, worker lifecycle, integrity, unchanged
stream/decode limits, real emitted budgets and full frontend/browser behavior.
Residual failures must then be fixed against the actual current-source artifact.

One collection failure is repaired separately: the72 frozen pre-migration orbit
camera frames were recovered byte-for-byte from Git blob
`2cf65cd5383bc3676c9c6d17815c7686f78e12ae` (before the duplicate math-core crate
was removed), placed under tests/fixtures, and the stale test import was updated.
All12 existing camera tests pass. Expected values and tolerances were not
regenerated or weakened, and no duplicate Rust crate was restored. This focused
result does not supersede the failed full frontend baseline.
