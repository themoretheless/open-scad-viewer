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

## Qualified UV regions retained in source shell assembly

`qualify_original_region` independently audits original material loops and
world/pcurve agreement through the existing face recipe, then requires exact
source-expression joins before creating an immutable SourceRegion. A damaged
world boundary cannot acquire region authority. `SourceRegion::split_boundary`
partitions one qualified source restriction at a bound root, preserving its
original curve and UV region; wrong source roles and invalid addresses refuse.

`assemble_regions` builds paired wires directly from these immutable regions
and retains the same qualified region snapshots in the resulting source shell.
Raw wire incidence continues to carry no region qualification. Region loops,
original loop indices and the exact paired wire payload cannot diverge. All
canonical edge checks and chart injectivity requirements remain independent.

All 39 selected source tests passed (`source-qualified-region-shell.log.gz`).
The root-partitioned tetrahedron now qualifies all original face regions, splits
two adjacent restrictions, assembles the qualified regions, checks their payload
identity and proves all original charts injective. Damaged world curves and a
wrong crossing role refuse. This combines qualified UV regions with exact closed
incidence and chart injectivity; cross-face contacts, material/volume admission,
fillet transitions and WASM/UI remain outstanding.

## Different-face contact search on root-valued material regions

The native surface contact search now accepts conservative original-domain
classifiers through an explicit rectangle classification contract. Existing
TrimDomain callers retain the same geometry search. Classification work cannot
exceed its allowance. Only complete Outside classifications prune geometry;
contact roots require complete Inside classifications on both original domains.

`source_face_contacts` supplies the trusted native adapter over immutable
SourceRegion loops using root-aware original-curve winding. Original surfaces
and root-valued restrictions stay unchanged. Shell pair enumeration includes
adjacent faces and uses shared geometry/domain budgets. Reports preserve all
unresolved product cells and the first unvisited pair address; an existing
shared edge never silently excludes a possible extra contact. Raw incidence
without qualified material regions cannot enter this search.

All 40 selected B-rep source tests and all 643 NURBS library tests passed
(`source-face-contact-regression.log.gz`, `source-domain-nurbs-regression.log.gz`).
A transverse interior crossing is witnessed on two original faces. After one
region is clipped at root-valued boundaries, the same chart crossing is proven
outside the retained material and the complete pair search proves absence.
Geometry/domain exhaustion preserves unresolved coverage. The qualified
root-partitioned shell exercises adjacent pair enumeration and pending suffix.

This stage searches interior contact and absence. Complete classification of
allowed shared boundary contacts, tangencies, neighborhood embedding, material
volume and fillet transitions/radius/tangency/walls remain open. General fillet
WASM/UI and STEP admission are still outstanding.

## Exact allowed contact on complete shared straight boundaries

`source_allowed_contact::certify` recomputes chart injectivity, exact planarity
of both original positive-weight surface control nets and an exact supporting
plane over canonical boundary control hulls. Qualified planar material regions
lie in their original outer boundary hulls. Compatible one-sided supports
restrict possible contact to zero-support controls; a verified zero-support hull
on the complete shared line confines every contact to that owned boundary.
No geometric tolerance or adjacency label authorizes pair exclusion.

All shared fragments of the same original canonical line must form a complete
source restriction chain on both faces, from natural start to natural end, with
exact source joins. This includes root-valued partition chains without rounding
endpoints. The immutable certificate retains both original qualified regions,
shared edge expressions and supporting plane. Exact predicate work and chart
spans are bounded; uncertainty yields no certificate.

`inspect_shell_with_allowed` recomputes these certificates internally with
shared budgets. Other pairs retain the original-region contact search and its
unresolved cells. Reports distinguish absence from allowed common-boundary
contact and record whether every pair has one of those proofs. Caller-supplied
certificates are never consumed.

All 40 selected source tests passed (`source-allowed-line-regression.log.gz`).
All six pairs of a root-partitioned tetrahedron qualify by exact boundary
contact, including the split shared line. Raw unqualified regions are refused.
Exact/chart exhaustion yields no allowed certificate and falls back to an
unresolved search. Curved shared boundaries, partial line coverage, tangencies,
neighborhood embedding, material volume and general fillet transitions remain
outstanding. This pair audit is not a whole-volume certificate.

## Exact source composition on retained chart restrictions

`verify_exact_algebraic` exposes a distinct report for homogeneous cross-product
identity of a formal tensor Bezier composition. It does not prove chart
membership or denominator positivity. The existing `verify_exact` full-chart
API retains its original control-hull membership requirement and behavior.

The source shared-edge gate can use the formal identity after immutable
Fragment construction has independently proven the complete actual restriction
inside the positive-weight source chart. Identity then proves equality on that
restriction, without requiring unused original UV tails inside the chart.
Source crossing points similarly prove their actual UV point belongs to the
chart before canonical cutter identity is used. Neither source curve is trimmed
or replaced; root-valued endpoints retain their original definitions.

All 41 selected B-rep source tests and all 644 NURBS library tests passed
(`source-restricted-composition-regression.log.gz`,
`restricted-composition-nurbs-regression.log.gz`). A nonuniform rational line
with source controls beyond the chart shares a root-valued retained restriction
between orthogonal surfaces. Its complete original UV curve remains inadmissible
as a chart fragment; a displaced world curve refuses equality. A bilinear graph
case separately verifies formal identity, reversal, mismatch and work refusal
while full-chart identity remains unproven for the outside source definition.

Multiple original knot spans, general parameter remapping, partial/curved allowed
contacts, exact corner ownership, volume admission and general fillet
transitions/WASM/UI remain outstanding.

## Exact plane intersection on a natural source chart boundary

`source_plane_fiber::certify` binds an immutable original positive-weight
single tensor Bezier surface and an exact nondegenerate plane. Every control
pole on the requested natural boundary lies exactly in the plane; every other
pole lies strictly on the same side. Positivity of Bernstein bases proves the
entire plane preimage is that natural boundary, including all chart corners.
No sampled normals, rounded coefficients or distance tolerances authorize this
claim. Unsupported layouts, additional zero poles, opposite signs, degenerate
planes and exhausted exact work return no certificate.

All 43 selected B-rep source regressions passed
(`source-plane-fiber-regression.log.gz`). Tests cover a nonuniform rational
curved boundary, both chart axes, lower and upper ends, displacement by 1e-12,
additional possible contacts and budget refusal.

This certificate establishes only the source chart intersection locus. Retained
region ownership, fresh shared-edge binding, chart injectivity and shell pair
admission remain separate requirements. Partial/curved allowed-contact
integration, corner ownership, volume admission and fillet WASM/UI remain open.

## Original multi-span plane fibers

The exact plane fiber gate now accepts original nonperiodic clamped NURBS
charts with interior knots on either axis, without knot insertion or Bezier
conversion. Nonnegative B-spline bases and partition of unity give a positive
contribution from off-boundary poles everywhere except the selected natural
chart edge; all of those poles have the same strict exact plane sign.
Clamping is checked on degree+1 endpoint knots, with every interior knot
strictly inside the natural domain. Positive weights remain required.

All 44 selected source regressions passed
(`source-plane-fiber-multispan.log.gz`). The new fixture retains the original
rational curved rim and a two-span kinked height chart. An extra plane contact
at the interior knot refuses certification. This broadens the chart locus
proof only; shared retained-region ownership and pair admission remain open.

## Original fragment contact with a natural plane fiber

`source_fiber_boundary::inspect` checks an immutable source Fragment against
the matching source-plane certificate. Constant source-coordinate control nets
prove an entire fragment on the fiber; strict one-sided hulls prove absence.
A one-sided single Bezier net proves its interior away even when nonmonotone;
otherwise fresh original-coordinate strict monotonicity and already qualified
chart containment exclude interior contact. Actual endpoints are checked through
exact natural endpoint poles, outward interval evaluation or immutable crossing
UV boxes. Potential endpoint contacts remain explicit, without tolerance welding.
Driver work is bounded and unsupported proofs remain unresolved.

All 45 selected source regressions passed (`source-fiber-boundary.log.gz`).
Tests cover full fiber paths, endpoint-only contact in either traversal, a
1e-12-separated path, a nonmonotone curved path touching at both ends and an
interior restriction of that same original curve. Shared ownership and complete
retained-region/pair admission remain outstanding; these locus reports alone
never authorize skipping a surface intersection search.

## Shared ownership of retained natural-fiber contacts

`source_fiber_contact::certify` recomputes both source-chart contraction proofs,
exact planarity of the first face and a natural plane fiber on the second. Every
retained boundary fragment is independently classified against that fiber.
Entire fiber fragments must have exact shared-edge partners on the first face;
potential endpoint contacts must join such a partner through the qualified
original wire. These immutable shared edges come from fresh native shell
assembly, including root-valued source restrictions, rather than caller labels
or proximity. The certificate retains both source regions and shared edges.
Unlike the previous full-line gate, this argument does not require natural
start-to-end coverage of the canonical world curve.

`inspect_shell_with_boundary_fibers` recomputes proofs in both face orientations
under shared exact/chart/driver budgets. Unproven pairs try the existing exact
full-line proof and then source-domain intersection search. Pair reports expose
fiber certificates separately; absence and allowed contact remain distinct.

All 45 selected source regressions passed (`source-fiber-contact.log.gz`).
The root-partitioned tetrahedron exercises exact shared-fragment ownership and
complete six-pair audit without geometry search. Raw incidence without qualified
regions is refused; exact exhaustion falls back to an unresolved contact search.
The standalone rational curved-fiber and fragment tests remain part of this run.
A partial curved nonplanar shell fixture and chart injectivity beyond contraction
remain to qualify. General corner ownership, neighborhood embedding, material
volume, fillet transitions/radius/walls and WASM/UI admission remain open.

## Curved canonical rims with root-valued adjacent-face restrictions

`certify_with_linear_chart` and
`inspect_shell_with_boundary_fibers_and_chart_work` permit a fresh whole-chart
oblique injectivity proof when coordinate contraction is unproven. Its cell
budget and consumed work are separate from exact predicates, chart spans and
source-coordinate driver work, shared across pairs and both orientations.
The previous APIs retain contraction-only behavior.

All 47 selected B-rep source regressions passed
(`source-curved-fiber-regression.log.gz`). A closed five-face rational wedge
exercises a planar cap, nonplanar rational rim and complete ten-pair audit.
A closed six-face rational curved strip splits a common degree-two world rim
at independently qualified UV roots on its planar cap and nonplanar side.
Both opposite restrictions preserve the complete original canonical curve and
retain their root-valued endpoint expressions. Exact fiber ownership admits
both fragments after fresh chart qualification. A one-cell oblique budget
refuses qualification; displacing the canonical rim by 1e-12 refuses shell
assembly. No sampled or rounded vertex substitutes are used.

These fixtures partition a complete original rim; they do not yet qualify a
body clipped to only part of that rim. Nonlinear UV parameter root equivalence,
general parameter remapping, exact corner ownership, neighborhood embedding,
material volume and fillet transition/radius/wall/STEP/WASM/UI admission remain
open. Rational source identity is not an exact-circle assertion for the rounded
binary weight used by these fixtures.

## Nonlinear local root equivalence through an exact world plane

`rational_bezier_composition_plane_identity` forms the plane normal and the
homogeneous surface/pcurve plane numerator with exact expansion arithmetic.
`curve_surface_plane::verify_algebraic` transports unchanged source definitions;
its report proves formal plane identity only, without source chart membership
or denominator positivity. Unsupported degrees/layouts and exhausted exact work
remain inconclusive. No rounded plane coefficients or extracted crossing curve
are introduced.

`source_shared_edge::qualify_with_planes` independently checks both local
crossing compositions against one supplied raw plane. Immutable SourcePoints
provide actual in-chart crossing authority. The original canonical world curve
must intersect that plane at at most one parameter: its plane restriction must
be an affine function of one strictly monotone source coordinate, with other
nonzero normal components multiplying exactly constant source coordinates.
Normal components are exact anchor determinants. Fresh canonical main-curve
composition and opposite traversal checks remain mandatory. Thus unrelated UV
root equations can identify the same world parameter without equating selectors
or rounding a root. Exact and source-driver budgets are recorded separately.

`assemble_with_root_planes` and `assemble_regions_with_root_planes` recompute
these gates under globally shared budgets. Raw plane inputs use unique pair/end
addresses; malformed addresses and duplicate specifications are refused.
Legacy assembly retains its existing behavior.

All 50 selected B-rep source regressions, all 645 NURBS library tests and all
24 exact-predicate tests passed (`source-plane-root-regression.log.gz`,
`source-plane-composition-nurbs.log.gz`,
`source-plane-composition-predicates.log.gz`). Fixtures cover a nonlinear UV
arc on a planar cap and a different nonlinear cut on a nonplanar side, with an
irrational canonical root, exact oblique plane identity and preserved source
definitions. A closed five-face curved wedge with those split roots assembles
and passes its complete ten-pair allowed-contact audit. Source plane displacement,
canonical displacement, work exhaustion and absent driver budget refuse.
A counterexample with two distinct world-plane roots refuses shared endpoint
ownership even though both local crossing images lie exactly in the same plane.
The kernel also tests changed rational weights, domain changes, reversal and a
degenerate plane.

Generic plane pullbacks with several varying coordinates, affine parameter
remapping between different original curve intervals, multiple knot spans for
formal composition, iterative retained-region cuts and new cap/corner ownership
remain outstanding. These fixtures still partition complete rims; material
volume, complete fillet transition/radius/wall/STEP and WASM/UI admission remain
open.

## Exact normalized identity across different original parameter domains

`normalized_parameter_identity` compares (t-lo)/(hi-lo), optionally reflected,
by exact expansion cross products. Domain widths must be strictly positive and
the source parameters inside their original domains. No division or rounded
affine parameter is introduced. Equal raw parameters can have unequal normalized
traversals; a rounded sum or quotient is never identity authority.

The source shared-edge gate no longer requires UV and world curves to use the
same authored knot domain. Fixed endpoints use exact normalized fractions;
linear rational crossing roots already provide exact normalized fractions from
source equations. The unique-world-plane gate works independently of domain
labels. Canonical world-cutter root selectors are mapped outward from each
original domain to the corresponding canonical domain, then rechecked by fresh
projected crossing uniqueness. Original curves, knots and root expressions
remain unchanged.

All 51 selected B-rep source regressions and all 25 exact-predicate tests passed
(`normalized-domains-source-regression.log.gz`,
`normalized-domains-predicates.log.gz`). Tests cover opposite rational paths on
orthogonal faces with UV domains [2,4] and [10,18], a canonical world domain
[-5,3], independently selected rational roots and cutters with additional domain
labels. A nonlinear world-plane root works with a different canonical domain;
canonical crossing curves also exercise two different canonical domains.
Endpoint displacement, wrong normalized orientation, equal raw parameters with
different fractions, rounded reflection, invalid widths and exact exhaustion
refuse identity.

This stage handles affine labeling of the same full normalized source traversal.
Mapping original curves with different geometric coverage onto a common carrier,
fixed/root mixed endpoint ownership, iterative retained-region cuts, new closing
cap/corner ownership and material volume remain outstanding. General fillet
transition/radius/wall/STEP and WASM/UI admission remain open.

## Exact mixed crossing/fixed endpoint ownership

`rational_bezier_plane_point_identity` checks plane membership of a canonical
curve at an exact normalized fraction from another original source domain.
Plane normals, Bernstein factors, powers, weights and parameter fractions remain
exact expansions. No divided parameter or Cartesian evaluation is used.
`curve_surface_plane::verify_curve_point` keeps original curve definitions and
returns point membership only; uniqueness is a separate requirement.

The source shared-edge gate now accepts crossing/fixed endpoint pairs through
fresh plane identity of the local crossing image, exact fixed-point plane
membership and strict uniqueness of the canonical world-plane root. Both
endpoint directions are supported. Whole original main-curve composition,
actual chart membership and opposite traversal still apply. Exact/driver work
remain bounded and shared by shell assembly. Common root/root and mixed
root/fixed proofs reuse the same native plane and uniqueness gates.

`Fragment::split_at_parameter` and `SourceRegion::split_boundary_parameter`
partition an unchanged source restriction at an explicitly supplied original
parameter, strictly inside its existing endpoint bounds. They preserve region
geometry and exact source joins; this API never converts a crossing root to a
fixed number. Root-valued and fixed splits share the same native partition path.

All 53 selected B-rep source regressions and all 646 NURBS library tests passed
(`source-mixed-endpoint-regression.log.gz`,
`mixed-endpoint-nurbs-regression.log.gz`). A nonlinear cap crossing shares its
canonical end with a fixed side parameter in a different domain, in either
direction. A closed curved wedge combines root and fixed partitions and passes
its complete ten-pair audit; moving the fixed split by 1e-12 refuses assembly.
Natural-end splits and exhausted budgets refuse. A two-world-root counterexample
refuses mixed ownership even though both candidates lie exactly in the plane.
The kernel checks exact rational fractions with reversal, degenerate planes,
resource refusal and the distinction between rational 2/5 and Binary64 0.4.

These fixtures partition existing complete rims. Different geometric coverage
on a common carrier, iterative retained-region cuts, new cap/corner ownership,
material volume and general fillet transition/radius/wall/STEP/WASM/UI admission
remain outstanding.

## Exact source use on different canonical geometric coverage

`rational_bezier_composition_affine_identity` proves
C(a+(b-a)t)=S(P(t)) with exact numerator/positive-denominator endpoints for a
and b. The unchanged complete canonical curve is pulled back homogeneously by
exact polynomial powers and cross products; no divided endpoint, extracted
curve, rounded control net or reauthored Cartesian vertex is introduced.
Either affine direction is supported, endpoints must be distinct in [0,1],
and exact work/layout limits remain explicit. The original full-normalized
composition API keeps its behavior and shortcuts.

`curve_surface_affine::verify_algebraic` transports original definitions and
returns formal composition only. `source_affine_use::qualify` binds that fresh
identity to an immutable Fragment whose actual source-chart membership is
already proven. MappedUse retains the complete original world curve, original
UV restriction and exact affine fraction endpoints. Traversal orientation is
an exact determinant. Canonical parameter bounds are outward intervals only;
root expressions remain authority and are never converted to fixed vertices.

All 55 selected source tests, all 648 NURBS library tests and all 25 exact
predicate tests passed (`affine-coverage-source-regression.log.gz`,
`affine-coverage-nurbs-regression.log.gz`, `affine-coverage-predicates.log.gz`).
Fixtures cover a closing-line source interval mapping to canonical fractions
1/6 and 71/96, a degree-two curved subinterval, a nonuniform rational canonical
curve, reverse source traversal and reverse affine mapping. A retained curved
fragment with an independently qualified crossing root preserves its complete
definition and encloses the actual canonical root parameter. Mapping
displacement, changed rational weights, invalid/zero ranges and exhaustion
refuse proof. The old full-traversal proof refuses the different-coverage line
that the explicit affine proof qualifies.

MappedUse is one source-use certificate. It does not yet prove matching
endpoints between two differently mapped uses, nor authorize shell-pair
exclusion. Shared-edge/shell admission with those mappings, iterative retained
region cuts, new closing caps/corners, material volume and general fillet
transition/radius/wall/STEP/WASM/UI remain outstanding.


## Mapped shared edges and closed source incidence

The following stage supersedes the preceding single-use limitation. Native
`source_mapped_edge::qualify` now freshly verifies both original compositions
and their affine fractional maps before comparing retained endpoints. Fixed
parameters and linear UV root fractions are compared with exact expansions.
Mixed root/fixed and nonlinear root pairs require fresh plane membership and
unique canonical world-plane root proofs. Original root definitions, source
curves, surfaces and maps remain stored; no rounded root replacement is used.

Shell assembly accepts uniquely indexed raw affine map inputs, rechecks each
pair under shared exact/driver budgets and retains the maps in its shared
edges. Cutter hints with mapped pairs are explicitly unsupported and rejected.
Whole-carrier allowed-contact proofs now refuse partial canonical coverage.

