# Exact nonperiodic unclamped sweep qualification

The Rust exact-blossom path extends original contour-domain certification to
nonperiodic unclamped profiles when every homogeneous interpolation and
dehomogenization is exactly representable. Rounded operations refuse identity.
The wall loop ownership gate now compares exact active-domain endpoints;
clamped data retains direct endpoint-pole comparison. No closure tolerance was
introduced. Periodic profiles remain outside this extraction path.

Native evidence: exact extraction/closure regressions 2/2 and sweep tests
153/153 passed. Public WASM suites passed 42/42 after correcting the fixture's
hole orientation. The complete constructor and Rush fixture certify the full
boundary error within budget. This does not turn the separate scalar component
reports into complete-boundary certificates.

Kernel SHA256:
`2dd56e829d8853d6b373d0df7482e544a68bb3495d83cc5d3094da6cedb24972`,
10,740,350 bytes; generated/public copies and packed artifact provenance agree.

Fresh export `external-step-unclamped/manifest.json` contains 35 cases. All
35 passed independent OCCT qualification, including 504 whole-domain cap
coedge checks. New case `rush-unclamped-hollow.step`:

- Native boundary embedding, material roles, orientation and Solid certified.
- OCCT valid Solid, one closed shell, 10 faces, 24 edges, two caps with holes.
- All edges have two opposite face uses and same-parameter agreement.
- Shared rational basis and control coefficients preserve the whole wall
  domain; 32 wall pcurves and 16 cap coedges pass whole-domain checks.
- Volume 15.625000000000002 mm³ against independent analytic 125/8 mm³;
  relative error 1.1368683772161603e-16.

The analytic area is 4×5/12=5/3 mm². A quarter-scale hole subtracts 1/16
of it; extrusion length 10 mm gives (5/3)(15/16)10=125/8 mm³.
Finite material-side probes remain sampled checks, not all-point containment
proofs. Native interval/exact certificates provide the stated native guarantees.

Logs: `sweep-unclamped-wall-closure-native-final.log`,
`sweep-unclamped-wall-closure-sweeps.log`,
`sweep-unclamped-wall-closure-public-final.log`, `sweep-unclamped-step-export.log`,
`sweep-unclamped-step-occt.log`. Independent report:
`external-step-unclamped/opencascade-sweep.json`.

Remaining: actual viewport/Solid UI qualification of this new fixture, periodic
original-domain ownership, arbitrary rounded extraction topology and the
original all-mode/global requirements. Selected fixture success does not close
the library goal.

## Wide UI Solid follow-up

The actual 1280×720 UI built `miter-unclamped-hollow.r` into a new Solid group
`Sweep unclamped qualification`, alongside the original scene group. The new
body retained B-rep storage. Its actual B-rep properties operation returned
V=15.6250 mm³, A=60.5147 mm² and center [0,0,5], matching the analytic volume.
The properties panel labels its mass-property output a numerical estimate;
that label is not evidence for the separate native boundary/Solid certificates.
Screenshot: `sweep-unclamped-solid-ui.jpg`.

After exiting isolation, Undo removed the test group. Reload restored the
original `model` group and single B-rep body with no test group and no selection.
The temporary browser tab was closed. Main source was not edited. This closes
wide-window positive group construction and restoration for this fixture only;
narrow-window, refusal and active-cancellation coverage of this new fixture
remain pending.
