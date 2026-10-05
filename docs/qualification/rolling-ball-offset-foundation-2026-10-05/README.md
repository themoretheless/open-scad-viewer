# Rolling-ball offset foundation

Status: native kernel implementation, under qualification. No new fillet capability or UI command is admitted by this change. General curved fillets and endpoint transitions remain required work.

## Geometry

For an authored rational surface S, the signed carrier is F = S + d n, with n = N / |N| and N = S_u × S_v. The sign is relative to the surface parameter orientation, not inferred material orientation. Reversing one parameter requires reversing d to preserve the physical carrier. B-rep face orientation and UV trim admission must be applied by the feature owner.

`surface_offset::bounds` encloses the original surface image and the original derivative cross product on every intersecting knot span. After positive scalar normalization, an outward interval norm with strictly positive lower bound encloses the unit normal. Component intervals are intersected with [-1,1], then multiplied by signed d and added to the image enclosure. These operations bound all source parameters in the requested rectangle. No analytic recognition, sampled radius, or sampled derivative is used for this bound. Incomplete span coverage and normals that cannot be separated from zero produce no offset-image bound.

`surface_offset::evaluate` computes numerical points and first derivatives for predictor/corrector proposals. It uses n_u = (N_u - n(n·N_u))/|N| and the analogous V derivative, with N_u = S_uu × S_v + S_u × S_uv. Numerical evaluations carry no root-existence or regularity authority. An inward radius-three offset of a radius-three cylinder collapses its U derivative; the API retains this result rather than inventing a regular patch.

`surface_offset::intersection_candidates` subdivides the complete pair of original UV rectangles. Only a positive interval separation between both offset-image enclosures excludes a box. Possible intersections and every unvisited, singular or precision-limited box remain in the report. Candidate boxes do not establish a root, unique branch, contact on trimmed faces, or a valid fillet. Both original operands are validated before early exclusion can hide an invalid second input. Initial enclosures are reused and included in reported normal-span visits.

## Native checks

- Offset foundation and JSON transport: 17/17 tests, including signed planes, rational cylinders, general spatial rational patches, derivative finite-difference regression, orientation reversal, very small regular normals, collapsed carriers, invalid inputs and exhausted budgets.
- Existing normal alignment: 5/5.
- Existing surface contact and contact search: 8/8.
- Existing circular blend: 26/26, including the explicit refusal of a regular G1 claim at the current collapsed transition tip.
- `git diff --check` passed.

The enclosure argument above is the basis for continuous bounds. Point samples and derivative finite differences are regression checks, not a proof of a whole surface.

## Interval section extension

`surface_offset::jacobian_bounds` bounds source homogeneous jets through second order and uses the quotient recurrence to enclose Euclidean derivatives. Interval differentiation of N and normalization then enclose the first derivatives of the offset carrier. Every incident knot side contributes; this union does not itself establish continuity. Point samples and finite differences independently exercise the bounds on rational cylinders and spatial rational patches.

Periodic natural-domain endpoints include their wrapped start branch. Both branch evaluations consume the shared span budget. A valid degree-one periodic fixture has different normal limits at the seam; bounds retain both incident limiting images while the numerical evaluator refuses an undefined source jet. No root certificate infers continuity from a periodic flag.

`surface_offset::certify_contact_section` reuses the existing `surface_contact::section_krawczyk` inclusion engine. It proves an isolated center contact for one fixed source parameter, using full interval Jacobian bounds and an independent interval midpoint residual. The contraction bound and strictly invariant box establish a unique section root. Source C1 continuity is required across interior knots, and periodic endpoint seams remain unresolved without a separate continuity proof. Coincident supports and collapsed offset carriers remain unresolved.

Native examples certify crossing offset planes and a rational cylinder/plane section. These are section certificates, not complete intersection curves, trim admission, rolling-ball envelope surfaces or fillet features.

Five JSON operations and TypeScript adapters are added: numerical offset evaluation, offset image bounds, offset Jacobian bounds, pair candidate boxes and section contact inclusion. Results explicitly distinguish root existence, complete-curve evidence and topology authority. The final WASM build and product checks pass. The three new offset tests exercise the five JSON operations through the actual WASM module; they do not yet exercise a worker offset request or a current UI command. Existing distance and real-worker regression tests also pass.

## Next required stages

1. Branch continuation and completeness, including tangencies and singularities.
2. Admission of contact parameters on original trimmed faces and rolling-ball radius/tangency bounds.
3. Envelope patch construction, trim replacement, endpoint/corner transitions, sewing, persistent naming and global volume/self-intersection checks.
4. WASM/worker/current UI admission and independent STEP measurement.

The complete P1 requirement includes general NURBS edges, cylinder/cylinder cases and complex corners. This native foundation does not replace that scope with its fixtures.

## Published module validation

- Native checks: 17 offset/transport, 5 normal bounds, 8 contact and 26 circular regression checks passed.
- Product checks: 54 tests across seven files passed, including three new offset tests and existing distance/real-worker checks.
- Vue and MCP TypeScript checks and Vite production build passed.
- WASM: 11,000,183 bytes; SHA-256 `08e0de0ed697f2a6e8e8499cfc8c0d86a189140519714e042699c3ca3d3ff31d`.
- Packed geometry chunk measured 3,674,022 bytes; production assets measured 7,906,737 bytes. Explicit package budgets move to 3,675,000 and 7,910,000 bytes respectively. No runtime dependency was added.

This delivery provides kernel queries and typed adapters. It does not admit general fillets to the worker or UI, nor prove a complete centerline or trimmed solid.

## Uniform contact band extension

`certify_contact_band` fixes a full interval of one source parameter and encloses the offset equation at the midpoint of the other three parameters over that entire interval. The same interval Jacobian and preconditioner enclose every section. A contraction below 0.5 and a strictly invariant tube prove existence and uniqueness within the supplied tube for every driving parameter, including endpoints. Continuity of the source offset equation and uniform contraction establish continuous dependence of the root on the driving parameter.

This is a local graph certificate within one tube. It does not exclude other branches outside that tube, prove global curve completeness, admit UV trims, certify an embedded envelope, or construct a solid fillet. A failed inclusion returns unresolved; it never substitutes sampled sections for uniform coverage.

Native checks exercise a moving plane/plane contact and a rational cylinder/plane band, and refuse a tube which contains the midpoint section but misses both driving endpoints. The JSON operation and typed adapter expose separate `rootForEveryParameterProven`, `uniqueWithinTube`, and `continuousBranchProven` flags; whole-curve, trim and topology gates remain false. Native offset/transport tests: 19 passed. WASM validation of the new band operation passes.

### Worker boundary

The existing MainSolid worker accepts the `offsetContactBand` job. Its response validator checks the exact driving interval, strict inclusion of all three free parameter intervals, a finite contraction bound below 0.5, and all three uniform-contact flags. It rejects promoted whole-curve, trim or topology claims. Non-certified outcomes must have a null witness and false uniform-contact flags.

The client expectation copies request intervals independently of the host object, because postMessage also snapshots the payload. A real-worker test mutates the host intervals after sending the job and expects the original driving interval in the response. This guards against a UI edit changing admission of an already-dispatched result. Forty existing worker runtime/transport tests and Vue/MCP type checks pass; the new real-worker test passes on the final WASM artifact, including precise invalid-input errors, successful Retry, cancellation after capture of a genuine successful worker response, and delivery of that late response to the original callback.

### Final band delivery checks

- 96 product tests across nine files passed, including actual WASM band coverage and real-worker transport/recovery tests.
- Vue and MCP type checks, production Vite build, package verification and diff whitespace checks passed.
- WASM: 11,003,806 bytes; SHA-256 `eec0102d42cfb8f512ac0ffd31e12a989752b438574c2c0b5116d706ed4c7cfd`.
- Production assets: 7,907,646 bytes; existing package limits remain unchanged.
- No new UI fillet command is admitted by this delivery.

## Original trim admission extension

`trimmed_offset_contact::certify` combines the uniform offset band with audits of both authored UV regions. Both domains are constructed and validated before any early offset exclusion. A successful admission requires simple exactly joined loops, valid outer/hole roles, disjoint loops, and whole-rectangle inside classification of both original source UV enclosures. All region audits and classifications consume shared pair, cell and domain-cell limits. Work stops and boundary intersections remain unproven.

Four new native checks cover inside admission, a contact inside a hole, an interval crossing a trim boundary whose middle contact is inside, an incorrectly oriented hole, shared-budget exhaustion, a multispan rational outer contour, and malformed second trims hidden behind otherwise excluded carriers. The trimmed-offset filter passes five tests including one existing curve-offset transport test; the offset regression filter passes twenty tests (the new transport case overlaps both filters). Vue type checks pass. The final WASM build and product tests pass.

The new JSON query and typed adapter report `trimMembershipProven` separately from `continuousBranchProven`. They retain false `worldCoedgeIdentityProven`, `wholeCurveComplete`, and `topologyAuthority` gates. This is admission on original UV regions, not replacement trims or a completed fillet solid.

### Trim admission worker integration

The existing MainSolid worker now accepts `trimmedOffsetContactBand`. Its response validator reuses uniform contact-band admission, checks shared budget totals against the sum of individual audits/classifications, and requires two valid region audits and two inside classifications before accepting trim membership. World coedge and topology promotion, incomplete region lists, inconsistent work totals and outside classifications cannot be admitted. Vue/MCP type checks pass. Prepared actual-worker cases cover inside admission, holes, boundary crossing, bounded work, precise invalid-input errors and successful Retry. All prepared artifact/real-worker cases pass on the final WASM build.

### Final trim admission delivery

- 98 product tests across nine files passed, including complete-band trim admission and existing distance/worker regression checks.
- Native trim-contact filter: five tests passed (four new checks and one existing offset transport check). Native offset regression: twenty tests passed, including one new trim-contact transport case also included by the other filter.
- Vue/MCP type checks, production Vite build, package validation and diff checks passed.
- WASM: 11,014,718 bytes; SHA-256 `9a19dd762b2971925a79d8d954b61c23b1d8934cafdfbed55be10d0643e30af6`.
- Packed geometry chunk: 3,677,712 bytes. Production assets: 7,914,653 bytes (+7,007 from band delivery). Explicit limits move to 3,679,000 and 7,917,000 bytes.
- No replacement trim, envelope patch, endpoint transition or UI fillet command is admitted.

## Original spatial coedge admission

`offset_source_boundary::certify` links each authored UV trim curve to exactly one original spatial coedge, including its traversal reversal. Both surfaces and every original UV/world curve are validated before an early contact or trim rejection. Region admission remains a separate prerequisite. Every coedge must pass either exact authored curve/surface identity or conservative full-parameter agreement within the declared millimeter tolerance.