A five-face polynomial wedge closes with extended straight canonical carriers
and exact one-third/two-thirds source coverage; all ten face pairs qualify via
actual boundary fibers. The legacy full-traversal admission refuses this
fixture. Displaced maps, duplicate map addresses, displaced fixed/root ends
and exhausted budgets refuse admission. This proves source incidence and
contact qualification for the tested classes, not material volume or a fillet
body. General root identities, iterative region cuts, closing caps, complex
corners, radius/wall qualification, STEP acceptance and WASM/UI remain open.

Validation: 58 selected source B-rep tests, 648 NURBS tests and 25 exact
predicate tests passed. Compressed native test logs accompany this stage.


## Exact original UV root parameter witnesses

A new exact homogeneous de Casteljau predicate compares unchanged positive
single-span rational UV curves at original authored parameters without division
or Cartesian point rounding. Parameter domains are independent. Degree 1..32
is supported; unsupported original multispan layouts remain unproven.

`source_root_parameter::verify` requires both candidates strictly inside the
immutable SourcePoint original unique-root selector, then verifies the original
UV equations exactly. Thus the candidate identifies that unique root; it never
replaces the root expression, original definitions or root enclosures. Current
candidates are finite exact binary64 parameters, not arbitrary rational or
irrational parameter values. Plane/root and linear-root paths remain available
for other cases, subject to their existing qualification limits.

Mapped edge admission and shell assembly now accept raw indexed root candidates,
recheck both compositions and endpoint identities under the shared exact budget,
and preserve original root payloads. Duplicate/unknown addresses are rejected;
explicit cutter hints with witnessed pairs remain unsupported and rejected.
A nonlinear polynomial rim with a mixed root/fixed end qualifies without plane
hints or plane-driver work. A five-face wedge closes with two such rim fragments
and all ten face pairs qualify using actual boundary fibers. Displaced candidates
or fixed ends, selector-boundary candidates and exhausted budgets refuse proof.
A rational point fixture compares two differently oriented and independently
parameterized curves at a non-binary Cartesian point without dividing weights.

Validation: 60 selected source B-rep tests, 649 NURBS tests and 25 exact predicate
tests passed. After the final unsupported-degree guard, the targeted NURBS point
regression and the selected B-rep tests passed again. Compressed logs accompany
this stage. This establishes additional source endpoint admission; it does not
construct a new cap, material volume or general fillet body. Iterative retained
region cuts, end transitions, variable radii/corners, global radius/wall and
self-intersection qualification, STEP acceptance and WASM/UI remain open.


## Root-aware half-plane sides and a new closing cap

Native `source_halfplane_side::prove` uses an immutable original UV crossing
on an axis-aligned source line, the original retained endpoint order and strict
whole-original coordinate monotonicity to prove the side of a curved fragment.
This avoids treating an outward root box straddling the line as evidence that
the actual fragment crosses it. It keeps the original root expression, curve
and contact unchanged. A fresh monotonicity proof is required; wrong source
ownership, the opposite requested side and exhausted driver work refuse this
proof. Non-axis-aligned contacts retain the existing interval path.

`qualify_linear_region` incorporates this gate and shares driver work between
the interior contact proof and all retained/removed curved-side checks. The
new report records that work separately from source controls. Existing interval
and line proofs remain available.

A polynomial curved wedge is now cut on four of its five original faces and
closed with a newly authored planar cap. Native qualification proves each
retained half-plane region from the original boundaries, then assembles six
source faces with twelve exact shared edges. The fixture uses the original
root expressions and exact raw endpoint witnesses plus affine maps for the
long closing-line carriers. Evaluated endpoints propose fixture pair addresses
only; shell admission independently rechecks all source compositions, maps and
root identities. Reversing a fragment preserves the side proof. A cap shifted
by 1e-12 and exhausted assembly work refuse admission. Orientation closes
consistently with the original wedge fixture; outward material orientation and
volume are not certified.

Validation: 61 selected source B-rep tests passed. The final closing-cap contact
audit passed separately and visits all fifteen face pairs. Complete contact
qualification remains false: the cap/curved-side seam lies inside the original
surface chart and is not covered by the existing natural-boundary fiber gate.
The regression explicitly prevents reporting this shell as fully qualified.
Compressed logs accompany both checks. This is a source-shell closing cap,
not a general fillet end transition or admitted Model volume. Interior fiber
ownership/contact qualification, volume, variable radii/complex corners,
radius/tangency/walls, STEP acceptance and WASM/UI remain open.


## Supported interior plane fibers and complete cap contact audit

This stage supersedes the preceding cap/curved-side contact limitation for the
qualified source class. `source_interior_fiber::certify` checks unchanged
positive clamped tensor-product source charts. Identical original weights and
one world-coordinate pole across the other tensor axis make that coordinate
independent of the other parameter. An exact original rational curve/plane
point identity at an interior authored UV level, plus strict whole-original
coordinate monotonicity, proves that the complete plane preimage is exactly
that UV coordinate line. No extracted/fitted curve, divided point or rounded
root is admitted. The current plane is aligned with a world coordinate axis;
the varying original curve must have the single-span layout supported by the
exact point gate. Other planes/layouts remain unproven. Both UV axes and
independent original parameter domains are qualified in regression fixtures.

`source_interior_boundary::inspect` reports the entire fragment, possible
endpoint contacts, absence or unresolved work, together with a proven support
side. Original positive Bernstein hulls and source-owned implicit roots with
fresh strict monotonicity are sufficient gates. A fragment crossing the level,
a folded curve crossing it twice, or an interior tangency cannot be excluded
from endpoints alone. Changed source ownership and driver exhaustion refuse
proof.

`source_interior_contact::certify_with_linear_chart` freshly checks both chart
injectivity proofs, planarity, the complete original fiber, one common support
side for every retained region boundary, and exact shared ownership of every
possible fiber contact. Extrema of a linear UV coordinate on a bounded qualified
material region lie on its boundary: common support excludes interior contact,
so the admitted locus consists only of the checked source-owned boundaries.
Caller certificates are not accepted. Native shell contact inspection invokes
this gate after natural fibers, sharing exact, span, driver and oblique-chart
budgets across all face orientations and pairs. Reports retain an immutable
interior-fiber certificate separately from natural-boundary certificates.

The six-face root-clipped wedge and its new planar cap now qualify all fifteen
face pairs. The previously unresolved cap/curved-side pair obtains a fresh
interior v=0.625 fiber certificate with one exact shared source edge. Original
root definitions, retained fragments and affine coverage maps remain unchanged.
Polynomial and rational positive-weight charts pass; a plane or level shifted
by 1e-12, mismatched tensor poles/weights, an additional plane root, degenerate
planes and exhausted budgets refuse proof. The capped fixture also refuses its
interior contact certificate with only one driver cell available.

Validation: 64 selected native source B-rep tests passed. The final formatted
cap fixture separately passed and logs complete qualification of all fifteen
pairs. Compressed native logs accompany this stage. This proves the tested
shell's different-face contact audit; vertex-link manifold qualification,
outward material orientation and volume admission, general end transitions,
variable radii/corners, radius/tangency/wall checks, STEP acceptance and WASM/UI
remain open. General B-rep self-intersection support is not claimed.


## Vertex links and fresh embedded-shell geometry admission

`source_vertex_links::inspect` builds every vertex link from immutable source
corner incidence and paired canonical edge endpoints. Each link must be one
connected cycle, including parallel link edges at a degree-two partition
vertex. Disconnected cycles, open/branched links and exhausted corner work
refuse this proof. Original source vertex owners remain unchanged. For qualified
bounded material regions, the face contribution to Euler characteristic is one
minus its retained hole count; a connected oriented closed shell requires an
even characteristic at most two. The report records its genus and uncertain
vertex. Raw wire shells have no qualified region/genus authority.

`source_shell_geometry::qualify` owns the immutable source shell and recomputes
vertex links, whole-original chart injectivity and every different-face contact.
The latter checks require a complete matrix, no pending pair and every pair
qualified. Chart and contact span/oblique work share budgets. Corner, exact and
driver limits are explicit; the caller supplies the positive UV classification
tolerance. Cached public diagnostic booleans and caller certificates cannot
authorize admission. The resulting geometry privately owns its source payload
and fresh evidence, with read-only access. Failed checks retain an uncertain
vertex, face or pending pair for localization.

The six-face capped curved wedge qualifies with eight vertex cycles, twelve
shared edges, Euler characteristic two and genus zero. Its original charts and
all fifteen different-face pairs are freshly qualified again at admission.
A root-partitioned tetrahedron also qualifies; its extra degree-two vertex and
edge preserve Euler characteristic two and genus zero. Raw wire incidence is
rejected by geometry admission. A face with an exact polynomial fold retains
its exact triangle boundary and qualifies the original UV material domain, but
geometry admission rejects its non-injective original chart and identifies
face zero before any different-face checks. A twenty-three-corner budget or a
fourteen-pair budget refuses the capped shell and localizes unfinished work.

Validation: 65 selected native source B-rep tests passed. The final formatted
cap fixture separately passed and logs all vertex/chart/pair admission gates.
The folded-face regression passed separately and again in the final selected
suite, including face localization. Compressed logs accompany this stage.
Embedded-shell geometry admission precedes material orientation and signed
volume; it is not a material Model body, general fillet result or STEP/WASM/UI
acceptance. Interval volume/orientation, general end transitions, variable
radii/complex corners, radius/tangency/wall checks and the remaining UI/P0/P2/P3
work remain open.


## Source volume and material orientation

`surface_flux::bound` encloses the divergence-one flux of the unchanged original
NURBS chart. Physical original-UV derivatives and physical UV area are applied
once. Every intersected original knot span participates; span exhaustion returns
no bound. Position, cross product and integration use outward intervals.

`source_volume::qualify` consumes the privately admitted embedded geometry.
Original winding classifies retained material, including root-valued trims;
unresolved rectangles retain the full [0,1] material-mask contribution. No
unclassified area is discarded. Adaptive subdivision replaces a parent only
when both child bounds are available, and recomputes interval sums along the
partition tree. A failed refinement preserves the full-cover candidate bounds
and localizes an uncertain face/UV rectangle. Initial incomplete coverage has
no candidate total. A requested-width signed interval excluding zero admits a
native source Body and records whether its global material orientation needs
reversal. Original surfaces, boundaries, weights and root definitions remain
unchanged. This is one connected source shell, not compound/cavity admission.

A private original-region proof recognizes exactly four CCW rational straight
boundaries covering the natural chart rectangle without holes. Only those
regions can bypass winding integration; cutting regions cannot inherit this
shortcut. Original boundary partitioning preserves the unchanged material area.

Control cases:

- An outward tetrahedron with a partitioned shared edge encloses volume 1/6.
- The clipped curved wedge retains its authored cap and encloses the analytical
  volume 18995/24576 within an interval of width at most 0.02; orientation is inward.
- A rational wedge retains the authored sqrt(0.5) weights and encloses the pi/4
  circular reference at requested width 0.05. The certificate applies to the
  authored binary64 rational definition, not an exact circular replacement.
- A translated 2 x 3 x 4 cuboid, with non-unit original UV domains, encloses 24
  to width at most 1e-8 on each flux axis using six cells and no domain queries.
- Insufficient initial cell budget and missing domain budget refuse body
  admission; uncertainty is retained instead of being treated as outside.

Validation: 67 selected source B-rep tests and all 651 NURBS tests passed.
Compressed logs accompany this stage. Volume tolerances on curved material
regions still require substantial adaptive work; interactive performance is
not qualified. Concrete Model/STEP/WASM/worker/UI admission, general fillet
construction and end transitions, differing radii/complex corners,
whole-interval radius/tangency/wall checks, and P0/P2/P3 remain open.


## Whole-source seam tangent planes

`source_seam_tangency::qualify` accepts an immutable admitted SharedEdge,
which already owns both positional source compositions and matching retained
ends. It covers the outward canonical interval of the actual fragment,
including root endpoint enclosures, then inversely maps every subinterval to
both unchanged original pcurves. Full traversal direction is separated from
forward-source parameter direction. Exact rational affine carrier ranges and
independent original parameter domains are retained.

Original interval pcurve evaluation yields source UV rectangles. Fragment
admission independently establishes chart membership, so chart intersections
retain all actual seam points. `normal_alignment::inspect_pair` checks every
incident original knot side and transverse derivatives at collapsed UV axes.
Each admitted cell bounds the sine squared between all possible normal pairs;
the union of all admitted cells covers the complete source seam. A private Seam
owns its original SharedEdge, requested tolerance and aggregate angular bound.
Subdivision, pcurve spans and normal spans have separate global budgets.
Partial coverage never authorizes a Seam. Failures retain a canonical interval.
An oblique envelope rejects admission; it is not necessarily a witnessed angular
break on the exact seam because root enclosures can include unused endpoint tails.
Antiparallel normals describe the same tangent plane; material orientation is
handled by shell/body admission. This is tolerance-based tangent-plane (G1)
qualification, not a fillet radius, second derivative/G2 or regularity proof
at an excluded singular endpoint.

`Body::qualify_edge_tangency` binds the check to an existing original edge of
the admitted source body and rejects an invalid edge index. Deliberately sharp
body edges remain valid bodies and do not automatically receive tangent status.

Five new regressions cover rotating tangent planes on a quadratic world seam,
a positional seam with a crease, rational root restrictions without changed
source definitions, reversed original pcurves and partial affine carriers with
independent domains, and a singular endpoint. Each work-budget exhaustion is
checked. The admitted root-partitioned tetrahedron additionally refuses tangent
status for its selected sharp edge and rejects an out-of-body edge index.
Validation: all 72 selected source B-rep tests passed; compressed focused and
complete logs accompany this stage. No new WASM, worker, STEP or UI acceptance
is claimed. General transition surfaces/endcaps, variable radius and complex
corners, radius/wall checks, and the remaining P0/P2/P3 work remain open.


## Moving-center radius relation on original NURBS charts

`moving_radius::qualify` bounds |S(u,v)-C(u)|^2-r(u)^2 using correlated
homogeneous Bernstein products over every original cell. The center curve and
radius-law curve share the original surface U domain; radius is coordinate zero
of a two-dimensional law curve with nonnegative authored controls. The common
U partition includes all original surface, center and law knots. Every original
V span participates. Original representations and knot domains are unchanged;
interval section controls are proof machinery, not replacement geometry.

For homogeneous surface (P,W), center (C,A), and radius (R,B), the common
positive denominator is D=W*A*B. Numerators are (P*A-C*W)*B and R*W*A.
Their squared difference divided by D squared gives the squared radius residual.
Outward product Bernstein coefficients preserve parameter correlations.
Positive denominator coefficients make the rational coefficient quotient hull
a complete bound. The distance error is bounded by sqrt(max absolute residual),
and additionally by max absolute residual / lower radius when strictly positive.
Nonpositive interval denominator coefficients refuse qualification explicitly.

Cells and product coefficient pairs have global budgets. Incomplete coverage
has no aggregate radius certificate. Complete coverage beyond tolerance retains
the worst UV cell and aggregate bounds. A private certificate owns the original
surface, center, law and distance-error upper bound. Degrees above eight per
input axis leave qualification unproven; total product degree is at most 48.
This is a radius-distance relation, not regularity, a rolling-ball envelope,
G1/G2, absence of collisions, or admission of a sewn solid.

`CircularBlendSpan::qualify_radius` uses its existing original support surface,
center curve and authored cubic law. `Body::qualify_face_radius` binds the check
to a selected original chart of a source body; complete-chart coverage also
covers retained root trims. Invalid face indices fail explicitly. A tetrahedron
face does not acquire an arbitrary sphere-radius certificate.

Regression evidence:

- Original multispan tube and independent center/law knot partitions cover four
  complete cells on non-unit U/V domains. Certificates preserve all inputs.
  Insufficient cell/product budgets, displaced centers, negative law controls,
  and mismatched domains refuse qualification.
- Both directions of existing constant and variable circular support patches
  pass full-domain qualification. Positive-radius transitions have distance
  error below 1.6e-12 mm at requested 1e-9 mm; zero-radius ends have error below
  9e-7 mm at requested 1e-6 mm. Each varying patch uses 10236 product pairs.
- Zero-radius distance agreement does not certify a regular tangent plane or
  sew the degenerate endpoint; those gates remain separate.

Validation: all 653 NURBS tests, 27 circular-blend tests and 72 selected source
B-rep tests passed. Compressed logs accompany this stage. No STEP/WASM/worker/UI
integration is claimed. General transition construction, complex corners,
whole-wall checks, source Model conversion and P0/P2/P3 remain open.


## Moving radial/normal envelope qualification

`moving_envelope::qualify` consumes a private whole-chart moving-radius
certificate. Fresh original physical derivative bounds enclose both incident
knot sides. Surface normals are compared to S(u,v)-C(u), with positive normal
and radial norm lower bounds required. Original source definitions are immutable
and already pass native C0 knot validation. Singular normals and zero radial
vectors cannot receive a regular envelope certificate.

`moving_radius::radial_box` preserves the common-U correlation in the displacement:
(P*A-C*W)/(W*A), with outward homogeneous Bernstein products on the union of
original surface/center knots. This avoids widening the displacement by treating
the surface and center as unrelated Cartesian boxes. The positive coefficient
quotient hull covers every original displacement, including closed cell edges.
These interval controls are proof machinery and never replace source geometry.

Every accepted full rectangle encloses the sine squared between its radial
vectors and surface normals. Adaptive subdivision replaces unresolved rectangles
with two full-cover children; breadth-first work helps find local breaks. A
successful private Envelope owns the original radius certificate, aggregate
angular bound and requested tolerance. No partial accepted subset authorizes it.
Subdivision, derivative spans, center sections and radial-product work have
separate global budgets. Failure retains a source UV rectangle.

A fresh interval evaluation at the chart midpoint may produce a rigorous
counterexample with angular lower bound above tolerance. This rejection records
the point UV and angular interval. A midpoint can never authorize success;
acceptance still requires complete original-chart coverage. Opposite normal
orientation is allowed because the gate checks tangent planes; material
orientation and inside/outside ownership remain separate body checks.

`CircularBlendSpan::qualify_envelope` first recomputes its whole original radius
relation, then checks normal agreement. Its combined report preserves radius
error/budget/localization diagnostics when the radius stage fails. When its
private radius certificate is transferred to normal qualification, the radius
report retains numerical diagnostics and a successful Envelope owns the proof.

Regression evidence:

- An original rational cylindrical patch passes full chart coverage; a varying
  cone has a valid radius relation but a proven angular violation. Cell, surface
  span, center section and radial product budget exhaustion never authorize it.
  Disconnected original knot definitions are rejected before the radius pipeline.
- Existing constant rim patches in both directions pass complete coverage at
  max sine squared 0.02: 3364/11162 queries, 1682/5581 accepted rectangles.
  This coarse angular gate does not qualify tight tolerances or interactive
  performance for general patches.
- The existing variable section has midpoint sine squared about 0.00111246.
  At max sine squared 0.0001, the native interval point query proves violation
  in one query in both directions. The independently evaluated midpoint is a
  regression reference; the authority is the outward angular lower bound.
- Zero-radius end transitions remain unqualified under the bounded regular
  envelope check; radius agreement does not authorize their singular topology.

Validation: all 657 NURBS tests, 72 selected source B-rep tests and the complete
constant/variable/pole envelope regression passed. Compressed logs accompany
this stage. Support-face contact, tight angular qualification/performance,
general fillet construction and sewing, endpoint topology, varying-radius
rolling-ball geometry and complex corners remain open. STEP/WASM/worker/UI,
wall checks and the remaining P0/P2/P3 scope are not qualified by this stage.


## Authored linear-spine variable-radius sphere envelopes

`brep_core::linear_canal::construct` authors rational support patches for an
arbitrary translated/rotated straight 3D center spine C and a linear radius law.
For speed h=|C1-C0|, tangent T and a=(r1-r0)/h, the characteristic sphere circle
has center C-r*a*T and radius r*sqrt(1-a^2). Accounting for this radius derivative
avoids the oblique-normal behavior of circles placed directly at C. Increasing
and decreasing radii are supported; |r1-r0| >= h is rejected. The authored radial
direction fixes a resolved start direction normal to the spine. Positive and
negative arcs up to a full turn split into patches of at most pi/2.

