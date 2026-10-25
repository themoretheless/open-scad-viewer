# Early cached distance exclusion

Before same-face intrinsic chord work, the native traversal now consults only complete position enclosures already stored for both exact face/UV keys in its immutable-shell-local cache. If their outward interval distance lower bound is at least the requested minimum, the product is excluded by distance. No new position enclosure is computed or charged at this early stage; the later distance stage remains unchanged. Zero/exhausted-budget handling still precedes this lookup.

A first focused invocation used a filename as test filter and selected zero tests; it is not regression evidence. The corrected `assembled_shell::wall::product_tests` filter covers wire/containment invariants only and does not exercise the new traversal branch. The matched complete-carrier integration run is live with 10,000 cells, 1,000,000 spans and 100,000 trim cells. Its baseline is the separately running original-endpoint-normal-sharing generation. Request and source manifest: `mapped-retained-carrier-2026-10-08/cached-distance-early-trimmed-wall-request-2026-10-10.json` and `cached-distance-early-native-inputs-2026-10-10.json`. Log `/tmp/cached-distance-early-trimmed-wall-20261010.log`.

Specimen-scale benefit and coverage preservation remain unverified until both runs terminate. If no measurable benefit appears, reconsider retaining this lookup. Existing pending reports are not geometry admission. The currently running WASM optimizer and full B-rep suite predate this traversal change; do not attribute their outcomes to it.

## Completed experiment

The integration test passed (1 test, 1355.91 s). Pending products, their order, UV intervals and refusal reasons are exactly unchanged (4081). Span work decreased only from 19007 to 19006; intrinsic_checked changed from 1079 to 1076 and cache_hits from 74027 to 73670. The strict work-reduction comparator correctly refused the altered non-work counter. No comparator relaxation was made.

The early lookup was removed: one saved span does not justify an extra traversal branch. The published endpoint-normal-sharing WASM and its full native suite predate this experiment and now again match the production traversal. The experimental report and source manifest remain as evidence; they are not a wall qualification.