The existing exact identity predicate supports a single Bezier chart. Unsupported multispan representations, exact inequality and exact work stops can use the existing interval agreement verifier; this never promotes tolerance agreement to exact identity. All exact predicates share `maxExactWork`, and all tolerance queries share `maxAgreementCells`. A mismatch has an enclosed distance witness and original normalized pcurve parameter. A work stop retains explicit total/checked coedge counts and cannot certify the whole source boundary.

Five new native checks pass: full-boundary exact agreement with both traversal directions, tolerance-only agreement for a small authored departure, a mismatch on the second support, multispan rational contours, out-of-domain refusal, bounded work, missing coedges and malformed second operands despite otherwise excluded carriers. Seventeen existing curve/surface agreement regression checks pass, including periodic seams, arbitrary parameter domains and surface knot crossings. The JSON operation, typed adapter and product tests are prepared. The final WASM and existing MainSolid worker are qualified by the checks below. Current UI fillet admission remains open.

Results explicitly distinguish `worldCoedgeIdentityProven` from `worldBoundaryWithinToleranceProven`. Neither creates replacement trims, constructs a rolling-ball envelope or admits a solid topology. General centerline completeness, the envelope, endpoint/corner transitions and volume qualification remain required.

### Source boundary worker admission

The existing MainSolid worker accepts `offsetSourceBoundary`. Its expectation snapshots the ordered side/loop/curve addresses and trim query dimensions. Admission checks every audit against that order, validates shared exact/tolerance work totals, and requires all original coedges before accepting a whole-boundary claim. Exact equality, tolerance-only agreement, mismatch and unvisited work remain distinct. Mismatch witnesses must have a positive lower distance bound above the requested tolerance. A promoted topology flag, omitted audit, wrong source address or inconsistent total is rejected.

Vue/MCP type checks pass. Prepared product/real-worker tests cover exact admission, source mismatch, tolerance-only admission with no exact budget, shared-work exhaustion, missing input coedges and Retry. The final WASM build passes these artifact and real-worker checks.

### Final source boundary delivery

- 5 new native checks and 17 existing curve/surface agreement checks passed.
- 100 product tests across nine files passed, including the real worker, exact/tolerance distinction, spatial mismatch, bounded work, missing inputs and Retry.
- Vue/MCP type checks, Vite production build, package verification and diff checks passed.
- WASM: 11,027,460 bytes; SHA-256 `9b1c0a16a65c6cb075d0f076f9c923509ca5729b59a229312f2e777183624013`.
- Packed geometry chunk: 3,681,036 bytes; total production assets: 7,921,097 bytes (+6,444 bytes from trim admission). Explicit limits move to 3,683,000 and 7,924,000 bytes.
- This verifies original source boundaries. Replacement trims, a rolling-ball envelope, corner/end transitions, global volume qualification and UI fillet admission remain required.

## Uniform center tangent extension

`offset_contact_tangent::certify` differentiates the already certified constant-offset contact branch implicitly. Over the full contact band, the three free source parameter derivatives solve `J y' = -F_t`. Row scaling avoids unnecessary magnitude growth; outward interval Cramer's rule requires a determinant enclosure separated from zero. Tangent images evaluated from both original offset supports are intersected, and an outward norm interval bounds center speed. Only a positive speed lower bound proves a regular centerline.

The offset equation must be differentiable over the complete source enclosure: C2 source jets for nonzero offsets, or C1 for zero offsets. Repeated knots with insufficient continuity and unqualified periodic endpoint seams remain unresolved. A complete isolated contact band alone does not prove centerline regularity.

Five new native/transport checks pass: moving plane contacts, a curved rational cylinder/oblique-plane centerline in original and rotated/translated frames, a nearly collapsed cylinder offset with a unique contact band but a speed enclosure containing zero, continuity guards and false envelope/trim/topology gates. Twenty-two offset regression checks pass (including the new tangent transport case). Vue type checks pass. JSON/typed adapters and actual WASM tests are now qualified by the final delivery below.

The center tangent certificate does not construct or certify an envelope, replacement trims, end/corner transitions or a fillet solid. General curve completeness and complete P1 qualification remain open.

### Periodic-start correction and worker preparation

A new regression exposed an actual false positive: the periodic-start normal evaluator refused an undefined normal, but a contact band starting at that parameter could still receive a regular tangent certificate. The pre-fix failing reproduction is retained. Offset contact continuity now refuses both unqualified periodic endpoints. Offset image/Jacobian branch coverage at the start also includes the incident end side, sharing the existing span budget; a missing budget does not silently discard that side.

Six tangent/transport checks and twenty-three offset regression checks now pass (the tangent transport case overlaps both filters). The corrected WASM build is complete; the first tangent artifact predates the seam correction and was replaced before delivery. Prepared product tests check both limiting start images, budget exhaustion and rejection of the undefined seam normal.

The existing MainSolid worker accepts `offsetContactTangent`. Its response validator requires the original whole driving interval, finite derivative/tangent/speed enclosures, consistent regularity and reason fields, and all envelope/trim/topology gates false. A speed enclosure starting at zero cannot become a regular centerline; neither can a tangent box containing the zero vector. Prepared real-worker tests include regular plane contacts, a nearly collapsed offset, invalid input and Retry.

Tangent and speed values are derivatives in millimeters per unit of the original fixed source parameter. They are not curve lengths or measurements of a finished fillet. The complete envelope and topology still require independent qualification.

### Final center tangent delivery

- Six native tangent/transport checks and twenty-three offset regression checks pass; the transport check overlaps the filters. The pre-fix periodic-start failure is retained separately.
- 105 product tests in nine files pass on the corrected artifact, including original and rotated curved contacts through the real worker, seam refusal, bounded work, invalid input and Retry.
- Vue/MCP type checks and production Vite build pass.
- WASM: 11,039,983 bytes; SHA-256 `7c8ab55a3714f572cb5f28556030076d29af134f030501f47c3ec31d16f02061`.
- Packed geometry chunk: 3,685,512 bytes; production assets: 7,927,322 bytes (+6,225 bytes). Explicit limits are 3,688,000 and 7,930,000 bytes.
- This certifies center tangent regularity on the declared band. Envelope construction, replacement trims, transitions and solid/UI fillet admission remain open.

## Local constant-radius envelope (native development)

`offset_envelope::certify` constructs an analytic rolling-ball surface over a certified center band. The cross-section parameter is a rational quadratic minor arc; the driving parameter retains the implicit source contact solution. This is not yet a fitted tensor NURBS patch.

For radius `r = abs(dA) = abs(dB) > 0`, contact directions are `U = -sign(dA) nA` and `V = -sign(dB) nB`. Constant-offset contact implies `|U| = |V| = 1` and `U dot C' = V dot C' = 0`. With `D = 1 + U dot V > 0`, relative controls are `U`, `(U + V)/D`, `V`, with weights `1`, `sqrt(D/2)`, `1`. Hence every section lies on the rolling sphere and its plane is perpendicular to `C'`. Both section endpoints are the original support contact points.

Source velocities come from original surface first jets and certified implicit parameter derivatives. Direction velocities are `(S_i' - C')/r`; the middle control and weight are differentiated with outward interval arithmetic. Relative coordinates avoid subtracting large translated world controls. Interval quotient differentiation bounds both surface derivatives. A positive lower bound on `|P_t cross P_s|` certifies local immersion over each entire driving band and arc cell. Endpoint contact tangency follows from the circle's radial directions and the original support normals. This does not certify embeddedness, original trim membership or a final solid.

Arc subdivision retains every failed leaf and covers `[0,1]` without gaps. The budget counts failed parents as well as final leaves and reserves work for all pending leaves. A work stop cannot silently omit an arc section or promote an incomplete result. Near-antipodal contacts, zero/unequal radii, unresolved source jets and irregular centers cannot be admitted. Global thickness, self-intersection, replacement trims, endpoint/corner transitions and finite NURBS fitting with a certified error remain required.

Five new native checks pass: the planar quarter-cylinder envelope with independent analytic image/derivative/area-speed checks; curved cylinder/oblique-plane contact in original and rotated/translated frames; all four signed contact-side combinations in both frames; whole-arc budget refusal; malformed radii/budgets. The combined offset regression filter passes 54 tests. The full-band interval certificate, rather than the finite analytic oracle checks, is the regularity proof. The WASM/worker query is qualified below; no new UI fillet operation is admitted.

### Envelope transport and worker admission

The JSON query `surface_offset_envelope` and typed MainSolid job `offsetEnvelope` expose the local analytic envelope certificate. Response admission reuses the original full-band tangent validator, verifies the requested radius and total cell budget, requires an ordered gap-free arc partition from zero to one, and checks the binary subdivision work identity `visitedCells = 2 * leaves - 1`. Every cell needs finite ordered image/derivative/area-speed intervals. A positive regularity claim requires a positive area-speed lower bound and a derivative cross-product box separated from zero. Missing cells, parallel derivative boxes, zero-inclusive area speed, radius changes, invalid work counts and promoted topology/trim/embedding/finite-patch gates are rejected.

Center derivatives are millimeters per unit of the original driving source parameter; arc derivatives are millimeters per unit of the rational arc parameter. Area speed has units of square millimeters per product of those parameter units. None is a finished-part dimension or volume measure.

Native transport plus offset regression: 55 tests pass. Actual WASM and real-worker checks pass for full arc admission, analytic image/derivative comparisons, curved/rotated contacts, work stops, malformed inputs, Retry, cancellation and genuine successful late-response refusal. The final WASM and real-worker qualification is complete as recorded below.

### Final envelope worker delivery

- 55 native offset regression tests pass, including six envelope/transport checks (five geometric and one JSON query).
- 112 product tests in ten files pass on the final artifact. Tests include complete arc partition, independent analytic image/derivative/area-speed comparisons, curved and rotated contacts through the real worker, bounded work, malformed input and Retry, actual cancellation followed by a captured successful late response, and rejection of omitted cells and promoted topology.
- Vue/MCP type checks, production Vite build, distribution/source payload verification and diff checks pass.
- WASM: 11,056,362 bytes; SHA-256 `ff6d979f30a614cbc0263930c8b3498ac3b5074f98bf3817ca6279189f0c899f`.
- Packed geometry: 3,689,468 bytes. Assets: 7,933,678 bytes (+6,356 bytes from center tangent delivery). Explicit budgets: 3,692,000 and 7,936,500 bytes.
- This is an analytic local envelope query through the existing worker. A finite NURBS patch with certified fitting error, replacement source trims, endpoint/corner transitions, embedding/volume checks and UI solid admission remain required. Variable-radius envelopes remain open.

