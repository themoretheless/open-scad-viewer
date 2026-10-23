# Unequal-radius corner boundary qualification

Latest verification: **2026-10-21, current working tree**. Exact owned-source hashes and results are in [collars-source-result.json](collars-source-result.json).

## Confirmed

- Three root-trimmed open G2 strips on a convex orthogonal corner, with independently prescribed rolling-centre radii **0.1, 0.2 and 0.35 mm**, runout **0.1 mm**, span **2 mm**.
- Shared literal endpoints, six endpoint G0/G1/G2 joins, an ordered six-curve rim, original trimmed-face membership of its three connectors, individual injectivity and whole-rim simplicity.
- Positive certified pairwise strip distance lower bounds; they exclude intersections without asserting a converged minimum distance.
- Six stored NURBS boundary collars, each freshly checked against its complete original support: three constructed strip surfaces and three original body faces. Every original rim curve also agrees with both its support and stored collar.
- Whole-seam position, normal and shape-tensor bounds retain tolerances **1e-7 mm / 1e-5 / 1e-4 mm^-1**. Collar preparation uses **183 cells**, **1815 exact-predicate work**, and **1,039,125 coefficient work** within the supplied budgets. These counts exclude the preceding strip construction.

The correlated proof now composes directly from original B-spline coefficients by Cox-de Boor recurrence on one retained knot interval. It uses no rounded extracted reference surface. Mapped collars crossing original knot intervals remain refused. The original single-Bezier path remains covered by regression tests.

## Validation

**11 current-worktree native tests passed:** six NURBS collar tests and five B-rep/rim tests. Controls cover rational multispan coefficients, independent analytic positions, changed curvature, changed original surface/pcurve, a crossing knot interval, wrong radii, exhausted work, a spatial bow-tie and mismatching literal endpoints. Two execution-monitor tests and the benchmark noninterference test also passed.

Independent **OCCT 7.7.2.0** reconstruction of the actual saved collar coefficients checked **198 stations**. Maximum sampled errors: position **1.34e-15 mm**, normal **1.13e-13**, world shape tensor **1.53e-10 mm^-1**. Samples supplement the native whole-interval proofs. This check reconstructs stored NURBS coefficients, rather than importing a closed STEP solid. See [collars-occt-report.json](collars-occt-report.json).

The three open-strip STEP files independently import with one face/four edges/four vertices and zero solids each. Surface area is integrated over the actual imported UV trim and compared with an independent quintic-section calculation. OCCT's separate BRepGProp estimate remains in the report for comparison; its reported integration error underestimated the observed discrepancy. The acceptance threshold remains 1e-7 mm². See [occt-report.json](occt-report.json).

## Test suspension: root cause fixed

SDF GPU benchmark runners sent **SIGSTOP** to competing same-user cargo/rustc/test processes to force quiet measurement windows. The paused-process journal contains the exact identity of the monitored CAD cargo process; the benchmark's recorded runner hash matches the pre-fix source. This establishes the cause without relying on an inferred hang.

Five runners now **observe competing work**, wait for an uncontaminated window and never stop another session. Their own descendants are excluded without exempting unrelated jobs under the same desktop application. Legacy event fields remain compatible. Completed measurement data were preserved; the active old controller was terminated through its existing recovery handler, which resumed its journaled identities.

`run-native-monitored.py` starts the command in its own session, records actual stopped/continued/exited statuses and propagates the native exit code. It never restarts or automatically resumes a stopped child. A synthetic stop/continue regression verifies the same PID reaches its terminal status. All three post-fix current-worktree runs completed with exit code zero and no stop events.

## Still required for the full goal

The central spanning surface is **not constructed**, and there is **no closed filleted body**. Next are the six-sided G2 fill, source-face cutting and grafting, end transitions and solid assembly. Endpoint curve G2 does not certify a future spanning surface.

General NURBS edges, concave/multi-edge corners, full wall thickness, whole-body intersections/gaps, WASM/worker/UI admission, three-part STEP/history acceptance, P0 and P2–P3 remain unqualified. Radius values here describe the rolling-centre law of quintic G2 sections; constant circular-section curvature remains unqualified.

Raw evidence and source snapshots are retained in ignored `output/qualification/test-suspension-20261021` and `output/qualification/trihedral-strips-20261020`. Historical pending metadata is retained with an explicit supersession link; it does not describe current acceptance.