This formula authors binary64 NURBS controls; it does not certify its own output.
Each Span owns unchanged surface, center and law definitions and uses the existing
moving-radius and moving-envelope gates. Full-domain positive-radius control
patches qualify at radius tolerance 1e-8 mm and max sine squared 0.01. Native
radius-error bounds on the rotated controls are at most 5.2e-13 mm. Tight angular
tolerances and general nonlinear spine/radius laws remain unqualified.

Shared angular endpoints are generated once. Neighbor rails therefore retain
bit-identical authored controls; they also pass fresh SourceSharedEdge exact
composition and endpoint admission. Natural chart boundaries use the original
endpoint row/column definitions. Exact algebraic curve/surface identity verifies
all four boundaries under the predicate engine's supported MAX_WORK budget.
Zero-radius ends retain authored pole rows and a degenerate boundary; native
regular-envelope qualification refuses them under the bounded check.

`Span::to_open_sheet` and `to_open_region` reuse the existing support-sheet Model
assembly and exact seam matching. Full turns and partial arcs in both directions
validate. Native Model JSON roundtrips preserve original surface/edge/pcurve
controls and topology identities exactly. These are open sheets with no bodies,
not closed fillet results or application Undo/Redo/reload acceptance.

`linear_canal_fixture` emits ten original control cases including poles, both
radius directions, full turns and partial arcs. `verify-linear-canal-occt.py`
reuses the existing OCCT NURBS decoder and independently rebuilds/evaluates each
natural face. On 12915 control samples, every OCCT face is valid, maximum radius
error is below 5e-15 mm and maximum radial-normal sine squared is below 2.1e-26.
Pole normal samples are explicitly skipped; the native gate remains unqualified
there. This independent control evidence does not replace whole-domain native
proof, native coedge import, closed-body or STEP acceptance. Fixture hash and
per-case results are recorded in the accompanying OCCT report.

Validation: all four new construction regressions and 72 selected source B-rep
regressions passed. Compressed logs, original fixtures and the OCCT report
accompany this stage. End caps/transitions, support-face contact and trimming,
closed-body admission, general NURBS spines/radius laws and complex corners
remain open, along with tight qualification/performance, wall checks,
STEP/WASM/worker/UI acceptance and the remaining P0/P2/P3 scope.

## Linear canal sphere end caps and pole boundary identity

The authored straight-spine, linear-radius canals now construct rational
spherical endpoint patches. Meridian arcs are split at at most pi/2; the
characteristic junction row is copied unchanged from the shaft. Both endpoint
seams retain exact original curve definitions. Native SourceSharedEdge admission
and full-seam tangent-plane qualification pass on increasing/decreasing radius
controls at translated, rotated positions, with max sine squared 1e-3. Each
cap also passes complete native moving-radius qualification at 1e-8 mm. This
does not certify a regular parameter chart at its collapsed pole.

Full-turn assemblies in both radius directions and zero-radius endpoint cases
validate and have paired opposite uses on every nondegenerate edge. Colliding
collapsed chart-boundary IDs now include the owning face and original UV curve;
no array index or arbitrary ordinal is used. Face permutation and JSON roundtrip
regressions preserve edge identities. Duplicate indistinguishable entities
remain rejected. These assemblies retain open-shell status and no native bodies.

Independent OCCT controls rebuild original surfaces, sew ten full-turn models,
and produce valid oriented solids with zero free or multiple edges. Across
25432 point controls, maximum radius error is 7.7e-15 mm. Adaptive Gauss-Kronrod
volume integration with BSpline spans agrees with analytical endpoint-ball
envelope volumes within 2.2e-14 mm3. Pole normal queries are explicitly skipped.
These independent solids do not authorize native body admission or STEP export.

Validation: all six linear canal tests pass. The full B-rep library suite has
871 passing tests, three ignored tests and one failure in
`material_wall_coverage::tests::all_open_enclosure_walls_and_floor_contribute_to_the_minimum`.
The same assertion fails with the previous committed topology-ID implementation;
its baseline log is retained. The full suite is therefore not green, and the
wall-coverage acceptance remains open. Native pole quotient topology, injectivity
at poles, complete face-contact/embedding and source-body admission remain
unfinished, as do general curved-spine blends, complex corners, STEP, WASM,
worker/UI and application acceptance.

## Whole-wall coverage: retain original-chart lower bounds

The enclosure regression previously consumed all 10000 distance subdivision
cells while refining early face pairs; later pairs contributed zero even when
their original control hulls were separated. Each inspected original chart now
retains its Cartesian control hull within the existing control-work budget.
Positive rational weights enclose the full chart in that hull, so the distance
between hulls is a lower bound for any retained trimmed subset. Outward interval
distance arithmetic is reused. Missing control work preserves a zero bound.

Every enumerated pair retains this lower bound even after refinement work ends.
Refinement is skipped only when that bound already meets the requested width
against the independently qualified material-chord upper witness and normal
check. Curved self-pairs, missing pairs and unfinished normal/control checks
continue to prohibit admission. No distance samples replace whole-face coverage.

The open enclosure wall/floor regression now passes at 1e-5 mm width around
1.4 mm thickness. A translated box with reversed face order also qualifies at
1e-5 mm width around 10 mm with distance budgets of one cell and one domain
cell, using zero refinement cells. General curved-wall coverage, thin-spot
search and complete product acceptance remain open.

Validation after the control-hull fix: the complete B-rep library suite passes
873 tests, with zero failures and three ignored tests. The compressed full-suite
log is retained. This supersedes the wall-coverage failure recorded in the
preceding cap stage; it does not expand admission to general curved walls.

## Exact original collapsed-boundary ownership

`source_collapsed_boundary::qualify` treats a finite world point as a candidate,
then independently checks homogeneous identity of the original surface/pcurve
composition against that constant. Only exact equality produces an immutable
certificate owning the original Fragment and point. Source endpoint restrictions
are retained; full source algebraic identity also covers every retained subset.
Unsupported composition layouts, exhausted exact work, wrong candidates and
nonconstant curves retain no certificate. This is a conservative supported-layout
gate, not a tolerance weld or a caller-provided pole flag.

Native regressions cover a rational collapsed boundary with a retained partial
parameter restriction, coincident endpoints with a nonconstant interior, a
1e-12 mm control deviation, wrong world point, exhausted work and invalid inputs.
Translated/rotated endpoint sphere caps now independently qualify every reported
collapsed boundary through this original-source identity gate. All 75 source
regressions and six linear-canal regressions pass; compressed logs accompany
this stage.

This first ownership gate does not yet contract shell incidence, certify a
vertex-link cycle, prove chart injectivity modulo poles, complete face-contact
coverage or authorize a native closed body. Those checks remain required before
Model/STEP/WASM/UI admission.

## Source shell incidence modulo exact collapsed boundaries

Source shell assembly now accepts addressed world-point candidates for collapsed
boundaries alongside ordinary opposite edge pairs. It recomputes original
composition identity under the same global exact-work budget, owns each private
collapsed-boundary certificate and contracts only that use's two local vertices.
Cross-face ownership still propagates through freshly qualified ordinary pairs;
world-coordinate equality alone never merges disconnected chart uses. Every use
must be accounted for exactly once. Missing, duplicate, wrongly addressed or
inaccurate pole inputs and exhausted identity work cannot produce a shell.
Root-plane, affine-map and root-witness inputs remain available in the combined
assembly entrypoint. The qualified-region entrypoint retains source regions.

Vertex-link inspection skips proven collapsed boundary segments when connecting
consecutive ordinary edge ends. It checks contracted endpoint ownership and
charges all original corners to the budget. Entirely collapsed wires are
rejected. Every surviving vertex must have exactly one connected two-regular
link cycle; neither boundary contraction nor valence alone proves that cycle.

Eight full-turn control assemblies cover both angular directions, increasing
and decreasing endpoint radii and zero-radius shaft endpoints, in a translated
rotated frame. All ordinary edges are paired, native pole contractions qualify,
and every vertex link is one cycle. Their raw disk-chart incidence has Euler
characteristic two. A pole candidate perturbed by 1e-12 mm is refused; omitted
uses, duplicate pole addresses and a one-unit exact-work budget are also refused.
All 76 selected source regressions and six final shell regressions pass.

This qualifies oriented topological incidence and vertex links only. Whole-chart
injectivity modulo poles, full original-face contact coverage, embedded geometry,
material orientation, volume, wall checks and closed Model/STEP/WASM/UI admission
remain required. The raw control assemblies do not carry qualified material
regions or a native body certificate.

## Source chart audit dispatches owned pole quotient checks

The source-shell chart audit now retains a separate quotient-injectivity report
for a face whose single exact pole use covers a complete natural U boundary.
The immutable collapsed-boundary owner supplies the original source definition;
partial restrictions, unsupported layouts and multiple qualifying pole uses do
not authorize quotient dispatch. The existing whole-chart weighted dominance
check is recomputed under the shared chart budget. A successful private shell
geometry audit can therefore use a freshly proven quotient chart, while a public
diagnostic or caller pole flag cannot grant admission.

A supported control S(u,v)=(u,u²v,0) qualifies through its owned pole boundary;
its reversed chart also qualifies, and a sub-256-cell budget remains unproven.
The two coincident control faces deliberately do not constitute embedded
geometry: these checks establish per-face injectivity only. All 77 selected
source regressions and seven shell regressions pass.

On all eight sphere/cone endpoint control assemblies the pole chart reports are
retained but do not prove injectivity. Sphere caps report weighted-order-not-proven;
the degree-one cone pole layout is unsupported by this source-frame theorem.
The present theorem requires stronger vanishing orders than the ordinary polar
cap parameterization provides. These are conservative proof failures, not
detected folds. The required polar-chart theorem remains unfinished; cap body
admission stays disabled. Existing vertex-link certificates remain valid, and
full face-contact, material/volume, STEP/WASM/UI and broader roadmap work remain.

## Ruled polar charts: original ray-direction quotient

A new native sufficient theorem covers a degree-one U chart with one constant
control row. With normalized U measured away from that pole, its unchanged
original rational image is P + u V(v)/((1-u)A(v)+uB(v)), where A and B are the
positive weight sums of the pole and far rows. Along a fixed ray the scalar
u/((1-u)A+uB) has derivative A/denominator² > 0.

Two fixed source-defined linear functionals h and g give the direction ratio
g(V)/h(V). Original rational Bernstein coefficient bounds prove h(V)>0 and
a single strict sign of g' h - g h' over the complete V domain. Thus different
V parameters cannot share a ray, no nonpole parameter reaches the pole, and
the radial monotonicity proves global injectivity modulo the complete pole
boundary. The source frame is formed from original far-row endpoint directions;
interval arithmetic covers that exact expression without modifying geometry.
Every subdivision contributes; an incomplete budget produces no proof.

The bounded supported class uses single clamped original spans, degree V <= 8,
positive weights and original natural domains. Independent pole/far-row weights
and nonunit domains are covered. Tests include both pole ends, a fold, repeated
rays, a 1e-12 perturbation of the constant row and exhausted work.

Source-shell chart audits retain the earlier quotient diagnostic and a separate
ruled report, recomputed from immutable original charts under the shared budget.
The zero-radius cone ends in all translated/rotated cap controls now qualify in
both angular directions. Sphere caps remain explicitly unproven and still
require a general polar-chart theorem. A successful cone chart is not a proof
of cross-face embedding, material orientation, volume, walls or body admission.

Validation: the complete NURBS library suite passes 660 tests and the selected
source B-rep suite passes 77 tests, with zero failures. Compressed logs accompany
this stage. No new WASM, worker, UI or STEP acceptance is claimed.

## Polar blowup proof for sphere endpoint charts

The native quotient audit now supports a second sufficient theorem on the
unchanged original chart. After translating by its exactly constant pole row,
the homogeneous position numerator has an exact factor u. The factor is removed
algebraically, giving S-P=u D/W. Fixed source-defined functionals construct
F=f.(S-P), G=(g.D)/(h.D). The F frame uses far-row endpoints; the direction
frame uses the adjacent-row endpoints. All expressions and polynomial products
are enclosed with outward Bernstein interval arithmetic, including the exact
source-frame expressions. No geometry is rescaled, trimmed, sampled or snapped.

Every cell proves W>0 and h.D>0. Global derivative bounds establish F_u>=a>0,
G_v>=b>0, |F_v|<=e and |G_u|<=c with a*b-e*c>0. For two points with equal F,G,
component differences along rectangle segments give a|du|<=e|dv| and
b|dv|<=c|du|, forcing du=dv=0. Thus the extended map is globally injective.
For u>0 the actual world image determines F,G; F(0,v)=0 and F_u>0 also separate
every nonpole point from the pole. Only the complete collapsed boundary is
identified in the original world image. This is a whole-domain sufficient proof.

The bounded supported class is single clamped original spans, U degree 2..8
and V degree 1..8 with positive weights. Complete shared chart budgets include
all proof cells. Incomplete work, nonpositive denominator or an unresolved
dominance margin admits nothing. Existing diagnostics are retained beside the
new polar report; immutable source ownership is required before shell dispatch.

All eight translated/rotated endpoint assemblies now prove every original
chart injective, including sphere caps, cone poles and ordinary shaft/cap faces.
Their previous incidence and vertex-link checks still pass. Native regressions
also cover nonunit domains, reversed pole ends, folds, repeated directions,
perturbed pole rows and exhausted work. The complete NURBS library passes
662 tests; 77 selected source tests and the final all-chart regression pass.
Compressed logs accompany this stage.

Per-chart injectivity and closed oriented incidence do not prove cross-face
embedding. Full retained face-contact coverage, qualified material regions,
orientation/volume, wall validation and closed Model/STEP/WASM/UI admission
remain unfinished, along with general nonlinear blend laws and complex corners.

## Qualified capped source assembly and complete contact audit entrypoint

`linear_canal::to_capped_source_shell` now builds a native source-shell candidate
from the authored support spans. It reconstructs and audits each original UV
material region, retains original surface/pcurve definitions, recomputes every
ordinary shared edge and contracts only independently qualified pole boundaries.
Region limits apply per face; original span count is bounded to 1..64 and exact
shell work is shared. Missing ordinary partners, open angular cuts, empty input
and invalid exact work are refused. This is an original-support factory, not a
conversion of rounded trims or a body/export admission.

All eight full-turn control cases retain qualified material regions. Their
source vertex-link audits now report Euler characteristic two and genus zero,
as well as a single cycle at each vertex. Independently constructing regions
in the regression and calling the production factory yield identical source
use addresses and vertex ownership. Every original chart remains injective.

The complete embedded-source geometry pipeline now executes on a 16-face
translated/rotated capped control. It inspects all 120 different-face pairs and
refuses geometry admission with
`source-shell-different-face-contacts-unproven`, localized to faces [0,1], the
shaft and a sphere endpoint patch. The recorded bounded run uses 2367 chart
spans, 23766 exact work and zero driver cells. Existing boundary fiber proofs
require a planar participant, so this two-curved-face seam remains unresolved.
Topology sharing and per-face injectivity do not prune or approve that pair.
This regression intentionally verifies refusal; it is not a body acceptance test.

Validation: all eight linear-canal tests and 79 selected source regressions pass.
Compressed logs include the full contact audit and its remaining obstruction.
The next required native proof is paired curved boundary contact ownership.
Full embedding, orientation/volume, thickness, conventional closed Model, STEP,
WASM/worker/UI and the rest of the roadmap remain unfinished.

## Paired curved natural-boundary contact ownership

`source_paired_fiber_contact` now proves supported contact between two original
curved faces. A plane candidate comes from original controls of an independently
qualified shared world curve. Each original surface separately recomputes its
complete plane preimage as a single natural chart boundary. Every remaining
control lies strictly on one side, with exact predicates; the two certified
sides must be opposite. Positive rational bases therefore exclude any contact
away from those fibers. No fitted normal, spatial tolerance or adjacency flag
is authority for this separation.

Every retained boundary restriction is independently classified against its
owned fiber. Entire fragments must be exactly shared edge uses; possible fiber
endpoints must belong to those shared uses. The private certificate owns both
source regions, both freshly admitted plane fibers and the canonical shared
edges. Root restrictions remain original payloads. Incomplete identity or
driver work grants no certificate. Current plane proposals require at least
three world-curve controls; supported noncollinear curved seams qualify.

The complete native face matrix and embedded-shell gate recognize this owned
certificate. The translated/rotated 16-face capped control still audits all
120 pairs. Its first unresolved pair advances from [0,1] (shaft/sphere endpoint)
to [0,4] (neighboring shaft angular spans). The bounded embedding run spends
2304 chart spans, 297090 exact work and zero driver cells. It still refuses
body geometry, so this is progress toward embedding rather than full admission.

Native regressions cover the sphere/shaft join in all eight radius/angular
control cases and both face orders, a one-unit exact-work refusal and two
coincident curved source charts with all boundaries paired. The coincident
faces are refused because their certified sides cannot be opposite. All 81
selected source regressions pass. Existing planar-fiber proofs remain available.

Straight shared rails, pole-only contact and boundary-plus-pole plane preimages
remain unsupported by this new paired method. Full embedding, volume/material
orientation, wall qualification and conventional Model/STEP/WASM/UI admission
remain unfinished.

Full validation after paired-fiber integration: 882 B-rep library tests pass,
zero failures and three ignored tests. Compressed source, full B-rep and
embedding-audit logs accompany this stage.

## Straight source rails: owned pole plane proposals

Paired-fiber contact now also proposes planes for a two-control world rail.
The first two anchors remain its unchanged canonical source controls. Third
anchors come from privately qualified collapsed-boundary world points in the
source shell. Bit-identical anchors are deduplicated and the proposal count is
limited to 64 per shared rail. A source-owned anchor only proposes a plane: both
original natural preimages, strict opposite-side control signs and all retained
fiber ownership are freshly checked under the same exact/driver budgets.
Degenerate or unsuitable proposals grant no certificate.

All four shaft neighbor rails qualify on six translated/rotated controls:
increasing, decreasing and constant positive radius, with both angular sweep
directions. Every pair also qualifies in the reversed face order. One-unit
exact work still refuses admission. Previous curved-seam and coincident-face
refusal regressions remain green. All 82 selected source tests and three
paired-fiber tests pass; compressed logs accompany this stage.

The full 120-pair control audit advances from unresolved pair [0,4] to [0,5],
where the shaft meets an adjacent endpoint patch at a vertex without a shared
edge. The recorded run uses 2286 chart spans, 344563 exact work and zero driver
cells, and still refuses embedded geometry. General isolated vertex/pole contact
and boundary-plus-pole preimages remain unqualified. Zero-radius rails with an
additional collapsed boundary need that broader preimage proof. Full embedding,
volume/orientation, wall validation, closed Model/STEP/WASM/UI and the remaining
roadmap are still incomplete.

## Original vertex-only supporting-plane contact

`source_vertex_contact` now proposes supporting planes through a source-owned
common vertex, using the two original control centroids only to choose a plane
candidate. Exact predicates then independently check every original control.
The nonzero controls must be strictly on opposite sides. Every zero-sign control
must be exactly the same world point P. Positive rational basis weights imply
that each chart's entire plane image is confined to P, including a collapsed
pole row whose controls all equal P. Thus possible contact is confined to that
one admitted vertex. A centroid, approximate normal or topology sharing alone
never produces the private certificate.

Vertex ownership is recomputed from a common immutable shell vertex ID and
original boundary endpoint expressions. The supported endpoint class requires
a parameter at an original clamped curve end, mapping to a clamped natural
surface corner. Original corner controls give the exact world point without
evaluation rounding. Both face expressions must give the same point. Interior
parameters, root-valued ends and unsupported nonclamped endpoints grant no
corner ownership through this method. The certificate privately owns both
regions, plane, point and original addressed uses.

