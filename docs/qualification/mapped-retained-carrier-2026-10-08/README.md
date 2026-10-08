# Retained coverage of mapped original carriers

A reproduced regression showed `SharedEdge::covers_complete_canonical_source` returning true when both affine source compositions cover the full original world curve but the admitted fragments retain only its middle half. The original composition identity does not prove retained endpoint coverage. The regression fails before the fix.

The native coverage gate now requires both original affine ranges to cover the canonical domain and both retained uses to have literal original-domain endpoints in their admitted traversal orientation. It retains exact shared-edge ownership for partial fixed or root-valued fragments; it declines the stronger full-carrier claim. Direct source coverage behavior remains unchanged. General root-ended union coverage still requires its own exact ordered-chain proof.

Five mapped-edge tests pass, including new partial fixed-end and root-end examples with full affine composition, fresh restoration preserving the distinction, positive full-domain controls, and reversed affine carriers. Existing different-domain and nonlinear mixed-root identity tests also pass. This is a coverage-admission correction, not a new arbitrary NURBS fillet algorithm.

Affected consumers include `source_body_model::convert`, planar flux admission, and mapped carrier shortcuts in allowed-contact qualification. Full B-rep regression and a refreshed WASM package have been started; their results are pending. Current published package a0c031f988b2aaee8a0fcababc21d7ae0121c6ff52788363cbbecbc3e1ef47bb predates this correction.