## Finite rational patch proposal and uniform error qualification (native)

`offset_envelope_fit::propose` builds a finite degree-(1,2) rational tensor surface from two isolated contact sections. The enclosed center midpoints and floating normals only propose its endpoint control rows. They do not establish an exact fit, radius, source tangency or final topology.

`offset_envelope_fit::certify` validates the authored candidate and the two original carriers, requires natural candidate domains equal to the driving source interval and arc `[0,1]`, and requires candidate C1 continuity over the complete rectangle. It qualifies the actual rational surface against the implicit rolling-ball envelope in the same pointwise parameter correspondence.

For each complete rectangle, source and candidate interval jets enclose both derivatives of their difference. A one-ULP driving band and a single arc parameter enclose an exact contact-root anchor; the candidate anchor uses an original-tensor interval point bound, not a floating sample. The mean-value theorem then bounds every difference component by the anchor residual plus the two interval derivative differences multiplied by parameter displacement intervals. An outward norm upper bound establishes the requested millimeter tolerance. Both the candidate and envelope also require positive local immersion bounds. A positive anchor-error lower bound above tolerance disproves the candidate.

Subdivision keeps all failed leaves and reserves work for every pending leaf. Total outer cells, including failed parents, are bounded by `max_cells`; at most two envelope oracle queries are used per evaluated cell, each with the declared incident-span limit. Split scores are heuristics only, not proofs. Incomplete work retains the original full rectangle coverage and never receives a fit certificate. Unqualified periodic endpoints, insufficient knot continuity and mismatched candidate domains cannot be admitted.

Four new checks pass: a planar quarter-cylinder patch with uniform `1e-4 mm` error qualification; a curved cylinder/oblique-plane contact with uniform `1e-3 mm` error qualification in original and rotated/translated frames; candidate modification/work-stop refusal; and domain/continuity admission. Independent analytic sphere-radius checks inspect the final authored curved NURBS as a regression oracle. The full interval comparison, rather than those finite evaluations, supplies the fit proof. The combined offset filter passes 59 tests.

This is native delivery. The finite-patch query is now exposed in WASM/worker as recorded below; final UI solid admission remains open. Adaptive multi-patch fitting, endpoint tangent tolerance, replacement source trims, stitching, transitions, embedding, volume/thickness qualification and final solid admission remain open. Constant-radius fitting does not establish varying-radius support.

### Finite patch transport and partition admission

The `surface_offset_envelope_fit` operation either qualifies an authored candidate or proposes an endpoint patch and qualifies it against the original supports. The actual surface remains beside its report. Authored candidates are preserved and compared against the request snapshot; proposals are structurally bounded degree-(1,2) patches. The declared proposal section count is two (or zero for authored patches); fit oracle work is counted separately by `envelopeQueries`. A failed contact-section proposal returns no candidate or qualification.

Each evaluated fit rectangle is represented by a partition node. Internal nodes contain an axis and two child references; leaves reference exactly one final error cell. The worker validator reconstructs every midpoint split, requires exact child domains, rejects repeated/cyclic/orphan nodes and duplicate/missing leaves, and checks the full requested root rectangle. This establishes coverage without relying on area sums or sampled addresses. Validation uses linear traversal and bounded work counters. Fit success requires all leaf error bounds within the requested tolerance and both immersion flags; finite-patch and approximation flags must agree. Trim, tangent tolerance, embedding, whole-curve and solid topology gates remain false.

Native transport plus offset regression passes 60 tests. Vue/MCP type checks and artifact/real-worker checks pass. Qualified cases include full qualification, authored snapshot immutability, source mismatch, bounded work, Retry, curved/rotated contacts, cancellation and captured successful late responses, as well as corrupted trees and promoted gates.

### Final finite patch worker delivery

- 60 native offset regression tests pass, including the finite candidate/partition transport case.
- 120 product tests in eleven files pass on the final artifact. Actual worker tests qualify planar and curved/rotated finite candidates, preserve an authored candidate despite post-dispatch input mutation, reject a modified patch, retain full-domain work stops, exercise Retry and refuse a captured successful reply after cancellation. Corrupted trees, missing/cyclic nodes, changed domains, malformed surfaces and promoted gates cannot be admitted.
- Vue/MCP type checks, Vite production build, distribution/source payload verification and diff checks pass.
- WASM: 11,074,670 bytes; SHA-256 `21959d18cb126d1b303a334e4c81bfb76be08c26cd9f5e82b97d79cb0ad62773`.
- Packed geometry: 3,694,954 bytes. Production assets: 7,944,599 bytes (+10,921 bytes from analytic envelope delivery). Explicit limits: 3,698,000 and 7,947,500 bytes.
- A qualified finite patch is now available through the existing worker. Adaptive multi-patch fitting, source contact trim curves and face clipping, stitching, endpoint/corner transitions, variable radii, tangent tolerances, embedding/volume/thickness checks and final UI solid admission remain required.

## Source contact pcurves and trim-curve qualification (native)

`offset_contact_pcurve::propose` creates two source UV curves from enclosed contact sections. `certify` qualifies each against the same isolated source parameter branch over the complete driving interval. A lifted rational pcurve supplies interval UV derivatives; the implicit contact derivative report supplies the original source UV derivatives. Their difference and an enclosed exact-root anchor establish a full-interval UV error bound by the mean-value theorem. Refinement queries narrow only already certified section-root enclosures; a failed tighter query retains the last valid enclosure. Root refinement, band queries and all evaluated cells are bounded and counted. The UV tolerance is in original chart coordinates, separate from the millimeter geometry tolerance. Lifted pcurves retain the existing tensor evaluator's 256-control-row limit.

A folded source regression demonstrates why world proximity is insufficient: two different UV curves have coincident world points on overlapping charts, but only one follows the selected offset-contact branch. The wrong chart receives a positive UV mismatch enclosure. No world-distance success can bypass parameter correspondence.

`offset_contact_trims::certify` composes the finite-patch fit, complete contact admission on audited original regions, the two UV correspondence proofs, full-pcurve region membership and full-parameter world/source agreement. Source pcurves and world curves are returned separately. At a nonperiodic clamped V boundary, the patch's world curve is the original U column with unchanged knots, weights and traversal; no floating interpolation manufactures its identity. Nonclamped endpoints retain an unresolved boundary identity. World/source tolerance agreement does not become exact source equality.

Both sides share UV-cell and world-agreement budgets. Original trim audits and added pcurve membership tests share the domain-cell budget. Holes, wrong UV branches, missing work, malformed second inputs and incomplete prerequisites cannot qualify the pair. All authored source curves and regions are validated before an earlier stop can hide them.

Six new checks pass in the final combined offset run (66 tests total): two source-UV tests and four composed contact-curve tests, including curved cylinder/oblique-plane contacts in original and rotated/translated frames with `1e-6` UV and `1e-3 mm` world tolerances. Final report serialization checks also pass, including side addresses and all remaining topology gates. This stage constructs and qualifies contact curves; it does not clip complete source face loops or sew a solid. Original whole-world boundary identity, replacement face trims, stitched topology, embedding and final solid authority remain false. No new WASM/worker/UI operation is exposed yet.

## Replacement source face contours (native)

`offset_face_loops::splice` replaces a contiguous arc of whole source boundary curves with a supplied, already oriented rational contact curve and two linear UV connectors. The arc may wrap across the original loop origin. Remaining curve definitions and unaffected loops retain their original geometry; origin records identify retained source curves. No numerical trimming, reversal, snapping or resampling changes a previously qualified curve.

Qualification requires complete original and replacement region audits, unchanged winding, complete contact-image membership in the original material region, and separation of the new path from every removed boundary curve. Endpoint pairs require a separating half-plane with only the original shared endpoint permitted. Other pairs require a strictly positive interval distance lower bound. Contact membership uses adaptive parameter intervals and outward rational image bounds, so a rotated curve is not judged solely by its full axis-aligned image box. Every visited parent and all classifier cells count against the shared domain budget. Pending intervals cannot be omitted on a work stop. Work counters share the original audit, replacement audit, membership and removed-boundary checks. A failed sufficient test remains unqualified. Combined with original/new simplicity, the interior contact and excluded boundary crossings establish that the replacement material region is a subset of the original.

The module accepts UV geometry only. Contact branch correspondence and world/source agreement remain separate prerequisites. Whole-curve partial-edge partitioning, oriented correspondence integration, world connectors, end transitions, canonical shared world edges, sewn topology, embedding and thickness are still required before a fillet can be admitted. No new WASM, worker or UI operation is exposed by this module.

Five new contour checks pass: unchanged retained rational definitions; wrong contact/hole and bounded-work refusal; multiple removed curves and unchanged holes; a wrapped arc in a rotated chart with original curve addresses; malformed contact validation before an exhausted original audit. The rotated case exposed two sufficient-proof limitations. Adaptive full-parameter membership resolves the oversized full image box; normalized adjacent direction proposals handle unequal neighboring edge lengths, with admission still based on outward interval signs. Final regression runs pass 71 offset tests and 25 trim tests. Logs are archived alongside this report. This is native contour delivery; no sewn solid or UI fillet is admitted.

## Canonical patch contact and end boundaries (native)

`offset_patch_boundary::extract` selects the four original rows/columns of a nonperiodic patch clamped on both axes. They form an oriented boundary: first contact, end section, reversed second contact, reversed start section. Reversal remains coedge metadata, so arbitrary original knot values are never reflected through floating arithmetic. Exact corner controls establish closure; original weights and curve definitions remain the canonical world edge candidates. Fixed source chart axes and endpoints identify each boundary's surface support.

Contact-curve qualification now reuses this extraction and requires complete patch endpoint identity on both axes. Unclamped or periodic candidates remain unqualified for this boundary authority. These spatial end sections are the finite patch's rational boundaries; they do not yet construct endpoint transition faces, qualify tangent tolerance against adjacent faces or sew a B-rep solid. Source-face connector world curves and their correspondence, end-transition patches, orientation integration and solid checks remain required.

Three new checks pass: a rational quarter-cylinder with nonuniform original U knots and exact four-corner closure; a rotated/translated patch retaining authored boundary controls and weights; and refusal of a valid unclamped patch plus malformed rational weights. Independent finite circle evaluations are regression checks; endpoint basis selection supplies the boundary-definition identity. The final combined offset regression passes 74 tests. No WASM, worker or UI solid operation is introduced.

## Owned B-rep end-boundary correspondence (native)

