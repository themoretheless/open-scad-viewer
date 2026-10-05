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

This is native delivery. The finite-patch query is not yet exposed in WASM/worker/UI. Adaptive multi-patch fitting, endpoint tangent tolerance, replacement source trims, stitching, transitions, embedding, volume/thickness qualification and final solid admission remain open. Constant-radius fitting does not establish varying-radius support.