Native regressions cover vertex-only contacts without a shared edge in all
eight translated/rotated radius/angular controls, both face orders, exhausted
work, invalid face pairs, interior-parameter refusal and coincident curved faces.
The complete control audit still checks all 120 pairs and advances its first
unresolved pair from [0,5] to [0,8], opposite shaft angular spans. The recorded
run uses 2204 chart spans, 720058 exact work and zero driver cells. It still
refuses embedded geometry. All 84 selected source regressions pass.

General root-owned vertices, separated control-hull proof for opposite faces,
boundary-plus-pole preimages and complete embedding remain unfinished, followed
by signed volume/material orientation, wall checks, closed Model/STEP/WASM/UI
admission and the broader roadmap.

Full validation after vertex-contact integration: 885 B-rep library tests pass,
zero failures and three ignored tests. Compressed source, full B-rep and
embedding-audit logs accompany this stage.

## Strict separation of original control hulls

`source_hull_separation` proposes a plane from original control centroids, then
freshly checks plane independence and every original control with exact
predicates. Each chart must have strictly constant signs, opposite to the other
chart. A zero or mixed sign refuses the certificate. Positive rational weights
keep the entire original surface and every retained trimmed subset inside that
control hull. The private certificate owns both unchanged source regions.
Centroids and approximate plane construction only propose a candidate; they
cannot grant admission. This is a sufficient proof, not a complete hull search.

Regressions qualify opposite shaft faces on translated/rotated increasing,
decreasing and constant positive-radius controls, both sweep directions and
both face orders. Identical faces, a shared cone pole and exhausted work refuse
separation. All 86 selected source regressions pass.

The full control audit checks all 120 face pairs. Its first unresolved pair
advances from [0,8] to [1,5], adjacent spherical angular patches sharing a
meridian and an additional collapsed pole boundary. The run records 2132 chart
spans, 981822 exact work and zero driver cells. Embedded geometry remains
unqualified. Boundary-plus-pole contact, complete embedding, signed volume,
wall validation, closed Model/STEP/WASM/UI admission and the broader roadmap
remain unfinished. Compressed logs accompany this stage.

Full validation after hull separation: 887 B-rep library tests pass, zero
failures and three ignored tests.

## Boundary-plus-pole plane image foundation

`source_pole_plane_image` separately certifies the plane image of an original
single positive rational Bezier chart. All controls on the chosen natural
boundary must be exactly coplanar. Transverse endpoint rows may additionally
be coplanar only if every original control in that row equals one world point.
Every remaining control must have the same strict exact sign. Interior
Bernstein positivity confines the plane image to the chosen boundary and
those collapsed points. Nonconstant zero rows, displaced controls, mixed signs,
unsupported layouts and exhausted work refuse certification. The old
`source_plane_fiber` complete-single-preimage contract remains unchanged.

Three focused tests pass. Original spherical patches qualify eight meridians
per control on six translated/rotated increasing, decreasing and constant
positive-radius controls with both sweep directions and opposite plane sides.
All 89 selected source regressions pass. Compressed logs accompany the stage.

This certificate establishes the geometric image only. It is not yet wired
into shell contact admission: retained main-boundary ownership and extra pole
ownership must be freshly checked against immutable shared edges and shell
vertex identities. The full embedding audit therefore still refuses pair
[1,5]. General root-trim ownership, full embedding and the remaining roadmap
are unfinished. No new full-library or WASM/UI acceptance is claimed here.

## Native boundary-plus-pole contact admission

`source_pole_paired_contact` freshly recomputes both original plane images and
requires strict opposite sides. Main boundary fragments must belong to exact
shared original edges; other possible boundary endpoints must have the same
immutable shell vertex identity as a shared-edge endpoint. Each extra pole
requires a privately qualified collapsed source use whose two endpoints have
one global vertex identity. That identity must also own a shared-edge endpoint
whose original clamped natural corner is exactly the same world point. A
coordinate match or topology flag alone cannot admit contact. Source coordinate
classification is reused through a private helper; the existing single-fiber
certificate contract is unchanged.

Native meridian regressions pass both face orders on eight seams per control
for six translated/rotated increasing, decreasing and constant positive-radius
controls with both sweep directions. Exhausted exact work refuses admission;
coincident curved faces remain refused.

The full translated/rotated increasing-radius control now obtains private
embedded geometry. All 120 pairs, including adjacent pairs, qualify; the audit
records 2132 chart spans, 761311 exact work and zero driver cells, with no next
unresolved pair. The old regression expecting refusal was replaced by explicit
geometry admission and complete-pair assertions. This is one complete control
embedding audit; it is not qualification of arbitrary fillets, root-trimmed
corners or all capped radius controls. Signed volume/material orientation,
wall checks, closed Model/STEP/WASM/UI admission and the broader roadmap still
require work.

Full validation after pole-contact admission: 891 B-rep library tests pass,
zero failures and three ignored tests. Compressed focused, embedding and full
B-rep logs accompany this stage.

## Native capped canal body and certified flux refinement

`linear_canal_body::qualify` now rebuilds original regions and incidence, then
chart injectivity and every face-pair contact, and finally signed volume. Each
private source payload moves to the next gate; diagnostic flags never admit
a body. Reports preserve stage work and refusal locations. A successful result
owns `source_volume::Body`; it does not authorize a conventional Model, STEP
export or WASM/UI operation. Per-face region budgets and separate incidence,
embedding and volume budgets remain explicit.

Original surface flux now intersects its previous enclosure with a certified
midpoint Taylor integral. Rational jets through total order three bound the
flux Hessian by the product rule. Integrated absolute remainder bounds are
Muu*hu^2/24 + Muv*hu*hv/16 + Mvv*hv^2/24 in normalized original knot coordinates.
The midpoint is outward enclosed, derivatives retain the original source net,
and interval widths apply original knot scaling exactly once. No sampled
quadrature or analytic canal formula supplies certificate authority.

Independent polynomial integrals on nonunit knot domains confirm enclosure,
orientation reversal and improved subdivision convergence. Previous rational
and multispan flux regressions remain covered. The translated/rotated increasing
radius canal confirms all 120 face pairs, then volume
[37.52914969854688, 37.77914345395185] mm^3 with width below 0.25 mm^3, using
46834 volume cells/spans. Independent axial frustum and spherical segment
integrals give 37.645933842430814 mm^3 inside that interval. The original signed
interval is negative, so the private body records orientation reversal. A
one-cell volume budget refuses the body after successful embedding.

The focused native factory test passes. This control is still expensive and
its 0.25 mm^3 width does not establish tight metrology or interactive latency.
Other radius/pose controls, walls, tangent normals at poles, arbitrary fillet
trims/corners, closed Model/STEP/WASM/UI integration and the full roadmap remain
unfinished.

Full validation: all 663 NURBS library tests and 892 B-rep library tests pass;
B-rep has three ignored tests and zero failures. Compressed focused and full
logs accompany this stage.

## Exact full-source Body to Model and STEP control

`source_body_model::convert` now accepts only a privately admitted native Body.
It copies unchanged original surfaces and canonical world curves. Vertices use
immutable source shell identities, with exact clamped canonical endpoint points
and privately qualified pole points; inconsistent points for one identity refuse
conversion. No distance welding occurs. Ordinary coedges retain source pairing
and directed pcurves; reversed clamped Bezier pcurves reverse coefficients without
knot arithmetic. Pole uses become explicit degenerate edges. Shell face senses
apply native material orientation, then the resulting closed Model gets stable
identity tables and full existing Model validation.

Current conversion explicitly refuses root-valued/partial trims, incomplete
canonical carriers, nonclamped canonical endpoint expressions and reverse
multispan pcurves requiring additional exact parameter transport. Their general
storage/transport remains a roadmap requirement; no rounded substitute is used.

The admitted capped canal control converts reproducibly, and a JSON reload
preserves the entire Model and identities. STEP /3 explicitly refuses poles,
so the control uses STEP /6. The importer changes index order and normalizes
face orientation. Regression comparison maps every entity by persistent ID,
checks exact original vertex/curve/surface payloads after existing orientation
normalization, directed loops modulo cyclic start, and all face/shell/body links.
It does not compare only counts or sampled positions.

Independent OCCT import of the exported STEP confirms one valid solid. Volume
37.645933930508455 mm^3 differs from the independent cap/frustum formula by
8.80776411804618e-8 mm^3. Axis extents match the fixture specification within
1.0000000294496658e-7 mm. The compressed STEP and hash-bound OCCT JSON report
are retained alongside `scripts/verify-source-capped-body-step.py`. This is one
control, not the bracket/flange/enclosure acceptance matrix, whole-wall proof,
arbitrary fillets, root-trim Model conversion or WASM/UI qualification.

Full B-rep validation after Model/STEP transfer: 893 tests pass, zero failures
and three ignored tests. Compressed focused and full logs accompany this stage.

## Original shared-edge restoration and directed-use ownership correction

Shared edges now privately retain their original qualification recipe: direct
world-to-forward-pcurve directions, original cutters and planes, or exact affine
ranges, planes and raw root parameter proposals. `definition()` emits original
world/UV/root definitions and those proposals. `source_shared_edge_restore`
restores each source crossing by fresh original qualification, then recomputes
shared-edge equality and endpoint ownership with the direct or mapped native
factory. Cached success/direction/bounds fields do not authorize an edge. Root
mapping budgets apply independently per use, while edge exact/driver work is
shared through the existing native gates.

Regressions restore the original mixed nonlinear root/fixed plane-owned edge
and the mapped nonlinear root/fixed edge with an exact parameter candidate.
Restored definitions, ranges, root expressions and directed-use signs match.
Removing the required plane or displacing a candidate by 1e-12 refuses the edge
even with an injected successful-certificate flag. Exhausted exact work also
refuses admission. Full forward/backward uses restore without replacing the
original endpoint expressions.

The Model bridge also corrects a double reversal: SharedEdge already records
the direction of the directed fragment, so conversion must not XOR the fragment
direction again. A regression creates a privately qualified forward/backward
world pair and checks that both uses own the same ordered canonical vertices,
before and after restoration. Previous full-source canal conversion remains
covered by the complete B-rep suite.

The selected 92 source regressions and three mapped-edge regressions pass.
This is shared-edge component restoration, not persistent source-shell/body
recovery or exact root restriction storage in Model/STEP/WASM/UI. Those remain
unfinished, together with wall qualification, general fillet trims/corners and
the broader roadmap.

Full B-rep validation after component restoration: 894 tests pass, zero
failures and three ignored tests. Compressed selected, mapped and full logs
accompany this stage.

## Qualified region and complete source-shell JSON replay

Qualified source regions now privately retain original construction recipes.
Original material recipes keep the authored surface, world/UV boundaries,
world directions, UV tolerance and original `ToleranceContext`. Linear and
curved crosscuts additionally keep their original contact curve, selected arc,
root target widths and driver axis. Fixed/root boundary partitions retain the
parent recipe and original split expression. `source_region_restore` replays
those native factories with bounded depth (1..64); material masks, winding,
root selectors and source loop indices are freshly derived. Replay audit budgets
apply independently per step/face. No saved region-success flag admits material.

`Shell::definition` emits qualified material recipes, canonical shared-edge
recipes with addressed uses, and collapsed-point proposals.
`source_shell_restore` replays each region, checks that every recorded edge
fragment equals the addressed restored material boundary, then freshly assembles
every canonical pair, affine/root input and pole use. Exact/driver incidence
work is shared through the existing native shell gate. The result is private
qualified oriented incidence; embedding and volume remain subsequent gates.

JSON byte roundtrips preserve region recipes, actual original fragment/root
definitions, winding and material/source-loop ownership. Controls cover linear
regions with retained holes, polynomial/rational curved cuts, root partitions
and eight translated/rotated capped-shell controls including zero endpoint
radii and both sweep directions. Restored shell use addresses and global vertex
identities match; fresh vertex-link checks pass. Removing a canonical pair or
pole refuses restoration even with an injected successful-certificate flag.
Exhausted exact work returns no shell. All 92 selected source regressions pass.

The capped native Body/Model/STEP control additionally serializes its shell to
JSON, restores it and freshly qualifies embedded geometry again, including
all 120 face pairs and identical global vertex ownership. This is native source
shell recovery. Editor document wiring, complete Body volume recovery, crash
and multi-tab scenarios, exact root restrictions in conventional Model/STEP,
WASM/UI, walls and the broader roadmap remain unfinished.

Full B-rep validation after region/shell replay: 894 tests pass, zero failures
and three ignored tests. Compressed selected and full logs accompany this stage.

## Native Body JSON recovery (2026-10-06)

`Body::definition` stores the original qualified shell recipes.
`source_body_restore::restore` replays material regions, canonical edges, poles
and incidence, then reruns the shared embedding/contact and signed-volume
pipeline. Saved success, volume and orientation fields are ignored.

The capped canal control now round-trips Body JSON and requires the identical
fresh signed-volume interval, orientation, 120 face contacts, global vertices
and converted conventional Model. Low incidence and volume budgets must refuse
admission even when injected saved fields claim success. This extends native
recovery of the existing control; editor document integration, crash/multi-tab
acceptance and root-restricted conventional Model/STEP remain unfinished.

Focused Body recovery control passed (97.64 seconds for the combined original,
restored and budget-refusal checks). This is a correctness measurement, not
an interactive latency claim.

Full `cargo test -p brep-core` completed successfully: 995 passing tests across unit, integration and documentation targets; three ignored.
Compressed focused and full logs accompany this stage.

## Canonical root restriction storage (2026-10-06)

`source_edge_restriction::Restriction` retains a privately qualified shared
world carrier, both original pcurves and their fixed/root-valued endpoint
expressions, including exact rational affine mapping proposals. JSON restores
the original shared-edge identity and endpoint gates before constructing the
restriction. No rounded root parameter or Cartesian vertex is stored.

The two directed uses are transported into increasing canonical parameter order
using outward interval arithmetic. Proven common endpoint enclosures are
intersected; disjoint bounds are an error. Endpoint boxes evaluate the unchanged
canonical world curve over those bounds with an explicit knot-span work limit.
`Body::edge_restriction` selects this representation by body-owned edge index.

Direct nonunit-domain and mapped nonlinear-root/fixed-end controls require the
known common point `[0.609375, 0.859375, 0]` inside the endpoint enclosure.
Root-partitioned curved rims preserve their original carriers and root
expressions after byte-level JSON replay. This is exact restriction storage
for later document/STEP integration, not conventional Model/STEP export of
root trims. That integration and the remainder of the roadmap remain open.

Validation: the initial source-focused run passed 92 tests; the final full
B-rep library run, including strengthened analytic endpoint checks, passed
894 tests with zero failures and three ignored (278.94 seconds). Compressed
logs are stored with this stage.

## Source Body editor transport foundation (2026-10-06)

The existing Rust JSON dispatcher now exposes `cad_source_body_restore`.
Input contains an original `definition`, bounded `limits` for every native
region/incidence/embedding/volume gate, and `endpointSpans` in 1..100000.
The endpoint work bound is checked before expensive geometry restoration.

A successful response returns the recomputed source Body definition, volume
interval, orientation, face/pole counts and exact ordinary edge definitions.
Each edge has its body-owned index, two original face/wire/edge addresses,
canonical vertex ownership IDs, parameter bounds and endpoint boxes.
The bounds do not replace the source root definitions. An unresolved native
gate returns `admitted:false`, no Body/edge payload, and stage diagnostics
including uncertain pair/face/UV addresses where available. Malformed inputs
use the existing structured bridge error response.

The native JSON execute test constructs a capped equal-radius canal, loads its
raw shell definition, checks all returned original edge definitions/addresses
and verifies the analytic volume 2*pi/3 lies in the computed interval. A saved
fake success/volume does not bypass a one-cell volume refusal. Invalid endpoint
work is rejected. The focused bridge test passes; compressed log accompanies
this stage. The control has full-source edges; root-valued bridge Body cases
still require additional qualification.

This wires the Rust dispatcher contract only. A new WASM binary, worker
operation, editor document field, scene preview, cancel/Retry/late responses
and source-root STEP transfer remain unfinished.

## Source Body worker/WASM transport (2026-10-06)

The existing MainSolid CAD lane has a `sourceBodyRestore` job and a typed
result contract. Its request binding compares original source definitions,
edge addresses and finite display intervals, including signed zero; native
Rust still owns all admission decisions. The normal lane cancellation destroys
the running realm and rejects late replies.

Two real-boundary failures were found before delivery:

1. The manual worker message allow-list omitted the new typed job and silently
   ignored it until timeout. A `Record<MainSolidJob['kind'],true>` now covers
   every job at compile time and is used by the actual worker boundary.
2. Browser binary transport gives integral numbers integer tags. Comparing raw
   recorded Fragment JSON against freshly serialized f64 fields rejected valid
   source ownership. The shell gate now freshly restores recorded fragments
   under the bounded source mapping work, then compares canonical original
   definitions. This preserves curve/root identity checks and avoids numerical
   proximity or tolerance-based equality.

The native bridge regression explicitly retags integral f64 values as the
browser does, and passes. All 92 source-focused native tests pass after the
change. `source-body-request.json.gz` is the native-authored capped-canal input
for subsequent real WASM/worker qualification. Failed and native replay logs
are retained for causal evidence.

Final validation: all 103 tests across the six CAD worker/client/runtime/real
boundary suites pass on the corrected shipped WASM. The separately instrumented
three source-worker tests also pass, including body restoration/refusal, source
definition and signed-zero binding, cancellation, superseding/late responses
and retry after worker replacement. Vue and MCP TypeScript checks pass.

The corrected WASM is 11,606,065 bytes, SHA-256
`068ffdea0f4034f8c08a57294558d643fb4ffdbb92a85dcf9c1aee4a5f923788`.
Production build and distribution audit pass, including exact packed/raw WASM
identity. The packed geometry JS chunk is 3,871,336 bytes; the full measured
asset total excluding the three separately counted raw WASM files is 8,130,274
bytes. Named budgets were updated to 3,873,000 and 8,132,000 bytes; the shared
CAD client/protocol chunk is 100,734 bytes with a named 102,000-byte limit.
The decoder's 16 MiB output and 4 MiB compressed limits remain unchanged.

The one instrumented equal-radius canal has 12 faces and 20 ordinary edges.
Host request-to-response restoration took 876.63 ms, worker warmup 122.82 ms,
and execution 594.54 ms. Execution includes the bridge adapter and native gates;
this does not isolate Rust time, measure rendered UI latency, or establish a
large-scene benchmark. The warm one-cell refusal took 248.88 ms execution.
The report stores actual timings, volume enclosure and artifact identity.

Editor document persistence, scene preview/selection/error highlighting,
root-valued Body cases through WASM, root restrictions in conventional
Model/STEP, crash/multi-tab acceptance and the remaining roadmap are still open.

## Native source inputs in the existing document (2026-10-06)

`DirectDocument.sourceBodies` stores identified source input records alongside
the existing body/sketch/curve/surface collections. MGV1 binary data is carried
as canonical base64 inside document JSON; this preserves signed zero and source
expressions without saving admission booleans or approximate display geometry.
The collection shares the 200-independent-object limit and global identity
namespace. Legacy documents omit the optional field and retain their shape.

Every source record is decoded and freshly qualified through Rust during
document validation, including the existing real `restoreDocument` worker.
Native refusal and malformed archive failures include the source object ID and
`CAD_SOURCE_BODY_RESTORE`. Ordinary history snapshots retain the encoded bytes
and source object identities. Saved success fields never skip native admission.
STEP assembly and Blender mesh snapshot exports explicitly refuse the source
collection until their geometry transfer is implemented, instead of dropping
those objects from an otherwise successful whole-scene export.

Validation: 114 tests across ten document/history/worker/interchange suites pass.
After error localization, nine archive/real-worker tests pass again. Coverage
includes document serialization/reload, metadata-only Undo/Redo, signed zero,
corrupted archive data, duplicate IDs, native work exhaustion, real document
worker load and explicit whole-scene exchange refusal. Vue/MCP TypeScript,
production build and packed/raw distribution audit pass. The measured asset
total is 8,136,343 bytes with an 8,138,000-byte named budget; WASM is unchanged.

