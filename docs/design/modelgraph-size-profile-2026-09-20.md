# ModelGraph language profile control

Follow-up to the [OpenSCAD profile control](language-size-profile-2026-09-20.md).
ModelGraph uses the Rust language kernel in production; its fused source-to-
prepared-graph path must be measured before adopting PR 21's global `s` to `z`
release profile change.

## Method

`benchmarks/modelgraph-language-profiles.mts --baseline <wasm> --candidate <wasm>`
runs ABI operation 4 with the production host codec/allocation/free helpers.
It checks every complete response against `prepareGraphRust('text', source)`
from the current product, outside timing. This covers source compilation,
canonical graph preparation, request encoding, response decoding and frees.
It does not time WASM compilation, startup, geometry execution, worker transfer,
UI, or arbitrary invalid-input admission. No qualification claim is made.

The isolated build tree `/private/tmp/osv-cleanup-check.1thRbw` has byte-identical
source trees for languages-bridge, languages-wasm, modelgraph-runtime,
modelgraph-text, openscad-core, value-codec and math-core. `s` is byte-identical
to the current shipped language WASM. Rebuild with
`CARGO_PROFILE_RELEASE_OPT_LEVEL=z node scripts/build-language-kernel.mjs`
using unchanged packaging/Binaryen settings. Main's generated artifacts and
Cargo profile are never replaced by this experiment.

Each campaign uses independent module instances, 20 warmups and 31 alternating
paired samples per fixture. Campaign B reverses the baseline/candidate roles,
including compilation and initial call order. No local builds/tests run during
either campaign. Node 22.23.2, macOS arm64; allocation/GC included, no CPU pinning.
Fixture and response hashes match across both campaigns.

| Fixture | s A, ms | z A, ms | s B, ms | z B, ms |
| --- | ---: | ---: | ---: | ---: |
| ring-pattern | 0.363375 | 0.445417 | 0.362792 | 0.447334 |
| indented-functions | 0.248500 | 0.331000 | 0.256209 | 0.339125 |
| skadis-box-linq | 2.117500 | 2.812666 | 2.137000 | 2.840666 |
| planetary-spinner | 1.047667 | 1.322959 | 1.068917 | 1.341208 |
| repeat-16 | 0.113208 | 0.143875 | 0.121041 | 0.149209 |
| repeat-128 | 0.636458 | 0.781625 | 0.633833 | 0.781458 |

`z` is approximately 23-33% slower on these workloads, with identical results.
That is a frontend preparation penalty, not a 23-33% complete-model slowdown.
Its raw module is 1,133,177 bytes versus 1,341,106 (`s`), 15.5% smaller.
HTTP compression/network effects are not measured here.

Artifact SHA-256:

- `s`: `541a186889fc102a8f26486c614a4a7c2ea6d9261fbaf72cf4cd795e186109c4`.
- `z`: `ce696669bd397fcc06cfb45419bb525df5217cd690086698b3497a464f0efe8b`.

Reports: `/private/tmp/osv-modelgraph-profile-sz-a.json` and
`/private/tmp/osv-modelgraph-profile-zs-b.json`. The benchmark records artifact,
fixture, response and harness hashes with all samples. This is not a complete
reproducible toolchain attestation.

## PR 21 Disposition

All four changed paths in `themoretheless-reduce-wasm-size` are accounted for:
the fetched head is `a393af1a315c64941206be6f2aed9063061343db`, comprising
`b690edbe` and `a393af1a`. Main before this reconciliation is `6e086014`.

- `crates/Cargo.toml`: do not adopt global `z`. Keep `s` and the individually
  measured package overrides; a smaller file alone is not sufficient evidence
  to accept this observed preparation slowdown.
- `src/generated/photogrammetry/bytes.ts`: do not restore an old generated
  payload. Keep artifacts regenerated from the retained package profiles;
  the branch's historical base64 module is not an independent source feature.
- `docs/photogrammetry-improvements.md` and `docs/photogrammetry-performance.md`:
  retain the useful historical-profile clarification, corrected to match the
  actual retained settings. Do not publish the branch's claim that the current
  workspace has switched to `z`.

The reconciliation merge preserves the experiment's original commits in history
and integrates the documentation intent and executable measurement, while
explicitly rejecting the global profile change. It must not be reported as a
production switch to `z` or a performance win. No runtime/source/WASM behavior
changes in this reconciliation. Existing qualification-binding failures remain.
