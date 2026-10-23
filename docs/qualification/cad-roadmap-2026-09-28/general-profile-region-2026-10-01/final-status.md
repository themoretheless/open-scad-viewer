# General retained-profile admission and preparation — 2026-10-01

## Admitted behavior

Separate document NURBS curves can be selected with open sketches in Prepare profile. Native assembly preserves curve definitions and segment provenance; successful application consumes selected inputs and keeps the first identity/name/group. Failed preparation preserves inputs. Exact clamped nonperiodic endpoints are required. Separate contours must be prepared separately; retained regions can contain multiple proven loops.

Retained validation admits general rational XY chains when bounded simplicity, winding, pair separation and nesting queries establish their roles. Even-odd input only reverses original definitions. Native tests cover unordered outer loops, holes, islands and disjoint components. A material-area interval is retained; its midpoint is `areaMm2`, with overall `numerical_uncertified` status. Existing analytic arrangements remain available. General profiles can be transformed and extruded with rational side boundaries.

Spatial preparation projects positive-weight control nets into the first sketch plane, or a plane inferred from selected controls. Constant-Z curves retain the XY basis. A measured reconstruction deviation above 1e-7 mm refuses admission. This floating-point projection check is not an exact coplanarity proof. The worker report exposes the measured deviation and rejects malformed values.

## Scope limits

General Boolean/offset, independent loft, sweep, profile dimensions/fillets and broader NURBS preparation remain open. The general proof route refuses unsupported single-curve closed loops, periodic/unclamped endpoints and any insufficient proof/work budget. Existing nonuniform transform restrictions for analytic circles remain. Geometry work may still be expensive; there is no complete P0 latency or general-geometry claim.

Resource admission: 64 loops, 254 curves, 8192 controls; 100000 geometry cells/pairs, 100000 area cells and 1000000 domain cells. Pair-distance work is capped at 4096 cells; separation uses a positive lower bound even when full distance convergence is absent.

## Evidence

- Full native B-rep suite: 682 passed, 3 existing roadmap tests ignored; `brep-tests.txt`.
- Native bridge profile integration: 22 passed; `bridge-tests.txt`.
- Host and actual Node worker boundary: 32 passed; `runtime-tests.txt`.
- Full CAD regression expanded to include new profile suites: 834 passed in 71 files; `full-roadmap-tests.txt`.
- UI suite after the panel fix: 273 passed; `ui-tests.txt`.
- TypeScript and Vite passed; distribution verification in `dist-check.txt`.
- Current Chromium mouse and keyboard scenarios: `browser-panel-final/` and `browser-keyboard-panel-final/`. Preview/cancel, Apply, Undo/repeat, extrusion, Undo restoring curve inputs, JSON reload and STEP export. Scripts wait for asynchronous document transitions and enabled controls. A layout assertion checks that the command heading does not cover help text.
- Independent OpenCascade 8 checks browser-generated STEP from those scenarios: one valid solid, volume 70/3 mm3 and bounds [0,-0.5,0]..[2,2,5], with manifest tolerances. Scene and OCCT previews were inspected.

This stage does not close the original P0–P3 plan. Earlier `status.md` entries and failed preliminary browser runs are historical; current scenario results govern browser admission.

Final assets: 7,173,589 bytes (+8,201 versus the preceding stage); budget 7,174,391 preserves 802 bytes of headroom. Optimized geometry WASM: 9,600,320 bytes.