`source-body-document.json.gz` is a persisted native-authored control for the
next scene integration stage. This proves source input retention and fresh
loading of that control. It does not prove twenty geometry edits, source-aware
scene preview/selection, full error highlighting, general root-body acceptance,
STEP transfer, crash/multi-tab acceptance or completion of the roadmap.


## Native restricted-edge display sampling (2026-10-06)

`Restriction::display_segments` samples the original world carrier over the
outer enclosure of the qualified endpoint intervals. Each segment retains a
native interval box containing the carrier on that parameter interval. Root
recipes and endpoint enclosures remain authoritative; samples do not replace
root vertices and are not a chord-error or admission certificate. Rendering
must distinguish endpoint uncertainty; the outer enclosure can extend beyond
the exact root. Segment counts are bounded to 1..4096.

Eight `source_shared_edge` tests pass. Replay controls exercise segment
continuity, preservation through fresh restriction restore, enclosure checks
at interior samples, and invalid work limits. This is native display support;
WASM transport, scene rendering, face tessellation and UI selection remain
unfinished. No new WASM artifact was built for this change.


## Source edge preview through WASM and the existing UI (2026-10-06)

Native restoration optionally returns display segments with original-carrier
interval boxes. Limits: 4096 segments per edge, 65536 total. The UI chooses up
to 32 segments per edge, reduces density for larger bodies, and refuses more
than 65536 edges. This is display sampling, not a chord-error certificate.
Endpoint root definitions remain unchanged; outer endpoint enclosures can
include points beyond the exact trim and are shown for the selected edge.

The existing DirectModeler SVG scene displays admitted source edges, supports
hover, mouse and Enter/Space selection, Esc, cancellation and Retry. A separate
worker client and request generations reject publication after cancellation,
document replacement or closing. Hidden/isolated objects are filtered; locked
edges cannot be selected. Fit includes native display points. This selection
is read-only and is not yet integrated with geometric edit command inputs or
the scene outliner. Faces are not yet displayed. Native diagnostics survive
refusal; errors have body identity and a next action.

The production browser control displays 20 edges of the equal-radius capped
canal. Mouse selection, keyboard selection, Retry and reload pass without
page errors; the screenshot shows the CPU SVG path. This is not root-valued
body UI qualification or a full command matrix. The real WASM worker preview
and its malformed-response checks pass. The combined five-file run has 77
passing tests and one 30-second history timeout; the isolated archive run
passes all six tests unchanged (27.64 seconds test time). Do not claim the
combined run was wholly green. Controller/client tests pass all 32 tests.

Production build and artifact audit pass: 151 files, 8145420 asset bytes plus
14906292 raw WASM bytes. New geometry WASM: 11609900 bytes, SHA-256
`605e1a122e6fb0935e6df556df05e7a4c1467b24a8d803728d82f38cb13e0b90`.
Named size budgets reflect the measured UI and kernel growth. Mesh and SCAD
exchange now refuse documents containing source bodies instead of silently
omitting them, matching the existing STEP/Blender restrictions.

General trimmed face preview, editing, exact root restriction transfer to
conventional Model/STEP, the three-part acceptance chain and all remaining
roadmap requirements remain unfinished.


## Original material face preview (2026-10-06)

Rust `source_region_display::prepare` partitions the original surface chart
into bounded rectangles. Whole-chart material uses its freshly replayed source
region; trimmed regions require whole-rectangle winding classification with
the original root-valued contours. Only proven material cells yield display
corners. Outside cells are counted and boundary/work-limit cells remain
explicitly unresolved. Source-region replay controls check identical preview,
partition accounting, work limits and refusal to turn exhausted work into
material. Bridge tests cover all original faces of the capped canal.

Optional `faceDisplay` restoration transports tiles, unresolved UV regions
and native interval boxes for the unresolved surface images. Up to 64 divisions,
65536 total face cells and 1000000 total domain classification cells are
allowed. UI requests up to eight divisions and a bounded per-face work share.
Display triangles are linear approximations; neither a chord-error bound,
watertight mesh, boundary welding nor a complete clipped tessellation is
certified. Unresolved boxes can include removed material and indicate the
unclassified area; they are not replacement geometry.

Existing SVG scene renders the surface preview and orange uncertainty boxes;
Fit includes these representations. The native source outliner supports
visibility, locking, isolation and workspace-state restore. Source edge
selection cancels an active regular command and clears its old operands.
Normal object selection clears source edge highlighting. Source selection is
still separate from geometric editing and the native body cannot yet be
passed to general fillet/edit commands. Manufacturing refuses source-body
scenes pending closed mesh preparation, consistent with mesh export limits.

36 real-worker/controller/client tests pass. Production browser checks pass
for 1536 preview triangles and 20 edges of the equal-radius capped canal,
mouse and keyboard selection, Retry, reload, source/regular selection handoff,
locking, isolation, persisted hiding and explicit manufacturing restriction.
No page errors. Screenshot inspected on the CPU SVG path. This browser control
has full original face regions; trimmed/root-valued body UI qualification is
still pending. Native partial region controls are separate evidence.

New WASM: 11616821 bytes, SHA-256
`eb2c6335af30619579cbd14ff279658ea2109c9e135314468be42f1391816a41`.
Production build and artifact audit pass: 151 artifacts, 8154777 asset bytes
plus 14913213 raw WASM bytes. Size budgets reflect measured kernel, worker and
UI growth. Full fillets, exact root STEP transfer, the three-part acceptance
chain and all remaining P0/P2/P3 requirements remain unfinished.

Final existing-UI and source archive regression: 323 tests pass in two files
(100.19 seconds wall time). This does not cover the entire 95-command matrix.


## Common native endpoint network (2026-10-06)

`source_boundary_network::inspect` uses the privately qualified Shell vertex
identities to aggregate canonical edge ends and collapsed poles. For every
identity it intersects original-carrier endpoint enclosures; disjoint boxes
are refused. Separate IDs remain separate even with equal coordinates. No
rounded root or distance welding defines topology. Roots remain on original
edge definitions; the report is display data and does not admit new geometry.
Global limits count all carrier spans twice (one evaluation per end) and all
ordinary endpoints plus poles, before enclosure work begins.

Two unit refusal/identity tests and the native bridge control pass. The latter
checks all edge ends and poles of the full-source capped canal, refusal under
span/endpoint limits, and identical network after fresh shell restoration.
This does not yet qualify a root-valued closed body network, weld a display
mesh, establish chord tolerances, or enable editing. WASM/UI transport is not
connected and the shipped WASM is unchanged for this native-only step.

### Root-partition boundary network — 2026-10-06

The existing four-face closed tetrahedral source shell splits one shared edge
using independent UV root recipes on neighboring faces. The new network checks
five vertex identities, fourteen canonical ends and fourteen carrier spans.
The degree-two split vertex lies strictly inside both original parameter domains;
its common enclosure is contained in both original endpoint enclosures.
Thirteen-span and thirteen-end budgets refuse this fourteen-work fixture.
The source-shell replay helper now compares the complete network after fresh
JSON definition restoration, including this root-partition control.

Targeted native test: 1 passed, 8.20 seconds, log
`source-root-boundary-network.log.gz`. This qualifies endpoint aggregation for
this closed shell, not general curved fillet bodies, volume, closed tessellation,
root-valued Model/STEP transport, or browser rendering of partial faces.

### Coherent native boundary display — 2026-10-06

`source_boundary_display::prepare` uses freshly computed exact shell vertex
identities to give every incident display edge the same endpoint representative.
Original root definitions and parameter ranges remain authoritative. Endpoint
segment boxes are expanded to contain the common uncertainty enclosure.
The display representative is explicitly not an exact root or geometry proof.

The root-partitioned closed shell verifies matching display endpoint coordinates
with 1, 2 and 8 segments, enclosure containment, invalid segment limits and
complete preview equality after fresh source restoration. A separate numerical
control covers extreme finite and subnormal bounds. Both targeted tests passed.
Logs: `source-boundary-display-native.log.gz`,
`source-boundary-display-anchor.log.gz`. This native API is not yet connected
to the WASM bridge or UI; closed tessellation and STEP restrictions remain open.

### Coherent endpoints through WASM and worker — 2026-10-06

The source-body bridge now obtains display segments from the native shared
vertex network. Original restrictions, roots and parameter intervals are kept.
With edge display, `endpointSpans` limits the total original carrier spans
over both ends; the display network additionally bounds endpoint work.
Worker validation requires each displayed point inside its segment box, each
end inside its original endpoint box, and equal finite coordinates
for repeated vertex identities. Altered identities, disconnected endpoints
and invalid segment boxes are refused. These transport checks do not certify
exact root coordinates, chord error or closed mesh topology.

New shipped WASM: 11,637,190 bytes, SHA-256
`c79974ab232b5f137d936d5980eb1be71a1c7508fa5f408a2f7eb0364f4db62d`.
Native bridge test passed (10.61 seconds); all seven source-shell tests passed
(17.03 seconds), including eight full-cap/pole variants and the root-partition
control. Vue and MCP TypeScript checks passed. On the newly packaged WASM,
42 tests across sourceBodyWorker/sourceBodyArchive/sourceBodyDisplay and
mainSolidWorkerClient passed (5.95 seconds). The real worker restored the
original 12-face/20-edge capped canal, refused invalid volume work, and loaded
its document archive. One observed host restore took 1472.64 ms, with
207.11 ms warmup and 772.89 ms execution; execution includes the adapter and
native gates, and does not isolate Rust or rendered latency.

The earlier combined UI/archive run on the prior artifact passed 322/323
tests but timed out the archive/Undo scenario (39.90 seconds, 30-second limit);
a separate cold retry also timed out (54.04 seconds). Both logs are retained.
Archive tests now warm the kernel before document operations, matching the
browser lifecycle; the operation timeout remains 30 seconds. Six archive
tests passed on the old baseline after warming, and all six also passed in
the 42-test run on the new artifact. This is a test lifecycle correction,
not a claim that cold startup or all P0 latency requirements are complete.

Production build and distribution audit passed. Geometry packed chunk:
3,881,800 bytes (+5,362); all non-streaming assets: 8,160,939 bytes (+6,162).
Budgets are now 3,882,400 and 8,161,700 respectively; existing worker/UI limits
remain unchanged. Browser qualification passed pointer/keyboard selection,
selection handoff, Retry, reload, visibility persistence, locking, isolation
and the explicit manufacturing restriction; no page errors. The saved
preview was inspected on the CPU/SVG fallback path.

Browser and real-worker body scope remains the full-source equal-radius
capped canal. Root-valued boundaries have native closed-shell coverage,
but root-body browser rendering, exact Model/STEP restriction transport,
closed tessellation, geometric source editing, general fillets, full wall
checks, named part acceptance, the 95-command matrix and P2/P3 remain open.

### Exact source restrictions on indexed topology — 2026-10-06

`source_body_topology::convert` projects a privately admitted Body onto the
existing generic `brep_topology::Model`, with original canonical Restrictions
or CollapsedBoundaries as carriers, original directed Fragments as pcurves,
and source vertex identities with interval enclosures. No numerical root
coordinate is selected. Edge direction and common vertex ownership are checked
and the indexed closed topology is validated. The projection is immutable;
edited generic copies do not authorize new geometry. Persistence uses the
original Body definition and must replay native admission when restored.

The native root-partitioned tetrahedral Body preserves all seven original edge
restrictions, every pcurve definition, five vertex identities and the original
Body definition. Invalid tolerance/span work refuses. The coordinate-based
Model converter still explicitly refuses this root-valued Body. Targeted
native test passed (1.69 seconds), `source-exact-topology-root.log.gz`.
The source-body bridge's native test restores its Body again, projects its
indexed topology, and verifies every pole carrier remains a degenerate edge
with one vertex and the exact collapsed point enclosure. It passed
(10.43 seconds), `source-exact-topology-poles.log.gz`.

This is native restriction storage on the shared topology library. It does
not enable root-valued coordinate-Model editing or STEP export, closed mesh
tessellation, new fillet construction, or root-body browser acceptance. No
production dispatcher or UI uses this new native projection yet; the shipped
WASM remains the previously qualified c79974ab artifact.

### Irrational roots across distinct UV charts — 2026-10-06

Original affine UV line carriers and rational Bezier cutters of equal degree
now have a sufficient exact chart equation identity predicate. Expansion
arithmetic compares original control and weight leaves after chart normalization;
no reconstructed floating root becomes authority. A fresh union-selector unique
root query prevents identical equations with different selected roots from merging.
Unsupported nonlinear main curves, multispan cutters and unequal degrees still refuse.

A planar tetrahedral Body with a shared edge split at sqrt(1/2) passes fresh
shell, contact, injectivity and volume admission. All seven restrictions, five
vertex identities and original recipes survive native topology and archive replay.
Negative two-root selection, reversal, shifted parameter domains, mixed input roles
and work exhaustion are covered. Native suites: 26 predicates, eight shells,
eight shared edges, and the selected-root test passed.

Fresh WASM SHA-256: 108c9616d6e55aae99d4b291d3cc83c5a8f347fb2ddbaa6579016a3e356f1ce1.
43 worker/archive/display/client tests passed. Irrational Body restore measured
722.72 ms host elapsed; the full archive/history scenario measured 4114.61 ms.
These are single-run host measurements, not isolated Rust latency or performance
qualification. Metadata Undo/Redo and original recipe reload passed; geometric
edits and the 20-edit acceptance remain open. Vue and MCP type checks passed.
Distribution audit: 151 artifacts, 8,166,531 asset bytes plus 14,951,533 raw WASM
bytes. The packed geometry chunk grew 5,592 bytes to 3,887,392; size budgets were
adjusted by this measured growth while retaining bounded headroom.

The existing full-cap browser scenario passed with 20 edges and 1536 triangles.
The first irrational browser run reached mouse/keyboard selection but its harness
waited for a horizontal SVG polyline to have positive area; the wait now checks
attachment, with actual mouse selection still asserted. Failure evidence is retained.

This qualifies straight world carriers with nonlinear UV cutters on planar faces.
General NURBS carriers, root-valued STEP/closed mesh, fillet bodies, whole-wall
checks, named part acceptance, the full P0 matrix and P2/P3 remain open.

The corrected irrational browser scenario passed: seven edges, 80 display
triangles and 132 explicitly unresolved face rectangles. Mouse and keyboard
selection, Retry, reload, visibility persistence, locking, isolation, selection
handoff and manufacturing restriction passed with no page errors. Screenshot
was inspected in the CPU/SVG fallback; this is not WebGPU qualification.

### Curved rational UV main curves — native stage

The normalized chord-chart predicate now compares every original control and
relative rational weight of both main and cutter Bezier equations. Nonlinear
main curves are accepted only when the complete equations agree under the
supported chart normalization. Distinct degrees, multispan curves, degenerate
endpoint chords and unproven chart relations still refuse. Fresh union-root
uniqueness remains mandatory; source world-carrier identity is independently
required by the caller.

A rational quadratic main with nonuniform weights passes exact equation
comparison across translated/scaled charts; perturbing its internal control
refuses. A curved quadratic main with reflected UV traversal passes original
SourcePoint replay and the fresh union-selector root query. Tests passed:
27 cad-predicates, the source root selection test, eight shell tests (11.97 s),
and eight shared-edge tests. These are native equation/root tests; a curved
world-carrier closed Body and worker/browser qualification are still required.
WASM rebuild is running; this stage has not been pushed or qualified in UI.

The new native shared-edge integration test also passed for an actual rational
quadratic 3D carrier with weights [1,2,1]. Two reflected UV charts independently
qualify the irrational root at sqrt(1/2); exact source composition, opposite
directed ownership and a fresh union-root check admit the common restriction.
Both original fragment definitions remain unchanged. A 1e-12 perturbation of
the world carrier's interior control refuses admission. This is a shared-edge
test, not a closed Body or STEP qualification. Evidence:
`source-curved-rational-world-root.log.gz`.

### Curved closed shell: pending contact admission

The irrational root tetrahedron now has an exact polynomial shear fixture
F(x,y,z)=(x,y,z+x*x/4). Original affine face charts are represented by exact
biquadratic coefficients and their original world boundary curves by quadratic
coefficients. The first fixture incorrectly retained line chords and fresh source
agreement refused it; correcting the authored world curves allows exact region
replay, closed incidence, five shared vertices, seven restrictions and coherent
boundary display (native test passed, 0.12 s).

Fresh embedded Body admission remains refused with
`source-shell-different-face-contacts-unproven`. This exposes the required next
proof: contacts of the curved neighboring faces. Closed topology and chart
injectivity are insufficient. The regression explicitly checks that no Geometry
is issued; volume, Body, mesh, STEP and worker use of this curved shell remain
unqualified. The recorded failed admission attempt is retained as evidence.

### Exact inverse shear foundation and rebuilt WASM

`quadratic_shear_chart_identity` checks all 27 original polynomial chart
coefficients under an inverse quadratic coordinate shear with Expansion
arithmetic. Biquadratic layout and constant weights are separate caller
requirements; the predicate alone issues equation identity, no contact or Body
authority. Perturbed interior coefficients and exhausted work refuse. All four
curved-shell fixture faces pass this predicate. The unresolved Body pair is
[0,2]; native work consumed 233956 units. Transporting contact proofs through
the common invertible shear remains required. All 28 predicate tests, nine
shell tests and nine shared-edge tests passed.

The rational UV-main production path was rebuilt as WASM SHA-256
6791d7bf8527b8d928b76ef7e00e23f65d1afcdcc7e76de01f27448e280b3d99
(11,655,091 bytes). The inverse-shear predicate is currently native-only and
is not routed through this WASM or worker. Vue/MCP type checks passed.
43 real worker/archive/display/client tests passed in 5.22 s. These replay the
existing planar irrational Body and full caps, not the currently refused curved
Body. A premature first test/build observed partially packed artifacts and
failed the identity guard; after the build process returned terminal success,
the same tests passed. Failure evidence is retained.

Distribution audit: 151 artifacts, 8,168,151 asset bytes; the packed geometry
chunk is 3,889,012 bytes, +1620. Budgets retain their previous bounded margins.

The rebuilt artifact passed browser replay of the existing irrational Body:
seven edges, 80 display triangles and 132 explicitly unresolved face rectangles.
Selection, Retry, reload, hiding persistence, locking and isolation passed with
no page errors. CPU/SVG screenshot was inspected; no WebGPU claim. The curved
closed-shell fixture remains refused pending contacts, so this replay does not
qualify that fixture or general curved Body editing.

### Exact affine inverse chart construction — native stage

`source_inverse_shear::qualify` validates a clamped biquadratic source layout,
constant positive weights, distinct driver/height axes, finite coefficient and
bounded exact work. It proposes affine inverse corners, then separately proves
the complete shear equation and exact equality of each proposed corner to the
original expression. No rounded inverse point becomes authority. Private
certificate storage retains the original surface, exact accepted inverse
surface, axes and coefficient. Original knot domains are retained.

A nonuniform-weight source, damaged interior coefficient, wrong shear and work
exhaustion refuse. An independent corner test proves that subtracting a rounded
0.1 cubed does not produce an exact zero inverse coordinate. All four curved
shell charts accept the exact inverse; Body contacts remain refused at [0,2].
Native inverse-chart and curved-shell tests passed; all 29 predicate tests
passed. This API is not connected to shell contact admission or WASM. Next work
is fresh source region/root transport and contact admission through the common
bijective shear. The previously qualified 6791d7bf WASM is unchanged.

Inverse-chart regression suites: nine shell tests passed (12.23 s), nine shared-edge tests passed (0.01 s).

### Fresh inverse restriction, region and shell transport — native stage

Native transport rebinds unchanged original UV curves to the privately accepted
inverse chart. Every Crossing endpoint is freshly qualified from its original
boundary/contact curves, selector and role. Parameter endpoints are preserved.
The curved fixture now deliberately partitions an edge with varying x, so its
root-valued carrier really changes under the shear; the earlier fixture selected
a constant-x edge. Original UV curves, parameter ranges, direction and selectors
are retained while world enclosures are recomputed.

