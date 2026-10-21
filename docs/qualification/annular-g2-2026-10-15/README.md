# G2 qualification — 2026-10-15

## Native evidence

Three annular cylinders: outer/inner radius, height, transition extent (mm):

- 8 / 4 / 16 / 0.5
- 10 / 7 / 20 / 0.5
- 5.15 / 2.3 / 10.3 / 0.25

Each saved body has 32 faces and 64 edges. Native exact seam audits accept all
32 profile and 32 station seams, including cap–wall transitions. Exact work:
130264, 136996, 761020 respectively, below the unchanged 2000000 budget.
Fresh material audits classify all 496 face pairs per body and certify face
injectivity and boundary agreement; no unresolved faces or pairs remain in
these cases. These results do not qualify arbitrary NURBS blends or curved
sweep cap–wall transitions.

Independent OpenCascade STEP checks accept one valid solid for each case.
Expected volumes are calculated independently by Fraction polynomial
integration of the designed meridian, using scripts/annular-g2-volume-reference.py.
Maximum reported dimension error: 1.000000082740371e-7 mm.
Maximum volume error: 2.5498593458905816e-8 mm³.
Local generated evidence: output/qualification/annular-g2-2026-10-15/native.

The existing automatic sweep cap audit additionally qualifies all 24 internal
cap seams across its three transformed fixtures with position tolerance zero,
normal tolerance 1e-7 and curvature tolerance 1e-5/mm. This is a bounded
geometric result; existing exact flags are retained. Ordinary sweep cap–wall
joins remain C0.

## Integration status

Optional Tube end transitions and a transition-size input are implemented,
with native admission, current-request cancellation, Retry and history.
The final packaged kernel is installed in the working checkout:
`483a8c92b7e324f5b9a2d12b7ccc8b288a3c69cf544d298aa546039de3358d43`.
All eight actual WASM cases pass in the checkout. Type checking passes.
The UI lifecycle regression passes: changed parameters, stale rejection,
localized error, Retry, Escape, late success and Undo.
The real Metal browser creates the 32-face tube without page errors;
Undo/Redo and reload retain byte-equivalent body data in project exports.
See scripts/check-annular-g2-browser.mjs and the local browser evidence.

Distribution verification passes: 173 artifacts, 7,988,148 asset bytes
against the unchanged 8,080,000 limit. Shared native edits were included;
their six curve-clearance/planar-toolpath regressions pass.

## Remaining scope

- The earlier 0.1 mm refusal is fixed in the subsequent
  [decimal transition qualification](../annular-g2-decimal-2026-10-15/README.md).
- General NURBS edges, unequal radii, complex corners and curved sweep
  cap–wall G2 transitions remain outside this constructor.
- The full bracket/flange/enclosure browser acceptance has not been repeated
  on this kernel. Earlier results must not be treated as current validation.