`brep_core::offset_end_boundary::qualify` binds a canonical start/end section to the original transition patch and an authored adjacent surface pcurve. The patch UV iso retains the original section parameter domain. Both surface correspondences use one unchanged canonical world curve; opposite coedge traversal stays in owner metadata. Distinct faces and the transition patch boundary orientation are required. A native boundary correspondence carries shell identity, cyclic uses, parameter orientation and the model tolerance context. No extra tolerance, snapping or knot reflection is introduced.

The existing affine-planar correspondence path previously inspected the control grid without requiring uniform source weights or a complete clamped Bernstein grid. A geometrically planar surface may have a nonlinear rational parameter map, so these conditions now gate the affine lift. A full-interval agreement check additionally bounds the actual original surface composition within the model tolerance; source near-planarity and curve coefficient errors cannot independently consume the full tolerance. General rational/multispan source charts do not receive this affine authority and still require a general source-composition certificate.

The base end-boundary path qualifies a shared edge on two supports. The combined path below additionally qualifies a complete replacement adjacent UV region. World authority for all retained boundaries, model face reconstruction, missing connector/transition surfaces, complete shell sewing and embedding/thickness qualification remain required. Neither path mutates a model or admits a solid.

### Boundary-contact corner replacement (native)

`offset_face_loops::replace_boundary_arc` accepts a contact arc whose exact endpoints equal those of two or more prepartitioned removed boundary curves. The replacement contains the original contact curve and unchanged retained curves, with no artificial endpoint connector. A certified interior parameter witness, complete original/new region audits, unchanged winding and exclusion of every boundary crossing except the two owned endpoints establish containment of the open contact arc and the replacement material region. Failed checks retain an unqualified report. Single removed curves with two shared endpoints remain outside this endpoint ownership path.

For a clamped rational Bezier, positive interior Bernstein factors permit controls on a separating plane when another control is strictly on one side and all non-shared endpoints are strict. This proves tangent quarter-arcs separate from removed straight boundary segments except at their owned endpoint. General multispan curves retain the earlier stricter separation test. No sampled tangent or polygon authorizes a boundary replacement.

`offset_end_boundary::qualify_clipped_end` combines the two owned world/source support certificates with the complete adjacent replacement UV region. A contour work stop returns an unqualified contour report, not a completed face or shell. The authored source face and all retained UV boundaries remain unchanged until a separately validated model transaction applies the recipe.

The combined face recipe separately certifies the pcurve used in the new contour against the coedge traversal. A reversed adjacent owner requires a reversed UV/world parameter correspondence; the canonical forward support pcurve cannot silently become the forward contour curve. Patch cyclic end addresses and the replacement contact's cyclic index zero are checked before ownership is admitted.

Final native checks pass: 75 combined offset regressions (including tangent rational corner replacement), 18 trim-sew regressions (including nonlinear rational planar-chart refusal), and four end-boundary checks on the final oriented-contour implementation. End checks cover both canonical end arcs, original support/owner refusal, patch cyclic address refusal, and complete adjacent corner replacement with rejection of a forward contour pcurve under a reversed owner. The current B-rep code and tests compile, and diff checks pass. Native end-boundary and UV-contour delivery is complete at this scope; retained world boundary authority, partial-edge partitioning, face/model reconstruction, closed-shell sewing, embedding/thickness, general endpoint transitions and UI solid admission remain open.

## Trimmed B-rep face fragment assembly (native)

`trimmed_face_recipe::assemble` accepts original rational world edges, explicit coedge reversal flags, forward loop pcurves and the original support surface. Complete rational UV-region audit must establish a CCW outer loop and CW holes, as required by the indexed B-rep face contract. Clamped world endpoints must join bit-for-bit under the declared coedge traversal; even a sub-tolerance gap does not silently move a vertex. Full-interval source agreement uses the original model-context spatial tolerance and a shared work budget. Every authored edge and pcurve is validated before any budget or region stop can hide it.

A successful result contains an immutable, local-indexed face fragment: vertices, original world edge definitions, forward pcurves, reversal flags, the support face and tolerance-context identity. Reversed canonical edges retain their definitions and swap vertex incidence only. Failed region, orientation, endpoint, world join or source agreement checks produce no fragment. No resampling, knot reflection, snapping or tolerance increase occurs. This is a face fragment, not a sewn shell or a geometrically valid solid; canonical edge sharing with other faces, model identity/provenance inheritance, shell assembly and embedding/thickness remain required.

The combined end-face recipe now also enforces B-rep UV winding. The upper planar end fixture uses a reversed chart so a CCW UV contour maps to the opposite world traversal while preserving one canonical rational end curve. Its four native regressions pass after that correction.

Final fragment checks pass: three native tests cover preserved canonical rational definitions with reversed edge incidence, refusal of a sub-tolerance world gap and a wrong source lift, and malformed world weights under an exhausted region budget. Four end-boundary regressions pass on the winding-corrected chart. B-rep and its tests compile, and diff checks pass. Both logs and the build check are archived. Shared-edge shell assembly, topology identity inheritance, geometric embedding/volume/thickness checks and final UI operation remain open.

## Closed boundary shell assembly from qualified faces (native)

`trimmed_shell_recipe::assemble` combines immutable qualified face fragments using explicit shared-edge keys. A reused key must identify the same complete rational curve definition, exactly two distinct face uses, matching canonical endpoint vertex definitions and opposite traversal after shell face reversal. Local vertices are joined only through those explicit shared edges. Equal coordinates alone never weld independent vertices or merge an undeclared boundary. Curve definitions and forward pcurves remain unchanged.

Topology counts bound aggregate fragment work. Face-context identities must match the shell context, and source model tolerances must equal the original spatial tolerance. The existing imprint solid assembler creates one connected closed shell, validates indexed manifold incidence and inherits persistent topology identities from source models. The result is exposed as an immutable `ClosedBoundary`; it certifies boundary assembly at this scope, not embedding, material volume or wall thickness. Periodic same-face seam reuse, multiple shells/cavities, actual transition-region replacement in a source model, complete material/geometry audits and UI solid admission remain open.

Five native regressions pass: a six-face cuboid reconstructs its eight vertices, twelve edges and original vertex, edge, loop, face, shell and body identities; coincident geometry with an undeclared shared key remains open; duplicate keys, wrong face orientation and foreign context refuse; a rational quarter-cylinder sector with curved side and end caps reconstructs one closed boundary retaining every source world curve and full edge/source agreement; and an explicit key collision between different full curve definitions refuses. These are shell assembly fixtures, not qualification of general rolling-ball fillets.

The final five-test run includes identity-set comparisons for every topology entity class on the unchanged cuboid reconstruction. B-rep/test compilation and diff checks pass. Native shell and build logs are archived. No WASM/worker/UI fillet command or geometric solid-validity certificate is introduced.

## Fresh geometric volume qualification of assembled shells (native)

`ClosedBoundary::audit_volume` runs the existing full geometric volume auditor on the same immutable assembled model. `VolumeReport` privately retains the candidate and fresh report. Only a proven report can produce an immutable `QualifiedVolume`; incomplete work returns the original report/candidate without solid authority. The full report remains available through read-only access. No caller-authored Boolean or independently supplied report can qualify a different model through this path.

The volume audit requires exact full edge/source identity and joins, complete rational trim audits, injective face charts, all face-pair contact classifications, consistent shell/material roles and outward orientation. It does not establish a requested minimum wall thickness, a rolling-ball radius or G1/G2 tolerances. Those remain separate feature requirements. General unsupported geometry retains unresolved evidence rather than gaining volume authority.

Nine combined native shell/volume regressions pass. Fresh audits qualify the reconstructed cuboid and rational quarter-cylinder sector with end caps. The curved fixture reports exact agreement, exact joins, valid trims, injective charts, classified pairs, embedded boundary and qualified volume. A one-pair work stop preserves its candidate without promotion; a globally inverted but combinatorially closed and embedded shell fails material orientation and cannot become a qualified volume. Earlier sharing, context, full-curve collision and identity preservation checks remain green. Actual source-model transition replacement, general fillet geometry, thickness qualification and WASM/worker/UI operation remain open.

## Qualified partitioning of original boundary curves (native)

`curve_partition_agreement::partition` proposes complete original-parameter pieces with numerical knot insertion/clamping, then independently qualifies each against the original definition. Every source/candidate knot break splits the proof domain so both original-span rational jets apply. A certified midpoint anchor and the difference of cell-normalized first derivatives bound full-cell deviation by the mean-value theorem. The binary64 midpoint is not assumed to equal the exact half parameter: outward interval ratios bound both possible displacements. Error bounds use the input coordinate units; world millimeters and source UV tolerances remain separate caller choices.

Visited parents count against one shared partition budget. Pending cells and all remaining pieces survive a work stop with unresolved reports; incomplete coverage never receives geometry-preservation qualification. Ordered cuts and original domains are checked before proposals, and every candidate retains the requested original endpoint parameters. This path currently requires nonperiodic continuous original/candidate curves. No knot reflection, parameter normalization or source mutation occurs.

Four native checks pass: rational multispan source partitioning in both 2D and 3D with complete interval coverage; altered candidate and shared-work refusal; malformed cut order and foreign parameter domain; and coincident midpoint anchors with wrong endpoints over the nonunit domain `[2,8]`. The last check refuses a candidate despite its matching midpoint and establishes that normalized derivative units are handled correctly. Geometry preservation is a tolerance bound, not exact source identity. Candidate endpoint joins are separately reported by bit equality; this does not authorize snapping or topology mutation. Source edge identities/lineage, coupled world/pcurve partitioning, original face reconstruction and final fillet application remain required. No WASM/worker/UI query is exposed yet.

Validation: four native partition tests passed; `cargo check -p brep-core --tests` passed with existing warnings. Logs: `curve-partition-agreement-native.log.gz`, `curve-partition-brep-check.log.gz`.

## Coupled world/pcurve partition proposals (native)

`boundary_partition::split` maps ordered forward traversal fractions into the original world and UV parameter domains. Reversed world edges pair pieces in reverse order while retaining their canonical definitions and reversal metadata. Independent full-interval source partition checks and world-to-surface lift checks are required for every pair; exact candidate endpoint joins are a separate gate. Work exhaustion retains all candidate pairs and omitted reports without qualification. Rounded parameter mapping is a proposal, not exact identity. No B-rep mutation, persistent identity assignment, WASM query or UI operation is admitted by this report.

Two native tests passed: forward/reversed ownership on distinct nonunit domains; wrong lift and shared lift-budget exhaustion. `cargo check -p brep-core --tests` passed with existing warnings. Logs: `boundary-partition-native.log.gz`, `boundary-partition-brep-check.log.gz`.

