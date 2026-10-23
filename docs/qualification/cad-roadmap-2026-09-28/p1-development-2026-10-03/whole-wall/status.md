# Whole original-face material-wall coverage

## Scope

The new `cad_whole_wall` Rust operation bounds the minimum interior material chord aligned with both endpoint normals within the requested angular tolerance. It enumerates every original face pair, including same-face pairs. Sampling provides candidate upper witnesses; it supplies no coverage certificate.

Exact positive-weight planar control hulls exclude same-plane chords and planar same-face pairs. A complete endpoint-normal comparison can exclude incompatible distinct faces. Remaining distinct pairs contribute original trimmed-face distance lower bounds. Shared pair, plane-control, normal-span, distance-cell and trim-domain budgets are explicit. Missing pairs or unresolved curved same-face domains contribute zero; missing pairs also prohibit convergence. No adjacency or trimming hole is silently skipped.

The interface adds “Minimum thickness of the whole part”, automatic candidate search, full pair counts and original-face uncertainty markers. Model/input changes terminate the worker. Late replies and substituted source models cannot satisfy the typed response contract. Search failures are localized and Retry runs a fresh calculation. Green minimum status requires complete enumeration and a narrow global interval, rather than a short sampled line alone.

## Geometry corrections

Newly authored exact coordinate planes use original polygon coordinates as UVs. Canonical shared vertices must lie exactly on the plane and inside its UV rectangle; otherwise existing general authoring remains. Existing loaded documents are not repaired by this authoring change. Decimal wall coordinates therefore retain exact original edge/face agreement.

Mass quadrature now keeps each computed node inside its admitted parameter interval. An evaluated rational trim coordinate is clamped to the surface rectangle only if its complete original positive-weight control hull proves containment. This fixes the observed one-ULP excursions on coordinate-plane caps and narrow charts after repeated cap edits and placement. It does not project source geometry or certify numerical mass estimates. `rotated-enclosure-mass.json` preserves the actual synthetic failure for native regression.

The cap API reference output is regenerated with native Rust from unchanged requests. Matching the current WASM response remains required; independent dimension checks are separate.

## Remaining

Curved same-face chord domains and coupled normal-compatible distance lower bounds remain incomplete. The cylinder case retains a zero lower bound and shows its original unresolved faces, even with a valid radial upper witness. This release does not establish arbitrary curved whole-body minimum thickness, a thickness heatmap, all 95 command executions, arbitrary fillets, or completion of P0–P3.

## Reproduction

- Native: `cargo test --offline --locked --manifest-path crates/Cargo.toml -p brep-core --lib` and the equivalent `geometry-bridge` suite.
- Contract: `cargo run --offline --locked --manifest-path crates/Cargo.toml -p geometry-bridge --example whole-wall-contract -- OUTPUT.json`.
- Cap references: `refresh-cap-contract` takes the original reference path and a separate output path.
- Real WASM/handler/automatic search: `node --import tsx scripts/check-cad-whole-wall-wasm.mts OUTPUT_DIRECTORY`.
- Browser: set `CAD_WHOLE_WALL_FIXTURES=OUTPUT_DIRECTORY/browser-fixtures.json`, then run `scripts/check-cad-whole-wall-browser.mjs EVIDENCE_DIRECTORY`, with `--keyboard` for the second interaction.
- Controlled part histories: set `CAD_MIXED20_STEP_OUTPUT` and `CAD_MIXED20_UI_OUTPUT`, then run `tests/cadRoadmapHistory.test.ts`; replay the UI fixtures with `scripts/check-solid-mixed-history-browser.mjs`.
- STEP verification: `scripts/verify-cad-step-occt.py DIRECTORY` and `scripts/verify-cad-mixed-dimensions-occt.py DIRECTORY --service` use independent OpenCascade.

## Delivered verification

Final optimized WASM: **9,872,097 bytes**, SHA-256 `b3792ddce52a024f1877245f56b1d11aa9309f81fdb0d217a576c2388980f5f2`. The artifact served at `http://127.0.0.1:5175/wasm/geometry-kernel.wasm` matches this hash.

- Complete native suites: **749 B-rep tests / 3 ignored**, **354 bridge tests / 1 ignored**. No failures. Both observed coordinate-chart mass failures have native regressions.
- Fresh-artifact CAD regression: **915 tests / 74 files**. The 17 part-history tests pass; final UI/protocol/search subset after hiding unused automatic coordinates passes **367 tests / 6 files**. Vue typecheck and distribution verification pass.
- Whole-wall real WASM: **7 cases, 7 typed handler calls**. Automatic box search converges after 4 of 54 generated candidates to `[9.999999999999995, 10.000000000000007]` mm. Automatic enclosure search converges after 9 of 64 candidates to `[1.3999999999999977, 1.4000000000000006]` mm. The enclosure includes **1,081 original face pairs**; all 21 box pairs are enumerated. Long-witness, incomplete pair prefix, missing planar proof, oblique line and curved-self cases retain refusal/wide intervals.
- Both whole-wall browser interactions pass three models, input cancellation, sanitized transport failure, Retry, source preservation and exact reload. The keyboard manual-curved scenario explicitly unchecks automatic search with Space, waits for completion and verifies the original requested line. Screenshots of planar success and curved uncertainty were inspected.
- Existing placed selected-wall browser tests pass **9 results per interaction**, including manual/automatic cancellation, original-face markers, normal qualification, oblique rejection, cavity refusal and Retry. Seven selected-wall WASM cases, seven handler calls and three automatic searches also pass on the same final artifact.
- All three controlled parts pass **20 actual UI edits, 7 cancelled previews, 20 Undo, 20 Redo and exact reload**, both mouse and keyboard. All 21 saved solids per part/interaction pass geometry/volume/untouched-tool audits. The history collector checks **422 documents** and preserves **140 distinct payloads**. Its optional Brotli archive decodes byte-for-byte to the original gzip payload, and every record and reference SHA-256 is verified.
- Fresh service STEP, STEP from both final UI histories, and actual menu STEP export/import/reexport in both browser interactions each pass independent OpenCascade: **6 files and 16 hard-coded original-face gauges per route**. These are the specified webs/walls, floor, bore/outer radii and supported radius-one fillets. Actual current-geometry menu export leaves the document unchanged. Independent acceptance is limited to these controlled bracket/flange/enclosure chains.
- Distribution: **142 artifacts**, **7,364,029 asset bytes + 11,928,217 raw WASM bytes = 19,292,246 bytes**. Limits increase only by measured feature growth, retaining the previous 397-byte geometry, 4-byte worker and 613-byte total headroom.

`artifact-index.json` records stored file hashes. JSON/log evidence and most STEP repetitions use gzip; the actual mouse menu STEP files remain directly readable. Decompress `.step.gz` to its manifest filename before independently rerunning OCCT. The complete history is `histories/documents.jsonl.br`; decode with Node's `brotliDecompressSync`, then verify each record's `json` string against its `sha256`. Regenerate it using the existing collector with `--brotli`.

For actual STEP-menu reproduction, set `CAD_MIXED_UI_EVIDENCE` to the final mouse/keyboard JSON history and `CAD_MIXED_UI_STEP_OUTPUT` to its reference STEP manifest, then run `scripts/check-cad-part-step-menu-browser.mjs OUTPUT_DIRECTORY`, optionally `--keyboard`.

The general curved-wall minimum, coupled normal/chord bounds, all-command/error matrix and the remaining P1–P3 backlog stay open.