Region transport supports original/splitRoot/splitParameter recipes. Proposed
inverse world boundaries must pass exact algebraic curve/surface composition.
Original contour/region admission and root splits are replayed freshly. Other
recipe classes remain unsupported. Replay limits apply per native recipe step;
the separate exact composition budget is accumulated. Reported root mapping
counts cover inverse selector queries, not all work inside recipe replay.

Shell transport rechecks all inverse charts and regions, reconstructs canonical
edge directions from underlying UV traversal, and freshly assembles every exact
paired restriction. The inverse shell has the same face/edge counts and passes
full native embedded Geometry qualification, including all face contacts.
The curved original shell still refuses primary contact admission; transfer of
this inverse proof into original Body admission/persistence is not connected.
No volume, STEP, closed mesh or original curved Body claim is made. These new
native APIs are not routed through worker/WASM; the shipped artifact is unchanged.

Regression checks passed: nine shell tests (9.87 s), inverse-chart test, and
the curved transport/embedding test with preserved vertex ownership, two root
replays and exhausted exact region work refusal (0.26 s).

### Original curved Body admission through inverse contacts — native stage

An explicit inverse-shear proposal now replays every inverse chart/region/root,
checks identical original/inverse incidence ownership, and qualifies the complete
inverse Geometry with bounded remaining exact/chart/contact work. A single global
bijective shear therefore transfers disjointness and owned contacts to the
original source shell. The private Geometry retains the inverse proof separately
from primary contact diagnostics; primary contacts alone still refuse this
fixture. No public diagnostic report or saved certificate authorizes Geometry.

Fresh original flux integration admits the curved Body with volume bounds
[0.15484635370860397, 0.17483001747559201], containing the independent determinant-one
shear volume 1/6. Contact admission used 312569 exact work units and 56 spans.
Original Body persistence stores only axes/coefficient in `inverseShear`; restore
rechecks incidence, exact inverse equations, regions, roots, contacts and volume.
Wrong coefficient 0.5 and omitted recipe both refuse. Body definition replay
is identical. Native admission/replay test passed in 11.47 s. Nine shell tests
passed, the capped-canal body regression passed (66.80 s), and the JSON bridge
regression passed (10.32 s). Vue/MCP type checks passed.

The worker protocol now binds optional inverse recipe axes/coefficient to the
request identity and rejects invalid axes/nonfinite coefficients. Fourteen
filtered protocol/display tests passed (25 deliberately skipped), including
recipe binding. `source-curved-shear-body-request.json.gz` stores original inputs
for a new real worker/archive/history scenario; it has not yet run on fresh WASM.
The new WASM build is still optimizing and this stage is not pushed. Browser,
worker, cold lifecycle, STEP, closed mesh, geometric editing and general NURBS/
fillet body admission remain unqualified. This family is one common quadratic
shear of affine charts; it does not cover arbitrary curved face contacts.

The inverse recipe lifecycle test passes: changing only coefficient 0.25 to
0.5 snapshots the new request, cancels and terminates the previous worker, and
ignores its captured late callback. Request identity validation also passes.
Two tests passed, six skipped (filtered run). The first harness assertion used
the wrong request envelope (options instead of job.options); corrected test
and original failure evidence are retained. Fresh WASM acceptance remains pending.

### Curved Body through shipped WASM, worker and browser

WASM build finished and packaging returned terminal success before acceptance:
SHA-256 54c4136e50aa767ad0ad5557068cc0f7fc312ea1040a3623869a05834f0d35f1,
11,695,280 bytes. All 46 real worker/archive/display/client tests passed
(10.47 s), including the curved Body, inverse recipe result binding, wrong
coefficient refusal, document restoration and metadata Undo/Redo/reload.
The curved restore measured 939.80 ms host elapsed; its full scenario measured
5156.75 ms. These single-run host values do not isolate Rust execution or UI
rendering and do not prove cold lifecycle or large-scene performance.

The production browser accepted the curved Body with seven edges, 80 preview
triangles and 132 explicitly unresolved face rectangles. Mouse and keyboard
selection, Retry, reload, visibility persistence, locking, isolation, selection
handoff and manufacturing restriction passed with no page errors. CPU/SVG
screenshot was inspected; no WebGPU or closed mesh qualification. Vue/MCP type
checks passed. Distribution audit passed for 151 artifacts: 8,178,675 asset bytes
plus 14,991,672 raw WASM bytes. Geometry chunk grew 10004 bytes to 3,899,016;
worker grew 260 bytes to 156737. Budgets preserve previous bounded margins.

### Fresh root refinement toward STEP — native only

`source_root_refinement::qualify` freshly replays nested original selectors
until the world enclosure's outward L1 diameter upper bound is below the
requested tolerance. Both fresh and original roots lie inside the original
Unique selector, so uniqueness proves identity. Original recipe storage remains
unchanged; no selected Cartesian coordinate, Model vertex or weld is issued.
Work exhaustion, stagnant bounds and unproven selectors refuse. Tests cover
sqrt(1/2) at 1e-8 and every Crossing endpoint of the admitted curved Body at
1e-7. Native root test and curved Body/replay/refinement test passed (11.39 s).
The first compile attempted an unavailable interval sqrt API; the final bound
uses outward L1 widths and is conservative for Euclidean diameter. Failure
evidence is retained. This new native helper is not routed through WASM/UI.

The current native/shipped Body class remains a common quadratic shear of
affine charts. General curved NURBS contacts, fillet construction/end transitions,
STEP/closed mesh export, geometric Undo/Redo, whole-wall qualification, named
part acceptance, full P0 and P2/P3 remain open. Earlier pending/unqualified
statements above describe their historical stages; this section supersedes
those only for this precisely tested shear family and worker/browser path.


### Bounded exchange endpoint representatives — native only

`source_exchange_endpoints::prepare` takes a privately admitted Body, freshly
refines its original Crossing selectors, replays shared-edge identity and
preserves world carriers, directions and parameter maps. Enclosures meet only
by original vertex ID. Five representatives of the curved control body have
outward L1 error bounds at most 1e-7 mm; seven original carriers remain intact.
The original Body recipe remains stored separately. Exhausted exact work and
an insufficient endpoint limit refuse preparation. The integrated curved
Body/replay test passed in 11.49 s (`source-exchange-endpoints.log.gz`).

This API is native only. It supplies endpoint accuracy bounds; it does not
certify full edge/pcurve trims, STEP output, mesh closure, new topology or
editing admission. Refinement mapping work is accumulated globally; shared-edge
replay mapping has a separate per-use limit and identity queries are reported
separately. No WASM or UI route changed in this stage.


### Original root-trimmed source STEP candidate — native only

`source_exchange_trims::prepare` binds a private endpoint preparation to the
original Body definition. For each original carrier and both original pcurves,
it chooses numeric trim parameters and uses interval evaluation to bound the
endpoint's L1 distance to the same original vertex representative. Surface
composition is evaluated with interval UV coordinates. Insufficient work,
collapsed/reversed trim proposals or unproven tolerance yield no certificate.
The integrated curved control passes at 1e-7 mm and refuses work=1 and 1e-30 mm.

`source_exchange_step::prepare` freshly runs these gates and emits an AP242
candidate directly from the original 3D carriers, surfaces and 2D pcurves.
TRIMMED_CURVE records numeric restriction parameters. Shared EDGE_CURVE
vertices use original shell IDs; no proximity welding or classic Model
conversion. Exact source emitters preserve distinct knots and near-unit
rational weights; existing v5 emission behavior remains unchanged. Pole
exchange is explicitly refused. This candidate is not a Model, whole-curve
certificate or general independent-reader qualification. It has no WASM/UI
route and does not change editing/manufacturing/export admission there.

Native integrated Body/replay/refinement/trim/STEP test passed (12.60 s),
source coefficient serialization regression passed, and four v5 AP242
regressions passed (0.39 s). The checked-in OCP script independently reads
`source-curved.step.gz` after decompression. OCCT reports valid B-rep, one
solid, four faces, seven edges and five vertices; volume is
0.16666666666666663 mm³ versus independently expected 1/6. All five vertex
coordinates match the analytic sheared tetrahedron within 1e-7 mm, including
x=1-sqrt(1/2), y=0, z=x²/4 at the root split. Bounding extrema are 0..1 mm
with OCCT's 1e-7 mm box gap. No root-value coordinate was authored into the
source Body.

The first independent oracle incorrectly assumed a different split edge;
its failure is retained in `source-curved-step-oracle-correction.log.gz`.
Inspection of the fixture's first changing-x paired edge establishes the
correct (1→0) edge and the corrected analytic oracle passes. Initial OCP
binding inventory also required explorer/IsSame counting and CornerMin/Max
in place of unavailable IndexedMapOfShape/Bnd_Box::Limits bindings.

This is independent STEP evidence for one qualified quadratic-shear Body,
not general curved fillets, named-part acceptance, geometric Undo/Redo,
closed mesh admission, general P0 or P2/P3 completion.


### Root-trim STEP through packaged WASM and worker

The optional sourceBodyRestore stepExchange request freshly restores the Body,
then runs native endpoint/trim gates and serializes original carriers. Work
exhaustion returns a preparation refusal while the independently admitted
Body stays available. Request options, tolerance, result counts and bounded
output are checked by transport; unsolicited or mismatched exchange results
refuse. Changing a request cancels the old worker and ignores its late response.
No new editor export action or geometric editing admission is introduced.

Packaged WASM: bdaba2e3ee02232ddf2b12746dcf4a59e58f8e485f06280feac00e40e0917b0f,
11744484 bytes. All 48 worker/client/archive/display tests passed (11.84 s).
The curved STEP scenario including success and work refusal took 1618.05 ms
host elapsed; it does not isolate Rust or rendering latency. Native endpoint
error upper bound was 1.550495842828071e-14 mm. Independent OCCT reading the
worker-produced STEP confirms valid B-rep, one solid, four faces, seven edges,
five vertices, analytic vertex coordinates within 1e-7 mm, and volume
0.16666666666666663 mm³. This remains one quadratic-shear control model.

Vue and MCP type checks passed. Production distribution audit passed for
151 artifacts: 8197363 asset bytes plus 15040876 raw WASM bytes. Packed
geometry grew 16692 bytes to 3915708 and worker grew 998 to 157735. Asset total
grew 18688 bytes; budgets preserve the previous bounded margins. New UI
export, named-part acceptance, general fillets, geometric Undo/Redo, P0 and
P2/P3 remain unqualified.


### New editor source-body STEP action

The File menu exports the selected source Body with a positive numeric
tolerance in mm. A separate worker freshly restores original recipes and
prepares native STEP. Cancel terminates that worker; scene/selection/tolerance
changes and closing/unmounting the editor invalidate the pending generation.
Only a matching live scene and selected original record can download. Errors
show a next action, native detail and Retry. Long File menus now scroll inside
the viewport. Existing assembly/manufacturing restrictions stay separate.

The production browser on the curved control passed mouse and keyboard edge
selection, Enter export, cancellation with no download, invalid tolerance,
Retry, corrected tolerance and one downloaded STEP. Existing source display,
reload, visibility, locking, isolation and manufacturing checks also passed;
no page errors. File controls screenshot was inspected after the scroll fix.
Independent OCCT read the UI download as valid one-solid B-rep, 4 faces,
7 edges, 5 vertices, volume 0.16666666666666663 mm³ and analytic vertex
coordinates within 1e-7 mm. Vue type check passed. Distribution audit passed:
151 artifacts, 8199738 asset bytes + 15040876 raw WASM bytes. DirectModeler
grew 2329 bytes to 419618; CSS added 46 bytes. Bounded margins retained.
WASM remains bdaba2e3ee02232ddf2b12746dcf4a59e58f8e485f06280feac00e40e0917b0f.

This supersedes the previous pending editor export statement for this tested
control and source-carrier action. It does not qualify arbitrary curved
fillets, geometry edits, named parts, closed preview mesh, full P0 or P2/P3.
Browser evidence uses CPU/SVG fallback; WebGPU is not qualified here.


### Native partial annular support assembly and planar contact

`source_support_shell::prepare` rebuilds original region lifts and shared
incidence from one complete support shell, including hole wires and explicit
poles. It checks face/use limits before orientation proposals, proposes
canonical face senses, then freshly checks every lift and shared world carrier.
Input Model closed flags/Body records do not grant source geometry admission.
The capped linear canal factory reuses this path. Unsupported reflected chart
arithmetic can still refuse at the fresh gates; this is not a general exact
chart-transport certificate.

The partial quarter annulus (R=20, inner R=5, H=6, transition radius=1.25)
now passes original source region and closed oriented incidence gates: 27
faces and two poles. The initial planar lift exhausted verification because
pcurve degree 5 exceeded the correlated composition path's limit of 4;
raising that limit to 8 retains the composed degree-32 and other work caps.
The first unoriented assembly also refused opposite-edge ownership; canonical
face/chart proposals resolve it and are rechecked against original 3D carriers.
Failure evidence is retained.

`source_pole_plane_image` now permits zero interior Bernstein coefficients
while requiring a consistent exact strict side, a strict opposite-edge
coefficient and strict noncollapsed opposite corners. Positivity of interior
Bernstein bases excludes extra plane images. Additional corner images, mixed
signs, unproven pole rows and exhausted exact work refuse.
`source_pole_planar_contact` binds that image to a freshly proven planar face,
original shared edges and source pole vertex IDs/points. Boundary classification
and exact work share global caller budgets. It grants only cross-face contact
ownership; chart injectivity remains separate. It runs after established
contact paths, before unresolved numerical search.

The annular transition/top-plane contact [0,1] is now certified, including its
collapsed endpoint, and exactWork=1 refuses. Whole-shell geometry still refuses
at [0,2], the transition/cylinder contact. This is the next actual missing
proof. No private Body or volume is issued for this annulus; radius, whole
tangency, wall thickness, general fillets and named-part acceptance remain open.

Native tests: annular incidence/planar proof and explicit remaining refusal
passed (45.34 s); input closed/Body claims do not change incidence, damaged
pcurve refuses and face-limit refusal passed (0.25 s); four pole image tests
passed (0.41 s); all 21 composition/agreement tests passed (0.07 s), including
rational degree-5 agreement and damaged interior detection. Existing capped
Body/volume regression passed (66.38 s) and root Body/replay/STEP regression
passed (11.65 s). Timings are test runtimes, not isolated performance evidence.

This is a native source stage. The packaged WASM/UI artifact remains
bdaba2e3ee02232ddf2b12746dcf4a59e58f8e485f06280feac00e40e0917b0f;
these new contact/assembly capabilities have not been packaged or browser
qualified. Earlier UI STEP evidence applies to its earlier control class.

## Exact projected Jacobian predicate

Added a native original-coefficient predicate for positive-weight rational
Bezier charts of degrees 1..8. It forms the homogeneous projected Jacobian
numerator using exact expansion arithmetic and returns signs of its tensor
Bernstein coefficients. Conversion uses common positive integer denominators;
no rounded division or sampled derivatives decide orientation. A consistent
nonzero sign, with zero coefficients allowed, proves strict orientation on the
open chart. Work exhaustion returns no signs.

Five regression cases pass: plane and reversed axes, collapsed boundary with
flat endpoint, an interior fold with both signs, nonuniform rational weights
and work refusal, and a rank-deficient projection. The full cad-predicates
suite passes. This predicate does not prove global injectivity, simple
boundaries, cross-face contact ownership, tangency, or a closed fillet body.
The annular transition/cylinder contact [0,2] remains unresolved. No new WASM
artifact or browser qualification is claimed for this stage.

The NURBS wrapper owns an immutable original Surface in each issued certificate
and validates clamped knots, dimension, weights, periodic flags, axes, and the
actual cad-predicates work cap (1,000,000). Two wrapper regressions pass.
Power-to-Bernstein common denominators now use exact integer LCM factors; the
full cad-predicates suite passes after this change.

Actual original annulus: face 0 refuses with ResourceLimit at 999,994 charged
operations; no orientation certificate is issued. Face 2 returns 45 zero
coefficients (10,777 operations), as expected for the XY projection of its
ruled extrusion. The explicit refusal/rank regression passes (0.09 s). This
is evidence of an unresolved computation limit, not successful annular
geometry qualification. The transition/cylinder proof still requires a more
efficient exact determinant and independent boundary ownership/injectivity.

## Exact Bernstein determinant and original annular projection reversal

Replaced power-basis determinant multiplication with exact Bernstein products
using common positive integer LCM denominators. Original coordinates are
translated by the first original control coordinate in exact arithmetic.
Added separately bounded original coefficient-row queries; every row uses the
same immutable arena and all rows must succeed before any chart certificate.
The wrapper caps aggregate charged work at 100,000,000; each underlying context
retains its 1,000,000 cap. Partial row results never grant orientation.

Six predicate regressions and the full cad-predicates suite pass, including
whole-chart versus independently queried rows of a curved rational chart.
Two original NURBS wrapper tests pass. Actual annular face 0 now completes
in 9,130,621 charged operations: 65 negative, 14 zero, 11 positive coefficients.
All positives lie on v=1, whose Bernstein restriction has strictly positive
sign for interior u; v=0 is strictly negative for interior u. By continuity,
every interior u has an interior v where projected Jacobian vanishes.
This proves a projection orientation reversal, not a 3D self-intersection.
No orientation certificate or annular Body is granted. Face 2 projection
returns 45 zeros (57,210 operations), consistent with extrusion rank deficiency.
The native actual-model regression passes; native authoring at the transition
endpoint now needs correction and renewed original-geometry qualification.
WASM/UI remain unchanged and whole annular contact/body remains unproven.

## Transition authoring correction and renewed original qualification

Independent weighted accumulation/division split the cylinder-side XY
columns. Copying contact XY alone still left seven positive projected
Jacobian coefficients. The transition constructor now copies the original
contact rail into the middle meridian column and moves each nonzero internal
XY coordinate 16 representable steps toward zero. Endpoint and collapsed
rows, contact rail, centers and weights stay unchanged. This is a rounding
scale authoring perturbation; it does not claim exact analytic tangency.
Fresh radius and normal qualification remain necessary. No loaded original
Surface is rewritten by the projection predicate or its wrapper.

The control annulus now has 77 negative and 13 zero Jacobian coefficients;
its strict interior XY orientation certificate owns the original corrected
Surface (9,111,713 charged operations). Eight varying-radius transition
cases, both directions with end radii 0/1.25 and 0.5/1.25, pass exact interior
orientation. One reverse transition exhausted a row context, so bounded
coefficient queries were added: an exhausted row's work remains charged,
individual coefficients use the same immutable arena, and the aggregate cap
remains 100,000,000. Missing coefficients never issue a chart certificate.
Whole/row/coefficient results match on an independent curved rational test.
All 80 cad-predicates tests pass.

Radius qualification passes both directions and all four end-radius pairs.
Nonzero-end upper errors are below 1.59e-12 mm against a 1e-9 tolerance;
zero-end cases retain their existing 1e-6 tolerance (upper bounds below
8.99e-7 mm). These are whole-patch certificate bounds, not sampled radii.
Eight existing transition regressions pass, including full-interval seam
normal coverage (38,798 and 46,894 checked normal spans), pole incidence and
explicit refusal of regular G1 at the collapsed tip. That batch's new
orientation case initially failed on its row work cap; after coefficient
refinement the eight-case orientation test passes separately (1.92 s).

Annular incidence and transition/top-plane contact still pass. Whole source
geometry still refuses at transition/cylinder pair [0,2] (87.67 s test run).
Projection orientation alone does not prove projected boundary simplicity,
containment, cross-face contact ownership or an annular Body. None of these
new native changes are packaged in WASM or browser qualified yet.

## Original tangent cusp separation and projected Jordan chart

Added exact Bernstein signs for the quadratic functional
cross(T-J,P-J) - beta*dot(T-J,P-J)^2 on two original rational UV Bezier
curves. J and T are original control coordinates; beta is a proposed finite
separator coefficient. Native curve evaluation only proposes beta; every
original polynomial coefficient supplies the actual decision. Positive
weights and strict opposite coefficient signs exclude any common interior
point. Shared endpoint coefficients must vanish exactly and the other
endpoint coefficients must have strict opposite signs. The immutable
certificate owns both original curves and its source frame/beta. It does
not prove individual curve injectivity. Wrong beta, changed controls,
coincident curves, a 1e-12 endpoint gap and exhausted work refuse. Two native
separator tests pass, including rational weights and reversed carriers.

