# Continuous material segment — 2026-10-02

The native audit checks the complete authored line segment
`P(t) = origin + t * direction`, `0 <= t <= 1`.
It requires a proven embedded and consistently oriented material boundary,
an inside origin established by ray parity, and exclusion of intersections
with every trimmed source face over the entire finite interval.

An unresolved root, trim classification, endpoint band or work limit keeps
the segment unproven. A boundary-free segment with an outside seed is refused.
Source geometry is unchanged. Reports retain original face indices, UV boxes,
line parameter enclosures and stage work counts.

## Verified native scenarios

- Interior cuboid segment; segment leaving the cuboid; outside seed.
- Boundary endpoint remains unqualified; geometry work exhaustion stays explicit.
- Cuboid cavity crossing with both endpoints inside material: two contacts.
- Canonical partial annular transition: interior wall segment passes;
  cylindrical cavity crossing reports two contacts; cavity seed is outside.
- Nonfinite coefficients, zero direction, nonpositive tolerance and absent
  work budgets are rejected; trim work exhaustion remains explicit.
- Four material-segment tests passed. Seven line/surface tests passed,
  including mid-plane roots, negative directions, rational weights,
  multiple spans, tangency, coincidence and exterior UV boundaries.

The line/surface solver now uses an asymmetric dyadic subdivision to avoid
placing common mid-plane crossings on internal cell boundaries. Certification
still requires strict interval Krawczyk inclusion. A subdivision whose local
domain endpoint cannot be represented exactly remains unresolved, so a rounded
cell boundary cannot silently move the polynomial domain.

## Remaining work

This is a strict interior segment audit. It does not yet admit boundary-to-boundary
wall chords, prove normal alignment or automatically choose opposing regions.
Original distance witnesses require explicit endpoint admission before they can
be used as complete material thickness chords. General minimum wall thickness
therefore remains open. Bridge, WASM and UI integration are pending.

The complete native regression suites passed after these changes:
730 B-rep tests passed, zero failed, three existing roadmap gates ignored
(101.29 seconds); 240 NURBS tests passed, zero failed (32.21 seconds).
The ignored roadmap gates remain unfinished work.
