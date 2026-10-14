# Offset document and worker integration

2026-10-01, local candidate; not published.

- `solidCurveOffset` preserves source curves and groups and assigns separate output identities. XY curves with exact constant Z are supported. Other planes are refused.
- Zero distance preserves the original curve definition, including absence of the optional periodic flag.
- The real worker entry and loaded WASM executed a line offset at Z=7. Output endpoints were checked against the returned positional error bound, rather than exact decimal coordinates.
- The real worker refused a degree-one corner requiring an explicit profile join, then successfully executed a zero-distance request on the same worker.
- `vitest run tests/mainSolidWorkerRealBoundary.test.ts tests/solidCurveOffset.test.ts tests/curveOffsetProtocol.test.ts`: 11 passed across 3 files.
- `vue-tsc --noEmit` and `git diff --check` passed after UI integration.
- The command and preview have been added to DirectModeler, but browser rendering, keyboard/mouse lifecycle, cancellation, undo/redo and reload remain unverified for this new command.
- The geometry build compiled successfully; the build/optimization/packing process was still live when this note was written. This is not proof that final packaging finished or that asset budgets passed.
- No guarantee is made for original-offset regularity or region topology. General corner joins and general NURBS profile region operations remain open.

## Browser evidence

The isolated Chromium lifecycle completed using the emitted production build. A noncircular rational quadratic at Z=7 showed a separate green preview. Esc left the exported document unchanged. Enter added offset curves while preserving the source; undo restored the source document, redo and reload restored the applied document. Six JSON snapshots and `browser/contract.json` retain evidence. `browser/preview.png` was visually inspected. This covers this command scenario, not the complete P0 matrix.

The current production build exceeded the existing DirectModeler and geometry payload limits. Asset-budget qualification remains open; limits were not increased.

## Split-panel qualification

Curve offset and point trim controls are loaded separately. The emitted DirectModeler module is 376,911 bytes and satisfies the unchanged 377,000-byte limit. Offset lifecycle passed again after this change. The existing point-trim browser scenario also passed: numeric and screen selection, both retained ends, endpoint refusal, worker cancellation, rotated view, identity/weights, undo/redo and reload. Its expected endpoint hint was updated to the message confirmed in the rendered page; refusal checks remain in place.

`verify-dist.mjs` still fails on the geometry chunk: 3,293,132 bytes against the existing 3,259,000-byte limit. Final geometry optimization/packaging remains live. Overall size qualification is incomplete.

## Final packaging and late-result checks

The original geometry build handle completed successfully after optimization: 10,794,609 -> 9,587,317 bytes of raw WASM. No build process remains associated with that handle. A fresh production build completed; the geometry JavaScript payload is 3,295,368 bytes, so the unchanged 3,259,000-byte gate still fails.

The offset Chromium scenario now captures actual completed worker responses before delivery. Both Esc and replacement-document import terminate the held worker. Delivering each old captured response afterwards leaves the corresponding exported document unchanged and leaves no offset preview. Two terminations and two delayed deliveries were observed. After reimport, lifecycle comparisons use the new project identity rather than the earlier imported project's identity. Final emitted build browser lifecycle and the 11 real-worker/document/protocol tests passed again.

## Represented-chain diagnostics boundary

The document adapter and worker response validator now require the full represented-chain report for nonzero cell counts. Segment indices, pair uniqueness across categories, pair-budget counters, total pair count, degeneration indices and completeness/simple flags are checked for consistency. Reports claiming original-offset topology certification are refused. Zero identity continues to require null diagnostics. The panel displays crossing/contact/unresolved counts and checked/total pairs; incomplete checks give a next step instead of implying success.

13 tests across document, diagnostics, protocol and real-worker boundaries passed; type checking and production build passed. The browser scenario confirmed the diagnostic panel and repeated late-result cancellation/document-switch plus apply/history/reload. DirectModeler remains 376,911 bytes. Geometry and total asset-size gates remain open.

## Local defect visualization