## Partitioned boundaries in face recipes (native)

`trimmed_face_recipe::split_boundary` exposes candidate contour boundaries only after source partition agreement, candidate world/UV joins, source lift and bit-identical original world endpoints. The final face assembly independently rechecks the complete region, winding, world joins and every lift. No shell mutation or persistent source-edge identity assignment is performed.

A rational quarter-arc regression exposed different endpoint bits from independent original trims, despite successful source and lift agreement. Partitioning now first refines one shared curve at all cuts, then extracts pieces. This is a numeric proposal: every piece is still qualified against the original definition. No endpoint snapping or tolerance growth is used. Shared refinement can refuse the existing control-point resource limit.

Five face-recipe tests and eight partition-related native tests passed. The rational split reconstructs a five-edge face on the unchanged support surface; wrong lifts retain reports but expose no usable boundaries. B-rep/test compilation passed with existing warnings. Logs: `split-face-recipe-native.log.gz`, `shared-partition-native.log.gz`, `split-face-recipe-check.log.gz`. The initial failed diagnostic is retained in `split-face-recipe-diagnostic.log.gz`.

## Source face boundary replacement (native)

`trimmed_face_recipe::replace_boundary_arc` independently qualifies the complete original face, the replacement UV region and the resulting world-boundary face. It uses the contour audit origins to retain unchanged world curves, pcurves and reversal metadata; the contact supplies its authored world/pcurve pair. Limits apply independently to each phase, as documented by the API. Replacement inputs are validated before original-face refusal. A successful UV contour cannot bypass full world joins or source-lift verification in the replacement face. This returns face fragments only, without shell mutation, source-edge identity reassignment, global embedding, radius, tangent or thickness admission.

Six face-recipe tests passed, including replacement of two square boundary edges by a rational quarter-arc on the unchanged support, preservation of the retained world edges, and refusal when the same UV arc has an incorrect world lift. B-rep/test compilation passed with existing warnings. Logs: `replace-face-recipe-native.log.gz`, `replace-face-recipe-check.log.gz`.

## Canonical transition ends in replaced support faces (native)

`offset_end_boundary::qualify_replaced_face` binds the unchanged canonical patch end to its support and to the separately oriented loop pcurve, then uses the complete source-face/UV-region/world-face replacement audits. The resulting adjacent face carries the same world curve and explicit reversed coedge metadata; no floating curve reversal or endpoint snapping is performed by the operation. Existing ownership checks require the replacement contact at cyclic index zero. Limits apply independently to each replacement audit phase. Shell assembly, geometric volume validity, radius/tangency and thickness remain separate.

Five end-boundary tests passed. The new case builds both endpoint support faces, including the upper end with a reversed world coedge and a swapped planar chart, and verifies unchanged retained world edges and support surfaces. B-rep/test compilation passed with existing warnings. Logs: `replaced-end-face-native.log.gz`, `replaced-end-face-check.log.gz`.

## Replaced partial-corner caps in a qualified closed volume (native)

A finite integration fixture uses a unit-square profile with a rational radius-0.25 corner and extrusion height 2. It first independently audits sharp prepartitioned cap contours, replaces the two corner edges on both caps, retains the other cap edges, and sews the resulting caps with the curved wall and four planar side faces using declared full-definition edge keys. All seven faces form one closed shell. Fresh volume qualification proves exact world/source boundary agreement, exact joins, complete trim validity, chart injectivity, classified face contacts and material orientation for this candidate.

The focused native test passed (`partial-corner-shell-native.log.gz`). The curved side and retained side faces are supplied by the existing prism constructor; this is an integration/qualification fixture, not an automatic fillet operation on arbitrary source B-rep. Rolling-ball construction, general side-face replacement, source-edge lineage, radius/G1 qualification, wall thickness and WASM/worker/UI admission remain open.

## Full-contact tangent-plane angular qualification (native)

`contact_normal_agreement::certify` first requires complete world/pcurve/source position agreement on both supports. It then adaptively covers the full normalized world traversal, maps each side through its original pcurve domain and reversal, and bounds the squared sine of the angle between the original surface normals over both UV image rectangles. The normal enclosure covers every intersecting knot span, including both incident branches. Positive normal-length lower bounds are required by the underlying angular verifier. Opposite normals represent the same tangent plane; shell orientation is a separate gate.

The original positive-weight pcurve control hull independently proves natural-domain inclusion; intersecting interval image bounds with that hull removes outward rounding beyond exact boundary coordinates without snapping. A control hull outside the natural domain remains unqualified even if the actual curve might be inside. Failed parents count toward work. Exhaustion retains complete parameter coverage with unresolved cells. This certifies contact positions and tangent-plane angular tolerance, not radius, branch topology, full G1/G2, retained material thickness or a general fillet operation. There is no WASM/worker/UI query yet.

Three native tests passed: coincident endpoint normals with an interior tangent defect; a reversed pcurve on distinct nonunit parameter domains and positional work stop; both planar contacts of a rational quarter-cylinder transition plus normal-span work stop. B-rep/test compilation passed with existing warnings. Logs: `contact-normal-native.log.gz`, `contact-normal-check.log.gz`.

## Joint contact-trim and tangent-plane gate (native)

`offset_contact_trims::certify_with_tangency` performs fresh full-envelope fit, original offset-contact UV branch checks, source trim membership, world/source contact agreement and both contact tangent-plane checks on the same supplied definitions. Canonical patch boundary curves remain unchanged; linear patch pcurves use the original driving domain and exact clamped V endpoint. Normal checks run only after contact-trim qualification. Invalid angular work/tolerance and periodic source pcurves are rejected before any earlier fit stop. Each side has independently bounded extra positional, normal-cell and normal-span work.

The joint `contact_curves_and_tangent_planes_proven` result requires both complete normal reports. Positional fit alone, one successful side or a normal work stop cannot promote it. This gate does not admit replacement topology, global G1/G2, wall thickness or a complete fillet solid. The existing serialized contact-only report still truthfully leaves tangent authority false; the joint native query is not exposed through WASM/worker/UI yet.

Four contact-trim regression tests passed. The planar rolling-ball fixture additionally passes the joint fit/UV/trim/tangent gate on both sides; a normal-span budget of one retains successful contact-trim evidence but refuses the joint result. The other existing curved, malformed-input, hole and work-stop checks remain green. B-rep/test compilation passed with existing warnings. Logs: `contact-tangent-gate-native.log.gz`, `contact-tangent-gate-check.log.gz`.

## Contact qualification through WASM and worker (local product)

`surface_offset_contact_qualification` returns the full request snapshot, actual authored candidate and combined contact/tangent-plane report. Delivery retains the complete fit tree, original source UV reports, membership/agreement reports, both angular parameter covers and unknown cells on work stop. Topology, radius, global G1 and embedding gates remain false. Native delivery tests pass for qualification, a normal-span work stop and malformed second pcurve despite a preceding fit work limit.

`nurbsOffsetContactQualification.ts` and the `offsetContactQualification` MainSolid job bind responses to a cloned request snapshot and the authored candidate. Validation reuses the complete fit-tree validator, checks source pcurves and canonical patch boundary definitions, full UV/normal interval coverage, angular verdicts, both positive normal enclosures and declared budgets. A partial/mismatching fit cannot expose angular authority. Stale request snapshots, changed candidates, omitted angular cells and inconsistent angular bounds refuse.

Four actual-WASM/product tests pass, including real worker-handler execution, busy/error recovery, work stops and candidate mismatch. The earlier five-file regression run passed 79 tests (including three of the four new product cases); the final new-file run passed all four. Vue and MCP type checks, Vite production build and dist verification pass. These are local checks; no CI or new interactive fillet command is claimed.

WASM SHA-256: `d846ac669bcd6a65a37be3a0833cba5131b81c754130b4e415b1079e67e0ef1a`; raw geometry WASM: 11,113,932 bytes; packed geometry JS: 3,708,032 bytes; production asset bytes excluding streamed WASM: 7,963,849. Size budgets record these measured feature additions with bounded headroom. Logs use the `contact-qualification-*` names in this directory. General source-side replacement, owned boundary endpoint admission, radius/wall qualification and the interactive fillet operation remain open.

## Original B-rep edge support preparation (native)

`offset_edge_supports::prepare` resolves one authored edge to exactly two distinct adjacent faces in the same closed body shell and requires opposite effective edge traversals. It independently audits each complete source face region and every world/pcurve lift through the face recipe. Preparation refuses material-free/open/ambiguous ownership and retains both face audits when geometric qualification stops. This is combinatorial body ownership plus face-region/lift qualification, not a proof of source volume embedding or material orientation.

The immutable prepared input preserves the canonical edge curve and edge/body/shell identities. Each immutable support carries its original surface and complete loops, original face identity, selected source loop/cyclic address, pcurve and reversal, shell face reversal, and source identities of the canonical local edges. No pcurve/world reversal or source model mutation is performed. Private fields bind the metadata to the copied qualified face snapshot; immutable qualified face fragments now support cloning. This native input is not exposed through WASM/worker yet and does not change the shipped WASM query set. Automatic offset-band selection, signed material-side/radius decisions, endpoint handling, source lineage and complete fillet solid admission remain open.

Three native tests passed: every edge of a cuboid binds two unchanged original faces/curves/IDs; a rational prism edge retains its curved and planar supports; source-face work stop and missing body ownership refuse preparation. B-rep/test compilation passed with existing warnings. Logs: `offset-edge-supports-native.log.gz`, `offset-edge-supports-check.log.gz`.


## Automatic local section proposals from source edges (native)

`offset_contact_predictor::propose` uses bounded damped Newton steps on original offset surface evaluations, with one explicit fixed source coordinate. It never clamps a proposed iterate to the source domain. A small numerical residual only triggers a fresh independent interval section query; it does not certify the root itself. The returned center is a numerical midpoint, while the optional certificate carries separate interval evidence. Singular derivatives, unsuccessful descent, insufficient work and unrepresentable search padding retain an unqualified numerical result.

`offset_edge_supports::propose_section` derives UV seeds from the original selected pcurves and their traversal metadata. Its numerical axis choice is local; no global monotonic driver, whole-edge correspondence or continuous branch is claimed. Equal radius magnitudes and signed source distances are explicit inputs. Face-use reversal alone is not material-side authority. Root trim membership, contact radius/tangency over the complete edge, source-face replacement, shell volume and an interactive fillet remain separate required gates. This native API is not yet exposed through WASM/worker/UI.

