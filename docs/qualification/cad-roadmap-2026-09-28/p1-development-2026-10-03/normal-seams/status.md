# Original surface normal pair diagnostics — 2026-10-03

`nurbs_core::normal_alignment::inspect_pair` encloses angular compatibility of two original surfaces over two complete UV rectangles. It uses original rational derivatives, outward arithmetic, both knot sides and a shared span budget. Degenerate normals and exhausted work remain unresolved. Both inputs are validated before any early budget refusal. Surface geometry is not edited.

This angular predicate alone proves neither G0 coincidence nor a shared seam correspondence. It must be combined with original edge/lift agreement before a fillet seam can be qualified as G1. It does not construct arbitrary curved fillets.

Five normal-alignment tests passed. Pair tests include translated surfaces with matching normal directions (explicitly not G0), opposite orientations, a 45-degree break, a collapsed normal, shared-budget refusal and invalid second-domain rejection despite an exhausted budget.

Actual partial plane/cylinder transition test now checks the complete meridian join to its constant-radius neighbor with adaptive closed-interval coverage at sin² <= 1e-6, both sweep signs. The initial 10,000-cell trial exhausted its work budget; no whole-seam certificate was admitted. The follow-up passed in 88.51 s with at most 100,000 normal spans per orientation: negative sweep 19,399 visited cells / 9,700 admitted intervals / 38,798 spans; positive sweep 23,447 visited cells / 11,724 admitted intervals / 46,894 spans. Both complete closed-interval covers meet the unchanged angular tolerance. Collapsed zero-radius tips remain explicitly unresolved. This certifies normal alignment for these authored specimens, not arbitrary transitions or full G1 positional agreement.

P0 protocol qualification: 27 tests across material wall, material volume and self-intersection passed. The new wall worker test covers changing face groups, cancellation, late replies, forged original source rejection and successful retry through a fresh worker. This is a targeted lifecycle qualification, not closure of the complete 95-command matrix.

Remaining: package the predicate, use original seam correspondence and adaptive work reports, integrate diagnostics into the current redesigned UI, qualify arbitrary NURBS/cylinder blends and complex corners. General self-intersections and remaining P0 scenarios remain open.