The projected Jordan chart extracts original natural boundary coefficients
by selecting XYZ axes, preserving weights, reversing coefficient order
where needed, and using normalized Bezier parameters. Constant projected
boundaries are recorded explicitly. Every retained boundary curve must be
individually injective and every pair separated, with exact joins. Existing
sufficient simplicity proofs are reused; tangent-adjacent pairs may use
the new quadratic separator under the remaining aggregate exact budget.
No incomplete pair list, partial signs, or sampled geometry grants a chart.

Strict interior Jacobian orientation plus a simple Jordan boundary implies
global interior projection injectivity: the boundary degree is +/-1 inside
and zero outside; every interior preimage has the same local degree.
Openness excludes interior images on the Jordan boundary. This proves an
interior chart property; collapsed projected boundaries still need original
3D pole/contact ownership before any cross-face or Body admission.

Actual transition cusp (R=20, H=6, radii 0->1.25, sweep pi/6): beta
0.01969084457822929 gives strict opposite sides in 166,612 charged operations.
The complete projected Jordan certificate passes in 8,948,517 operations.
The canonical original annular source region's transition also passes in
12,488,524 operations (0.26 s test runtime). Its copied input Surface is
retained in the immutable certificate. Square and collapsed-triangle
regressions pass; folded charts and low work refuse. The full 80-test
cad-predicates suite passes.

Next: match the ruled wall's entire original projected trace to the Jordan
boundary and bind remaining fibers/poles to shared source identities.
Transition/cylinder contact [0,2], whole annular Body/volume, full fillet
acceptance and new WASM/UI integration remain unproven.

## Original transition / ruled-cylinder contact ownership

Added a private source contact certificate for a complete original curved
material chart with a Jordan projection and an adjacent positive-weight
clamped ruled Bezier wall. Original wall XY (or XZ/YZ) coordinates and weights
must agree exactly between its two linear-axis control columns. Its entire
projected trace must match a natural boundary's original coefficients and
weights, forward or reversed. No analytic-circle substitution, fitted rail,
coordinate welding, tolerance identity or sampled contact admission occurs.

The Jordan certificate excludes interior projection images from that trace;
boundary simplicity confines boundary preimages to the matched natural edge
and explicitly collapsed boundaries. Each collapsed boundary must be truly
constant in original XYZ coefficients and must own an original shell pole
with the same shared-edge endpoint identity. Original source fragments on
the contact boundary must own shared shell uses; possible endpoint contacts
must own the same original vertex IDs. Chart injectivity remains the
separate shell geometry gate. Arbitrary trimmed curved material charts and
nonmatching/multispan ruled traces remain unproven in this certificate.

Actual annular faces [0,2] now qualify in 12,488,524 charged operations.
Reversed face order also qualifies; work=1 and wrong neighbor [0,3] refuse.
A one-ULP projected coordinate or wall-column weight change prevents trace
admission. Changing only extrusion Z preserves the projected trace but
does not bypass independently required original shared carriers. These
positive/negative native tests pass (0.38 s and 0.01 s).

Integrated the immutable certificate into native pair diagnostics and shell
geometry admission. Whole annular geometry passes [0,2] and now reports the
next unproven pair [0,6], the transition and neighboring trimmed top plane.
The updated full gate regression passes (45.95 s); its initial assertion
expecting old [0,2] failed and is retained as advancement evidence.
No whole annular Geometry, Body or volume certificate is issued.

Existing native regressions pass: irrational-root flat body/topology
(1.70 s), curved root/body/inverse-contact/volume replay (11.77 s), and capped
body with independent analytic cap/frustum volume enclosure (66.71 s).
These are test runtimes, not performance comparisons. Packaged WASM, UI,
independent STEP reader and named-part acceptance were not refreshed here.


## Original material vertex and corner-only plane contact

Native annular pair [0,6] now qualifies using a fresh planar material
boundary hull and exact canonical carrier endpoint ownership. This stage
requires one injective clamped bilinear planar chart, positive original
carrier weights, full source UV endpoints and direct private SharedEdges.
Mapped ranges and root-valued endpoints refuse this shortcut. The private
certificate retains original regions, carrier definitions and chart proof;
no rounded world endpoint or saved topology flag authorizes contact.

Pair [1,2] now qualifies by exact original Bernstein control signs. Uniform
strict nonzero signs exclude open surface contact with the other face's
plane. A zero boundary must map entirely to the same owned original point;
every zero corner must equal that point. Zero interior controls are allowed
because the open tensor basis is strictly positive. This is contact proof,
not an independent chart injectivity or G1 claim.

Actual pair tests pass: [0,6] uses 9,254 exact operations; [1,2] uses 14,985.
Negative tests reject opposite interior signs, a different zero corner,
a zero boundary containing another point (including one subnormal step),
and exhausted work. An explicitly collapsed boundary at the owned point
is accepted. Existing original vertex contact regressions pass.

The full annular gate advances to [1,6], still reporting
`source-shell-different-face-contacts-unproven`. The initial outdated pair
assertion failed and its log is retained as advancement evidence. No whole
annular Geometry/Body/volume certificate, new WASM, UI or named-part STEP
acceptance is claimed by this stage.

Updated full annular gate regression passes (37.57 s). Curved irrational-root
body/contact/volume replay also passes; see retained logs.


## Coplanar adjacent original material faces

The [1,6] annular pair consists of two trimmed material regions on the same
original bilinear top chart, sharing one complete original linear carrier.
Previously candidate separator points came only from the source planes;
all candidate planes coincided with the top plane and could not separate
the material. Native shared-line proof now also proposes finite transverse
coordinate points. These are guesses only: fresh exact carrier signs,
opposite strict sides and confinement of the zero hull to the complete
owned shared segment still decide admission. No topology-only exclusion,
tolerance weld or sampled contact claim was introduced.

Actual [1,6] qualifies in 135,124 charged operations; reversed face order
also qualifies. Work=1 and a nonadjacent face without an owned shared line
refuse. The whole annular gate advances to [1,10]; its initial stale
assertion failure is retained. Added explicit root/mapped endpoint refusal
checks to the irrational-root ownership regression. That test passes
(2.04 s), as does the curved root/body/contact/volume replay (11.49 s).
No complete annular geometry or body, refreshed WASM/UI or full named-part
acceptance is established by these results.

The updated whole-gate regression passes; the next unresolved pair remains
[1,10]. The terminal log is retained as annular-transverse-qualified.log.gz.


## Strict separation using original planar material hulls

Annular [1,10] has no owned common edge. The retained top face's untrimmed
square support chart overlaps the other transition's control hull; this
prevented the previous full-surface enclosure from proving separation.
The native hull gate now first retains the original full-surface proof.
On failure it recomputes bounded planar material hull certificates and
retries exact strict separation using those immutable enclosures. A curved
or unsupported trimmed chart keeps its full original positive-weight
surface controls. All attempted exact work, including failed initial
proposals and planar certificates, shares the caller's remaining budget.
If neither material hull is supported, the identical original proposal is
not repeated. No sampled or tolerance-based separation is admitted.

Private disjoint certificates retain the material hull authority when used.
Actual [1,10] qualifies in 10,178 charged operations; reversed order also
qualifies. Work=1 refuses; the touching pair [1,6] cannot be certified as
disjoint. Existing rotated-shaft and shared-cone-pole regressions pass.
The integrated shell gate advances to [1,13]; the initial assertion failure
is retained as advancement evidence. Irrational-root closed body/topology
regression passes. Complete annular Geometry/Body/volume, packaged WASM/UI,
and named-part STEP acceptance remain unproven.

Final optimized full-gate regression passes at [1,13]. Curved root/body/
contact/volume replay also passes; its contact work is 674,285 operations.
These local test runs do not establish performance improvements.


## Coordinate proposals with fresh strict hull admission

Annular pair [1,13] is a trimmed top material region and a distant inner
wall. Their original/enclosed controls have a strict X gap, but the previous
centroid-normal candidate intersected the material hull. Native strict
separation now tries the original centroid candidate followed by at most
three coordinate-gap midpoint planes. Bounds and rounded midpoints propose
planes only; exact independence and fresh strict signs of every original
or privately certified enclosure control remain necessary. Failed candidates
and work exhaustion do not authorize separation. All candidates share one
remaining exact-work budget and successful certificates retain original
regions and any planar material enclosure authority.

Actual [1,13] qualifies in 10,900 charged operations; reversed order also
qualifies, and work=1 refuses. Synthetic point-hull admission tests show
that adjacent representable values whose rounded midpoint reaches an
endpoint refuse, coincident points refuse, and a two-step gap with a strict
representable midpoint can qualify. These isolate numeric admission rather
than asserting synthetic shell validity. Original rotated shaft and shared
cone-pole regressions pass, as does irrational-root closed body/topology.
The full annular gate advances through all face-1 pairs to [2,4]; the initial
stale assertion failure is retained. Complete annular Body/volume and new
WASM/UI/named-part acceptance remain outstanding.

Final full annular gate regression passes at [2,4] (19.93 s), as does curved
root/body/contact/volume replay (23.35 s). These concurrent local runtimes
are not controlled performance comparisons.


## Whole original annular embedded geometry admission

The original [2,4] outer-wall/bottom contact already qualified independently
via the existing natural plane-image proof (14,659 exact operations, zero
classified driver cells), including reversed order and work=1 refusal.
Scheduling diagnostics identified the integrated failure: [1,23] consumed
all 100,000 driver cells in heavy fiber guesses before the cheaper exact
shared-line proof ran. Later independently supported contacts were skipped
because the global driver budget had already been exhausted.

The native pair scheduler now attempts exact shared-line contact and the
natural plane-image/owned ruled-projection proofs before the heavier paired
fiber and chart guesses. All failed attempts remain charged; the original
100,000 driver and 100,000,000 exact-work limits are unchanged. No caller
flags, topology-only exclusions, samples or increased budgets authorize
contacts. Private shell geometry separately recomputes vertex links and
chart injectivity before accepting the original pair certificates.

The actual 27-face annular fixture now issues
`source-shell-embedded-geometry-qualified`, with no uncertain face/pair:
all 351 pairs qualify, 28,005,677 exact operations, 712 spans, 48 driver cells
and 2,337 linear cells. It does not use an inverse-shear proposal. The initial
assertion expecting incomplete geometry failed; retained logs show this
advancement. The rewritten full-gate regression passes (1.98 s).
This is original embedded shell Geometry, not a volume/orientation Body
certificate. Full interval fillet/radius/tangency/wall qualification,
independent STEP/named-part acceptance and packaged WASM/UI remain separate.

Four natural plane-image positive/negative regressions and the root-clipped
contact search regression pass. The flat-root integrated audit originally
expected a fiber certificate; scheduling now selects exact shared-line
certificates first. Its direct fiber checks remain and the audit now requires
an original shared-line certificate on every pair. The old failure log is
retained; this changes scheduling assertions rather than root geometry.

Final bounded annular geometry regression passes. Flat irrational-root
body/topology replay passes (3.16 s); curved root/body/contact/volume replay
passes (15.23 s). Capped body independent analytic volume and fresh restore
regression passes (88.44 s); its interval [37.52914969854688,37.77914345395185]
contains the independent value 37.645933842430814. These concurrent local
test runtimes are not performance comparisons or annular volume evidence.


## Original annular Body volume through closed carrier Green flux

The existing 2D trimmed-domain flux search exhausted 99,999 cells/spans and
1,000,000 classification cells at requested 20 mm3 error; its broad interval
[6891.509073457432,7231.33740643522] did not authorize Body admission. The
terminal failed initial test is retained. A native planar flux path now uses
Green's theorem on privately owned original world boundary carriers when
all surface coefficients share the requested flux coordinate. It requires
literal complete source endpoints, direct complete SharedEdges, unchanged
positive clamped single-Bezier carrier definitions and exact closed joins.
Projection copies the two original complementary coordinates and weights;
direction reverses coefficient order without recomputing authored knots.
All oriented loops, including holes, contribute their signed area. The
constant coordinate minus origin multiplies the outward interval area.
A coordinate identical to origin has provably zero flux. Unsupported
root/mapped/multispan carriers or curved surfaces retain the previous gate.

Green interval work is charged to both volume cell and span budgets. Fixed
full-face bounds are excluded from surface subdivision; the final global
width/sign test still decides admission. A coarse or exhausted area bound
never skips that test. Native annular admission now yields positive Body
orientation and volume [7061.341216618879,7061.591214207668] mm3, width under
0.25 mm3, using 28,287 cells/spans and zero 2D domain cells. This is one
specified original coefficient fixture, not arbitrary fillet qualification.

The independent Python verifier evaluates original rational Bernstein
coefficients with Gauss quadrature and uses boundary flux only for horizontal
affine trimmed planes. Exact binary-rational XY rank proves zero Z flux for
vertical trimmed planes; unsupported tilted trims refuse. At orders 16,
32 and 64 the numerical values are 7061.457370838773,7061.457370838772 and
7061.4573708387725 mm3, maximum difference 9.1e-13 mm3. The value lies inside
the native interval. Floating convergence is not a rigorous quadrature error
bound, STEP-reader proof, or source-edit/radius/tangency certificate. The
coefficient fixture and SHA-linked verifier result are retained.

Negative tests keep curved fallback and insufficient-work refusal, and prove
zero flux on the original bottom coordinate plane without sampling. Flat
irrational-root body/topology and curved root/body/volume regressions pass.
The current source STEP writer explicitly refuses pole-bearing bodies;
this annulus has two poles and still requires explicit degenerate exchange
topology. Packaged WASM/UI, full fillet/wall diagnostics, named-part and P0
acceptance remain outstanding.

Native annular restore recomputes original shell, embedding and volume even
when the saved value adds false volume [1,1], success=true and reversed
orientation=true claims. The restored source definition, vertex identities,
orientation and volume interval exactly match the admitted original body.
This regression passes (84.66 s). It does not establish UI Undo/Redo, crash
recovery or multiple-tab behavior.

Final positive-weight/single-Bezier guarded Body and fresh restore test
passes; its terminal log is retained. The final independent quadrature
was rerun against the emitted original coefficient fixture and passes.


## Explicit original pole STEP topology and independent OCCT qualification

Source STEP preparation now admits privately proved collapsed UV boundaries
with literal parameter endpoints. Each original UV carrier and source
surface is emitted unchanged as a trimmed PCURVE; EDGE_CURVE references
that PCURVE and the same original source vertex at both ends. It has no
new surrogate 3D carrier. Face loops retain the directed pole boundary.
The 2D definitional context explicitly includes PARAMETRIC_REPRESENTATION_CONTEXT.
The reference schema describes PCURVE as a curve with a basis surface and
2D definitional curve: https://www.steptools.com/docs/stp_aim/html/t_pcurve.html .
The independent OCCT writer was inspected on cone/Bezier pole controls;
its omission of pole edges did not substitute for this explicit topology.

Two literal pole endpoints each are reserved within the unchanged global
trim-work cap. Insufficient positive work returns no candidate; zero or
work beyond 10,000,000 refuses. Root-valued pole trims still explicitly
refuse pending independent parameter representatives. Existing original
rooted shared-carrier preparation remains unchanged. The native annular
candidate has 27 faces, 55 edges (53 shared carriers plus two poles), 26
vertices and endpoint error upper 2.2026824808563116e-13 mm.

Independent OCCT reads the final candidate as one valid solid with all
27 faces, 55 unique edges, 26 unique vertices and exactly two degenerate
edges. Both poles match (20,0,6) and (0,20,6) mm exactly. Volume is
7061.457372466771 mm3, differing from independent original-coefficient
quadrature by 1.6279982446576469e-6 mm3. Maximum extent difference from
[-20,-20,0,20,20,6] mm is 1.0000001182675078e-7 mm. STEP and SHA-linked
verifier report are retained. This proves one specified source export,
not arbitrary pole trims, geometric editing or named-part acceptance.

A deliberate 0.1 mm pole vertex corruption keeps OCCT's general validity
flag true but fails the verifier's explicit pole coordinate test. Both
expected and observed pole sets are checked in both directions. Native
budget tests, irrational-root body/topology/export regression and original
coefficient serialization test pass. The TS response gate now expects STEP
edge count = original shared carriers + pole count. Ten existing worker
compatibility tests pass against the unchanged packaged WASM; they do not
qualify this new native pole path in WASM or the browser. Fillet/wall
qualification, new WASM/UI, named-part and full P0 acceptance remain open.


## Original admitted body radius and regular middle seams

The annular Body fixture now checks the actual original faces 0, 5 and 10
against the authored center curves and cubic radius laws. The exposed
CircularBlendSpan radius_curve returns those same original coefficients;
existing authoring qualification uses it without changing geometry. Exact
source surface equality, including reversed U orientation on the exit
face, is required before qualification. Whole-domain radius error upper
bounds are 5.610988272093926e-7, 3.622329271863914e-13 and
8.407770510685909e-7 mm respectively. A corrupted radius coefficient
(+0.1 mm) and work budget 1 refuse certificates on each face.

Both original middle rails have fresh whole-interval regular tangent-plane
certificates: edge 6 sine-squared upper 2.3404439736769268e-26, and edge 8
upper 4.7060135238022814e-7. Budget 1 refuses both. Four transition rails
with collapsed endpoints refuse regular G1 certificates and report an
uncertain canonical interval under the stated bounded work. Work or
resolution exhaustion is not a proof of a geometric defect. No endpoint
limit tangency or regular pole tangent plane is claimed.

The combined original Body test passes in 22.05 s; all five native seam
regressions pass, including crease rejection, singular endpoint refusal,
root endpoints and reversed partial carriers. Existing moving-radius
authoring regression passes. Terminal logs are retained. This does not
qualify general fillets, complete wall thickness, packaged WASM/UI,
named-part acceptance or the remaining P0/P2/P3 scope.


## Bounded endpoint normal diagnostics

Regular seam qualification now checks both literal full-carrier endpoints
before adaptive interval subdivision. An unresolved endpoint normal returns
source-seam-endpoint-normal-unresolved with a point canonical interval;
this is incomplete qualification, not a geometric defect certificate.
Root endpoint enclosures and affine carrier ranges keep the original
whole-interval path because their enclosure endpoints are not exact
source representatives. All work remains charged to the same limits.
The singular endpoint regression now requires the diagnostic in one cell.
All five seam regressions pass, including rooted and reversed partial
carriers. This does not provide one-sided pole limit normal certificates.

The final annular Body radius/seam regression passes. Middle rail bounds
remain unchanged; endpoint preflight adds two cells to each accepted rail.
The four collapsed-end rails report canonical 0 or 1 in one or two cells.
Both final terminal logs are retained. WASM/UI packaging remains pending.


## Selected source seam bridge and worker response contract

Source Body restoration accepts an optional seamQualification request for
one original owned edge. Native limits and edge index are validated before
restoration, including when restoration cannot admit the Body. After fresh
Body admission, the original seam qualifier reports qualified status,
angular bounds only with its private certificate, reason, bounded work
and uncertain canonical interval. A sharp or unresolved seam does not
invalidate the independently admitted Body. The exact request is echoed.

The worker response gate requires the matching edge and all request limits;
missing or unsolicited seam results, over-budget work, invalid uncertainty
intervals, an unsupported reason and false success bounds refuse. Native
JSON dispatch test passes, including invalid edge and zero-budget requests.
Ten worker tests pass; new seam responses there are explicit transport-only
fixtures against the previous packaged WASM, not proof of a new WASM path.
Direct vue-tsc --noEmit passes. The npm pretypecheck build is still in
progress at this checkpoint. Generated WASM, real new seam worker execution
and localized UI interaction remain to be qualified separately.


## Source seam UI wiring checkpoint