Offset preview renders retained degree-one control polygons directly instead of sampling them at a fixed display resolution. Diagnostic segment indices are flattened over each retained chunk's actual edges; a duplicated chunk endpoint does not create a new seam edge. Crossings/contacts/degeneracies are red, unresolved pairs amber. Two mapping tests passed, including cross-chunk indexing and avoiding chord interpretation of a zero-offset higher-degree curve.

The `--crossing` Chromium fixture uses a smooth cubic loop with distance 0.1 mm. Native diagnostics reported one represented-chain crossing after 7,260/7,260 pair checks. The browser asserted defect strokes exist, captured a screenshot, and passed delayed-response cancellation/document replacement, Apply, undo/redo and reload. The screenshot was inspected. This is a represented-chain crossing demonstration, not proof of complete original-offset topology. DirectModeler is 376,809 bytes; geometry asset-size gate remains open.

## Asset size gate restored

The packing alphabet was expanded to 108 literal-safe characters (`bAx:`), with corresponding build/runtime decoders and emitted-artifact verifier support. The compressed payload and optimized WASM bytes are unchanged. An emitted production build independently decoded exactly the original WASM and passed all unchanged artifact limits: geometry chunk 3,216,132 bytes, DirectModeler 376,809 bytes, aggregate assets 7,120,229 bytes. Previous geometry-size failure is resolved for this measured build.

29 tests across packing, bounded Brotli, real-worker boundary and base encodings passed. The base108 corpus covers all word values/alignments, zeros, arbitrary bytes, literal evaluation, truncation and allocation bounds. Additional malformed-control/padding cases were added. The crossing-offset Chromium lifecycle passed on the new emitted packing format, including late-response cancellation, document replacement, apply, undo/redo and reload. These gates do not close unrelated roadmap requirements or prove the full P0/P1/P2/P3 plan.

## Persistence preconditions

The document adapter refuses additions exceeding the existing aggregate limit of 128 curves plus surfaces. For nonzero offset it checks degree-one/unit-weight retained geometry, clamped knot count, exact chunk endpoint connections, cell-to-knot parameter correspondence and an exact closing endpoint when labelled closed. This prevents attaching a cell certificate to a different retained chain and prevents producing a document rejected during later history serialization.

Document-limit and mismatched-cell regression tests were added. 12 document/real-worker tests passed, including the actual generated WASM. Provenance/report persistence after applying an offset remains open; reports currently belong to the active preview. General profile/corner support remains open.

## Explicit join primitive candidate

`curve_offset_join` adds bevel and miter constructors for represented binary64 planar endpoints/tangent lines. Bevel preserves endpoints. Miter encloses the line intersection using outward interval arithmetic, bounds midpoint construction error and rejects unresolved parallelism, extension beyond the supplied limit and errors beyond tolerance. Three native tests passed for perpendicular lines, fractional directions under translation, invalid inputs, zero directions and excessive extensions.

This is a native join primitive only. Its bound concerns the supplied represented lines; source normal/tangent uncertainty, corner trimming, region topology and integration into piecewise-NURBS offset remain required. It is not evidence that general corner offsets or the full roadmap are complete. WASM/UI do not yet expose these joins.

## Source-knot miter candidate

`miter_at_knot` validates a planar nonperiodic source and a continuous existing interior knot, obtains both one-sided rational jets with outward bounds, constructs the two normal-offset endpoint enclosures and intersects their tangent lines. Endpoint and intersection midpoint error are bounded over the whole two-segment canonical join by the maximum vertex enclosure radius. Unresolved tangent intersection, zero tangent, excessive extension and excessive construction error refuse the join. This concerns the specified source-knot normal-offset tangent-line join, not trimmed region topology or regularity of adjacent offset curves.

Five focused native join tests passed, including rational curved degree-two sides with a C0 knot, fractional weights and both distance signs. Source geometry preservation and extension refusal were checked. Full nurbs-core tests were started separately and remain live at this note's creation. Curve-span clipping/assembly and transport/WASM/UI join integration remain open.

The full native run completed: 175 unit tests and 2 doctests passed. That run preceded the final added rational-curved-side test; the subsequent focused run includes that test and passed all 5 join tests. No native test process from these runs remains live.

## Piecewise bevel-wire candidate

