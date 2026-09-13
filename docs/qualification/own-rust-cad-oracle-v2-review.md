# Own Rust direct oracle v2 review

This is a bounded baseline migration for the five existing direct-evaluator
fixtures. It does not qualify new language features, the application release,
or binary compatibility with Manifold. The immutable v1 file remains
`tests/fixtures/own-rust-cad-oracle-v1.json`, SHA-256
`4564124cc236974f353d66944cff953f4ae4fbd71f546d5c10310bf645c0a9c7`.

## Why the current bytes differ

The v1 own-Rust snapshots originated in commit `50c7f8c`. Two later changes
explain the two current mismatches:

- `4e46a9f` (2026-09-11) replaced the cube's rectangular extrusion with a
  direct primitive constructor. This changes cap diagonals and array order.
- `383f9f9` (2026-09-11) replaced polygon-core's sequential `hypot` norm with
  the shared `sqrt(dot(v,v))` norm. Coplanar simplification can consequently
  produce tiny different normal components.

On 2026-09-13, one isolated copy under `/private/tmp` restored only these two
behaviors. It retained the current host code and all other current Rust code.
An offline release build used its own writable `CARGO_TARGET_DIR` and completed
in 44.47 seconds. The authoritative worktree was not modified.

The copied current WASM was
`99088901e508d253d64ec453fa88643f629c61b99c94a80169182b70dcc616af`;
the reconstructed WASM was
`f095e2ef4a199bc9ed6ac50aa51dc55a0d644ac39e4e7f21c70d1d92a26059b2`.
Both were exercised through the production `parseOpenSCAD` entry point and
the independent `referenceLegacyDirectEvaluatorOracle` encoder. Restoring
those two behaviors reproduced **all five v1 LME1 and LSE1 hashes and lengths
exactly**, together with all five recorded volume/area pairs. This is a
diagnostic replay, not a clean release qualification run.

| Fixture | Observed difference from the reconstructed v1 result |
| --- | --- |
| `colored-transform` | Four cap triangles use different diagonals/order. The complete multiset of position/normal pairs is identical. `vertices`, `indices`, BVH triangle slots, edge indices and face IDs differ; the content-derived asset ID changes. |
| `boolean-difference` | The 12 changed geometric triangles all lie on the outer box; cavity triangle geometry is identical. The position multiset is identical. At matched positions, 22 render vertices have normal changes, maximum absolute component difference `1.8670150999996784e-16`. Indices, edge indices and face IDs are identical; vertex bytes, BVH triangle slots and the content-derived asset ID differ. |
| Three sphere/loop fixtures | Every frozen mesh/scene hash, length and aggregate metric is unchanged. |

For both changed cases, bounds, volume, surface area, reported topology,
entity identity, source provenance, color and transform are identical.
The cube remains 12 triangles, volume 24 and area 52. The cavity case remains
236 triangles, volume `60.073403610888725` and area `108.16668943855439`.
Its outer shell and inward-facing cavity remain separate closed components.

The evidence supports a new current byte baseline for these five fixtures.
It does not establish that the norm change is harmless for arbitrary inputs.
Triangle/face ordinals and geometry asset IDs are not compatible across these
two byte layouts; consumers must continue to bind such references to the
specific published geometry/version.

## What is inherited from v1

The v1 JSON contains only mesh/scene lengths and hashes plus aggregate volume
and area. It contains no recoverable arrays, colors, bounds or provenance.
The generator therefore never guesses semantic fields from a hash and never
uses a newly captured value as its own expected invariant.

It reuses the five v1 volume/area pairs and fixed record lengths after checking
the v1 file's immutable digest. It also requires unchanged exact hashes for
the three unaffected fixtures. Fixed source strings, quality and warning
policy are retained from the existing test. Explicit identities, colors,
provenance and topology requirements agree with the full historical outcomes
whose exact LME1/LSE1 hashes were reconstructed. In particular, the Boolean
fixture has a null source provenance run: operand lineage is not invented or
claimed to be qualified.

Additional checks integrate the published Float32 triangles independently:

- Expected source-derived bounds and vertex/triangle counts; finite unit
  normals aligned with the authored triangle winding.
- Exact-position welding, two oppositely directed incidences per edge and
  Euler characteristic two for each closed component.
- One positive-volume component for each cube/sphere, and one positive outer
  box plus a negative cavity component for the Boolean case.
- Independently integrated signed volume and surface area. Published kernel
  metrics retain the pre-existing `1e-9 + 1e-9 * |expected|` tolerance;
  integration of Float32 triangles allows `1e-6 + 2e-7 * |expected|`.
- Exact color, identity, transform, source span/original-ID partition, warning
  order, quality/reduction policy, reported topology and scene deduplication.

Mutation tests reject false metrics, altered source/color, invalid normals,
and wrong winding even when the producer's topology report remains unchanged.

## Reproduce and record the new version

After all source changes have finished, run:

```sh
node scripts/record-own-cad-oracle.mjs --version 2
```

The command rebuilds the geometry kernel, confirms the packed data decompresses
to the raw WASM, and captures all five fixed cases in **two fresh Node child
processes**. Every invariant must pass in both processes, both outputs must
match exactly, and source/WASM/packed-module/decoder hashes must remain stable
during the build and captures. It writes the new
`tests/fixtures/own-rust-cad-oracle-v2.json` only after those checks and refuses
to overwrite an existing v2. Publication atomically links a complete temporary
file after one final source/artifact check; an existing target is never replaced.
It never modifies v1, G0/G1 plans or release
evidence. A later migration requires another reviewed baseline version.

`legacyDirectEvaluatorOracle.test.ts` requires the generated v2 for exact byte
assertions and continues to test fixed semantics independently of those new
hashes. Until v2 has been generated, the missing artifact deliberately blocks
the corresponding baseline tests. Builds and tests cannot silently generate it.

The generator records the source/artifact fingerprints **at capture time**.
Those are traceability metadata, not a requirement that every future unrelated
source edit reproduce the entire source-tree hash. The ongoing regression
contract is the fixed case bytes and semantic invariants. Whole-kernel release
evidence remains a separate explicit `record-own-cad-evidence.mjs` run.

To repeat the diagnostic reconstruction in an isolated copy, copy the current
`crates`, `src`, generator and independent oracle support plus configuration
files; retain access to the installed Node dependencies. In that copy only,
replace polygon-core's imported `norm` with:

```rust
pub(crate) fn norm(v: [f64; 3]) -> f64 {
    v.iter().fold(0_f64, |n, x| n.hypot(*x))
}
```

Restore the `cube` function from
`git show 50c7f8c:crates/polygon-kernel/src/cad.rs`, changing its extrusion call
to the current `crate::solid::modeling::extrude_rings` location. Build with a
target directory inside the isolated copy, pack its generated WASM, then run
the read-only capture mode:

```sh
CARGO_TARGET_DIR="$PWD/cargo-target" cargo build --offline --locked --release \
  --config 'profile.release.strip="symbols"' --target wasm32-unknown-unknown \
  --manifest-path crates/geometry-wasm/Cargo.toml
node --input-type=module <<'JS'
import { readFileSync, writeFileSync } from 'node:fs';
import { brotliCompressSync, constants } from 'node:zlib';
const wasm = readFileSync('cargo-target/wasm32-unknown-unknown/release/geometry_wasm.wasm');
const size = Buffer.alloc(4); size.writeUInt32LE(wasm.length);
const compressed = brotliCompressSync(wasm, { params: { [constants.BROTLI_PARAM_QUALITY]: 1 } });
writeFileSync('src/generated/geometry-kernels/kernel_bg.wasm', wasm);
writeFileSync('src/generated/geometry-kernels/bytes.ts',
  `export default '${Buffer.concat([size, compressed]).toString('base64')}'\n`);
JS
node --import tsx scripts/record-own-cad-oracle.mjs --capture
```

The diagnostic capture mode prints hashes and verified measurements; it writes
no baseline. Compare its five mesh/scene hashes to retained v1. A result from
a different future source revision is supplemental evidence and must not be
represented as the recorded 2026-09-13 replay.