Seven native tests passed: all twelve rotated cuboid edges, a rational circular edge, original source identity preservation, preparation refusal, insufficient versus sufficient interval work on the same multispan cap carrier, singular parallel offsets and malformed inputs. Extrusion partitions profile spans into separate faces, so the budget fixture explicitly retains a multispan affine cap carrier. B-rep/test compilation and diff whitespace checks passed. Logs: `offset-edge-predictor-native.log.gz`, `offset-edge-predictor-check.log.gz`, `offset-predictor-unit.log.gz`. No WASM rebuild or interactive command is claimed for this native-only step.


## Continuous source-coordinate offset paths (native)

`curve_axis_driver::certify` checks the complete original clamped pcurve, including every nonempty knot span. Each adaptive cell has a strictly signed derivative interval in its own normalized parameter. Only signs are combined across cells. Continuous knot joins and consistent signs prove a one-to-one coordinate driver; endpoint positions use original clamped control points. A turning coordinate, zero derivative, periodic/nonclamped input or work stop cannot acquire driver authority.

`offset_contact_path::certify` builds numerical tube proposals from three bounded section solves per evaluated node. Every admitted leaf has an independent uniform interval root certificate over its entire driving interval. Adjacent tubes must additionally share a certified section root in their intersection; tube overlap alone is insufficient. Failed parents count toward the cell budget. Exhaustion preserves a complete ordered driving cover with unresolved leaves. Numerical, band and section query counts include section checks performed by the predictor; join checks are bounded by the number of neighboring leaves.

The uniform section operator preconditions the full driving derivative enclosure before multiplying the shared scalar parameter displacement. The midpoint residual remains an interval enclosure of original offset definitions. This retains coordinate dependency on rotated carriers while preserving contraction, strict inclusion, source continuity and whole-interval derivative gates. It does not substitute sampled motion for a bound.

`offset_edge_supports::propose_path` performs a fresh driver check against immutable original supports and constructs only numerical endpoint seed interpolants. The requested driving interval stays inside the original pcurve coordinate extent. This proves a connected root path in original charts, not source trim membership, material-side selection, completeness outside the tubes, endpoint transitions, global G1/radius or fillet volume. No WASM/worker/UI path query is delivered yet.

Five B-rep tests passed, including continuous paths on all twelve rotated cuboid edges and the original rational circular edge. Eighty offset regression tests passed, including a planar uniform path, adaptive quarter-cylinder path with certified adjacent joins and complete unresolved coverage under a one-cell budget. The separate driver test rejects an interior turn despite ordered endpoints and admits a decreasing coordinate on a nonunit domain. B-rep/test compilation and diff whitespace checks passed. Logs: `offset-edge-path-native.log.gz`, `offset-edge-path-check.log.gz`, `offset-path-regression.log.gz`, `curve-axis-driver-native.log.gz`. These native tests do not qualify the shipped WASM for the new path query or admit a complete fillet operation.


## Connected path membership on original face trims (native)

`offset_path_trims::certify` computes a fresh connected offset path, validates both original loop sets and audits both complete UV regions under shared pair, region-cell and domain-cell budgets. Every membership leaf must enclose both source contacts entirely inside the original regions. Unresolved classifications trigger bounded subdivision in breadth-first order, so one boundary contact cannot consume all work before distant intervals are examined; an outside rectangle refuses the whole path. Region or membership work stops retain a complete ordered driving cover with unqualified leaves.

Each refined membership query uses its unchanged original parent tube. A successful query tightens the root enclosure. An inconclusive query retains the already proven ancestor enclosure restricted to the exact driving subinterval; it cannot create new root authority. A contradictory root exclusion returns a numerical error. This preserves the parent's connected branch identity instead of gluing unrelated sampled roots. Additional root-band queries and membership visits have separate counts; domain work includes region audits and all classifications of failed parents.

`offset_edge_supports::propose_trimmed_path` uses both complete immutable source-face loop sets. Driver proof and numerical seed preparation are shared with the existing path query. No original curve/surface definition is reversed or changed. Strict interior membership still refuses boundary-touching contacts; owned endpoint admission, generated source contact pcurves, retained-face replacement, end transitions and volume/radius/tangent qualification remain open. The API is native-only.

Eighty-three offset regression tests passed, including both original regions, a crossing of a hole in the second source face, membership/region/domain work stops with complete unqualified coverage, and malformed second loops before a first audit work stop. Five B-rep tests passed with fresh full trim membership on all twelve rotated cuboid edges and the original rational circular edge. B-rep/test compilation and diff whitespace checks passed. Logs: `offset-path-trims-regression.log.gz`, `offset-edge-trimmed-path-native.log.gz`, `offset-edge-trimmed-path-check.log.gz`. The tests qualify interior contact paths; they do not construct a modified fillet body or qualify interactive behavior.


## Shared piecewise source-contact pcurves (native)

`offset_path_pcurves::propose` performs fresh connected-path and original-region admission, then constructs piecewise linear UV proposals. Each shared station is computed once, in the intersection of every incident original tube, with bounded independent section queries. Refined witnesses may tighten the chosen midpoint proposal; inconclusive refinement retains earlier evidence, while exclusion of an already certified root raises a numerical error. Both neighboring curves copy the same stored UV coordinates and parameter bits. Complete endpoint identity is checked explicitly by binary64 bits.

Every candidate piece requires two complete original-branch correspondence reports and two full candidate membership classifications on the original trim domains. UV tolerance is measured in source parameter units; it is not a world-space radius or positional tolerance. Failed proposals subdivide in breadth-first order. Failed parent attempts and all station/correspondence/domain work counts remain in the report. Work stops retain an ordered complete driving cover with unqualified pieces; one successful side cannot admit a piece.

`offset_edge_supports::propose_source_pcurves` binds this construction to immutable original B-rep supports, fresh coordinate-driver proof and complete unchanged source loops. The returned curves are tolerance-qualified source-parameter approximations. Canonical world contact curves, exact world/source identity, owned boundary endpoints, retained-face replacement, end transitions, radius/normal/wall checks and closed volume admission remain required. No WASM/worker/UI pcurve query is delivered in this step.

Eighty-six offset regression tests passed. New cases cover full planar branch/membership evidence, adaptive rational circular contacts with shared station indices and bit-identical UV joins, and station/correspondence/original-region work stops retaining complete unqualified coverage. Five B-rep tests passed with generated source pcurves on all twelve rotated cuboid edges and the original rational circular edge. B-rep/test compilation and diff whitespace checks passed. Logs: `offset-path-pcurves-regression.log.gz`, `offset-edge-pcurves-native.log.gz`, `offset-edge-pcurves-check.log.gz`. Planar B-rep cases use UV tolerance 1e-5; rational cases use 1e-4. These are source-chart tolerances and do not establish equivalent world-space error, radius or solid admission.


## Original-source 3D contact curves and millimeter propagation (native)

`curve_surface_agreement::verify_on` verifies complete normalized world-curve traversal against an explicit interval of the unchanged original pcurve. Correlated Bernstein extraction and Cartesian fallback preserve the original source parameter mapping, nonunit domains and world-coedge reversal. No numerically trimmed pcurve replaces the authored definition. The existing full-domain verifier delegates through the same interval implementation.

`curve_surface_lift::propose` builds bounded adaptive cubic Hermite candidates, falling back to endpoint chords when numerical derivatives are unavailable. Numerical samples only propose geometry. Every admitted piece has a complete interval composition check against its original pcurve interval on the original surface. Candidate pieces share stored world endpoint coordinates by binary64 identity; source pcurves remain unchanged. Validated clamped pcurve endpoints use their exact original Cartesian controls instead of rounded weight cancellation. Failed attempts, pending interval coverage and point-station/sample/agreement counts remain available under work stop. Position tolerance does not admit exact world/source identity, G1 or topology.

`lipschitz_upper` bounds the Frobenius norm of the complete continuous original point-map Jacobian over every natural-chart span. It refuses periodic or discontinuous carriers and retains work-stop uncertainty. This is a positional bound without an injectivity or normal claim. `offset_edge_supports::propose_world_contacts` performs fresh source UV construction and both world lifts. It combines each final piece's complete UV error bounds with the original point-map bound and declared lift error using outward binary64 arithmetic. Both contact bounds must fit the requested millimeter tolerance; otherwise the result remains unqualified, even when its source curves or world lifts succeeded. Surface scale, source/lift tolerances and contact error bounds remain separate.

The complete NURBS kernel passed 635 tests. Five B-rep tests passed, including world contacts on all twelve rotated cuboid edges, the original rational circular edge, an intentionally tighter contact tolerance and insufficient world-lift work while source pcurves remain qualified. The 23-test curve/surface subset additionally checks unchanged rational subinterval definitions, reversed/nonunit mapping, adaptive rational 3D lifts, complete work-stop coverage and a weighted clamped endpoint at the original surface domain boundary. B-rep/test compilation and diff whitespace checks passed. Logs: `world-contact-nurbs-regression.log.gz`, `offset-edge-world-contacts-native.log.gz`, `offset-edge-world-contacts-check.log.gz`.

Rotated cuboid fixtures request lift tolerance 1e-7 mm and contact tolerance 1e-5 mm; the rational fixture requests lift tolerance 1e-5 mm and contact tolerance 1e-3 mm with source UV tolerance 1e-4. A 1e-10 mm contact request retains successful lifts but refuses the combined positional gate. No new WASM/worker/UI query, source-face replacement, owned endpoint admission, exact coedge identity, radius/tangent/wall guarantee or modified closed fillet body is claimed.

## Piecewise source-face boundary replacement

`offset_face_loops::replace_boundary_path` and
`trimmed_face_recipe::replace_boundary_path` accept an ordered contact chain.
Every internal UV endpoint must be bit-identical; contacts retain individual
source addresses, while retained boundary definitions remain unchanged. Original
and replacement region audits, every removed-boundary separation, original
surface/world agreement and exact world incidence remain required. Work budgets
are shared across the full contact chain. No connectors are manufactured.

Validation: 7 NURBS contour tests and 6 B-rep face recipe tests passed. The new
case exercises a two-piece planar contact, rejects a sub-tolerance UV gap and
rejects a world curve leaving the source surface. Existing rational corner,
hole, wrapped arc, work-stop and boundary partition regressions remain green.
Logs: `contact-path-nurbs.log.gz`, `contact-path-brep.log.gz`.

This admits a supplied, already endpoint-owned chain replacing a prepartitioned
arc. Automatic partition of adjacent original edges, generated contact-to-world
subinterval alignment, smooth end transitions, general sewn solids and UI/WASM
integration are still outstanding. This is not general fillet admission.