The source STEP/file panel now includes a selected-edge tangency check,
Cancel and Retry, localized incomplete endpoint-normal messages, angular
bounds and uncertain canonical interval details. A refused selected seam
changes its selected-edge stroke to the diagnostic color. A dedicated
worker and generation guards cancel on document/selection/open changes;
late responses also check record identity and disposal. Escape cancels.
Direct vue-tsc passes after the final UI edits. In isolated Vite preview
5188, the source canal document loads and edge 0 is selected by Enter.
The tangency button is present. Actual new WASM qualification and browser
Cancel/Retry/late-response acceptance are pending optimization/packaging.
The active root checkout/runtime 5175 was not changed.


Selected seam request supersession now has a focused worker test: edge 0
is superseded by edge 1; the first worker is terminated and its saved late
callback cannot settle the second request. The focused supersession and
abort/Retry tests both pass. In isolated browser 5188, the previous WASM
response without requested seam diagnostics is rejected by the protocol
gate and shown as a localized incomplete check with Retry. Retry followed
by Cancel returns to the idle tangency button with no error shown. This
is old-WASM compatibility/cancellation evidence, not new WASM qualification.
Optimization remains live; the pending new real-WASM test is uncommitted
until the rebuilt artifact is packaged and verified.


The seam response expectation now copies request limits instead of retaining
a caller-owned mutable reference. A focused regression mutates the original
edge, angular tolerance and cell budget after expectation creation; the
original request key and limits remain fixed. The regression passes.
This prevents changing response admission bounds while a worker is active.


## Rebuilt WASM original annular seam qualification

The complete npm pretypecheck geometry build and TypeScript checks finish
successfully. New packaged geometry WASM SHA256 is
ae848eb6e1e7e3c1778c865285c9a98694b1919f6c4d9e1857e978f39685e812,
11,847,649 bytes (original compiled payload 13,371,614 bytes).
Both source worker and seam transport suites pass: 13 tests in 14.02 s.
The real worker freshly restores the original 27-face, two-pole annular
Body and certifies middle rail 6; transition rail 0 refuses with
source-seam-endpoint-normal-unresolved, canonical [0,0], one cell.
A separate real selected seam test on the source canal also passes.

The native test emits the original Body definition without surrogate mesh
authority. Annular request fixture SHA256 (uncompressed UTF-8 JSON) is
b9269dc1cf938d0e1b05b8c7550bdef025ea371385c487c49ade35e8c27af9ea,
167,972 bytes. Native original radius/seam fixture test passes in 12.15 s.
This qualifies the specified new WASM/worker cases, not general fillets,
whole-wall diagnostics, complete browser acceptance or named-part chains.
Full distribution size budget remains to be measured after this rebuild.


## New packaged WASM browser and distribution checkpoint

After reload on isolated Vite 5188, persisted source canal restores and
edge 0 is selected by Enter. The actual browser worker returns
source-seam-tangent-planes-qualified; UI displays whole-edge confirmation,
sine-squared upper 9.978209658498833e-7 and 8013 cells. Screenshot retained.
This is source canal browser evidence; annular pole diagnostics are proved
in the real WASM worker suite but still need their own browser acceptance.

Vite production build passes. Initial size verification flags growth.
Measured DirectModeler 423231 bytes, source worker 158944 bytes, packed
geometry 3953416 bytes and non-streamed asset total 8243477 bytes.
Budgets are updated to these measured values plus the previous respective
511, 123, 608 and 761-byte margins. Other artifact limits and packed/native
identity checks remain enforced. Final verify-dist passes on 151 artifacts:
8243477 asset bytes plus 15144041 raw WASM bytes = 23387518 total bytes.
General geometry, pole limit tangency, wall checks, named-part and full
P0/P2/P3 acceptance remain open. No root runtime 5175 change or push here.


## Annular pole and regular rail browser acceptance

The original annular document imports through the new WASM in isolated
Vite 5188. Edge 0 selected by Enter produces the localized unresolved
endpoint-normal message, canonical [0,0], one cell, and its SVG stroke
changes to #ff9977. Retry immediately followed by selecting edge 6 clears
the old pending state and diagnosis. A fresh edge 6 check displays
whole-edge tangency confirmation, sine-squared upper
2.3404439736769268e-26 and 17 cells. Screenshots and the original archive
are retained. This exercises selection, localization and supersession on
the annular Body; it does not prove endpoint limit tangency.
The display separately reports 184 unresolved trimmed face tiles; these
remain explicit preview coverage limits, not admitted material triangles.
A browser STEP download observation timed out with no captured file;
no browser STEP export success is claimed at this checkpoint. Existing
native/OCCT pole STEP evidence remains separately scoped.


## Annular browser STEP, cancellation and reload

The in-app download event observer misses the browser export, but actual
Downloads files Annular_transition.step and Annular_transition (1).step
exist and both hash to ed039ea999e56d778679198607d879f91a28bf3b488bdc2e10312865af7bc756.
The second browser file is independently read by OCCT and passes all
original annular topology, bidirectional pole, extent and volume checks:
one valid solid, 27 faces, 55 edges, 26 vertices, two degenerate edges;
pole error 0 mm, maximum bounds error 1.0000001182675078e-7 mm,
volume difference from independent coefficient quadrature
1.6279982446576469e-6 mm3. Actual downloaded bytes and report retained.

Another browser export followed immediately by Cancel returns the panel
to idle and leaves exactly the same two matching STEP files, with no new
download. After browser reload, the annular source Body freshly restores,
all 53 shared edges are selectable and edge 6 can be selected again.
Screenshots retained. This confirms this fixture's export/cancel/reload
path, not geometric editing, Undo/Redo, crash or multi-tab acceptance.


## Original source face group clearance foundation

New native source_face_gap qualifies a requested positive separation
between two disjoint groups of privately admitted Body faces. Every
selected face pair is covered by original natural chart rectangles;
these are conservative supersets of all retained trims, including holes
and root-ended fragments. Original rational interval boxes supply lower
bounds. Unresolved rectangles subdivide with globally bounded cells and
original span visits. Missing work or unresolved numeric resolution returns
no certificate and the original face pair/UV rectangles. Certificates
borrow the exact admitted Body and retain both selected face groups.
No cached mesh, sampled upper witness or detached saved success is used.

The annular regression covers all 36 pairs of six original flat bottom
faces and six original flat top faces: requested 5.99 mm separation is
certified with 36 cells and 72 original span visits. A 7 mm request with
8 cells/16 spans refuses and localizes the unresolved pair; insufficient
span work and overlapping face groups also refuse. The combined original
radius/seam/body regression passes in 27.25 s.

This is a lower clearance certificate for specified face groups, not
minimum whole-wall thickness: curved transition faces are not in these
groups. Interior material chord, normal alignment, whole-wall coverage,
source bridge/UI integration and new packaging for this module remain
to be developed and qualified. The last packaged WASM is unchanged.


## Curved original cylindrical face union clearance

The annular test now covers all 36 pairs of original outer cylinder faces
[2,7,12,16,20,24] and inner cylinder faces [3,8,13,17,21,25].
A 14.5 mm lower clearance certificate succeeds with 5344 cells and
10688 original span visits within unchanged 10000/20000 limits. The
combined Body/radius/seam test passes in 15.18 s.

The first scheduler exhausted 10000 cells/20000 spans on pair [7,3],
subdividing both extrusion heights along with transverse coordinates.
For original charts with identical XY controls and weights across each
V row and overlapping Z boxes, subdivision now prefers U. Every
rectangle retains full original V coverage; the same 3D interval bound
remains the sole acceptance condition. This preference is conservative
and may still refuse difficult cases. It does not certify material chords.
A rational weight-change regression shows XY varies with V despite
identical XY controls, so the preference stays disabled. It passes.
Both failed initial and successful final terminal logs are retained.
Curved fillet transition faces, whole-wall thickness, interior material
coverage and this new gap module's WASM/UI qualification remain open.


## Original blend and transition face clearance

The source gap scheduler now chooses subdivision axes per surface. An
original chart whose XY controls and rational weights are identical across
V retains full V coverage when Z boxes overlap, even if the other chart
needs both parameters. Acceptance still uses the unchanged complete 3D
rectangle enclosure; this changes scheduling only. The rational weight
regression passes and keeps V active when weights vary.

The admitted annular Body test now covers all 54 pairs of outer/blend faces
[0,2,5,7,10,12,16,20,24] versus inner faces [3,8,13,17,21,25].
A 13.5 mm lower clearance is qualified with 12262 cells and 24524 original
span visits, within 50000/100000 limits. This includes the three original
transition surfaces. The combined original Body/radius/seam test passes
in 25.80 seconds. Terminal logs are retained alongside this record.

This remains separation of complete source surface chart supersets, not
material wall thickness. A source-domain boundary audit, interior material
chord and normal alignment are still required. No new WASM or UI coverage
is claimed for the gap module.


## Original source finite segment boundary audit

`source_material_segment::inspect_boundary` now audits an immutable admitted
Body directly. It isolates intersections on each original rational surface
and classifies retained root-ended source contours with original winding.
No conversion to Model, mesh or rounded trim endpoints is used. Work budgets,
trim uncertainty and closed-segment endpoint bands remain explicit refusals.
The public diagnostic report is not a material certificate.

On the original annular Body, segment origin [10,2,-1], direction [0,0,8]
has exactly two disjoint interior boundary root intervals, no unresolved
regions, 27 geometry cells and 112 contour cells. An exterior segment
[30,30,1]+t[0,0,1] is boundary-free; zero direction rejects. The combined
Body/radius/seam test passes; its complete terminal log is retained.

Boundary-free exterior segments illustrate why this report does not prove
material membership. Outside seed qualification, parity, normal alignment
and a private source material chord certificate remain required before
wall thickness admission; saved diagnostic fields cannot authorize them.


## Private original source normal material chord

`source_material_chord::qualify` recomputes the finite boundary audit on the
borrowed immutable admitted Body. It proves the authored start point is
strictly outside the union hull of every complete original source chart.
Exactly two disjoint isolated transverse root intervals with complete
trim coverage establish entry then exit of material. Both endpoint root
UV rectangles must pass the original normal/line angle check. The private
certificate retains the exact Body reference, face identities and length
interval; caller diagnostic flags do not authorize it.

For [10,2,-1]+t[0,0,8] on the original annular Body, the certified material
chord length is [5.999999999999997,6.0000000000000036] mm. Tests require
Body identity and enclosure of 6 mm with width below 1e-5 mm. Interior
seed, exterior empty segment, insufficient boundary cells, oblique line
and insufficient normal spans all refuse certificate issuance. The combined
Body/radius/seam regression passes in 16.45 seconds; terminal log retained.

This is a local normal chord certificate. The conservative exterior hull
seed may refuse valid starts in concavities. General outside-seed parity,
complete wall coverage and automatic thin-region search remain open, as do
bridge/WASM/UI integration of this source chord and clearance calculation.


## Whole selected source wall union minimum bounds

`source_material_wall::qualify` recomputes full selected face-pair clearance
and a normal material chord on the same immutable Body. Its private
certificate owns both proofs, verifies endpoint membership in opposite
selected groups and retains the interval for the minimum admitted normal
material chord length between those groups. Clearance alone cannot issue
this certificate. Convergence requires outward-rounded interval width no
larger than the requested positive millimetre tolerance.

All bottom/top source face pairs of the annular Body give minimum bounds
[5.999999999999997,6.0000000000000036] mm and convergence at 0.02 mm.
The same interval does not claim convergence at 1e-16 mm. Using a valid
vertical material chord for outer/inner cylindrical groups refuses with
`source-wall-candidate-outside-groups`. Body identity is asserted. The
complete combined Body/radius/seam test passes; terminal log is retained.
The initial test incorrectly expected a wider interval from a looser
threshold: the full rectangle bound remains tight independently of that
threshold. That failed log is retained and the tolerance test is corrected.

Scope remains the explicit groups and stated endpoint angle tolerance.
This does not enumerate every body wall or automatically find candidates.
Automatic whole-body wall coverage, thin-region search and source bridge,
WASM and UI integration remain required.


## Automatic original normal chord proposal search

`source_wall_search::search` generates finite lines from sampled original
chart points and normals with an exterior reach proposal. Every accepted
result independently recomputes source exterior membership, full boundary
root isolation, source trim membership and both endpoint normal bounds.
Only private normal material chord certificates are retained. The shortest
certified upper witness is returned; samples never certify geometry.

The original annular bottom/top groups with a 3x3 grid across six bottom
charts produce 54 attempts, 36 refusals and 18 certified candidates. The
best length interval is [5.999999999999997,6.0000000000000036] mm. The
initial one-point grid refused all six proposals; its failed acceptance
expectation and terminal log are retained. The final regression also checks
a one-attempt cap does not claim candidate exhaustion and overlapping
groups reject. The combined Body/radius/seam test passes in 18.37 seconds.

This search supplies automatic upper witnesses, not global minimum proof
or absence of thin regions. `candidates_exhausted` means only the requested
finite proposal grid was visited. Work is bounded by max_attempts times
per-candidate limits. Adaptive refinement, coverage of every body wall and
combining searched witnesses with certified lower bounds remain open.
No bridge, WASM or UI integration is claimed for this new module.


## Automatic selected wall thickness bounds

`source_material_wall::search_and_qualify` combines fresh whole selected
face-pair clearance and automatic original normal material chord search.
It verifies the immutable Body identity and opposing group membership,
then owns both private certificates and returns a bounded minimum normal
material chord interval. Proposal exhaustion does not replace lower-bound
coverage. Positive interval convergence uses outward-rounded width.

The original annular bottom/top unions qualify automatically at 0.02 mm
with interval [5.999999999999997,6.0000000000000036] mm using a 3x3 grid
and 54 proposals. A one-cell clearance budget refuses the combined
certificate even though the same search independently finds a valid chord;
the report retains this witness and marks convergence false. Tests assert
both admission and refusal and original Body identity. The combined
Body/radius/seam regression passes; terminal log retained.

This closes automatic candidate-to-bound composition for explicit groups.
It does not identify all opposing body wall regions, prove coverage of
every wall, or qualify bridge/WASM/worker/UI transport. Those remain open.


## Source wall qualification native bridge

`cad_source_body_restore` accepts optional `wallQualification`: original
face groups, minimumMm, toleranceMm, toleranceUv, grid, maxAttempts and
explicit gap/search/normal work limits. Owned, unique, nonempty disjoint
groups and every tolerance and budget are validated before Body admission.
After fresh original Body restoration, native automatic minimum bounds
are recomputed and the response echoes the exact request. It reports
qualified, converged, intervalMm, lower-clearance reason/work/uncertainty
and search attempt/refusal/exhaustion counts. Failed wall qualification
has a null interval; it does not invalidate an otherwise admitted Body.

The real native dispatcher regression verifies refusal on a one-cell
adjacent-face gap request, matching request identity, null interval and
false convergence, bounded work counts, and early errors for overlapping,
empty or foreign groups and zero geometry cells even when restoration
would otherwise refuse. The existing edge-address/seam/STEP restoration
regression passes together with these assertions; terminal log retained.

This proves native API behavior. Positive annular native thickness is
qualified separately above. Current packaged WASM, TypeScript transport,
worker cancellation/late results and new UI have not yet been qualified
for this option; no browser wall-thickness completion is claimed.


## Source wall TypeScript transport guards

Source Body options/results now carry optional wallQualification, validated
by sourceWallTransport. Expected groups, limits and all request scalars are
deeply copied. Matching exact canonical request identity, bounded geometry
and search work, explicit uncertainty on refusal, certificate interval on
qualification and outward-rounded convergence are required. Missing or
unsolicited wall responses reject. Failed Body admission cannot carry a
wall certificate. Transport never computes geometry or material admission.

Tests cover snapshot mutation, request substitution, missing interval,
excess work, no successful candidate, false convergence, lower bound below
request, unexpected uncertainty, null refusal interval, exact tolerance
boundary rounding and invalid owned groups/search limits. Four focused
transport tests pass; vue-tsc --noEmit passes. The real packaged worker
restoration/seam regressions also pass with these new guards, but those
worker runs do not request wall qualification from the old packaged WASM.
Complete terminal logs are retained.

New WASM packaging, positive native wall option through the real worker,
cancellation/late-response wall scenarios and UI controls remain required.


## Source wall panel selection acceptance, new WASM pending

The new panel offers explicit opposing face sets, minimum/tolerance fields,
localized result/refusal, Retry and Cancel. Request generations and sync
watchers cancel and clear on body/document/panel/input changes; Esc and
unmount cancel the worker. Selected source face tiles show side A blue
and side B purple; unresolved clearance faces use orange. This color is
selection state, not geometry admission.

Browser 5188 on the annular document verifies side A [5,10,15,19,23,27]
and side B [2,7,12,16,20,24] (one-based UI labels), selection by mouse
and Enter, enabling calculation only after both selections, and moving
face 5 between sides removes it from the opposite set. DOM shows twenty
preview polygons in each side color. The retained screenshot shows the
corrected field layout. The draft panel passes vue-tsc. A controlled
worker-port test passes group-change supersession, discarded late result,
AbortSignal cancellation and retry with a fresh worker.

The new real WASM annular thickness test is prepared but not yet run:
packaging/optimization is still live. No positive browser wall interval,
UI Retry/native cancellation or new packaged WASM completion is claimed.


### File menu Escape cancellation

Browser testing found File menu Escape consumes the event before the
workspace handler. closeFileMenu now explicitly cancels pending source
wall, seam and STEP workers before closing. Selection is retained for
retry. On annular UI sides [5] and [2], the real browser enters the wall
pending state, pressing Escape on its Cancel button closes the menu, and
reopening shows an enabled calculation button without pending/error. This
checks real worker cancellation while using the prior packaged WASM; it
does not prove new native wall results. Screenshot retained, vue-tsc passes.
The new optimized WASM build remains live; positive wall worker test is
still pending that artifact.


## New packaged WASM and positive wall browser acceptance

The completed build optimizes 13399375 to 11872215 bytes. Packaged SHA256
86ab0caa2618730830b58aa38b079471322e48df7f8f2a66efe9e33ca1c3f552.
All 18 source Body worker/wall/seam transport tests pass in 15.93 seconds.
The real worker annular automatic wall scenario takes 1541.54 ms for both
success and refusal runs combined on this machine (single observation,
not a performance qualification). Bottom/top groups qualify 6 mm bounds
[5.999999999999997,6.0000000000000036], with 36 clearance cells/72 spans,
54 proposals and explicit budget-refusal with null interval on one gap cell.
Request group substitution rejects; transport cancellation/retry tests pass.

Browser 5188 now shows the same confirmed wall interval through the new
UI and packaged kernel. Both face sides were selected with mouse/Enter.
A scene triangle click selects face 6 after stopping the background
pointerdown selection reset; normal scene interaction remains active when
the panel/menu is closed. Closing File normally also cancels a pending
wall request; Escape cancellation was qualified above. vue-tsc passes.
Terminal build/tests, exact worker JSON and positive UI screenshot retained.

This proves explicit selected flat wall groups through WASM/worker/UI.
Whole-body wall discovery, adaptive thin-region coverage, measured chord
scene coordinates, full command matrix and arbitrary fillet geometry are
still open. Production bundle size verification remains a separate gate.


## Source wall production distribution gate

Vite production build completes. The first distribution check rejects
DirectModeler at 429949 bytes against its prior 423742 limit. Measured
new artifacts: geometry packed chunk 3962910 bytes, mainSolid worker
161534 bytes, DirectModeler JS 429949 bytes, total assets 8265356 bytes.
The selected wall native/transport/UI addition increases assets 21879
bytes from the prior 8243477 baseline. Named budgets are adjusted only
for these changed artifacts, retaining prior margins: geometry 608 bytes,
worker 123, DirectModeler 511, asset total 761. Other limits remain intact.

The final verifier passes all 151 artifacts, packed/raw module validity,
unique packed payload and source identity: 8265356 asset bytes plus
15168607 raw WASM bytes = 23433963 distributed bytes. Both initial
rejection and successful terminal logs are retained. This is distribution
size/integrity proof; it does not qualify full wall coverage, runtime
latency distributions, geometric editing or the remaining full goal.
