# Automatic retained-domain pass

The native whole-wall qualifier adds a separate bounded pass over unresolved distinct face pairs. Total cells/spans are capped at 100,000, total contour cells at 1,000,000; each pair has independent cells, spans and contour-cell limits. Each contour classification is separately bounded. Existing proofs retain scheduling and reasons. Unattempted and unresolved pairs remain unproven. The diagnostics preserve actual per-pair work and every unresolved original domain product.

Reference budgets: 256 cells, 512 spans, 10,000 contour cells per pair, 256 contour cells per query, 1e-8 UV inflation. All 61 unresolved pairs are attempted; 16 newly qualify, leaving 45 unresolved. All 15 independently obtained retained facts and every previous complete-chart/normal/self proof are retained. Native work: 12,152 cells, 23,220 spans, 406,655 contour cells. Whole-wall qualification remains false.

The bounded cuboid regression passes and confirms that a one-cell pass preserves pending products and produces no certificate; zero per-pair work is rejected. Earlier aggregation source tests pass (127 unit tests, one integration test). The bridge now parses optional `wallCoverage.retainedGap` budgets and emits separate proof/work diagnostics; its request-binding and invalid-budget test passes (23.65 seconds). WASM packaging and TS admission/UI are pending.