## Original endpoint-edge partition and contact replacement

`trimmed_face_recipe::partition_and_replace_path` partitions two distinct
original boundary edges at supplied forward pcurve fractions and replaces the
cyclic arc between those cuts. Each coupled world/pcurve partition is freshly
qualified against its original definitions and surface. Original face audit,
expanded contour audit and final face audit remain independent requirements.
Reversal metadata is retained; no world vertex welding or endpoint snapping is
performed. Reports retain both partition results and all available face audits.
Limits apply per audit/partition phase, not as one aggregate operation budget.

Seven face-recipe tests passed (`source-cut-replacement.log.gz`), including
reversed canonical source edges, wrapped cyclic arcs, unchanged retained whole
edges and rejection of a sub-tolerance world endpoint mismatch.

Contact intersection parameters are still supplied explicitly. Automatic
root-to-boundary intersection isolation, authoritative exact source endpoint
representation, generated world/pcurve subinterval alignment and final shell
assembly remain outstanding. No general fillet or UI command is admitted here.

## Original UV curve crossing isolation

`uv_curve_crossings::isolate` searches the complete original nonperiodic
2D knot-span product under a shared cell budget. Original interval blossom
positions and cell-normalized rational derivatives feed a two-variable
Krawczyk enclosure. Only strict inclusion and an outward contraction norm
below one admit a unique transverse crossing. Disjoint original position
enclosures exclude cells. All unvisited or undecided boxes remain explicit;
coincidence, tangency, domain/knot boundary roots and numeric work stops do not
become tolerance-based crossing authority. Root intervals retain original
parameters, including nonunit domains. Midpoint evaluation restores the exact
source origin removed by homogeneous extraction.

Three focused tests cover nonunit line crossings/exclusions, rational arc
crossings and a near nonintersecting pair, coincidence and budget exhaustion.
All 639 NURBS library tests passed (`uv-crossings-nurbs-regression.log.gz`).

This isolates supplied UV curve intersections; it does not solve the implicit
offset contact system against an original face boundary. Root intervals are
not exactly representable split parameters or shared world vertices. Boundary
ownership, original source references, world realization, closed-shell
admission, WASM and interactive fillet integration remain outstanding.

## Root refinement and original face-boundary addresses

`uv_curve_crossings::isolate_refined` refines each already certified crossing
under the same global budget as span-product search. Every smaller box is
freshly checked against the original definitions. Refined enclosures intersect
the prior enclosure; exclusion/disjointness contradicting a certified root
is a numerical error. Work/conditioning stops retain the last valid enclosure.
`precision_proven` additionally requires complete search and outward width
bounds within both requested original parameter tolerances. It is distinct
from root existence/uniqueness. No caller report is accepted as authority.

`trimmed_face_recipe::locate_contact_boundaries` freshly qualifies the original
face and searches a supplied UV contact against all original contour edges,
including holes. Reports retain original loop/boundary addresses and original
parameter enclosures. One shared root budget covers all boundaries; untouched
boundaries after exhaustion retain full-domain unresolved boxes. Face audit
limits remain a separate phase budget.

Validation: 5 crossing/refinement tests and 8 B-rep face tests passed on the
final sources. A rational crossing reaches requested parameter widths; an
unattainable precision request preserves its certified root without claiming
precision. Source face tests find two addressed original boundary roots and
retain remaining boundaries when the shared budget is exhausted. Logs:
`uv-crossings-refinement.log.gz`, `source-face-crossings.log.gz`.

These are supplied curve crossings, not implicit offset-to-boundary roots.
They do not authorize midpoint split parameters, rounded shared vertices,
closed shells, WASM/UI fillets or general endpoint transitions. An exact source
endpoint representation and contact/world alignment remain required.

## Immutable original source-expression contact points

`source_contact_point::SourcePoint` privately owns original surface and both
original UV curve definitions plus a unique-root selector. Its geometric point
is the source surface evaluated at that exact crossing, rather than a rounded
Cartesian vertex. Cached root/UV/world interval enclosures are immutable.
Definitions persist original sources and selector only; `restore` freshly
rechecks the selector using `uv_curve_crossings::certify_box` and rebuilds all
bounds. Imported claims or bounds do not authorize a point.

Surface mapping requires independently proven chart membership: the complete
original boundary Cartesian control hull must lie inside the nonperiodic
source chart. Positive rational basis membership justifies intersecting outward
UV enclosures with that chart. Otherwise mapping stays unqualified. Active
surface-span products are counted before world interval evaluation.

`locate_source_contact_points` connects these expressions to freshly qualified
original face-boundary search addresses. Search, fresh point checks and mapping
use explicit separate phase budgets; point checks and mapping each have one
shared budget across all roots. A work stop preserves every root address with
no fabricated point. Point selector requalification currently uses the original
isolating cell, so its cached bounds may be wider than refined search bounds.

Validation: 6 original crossing tests, 2 source-point tests and 8 face recipe
tests passed. Source-point roundtrip reconstructs the same source definition
and bounds; forged readiness/bounds are ignored; changing a selector to a
root-free box produces no point. Unproven chart membership refuses world
mapping. Face tests map two original roots and preserve the unmapped second
root when the shared mapping budget ends. Logs: `source-box-crossings.log.gz`,
`source-contact-point.log.gz`, `source-points-face.log.gz`.

This source-expression point is not yet a `Model` vertex. It supplies neither
exact world-edge/source-surface identity nor equivalence of different source
expressions. Boundary fragments with root-valued endpoints, stable topology
ownership, serialization in document history, shell/volume admission and
WASM/UI integration remain outstanding. No general fillet is admitted here.

## Root-valued source restrictions and automatic contour proposal

`source_boundary_fragment::Fragment` stores the unchanged original UV curve and
surface with ordered parameter or source-crossing endpoints. Restrictions and
splits retain those source expressions; no rounded `Curve::trim` definitions
or Cartesian endpoint welding are generated. Endpoint roles must bind to the
exact source surface/curve; interval separation proves traversal order. Source
curve/surface knot continuity is required. Crossing joins use a shared original
source-point definition, including cross-curve joins through the same crossing.
Natural clamped endpoint joins use exact UV controls on the same source surface.
Restoration requalifies root endpoints with one shared mapping budget and
recomputes source binding, order and chart membership.

Chart membership uses the full original control hull or a restricted original
interval image. A single positive rational linear Bezier restriction additionally
uses the convex source chart and certified endpoint membership; this admits a
clipped straight contact whose untrimmed source extends beyond the chart. Two
inside endpoints do not admit a nonlinear arc that leaves the chart.

`source_contour_proposal::propose` freshly searches/maps contact crossings,
selects one root on each chosen original endpoint edge and builds the cyclic
replacement as source fragments. It finds both parameters automatically and
preserves original curve definitions, holes and the retained cyclic arc. Exact
source joins are checked around every contour. The output is explicitly only
a closed source-expression contour candidate, not a qualified replacement face.

Validation: 27 selected B-rep source tests passed on final sources
(`source-fragments-regression.log.gz`). New cases cover shared root split ends,
reversed traversal, cross-curve joining, source-definition preservation and
roundtrip, rejection of forged source binding/order and nonlinear chart escape.
The integrated planar case automatically clips a straight contact against two
original edges, preserves all retained source curves and rejects incomplete
search budgets.

General rational restricted chart membership, full candidate region containment
and hole/crossing audits, exact world-edge realization, topology/history and
volume admission, implicit offset boundary roots and WASM/UI fillets remain
outstanding. No final body or smooth end transition is admitted here.

## Interior contact and qualified straight half-plane source regions

`qualify_interior_contact` freshly builds the source contour, requires complete
original boundary search with only the two owned transverse endpoint roots,
proves contact simplicity through an original monotone coordinate, and
classifies an interval-valued interior station against the complete original
material region. Continuity and excluded further boundary contacts extend that
witness to the full open contact arc; owned endpoints belong to its closure.
An additional hole crossing or a station inside a void cannot admit the arc.
This arc proof alone does not authorize replacement-region containment.

`qualify_linear_region` adds exact half-plane clipping qualification for a
single positive rational linear Bezier contact. It does not approximate curved
contacts. Whole retained and removed original outer arcs must occupy opposite
sides. Original source control hulls prove complete-curve side membership;
linear partial edges use original clamped controls and root-expression endpoint
identity on the contact line. Other restrictions need complete interval image
side proof. Side arithmetic is outward-rounded and source control work has an
explicit shared budget/work-stop state. Source face orientation contracts remain
unchanged: clockwise outer input without a corresponding supported face recipe
is refused by original face qualification.

Each original hole must have a whole-source kept-side or removed-side proof.
Kept holes preserve their definitions; wholly discarded holes are removed from
the qualified UV region. Unresolved placement or a cut intersecting a hole
prevents admission. Immutable `SourceRegion` contains the qualified source
loops and their original loop indices. The proposal loop payload and its origin
map are updated together before region containment is marked proven, so that
proof cannot refer to the earlier unfiltered hole set. This is UV region
qualification, not an embedded 3D face or closed volume.

Validation: all 27 selected B-rep source tests passed on final sources
(`source-region-regression.log.gz`). Extended cases cover interior arc admission,
extra crossings through a hole, an arc traversing a void, horizontal and vertical
clipping, retained/removed/mixed holes with original indices preserved, clockwise
source-contract refusal and shared control-budget exhaustion.

General curved contact region proofs, partial rational side bounds near root
ends, cut holes, world embedding/exact edge realization, shell/volume admission,
implicit offset endpoint roots and WASM/UI fillets remain outstanding.

## Curved original-source regions and root-aware winding

`qualify_curved_region` now qualifies a simple contact crosscut of the original
outer UV loop after fresh source-face, complete boundary-crossing and interior
contact checks. Original polynomial and nonuniform rational contact definitions
are retained. The resulting Jordan region preserves the original outer
orientation. Each disjoint original hole is classified against the new outer
loop; retained holes keep their original definitions and loop indices, while
wholly discarded holes are removed. The proposal payload and lineage map are
updated together before region containment is admitted.

`source_contour_winding` uses original curve interval images and root-valued
endpoints. Root caps, adaptive middle cells and explicit endpoint join homotopies
must all stay separated from the complete inflated query rectangle. Chords are
winding witnesses inside these convex boxes, never replacement geometry.
Unresolved root caps, boundary bands and shared work-budget exhaustion refuse
classification and retain the uncertain fragment address.

