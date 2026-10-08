# Retained coverage of mapped original carriers

A reproduced regression showed `SharedEdge::covers_complete_canonical_source` returning true when both affine source compositions cover the full original world curve but the admitted fragments retain only its middle half. The original composition identity does not prove retained endpoint coverage. The regression fails before the fix.

The native coverage gate now requires both original affine ranges to cover the canonical domain and both retained uses to have literal original-domain endpoints in their admitted traversal orientation. It retains exact shared-edge ownership for partial fixed or root-valued fragments; it declines the stronger full-carrier claim. Direct source coverage behavior remains unchanged. General root-ended union coverage still requires its own exact ordered-chain proof.

Five mapped-edge tests pass, including new partial fixed-end and root-end examples with full affine composition, fresh restoration preserving the distinction, positive full-domain controls, and reversed affine carriers. Existing different-domain and nonlinear mixed-root identity tests also pass. This is a coverage-admission correction, not a new arbitrary NURBS fillet algorithm.

Affected consumers include `source_body_model::convert`, planar flux admission, and mapped carrier shortcuts in allowed-contact qualification. Full B-rep regression and a refreshed WASM package have been started; their results are pending. Current published package a0c031f988b2aaee8a0fcababc21d7ae0121c6ff52788363cbbecbc3e1ef47bb predates this correction.

## Ordered mapped root chains

A second regression uses a closed tetrahedral source shell whose shared edge is partitioned at independently qualified root points, with full affine original-source maps on every owned use. The individual retained halves are incomplete. Before the chain change, shared-face contact qualification refuses because it requires complete retained coverage from every half.

The native allowed-contact gate now distinguishes full original composition from retained coverage. It first verifies each mapped composition covers the canonical carrier. It then proves retained coverage from the ordered original pcurve chain: one face/wire, consecutive edges, identical source curve and direction, exact `Fragment::joins`, and literal original-domain first/last endpoints. No rounded root or proximity join is used. Both incomplete halves must be present; either half alone fails this coverage test.

The new mapped closed-shell test passes in 1.13 seconds, preserving root recipes, shared vertex enclosures, closed incidence and all existing contact checks. The extended source-family regression is running. The earlier full B-rep run for the individual-coverage correction passed 943 unit tests (3 ignored) and every integration suite. The current WASM build includes that individual-coverage correction but predates this subsequent chain extension; a later package refresh is required for mapped-chain support.

The extended source-family run passes: 131 tests, no failures, in 35.87 seconds, including direct, irrational-root, curved inverse-shear, mapped-chain, root restoration and source contact regressions. The original-composition helper remains crate-private; external consumers retain the stronger retained-coverage API.