A native `bevel_wire` assembles nonperiodic continuous XY source normal-offset cells into retained degree-one chunks, with explicit straight bevel edges at C0 source knots. Normal-offset cell bounds include shared-endpoint perturbations. Canonical bevel bounds inherit both one-sided endpoint errors through convex interpolation. Station parameters index output edges; role metadata separately retains original source cell domains or the bevel's source knot. These parameter systems are not interchanged.

The JSON transport adds `curve_offset_bevel_wire` with `wholeWire`, station/source roles, represented-chain diagnostics and explicit `regionTrimmed:false`, regularity/topology false. Two native tests passed: connected station/chunk geometry with both distance signs, and the real transport's inner-corner crossing report. The inward bevel creates a crossing; diagnostics reports [0,2], rather than treating it as a valid trimmed region. Periodic wires, miter assembly, inner-curve intersection trimming, region construction, WASM and UI exposure remain open.

## Closed bevel-wire candidate

The native assembler now handles periodic sources. C0 periodic seams receive an explicit bevel role; C1 seams share an exact endpoint with a bounded perturbation on the final source cell. It also recognizes positional closure of clamped nonperiodic sources only when endpoint knot multiplicities are degree+1 and endpoint controls match exactly. Those closures receive an explicit bevel without changing the source's periodic flag. The transport passes the actual closed state to represented-chain diagnostics.

Five native wire tests passed: open corners, inner-crossing reporting, periodic square with both offset signs, rational smooth periodic closure, and clamped closed triangle. Periodic outward square diagnostics completed without represented crossings; inward intersections remain visible. Retained chunks are exactly connected/closed and source definitions remain unchanged. Region trimming, miter wire assembly and WASM/worker/UI integration of this new wire operation remain open; existing UI still uses the previously qualified smooth-offset operation.

## Bevel document/UI integration candidate

The existing offset worker job now optionally selects the separate native bevel-wire operation. The panel selects smooth-only or bevel and identifies the latter as an untrimmed wire. Document/protocol guards distinguish whole-source-curve reports from whole-wire reports, validate source-offset/bevel roles and station domains, and preserve crossing diagnostics. The retained-edge count must now also fit the requested native cell budget, including joins.

Eight document/protocol tests and type checking passed. A real-WASM periodic-square test was added but has not yet run: the new geometry build is still live and compiling/optimizing. The previous generated WASM lacks this new wire entry. Browser bevel lifecycle, new asset-size checks and clipping/region qualification remain required. Existing smooth-offset qualification does not establish bevel completion.

## Whole-source role validation and browser preparation

The bevel document adapter now requires source-offset role domains to cover the complete original parameter interval in order. Interior bevels must match the current source knot; only a final closed seam may return to the starting knot. Missing roles, discontinuous coverage, consecutive joins and an open-wire trailing join are refused. A corrupted source-domain regression is included in the eight passing document/protocol tests; type checking passed.

The browser lifecycle script now has a periodic square `--bevel` mode and holds only successful real responses, so initial smooth-only corner errors are not mistaken for pending accepted bevel results. The new real-WASM periodic-square test and this browser mode remain unexecuted until the active geometry build finishes. Compilation completed; optimization/packaging is still live under the existing build handle. No completion or new asset-size pass is claimed.

## Independent bevel-wire samples

An independent Decimal(70) rational basis/one-sided derivative oracle now evaluates source-normal cells and canonical straight bevels against actual retained NURBS geometry at station parameters. It explicitly evaluates the left periodic seam endpoint and keeps output station parameters separate from original source domains. Existing smooth-periodic verification still passes after adding optional one-sided evaluation.

A periodic square with fractional weights [1,0.7,1.2,0.9,1], offsets -2 and +2, and requested tolerance 0.001 mm was exported through the native JSON example. Each sign produced 326 edges in two exactly joined/closed retained curves. 2,934 samples per sign passed; maximum sampled error 0.0002998446804550896 mm <= native upper bound 0.0004996983281230756 mm. Fixtures, native responses and oracle outputs are retained in `bevel-wire/independent`. This is sampled independent evidence, not an independent continuous certificate or valid trimmed-region proof. New WASM optimization/packaging remains live; real-worker/browser bevel tests remain pending.