The coordinate driver additionally proves monotonicity of a single clamped
positive rational Bezier with weakly ordered Cartesian poles and distinct
endpoints. This permits zero endpoint derivatives; it does not prove a regular
inverse or replace independent root Jacobian gates. Fragment chart membership
can use independently qualified coordinate monotonicity, with a bounded 4096
visits per required axis. Natural endpoint membership uses the actual last
control point, including nonlinear curves.

Validation on final sources: 29 selected B-rep source tests, all 643 NURBS library
tests and 5 original-edge support tests passed. Logs: respectively
`curved-source-region-final.log.gz`, `curved-region-nurbs-regression.log.gz` and
`curved-driver-supports.log.gz`. Cases include rational quarter-circle winding,
polynomial and nonuniform rational crosscuts, retained/discarded holes, budget
refusal and nonlinear endpoint chart rejection.

This stage qualifies source UV material regions only. Cut-hole boundaries,
non-monotone general contact simplicity, exact world-edge and vertex realization,
3D embedding, end transitions, shell/volume admission and WASM/UI fillets remain
outstanding.

## Original world-expression enclosure

`Fragment::world_enclosure` maps the complete original UV restriction through
its original surface, retaining both uncertain root caps. It produces outward
world bounds for S(C(t)); it does not trim or fit a Cartesian curve. Original
curve spans and active surface span products share a bounded work allowance.
Exhaustion returns no world box and an explicit incomplete state. Chart clipping
uses the independently proven fragment membership. Reversing traversal preserves
the same enclosure.

All 5 fragment tests passed (`source-fragment-world-enclosure.log.gz`), including
root-point enclosure containment, orientation invariance and mapping-budget
exhaustion. Exact Model edge/vertex ownership, embedding and volume admission
still require implementation.

## Source-composed world wire incidence

Qualified source regions now expose immutable world wires whose edges retain
the original surface, UV curve and root-valued endpoints. Directed edge vertex
indices are assigned only after every original source join, including closure,
is proven. World enclosure overlap cannot establish vertex ownership. Mapping
work is shared over the wire; an incomplete prefix carries the first uncertain
edge address and never claims complete geometry.

All 31 selected source tests passed (`source-world-wire-regression.log.gz`).
Integrated nonuniform rational clipping with a retained hole preserves contact
definitions and maps both closed wires. A 1e-12 source endpoint gap is refused;
shared mapping exhaustion retains the completed prefix.

These wires retain exact surface-composed expressions and local incidence;
they are not yet concrete Model edges, cross-face canonical ownership, embedded
faces or volume admission. STEP realization and interactive fillets remain open.

## Exact canonical world edge across original surface charts

`source_shared_edge::qualify` independently proves each complete original UV
edge composition equal to one canonical world curve using exact Bezier identity
with a shared work budget. Effective traversals must be opposite. The immutable
result retains both original fragments and the canonical world definition.
Caller keys, certificates and enclosure proximity cannot authorize ownership.

All 32 selected source tests passed (`source-shared-world-edge.log.gz`). The
new regression joins boundaries of two distinct surface charts and refuses same
direction, a 1e-12 world displacement and unsupported partial source restrictions.

This gate currently admits complete original natural-domain curves only.
Root-valued partial restrictions, cross-face root equivalence, assembly into Model
vertices/edges and embedded closed volume admission remain outstanding.

## Shared restrictions with original root endpoints

The exact shared-edge gate now admits restrictions when both UV sources use
the canonical world parameter domain without reversal of the world definition.
Opposite traversal remains mandatory. Fixed endpoint parameters must match
exactly; crossing endpoints must use the same original boundary/contact
equations, source role and root selector. Surfaces may differ because each
full composition is independently proven equal to the canonical world curve.
The canonical curve stays untrimmed and restrictions retain original endpoints.

All 32 selected source tests passed (`source-shared-restricted-edge.log.gz`).
The extended case shares a root-valued cut across different source surfaces,
preserves its source definition and refuses a different root displaced by 1e-12.
Different UV root equations, affine parameter remapping and reversed canonical
partial domains still need independent equivalence proofs. No enclosure overlap,
rounded root parameter or coordinate welding substitutes for these proofs.
Body assembly, end transitions and interactive fillets remain outstanding.

## Independently selected common source roots

Shared restrictions no longer require identical root selectors when their
original boundary/contact equations and roles match. A fresh one-box crossing
certificate inside the intersection of both original unique-root selectors
proves the same root belongs to both. Selector overlap alone does not admit
identity. Empty/degenerate intersections or an unresolved fresh certificate
refuse the restriction. The report counts these queries; at most two are
performed per edge pair, separately from the shared exact composition budget.

All 32 selected source tests passed (`source-shared-root-equivalence.log.gz`).
The new case joins independently selected roots across two surfaces. Different
UV equations, parameter remapping, body assembly and interactive operations
remain outstanding.

## Common world roots for distinct UV crossing equations

`qualify_with_cutters` supports different original UV equations on two surfaces.
Each optional canonical crossing curve must be exactly equal to the composition
of the other UV curve through each original surface. The main canonical edge
composition also undergoes independent exact identity checks. Original parameter
domains must match the canonical domains, without canonical reversal.

For root identity, the union of both original parameter selector boxes must
have a freshly certified unique root in at least one XY/XZ/YZ projection of
the canonical 3D edge and cutter. Each known source root satisfies those
projected equations and belongs to that union. Uniqueness therefore proves
that both roots have identical parameters; mere projection intersections or
enclosure overlap cannot authorize a shared endpoint. Cutter and main identity
work share one budget, and at most three projection queries per root are counted.

All 33 selected source tests passed (`source-shared-world-root.log.gz`). The
new regression uses distinct surface charts with swapped UV axes and different
UV crossing equations. It refuses absent cutter evidence, a displaced cutter
and a non-3D canonical edge. General parameter remapping, tangential projected
roots, body assembly, embedding, end transitions and UI admission remain open.

## Closed source shell incidence

`source_shell_incidence::assemble` requalifies every canonical edge pair across
original surface-composed wires. Each directed use must have exactly one partner
on a distinct face. Opposite traversal and source/world identity are checked
fresh. Shared vertex owners follow local source joins and cross-face pair
endpoints, never coordinate proximity. All faces must belong to one connected
component. Incomplete pair proofs produce no shell and retain the uncertain pair
address. Immutable shell payload includes original face wires, canonical shared
edges, pair addresses and global vertex indices.

Exact identity work is shared over the shell. Individual predicate queries are
capped at the predicate engine's MAX_WORK while retaining the caller's total
budget; passing a larger total directly previously caused ResourceLimit refusal.

All 36 selected source tests passed (`source-shell-incidence.log.gz`). An exact
tetrahedron assembles six edges and four shared vertices. Missing and duplicate
uses, a 1e-12 canonical world displacement, disconnected closed components and
work exhaustion refuse shell admission.

This is closed oriented incidence, not embedded/material volume qualification.
Face geometry, vertex-neighborhood embedding, self-intersections, retained
wall/radius/tangency, automatic end transitions, Model realization, STEP and
WASM/UI admission remain outstanding.

## Exact linear crossing parameter across non-coplanar faces

`line_crossing_parameter_identity` compares original transverse rational linear
Bezier intersection parameters using exact expansion arithmetic over authored
UV controls and positive main weights. The Cartesian chord fraction n/d maps
to original normalized parameter n*w0/(d*w1+(w0-w1)*n). Cross multiplication
proves parameter equality without computing or rounding a root. Zero transverse
determinants, invalid weights and arithmetic/resource uncertainty refuse proof.

The shared-edge gate uses this proof for different original linear UV equations
when no canonical cutter is supplied. This supports adjacent non-coplanar
surfaces whose local crossing curves are different 3D curves. Full world-edge
composition identity and matching original/canonical parameter domains remain
mandatory. Explicit supplied cutter evidence continues to be checked by the
existing world-root route.

All 37 selected B-rep source tests and all 23 predicate library tests passed
(`source-linear-root-identity.log.gz`, `line-root-predicates-regression.log.gz`).
The new B-rep fixture joins root cuts across orthogonal planes with different
local cutters and refuses a 1e-12 parameter change. Predicate cases distinguish
rational parameter from chord fraction, prove a dyadic weighted equality, refuse
a rounded 2/3 identity and cover invalid weights/work exhaustion. General
nonlinear root equivalence, parameter remapping, trimmed shell realization,
embedding/volume, end transitions and UI admission remain open.

## Root-partitioned shell with opposite canonical traversal

Shared restrictions now support opposite canonical direction for transverse
original linear crossings. The exact parameter predicate reflects the root
fraction n/d to (d-n)/d before cross multiplication. Fixed endpoints use an
exact authored sum test a+b=domain_start+domain_end, never rounded subtraction.
Original source domains must still match the canonical domain; general affine
remapping and reflected nonlinear root equivalence remain unproven.

A tetrahedron fixture splits one shared original edge at independent source
roots on its two adjacent non-coplanar faces (parameters 1/4 and 3/4). Original
curve definitions remain unchanged. The shell assembler requalifies both
restricted pairs and produces seven canonical edge restrictions and five shared
vertex owners with opposite use incidence throughout. No Cartesian root or
rounded Curve::trim is introduced.

All 38 selected B-rep source tests and all 24 exact predicate tests passed
(`source-reflected-root.log.gz`, `reflected-root-predicates.log.gz`). Exact fixed
parameter tests distinguish 1/4+3/4 from a tiny displacement and from the rounded
Binary64 sum 0.1+0.9. This validates partitioned source incidence, not geometric
volume admission or a complete fillet body. Trimmed face embedding, curved root
equivalence, end transitions, full fillet checks and WASM/UI remain open.

## Original face-chart injectivity in source shells

`Shell::inspect_face_charts` reuses the native whole-chart contraction and
optional oblique linear monotonicity kernels directly on the retained original
surfaces. Shared independent span/cell budgets cover all faces, with explicit
missing reports when exhausted. Whole-chart injectivity implies injectivity
of each retained UV subset; failure of this sufficient proof is unresolved,
not a positive self-intersection diagnosis. Face addresses and both native
reports remain available. This check does not compare different faces.

All 39 selected B-rep source tests passed (`source-shell-chart-regression.log.gz`).
The ordinary tetrahedron proves all four charts; a one-span budget leaves
remaining faces unproven. A degree-two polynomial bubble preserves all three
original triangle boundary curves exactly but folds the interior. Two distinct
interior UV positions evaluate to the same point in that synthetic fixture.
Exact shell incidence still closes, while whole-chart injectivity is unproven,
preventing a boundary-only success claim from becoming geometric authority.

Cross-face contacts, complete original trim-region qualification, neighborhood
embedding, material volume, fillet radius/tangency/walls, transitions and
WASM/UI admission remain outstanding.
