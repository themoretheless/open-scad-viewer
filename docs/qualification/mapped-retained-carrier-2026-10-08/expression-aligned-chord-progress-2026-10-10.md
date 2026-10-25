# Expression aligned-chord coverage — 2026-10-10

## Verified changes

- Reuse one constrained transition recipe for support positions and normal
  derivatives. The 10,000-cell native fixture passed; its report is byte-identical
  to the baseline (including unresolved UV products and all work counters).
- Share complete transition position and derivative hulls between whole-rectangle
  chord exclusions, support enclosures and endpoint-pair queries. The standard native fixture passed with the exact UI limits:
  10,000 cells, 100,000 spans, 100,000 trim-domain cells.
  Coverage, UV products, refusal reasons and exclusion counts are unchanged.
  Span work fell from 57,471 to 48,488 (8,983 fewer, 15.63%).
  All 4,085 unresolved regions remain unresolved. This is a work-count reduction;
  elapsed-time speedup has not been established.
- Nine native expression-radius/path/cache tests passed. Additional endpoint
  reuse checks passed on both source axes; exhausted and invalid-domain queries
  cannot borrow cached admission, and partial hulls allow a larger-budget retry.
- The instrumented WASM artifact is 12,969,392 bytes, SHA-256
  `55fa368b07f2b69792a0c5f886c7cae7c9ae27f66e4249865b03dce6793d9cf6`.
  Both actual worker checks passed (zero and standard budgets), including
  package identity, six-stage accounting, cancellation, late replies and Retry.
- The browser displayed the standard report and all eight work categories.
  Counts sum to 57,471. Screenshot: `expression-span-stage-ui-2026-10-10.png`.
  This artifact contains stage accounting, but does not yet contain the shared
  position/derivative cache.
- Actual browser Escape cancellation cleared the report and reenabled controls.
  Retry completed with the same 4,085 regions and 57,471 spans. Screenshot:
  `expression-span-stage-retry-ui-2026-10-10.png`.

## Extended baseline

The stage-instrumented native baseline passed with 1,000,000 spans and
1,000,000 trim-domain cells: 66,202 cells, 30,557 unresolved regions in 46 face
pairs. Every span is attributed; the prior coverage report is unchanged.
Corrected transition endpoint pairs consume 427,438 spans (42.74%).
The transition's self-pair has 10,419 unresolved regions.

The extended native run and WASM build for shared positions/derivatives are still
running. Whole-rectangle sharing also passed its native standard fixture and
exact coverage comparison; that later change is not in this in-flight WASM build.
Authoritative handles, reports and source fingerprints are recorded in
`expression-wall-stage-progress-2026-10-10.json`.

## Scope and remaining work

These checks bound chords subject to the requested endpoint-normal alignment
(1 mm threshold, sin² <= 1e-4, UV tolerance 0.05). They do not establish an
unconstrained minimum wall thickness. Unresolved products are never removed
from coverage or promoted to admission.

Next: finish the shared-cache extended run and WASM → worker → UI checks;
then continue native exclusions for unresolved transition self-products.
General NURBS fillets, mixed radii and complex corners, whole-body wall
qualification, full specimen acceptance, the complete P0 matrix, and the
remaining P2–P3 scope stay open.

## Parameter error locations

Editable distance threshold, UV tolerance and squared normal-direction sine
use the same parameter predicates as transport admission. Invalid fields have
an error border, a localized bound hint and linked `aria-invalid` /
`aria-describedby`. All three hints cleared after correction in the actual
browser. The 66-test panel/controller/transport suite and typecheck passed.
See `expression-field-errors-ui-2026-10-10.png` and
`expression-inspection-error-guidance-2026-10-10.json`. This is bounded inspection
panel coverage, not the full P0 command matrix.
