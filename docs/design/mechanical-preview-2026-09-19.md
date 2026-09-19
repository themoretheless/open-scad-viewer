# Mechanical preview isolation

The default planetary generator builds 52,064 display triangles. The CPU PNG
renderer rejects more than 20,000 triangles before rasterization. Switching only
the geometry service quality to `preview` still produced 52,064 triangles: the
generated source explicitly sets `$fn = 48`. Changing `flank_segments` also did
not reduce that retained-B-rep display mesh.

`modelgraph_generate` now keeps its full build for analysis and the returned
document, then compiles a separate `segments: 12` document only when the full
mesh exceeds the image triangle budget. The default assembly produces 10,240
triangles and three PNG views. No mesh faces are dropped or randomly sampled.
The renderer's triangle and eight-million raster-work limits are unchanged;
larger models can still return `images_status: unavailable` with an error.

`preview_document_sha256` identifies the document used to attempt the image
build, while `document_sha256` continues to identify the authoritative document.
The reduced document is not returned in place of the original and does not
change measurement or export input. Cancellation during the additional build
is checked before publishing a successful response.

## Measurement

Run `node --import tsx benchmarks/mechanical-preview.mts [output-directory]`.
The default output directory is `tmp/performance/mechanical-preview`; it contains
raw timings, environment and implementation fingerprints, and the three PNGs.

Three warmups, nine measured sequential samples, warm in-process service:

| Phase | Median |
| --- | ---: |
| Full build | 101.76 ms |
| Additional preview build | 21.51 ms |
| Three-view rasterization | 15.00 ms |

This is additional work that restores missing images, not an end-to-end speedup.
Cold startup, MCP transport, and production worker isolation are not measured.
All nine samples had identical full-detail volume and per-view PNG hashes.
The isometric output was visually inspected; all five gears are present.

## Contracts

MCP integration checks still require three images and successful 3MF export for
gear, planetary gears, and thread. They additionally require the returned
document to retain `segments: 48`, and only the default planetary image path to
use a different document hash.

Once the image assertion passed, it exposed an older expectation that eight
teeth must be rejected. Current gear code intentionally supports radial flanks
below the base circle; the undercut tooth threshold is a report, not an input
limit. The test now checks that evidence explicitly and retains rejection at
two teeth, outside the supported 3..256 range. The MCP guide matches that
contract and does not claim generated undercut or manufacturing certification.

This change is intentionally confined to mechanical generation. General
ModelGraph reports may contain explicit per-node tessellation or sampled input
meshes; changing a global segment value would not reliably reduce them.

## Verification

- Targeted MCP, preview and mechanical suites: 35 passed.
- Preview suite with explicit triangle/raster budget guards: 3 passed.
- Strict TypeScript check of changed tests and benchmark: passed.
- Full suite: 3,198 passed, 11 failed across 330 files (133.71 seconds).
  Remaining failures are the two previous B-rep refusal expectations and nine
  historical manifest/qualification fingerprint checks. No historical evidence
  was regenerated to make these tests pass.
- Logs: `tmp/performance/mechanical-preview-tests.log`,
  `mechanical-preview-limits.log`, and `mechanical-preview-full-tests.log`.
