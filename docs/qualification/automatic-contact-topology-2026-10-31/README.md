# Original B-rep contact discovery — topology milestone

Native entry points: `brep_core::fillet_contact::topology::{discover, inspect_source_edge, propose_edge}`.

`discover` resolves original shell edge/support identities through `plan_radius_network`, then walks adjacent face-loop corners to obtain one cyclic edge/face fan per selected endpoint. It accepts any valence >=3 subject to the shared work budget, preserves independently prescribed radii and unselected roles, and refuses disconnected/ambiguous fans. No angular sorting, geometric fitting or vertex welding is used. Ordering is deterministic (lowest edge first, then lowest neighboring edge). This order is combinatorial, not an outward orientation certificate.

`inspect_source_edge` rechecks the complete actual 3D edge against both actual original support pcurves and charts with coedge direction. Incidence alone cannot admit displaced or corrupted source supports.

`propose_edge` selects the two original supports automatically, requires the original edge lift check to pass, then calls the existing G2 section proposal builder with the remaining position-proof budget. Surface-side choices remain explicit in `FilletOptions`; the marcher must not guess which material side to retain. This function currently performs topology planning twice for clarity; the topology budget applies to each bounded planning call. The contact proof cell/exact budgets are shared between original-lift proof and the proposed contacts.

## Explicit limits

This milestone does **not** construct an admitted arbitrary B-rep fillet:

- Pair-surface marching can discover the wrong disconnected intersection branch. Matching the offset branch to the selected original edge remains open.
- New contacts still require complete retained-trim membership, endpoint-root isolation and common root splitting.
- Original-edge lift proof does not establish these offset-contact facts.
- `VerifiedContacts` in the existing marcher means original-support/skin positions, not G2 or curvature admission.
- 5/6/8-valent **topological** controls are covered. Central multi-patch G2 construction, compatibility of boundary two-jets, end transitions and sewing for those controls remain open.
- G2 and curvature <=10/mm must be inspected afresh over every seam/retained face of each final body using the existing interval gates. No tolerance or curvature limit was relaxed; this API grants no whole-body admission.
- No WASM, worker or UI integration is claimed.

Tests: 4 new topology/source-contact tests, 2 existing four-edge constructor tests and 3 existing original-NURBS trim/sew tests passed (9 total). The last suite includes complete G2/curvature/global embedding qualification of the prior sewn prism; it does not qualify a new high-valence body. See `test-results.txt` for the executed result.

The existing model validator also checks sampled edge/lift geometry and rejects the deliberately displaced support before whole-interval inspection; the negative test asserts refusal, not a particular stage. Whole-interval source-edge inspection still runs for valid inputs.
