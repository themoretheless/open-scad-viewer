# Active full Rust geometry goal — affine G2 checkpoint in qualification

Authoritative checkout `/private/tmp/osv-ideal-body-g2-20261015`, branch
`codex/ideal-body-global-g2`. Code HEAD `d1956c93609ddf68869289d6e0556c2652cde1a0`.
Primary checkout has unrelated dirty work and must not be reset/checked out.

The whole goal is ACTIVE, not complete. Read
`docs/design/exact-affine-profile-g2.md` and `global-volume-frontier.json`.

Nine fresh focused affine profile tests passed (log `logs/affine-focused-final.log`).
128-face/256-edge spatial rational quadratic body candidates have complete native
G1/G2, exact edge correspondence, connected vertex links and all-face regularity.
Common original-profile affine quintics avoid independent pole snapping. Positive
families: original spatial corrected Bishop motion; authored and guided spatial
frames with simultaneous scale/twist/affine laws; a genuinely nonplanar cubic
original profile. Sharp original profiles/stations remain C0; zero/short work,
changed basis/weights, nonexact closure, singular samples and singular motion
between nonsingular samples refuse. Cubic proof needs >2M total exact work; native
aggregate ceiling is now 4M, per seam remains 1M. One-short and oversized checked.

The new NURBS rational-normal Bernstein verifier proves the thin moving patch in
one cell, where derivative-only methods exhausted 100000. It handles literal
nonperiodic Bezier degree <=5 per axis, outward products, exact normalized dyadic
subdivision, conservative natural-domain labels and one created-cell budget.
Generic/periodic/higher bases retain the old derivative verifier. Rational folded
and collapsed negatives stay unproved. Original filled-interior fold integration
shares every profile boundary coefficient with a certified valid Solid but its
original global atlas refuses (work-limited/unproved, not a collision witness).

FULL FRESH NATIVE DRIVER IS RUNNING. NURBS lib completed successfully: 1575 passed; current stage is source-volume-matrices. Tool exec session **73882** belongs to:
`python3 docs/qualification/affine-profile-g2-2026-10-16/validate.py`.
Do not restart on an observation timeout. Read `summary.json`, then wait/poll the
actual session. All checks run fresh, no inherited lower NURBS credit, because
NURBS regularity/collection ceilings changed. Guard: **3733 inputs**,
**0864343ba9e6748addbf92a61e706c80c5c00c2cb31169ca3fa1ba13b5dd25b4**.
Native inputs are frozen until the driver terminates. Code commit changed no input
bytes. Qualification docs/logs are deliberately uncommitted while being written.
Full BRep previously took ~21 minutes (arc source cases); expect finite CPU work.
Driver checks: NURBS lib; all four source-volume integration matrices; surface
regularity/G2 integrations; BRep except AAG; AAG serial; bridge lib; native
JSON/STEP owners; no-codec libraries. Successful focused checks do not substitute
for terminal full qualification or public WASM/CI/STEP/browser results.

The independent full global-volume diagnostic DID terminate: old session 50989,
exit 101, 458.24 seconds, actual boundary/volume false, pending pair [94,96],
unresolved neighboring curved profile charts [0,1], [0,3], [1,2], [2,3], etc.
Its failed optimistic positive-Solid assertion is preserved in `probes/` and
`logs/global-volume-diagnostic.log`, not in the passing feature matrix. It supplies
NO new Solid credit. The candidate API has no Solid success flag. No old probe
process remains running; profiler sample is `/private/tmp/osv-affine-volume-sample-20261016.txt`.

Next main Rust work: one shared FILLED CARRIER volume atlas for actual quintic
poses, bound exactly to every retained wall coefficient and original UV contour.
Do not bypass face pairs using only regularity, a G2 seam, proximity or determinant
signs at samples. Existing common source-volume contact family only handles
polynomial four-wall degree-U=1 cells; neighboring rational curved U charts need
the full affine-carrier atlas. `ideal_volume/atlas.rs` already has a query-generic
inner algorithm; factor its context entry without weakening its exhaustive pair
cover, whole convex hull determinant/SPD proof or common wrapped-seam metric.
Then expose a native coefficient-owned affine-pose carrier evaluator with value
and Jacobian bounds, exact shared station/closed jets, no caller success flags.
Use normalized [0,1] volume parameters, split across actual quintic time intervals,
scaled original carrier jets and pose derivatives. Every failed/repeated query
shares aggregate work and finite primitive ceilings. Closed tail/head charts must
share ONE metric and exact literal carrier/pose endpoint identity.

Only after actual candidate atlas, exact material binding/ownership, every-owned
boundary G2, volume/nesting/orientation and original E+decomposition+candidate
Delta pass may a constructor-owned smoothed Solid be returned. Store original
Bezier profiles and retained sections privately in sealed ClosedBody, or regenerate
under charged exact source ownership; never accept caller-replaced models/reports.
Original and candidate atlases prove abstract parameter-domain topology, not an
ambient isotopy. General physical cap G1/G2, further source frame/pole/periodic
families and delivery gates remain open, alongside main integration preserving WIP.

Publication remains pending HUMAN authorization. Auto-review rejected external
GitHub push because destination/payload ownership/trust was not established.
Do not retry via another tool or indirect execution. Pending previous question
only covered through bae47c78b (older async item named f820/654); it does not silently
authorize this later HEAD. Destination is
https://github.com/themoretheless/open-scad-viewer.git, whereas origin in this clone
is the LOCAL primary repository. Local primary branch ref now preserves code HEAD d1956c936 without changing its working files. Local fast-forward ref saves are separate and
allowed; no GitHub push, PR, fresh CI or WASM credit exists in this checkpoint.
