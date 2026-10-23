# Early NURBS curve separation — qualified stage, 2026-10-01

`curve_distance::prove_separation` retains the original global interval search and every original knot-span pair. It stops once the global distance lower bound is strictly positive. This proves disjoint curve images; `reason=separated` does not claim full distance convergence. The existing `distance` operation retains its tolerance behavior. Contact, crossing, insufficient work and precision limits do not acquire a separation proof.

The new mode is connected to trim simplicity, trim-region separation and retained-profile region admission. Source definitions and material-role proof conditions remain intact. Added native cases cover one-cell rational separation, insufficient initial coverage, late-span endpoint contact, crossing and exact interior tangency. A host regression covers a noncircular profile with a hole, nested island and disjoint component, preserving inputs and checking extrusion volume.

## Validation

- 230 native NURBS tests passed. Two focused tests after adding the interior tangency case passed.
- 682 native B-rep tests passed; three existing roadmap tests remain ignored.
- 835 CAD tests passed across 71 files, including the actual Node worker boundary.
- Current Chromium mouse and keyboard runs passed preparation/cancel, Apply, Undo/repeat, JSON reload and extrusion; adjacent `browser-mouse/` and `browser-keyboard/` artifacts.
- Both browser STEP files independently passed OpenCascade: one valid solid, volume 70/3 mm3 and expected bounds [0,-0.5,0]..[2,2,5]. UI and OCCT previews inspected.
- WASM/Vite builds and 137 distribution artifact checks passed. Assets 7,172,503 bytes; optimized geometry kernel 9,600,358 bytes. Existing budgets passed without an increase.

## Controlled compute comparison

`comparison-idle/benchmark.json` is the primary timing evidence. Baseline raw WASM comes from commit `cd9c2fb5`; candidate is this stage's packaged artifact. Both are instantiated in one Node process and called through the same binary ABI. Seven measured runs per artifact alternate order after warming. Measurement started after test/browser jobs completed.

On the fixed four-loop weighted multispan fixture, median region validation was 1195.987 ms before and 730.259 ms after. Fixture and result SHA-256 values match across artifacts, and inputs remain unchanged. This measures Node/WASM region validation and binary transport; it excludes UI, worker messaging and rendering. It does not establish a universal latency target or close P0. Earlier baseline/comparison runs under other workloads are exploratory, not the primary timing evidence.

The original P0–P3 plan remains active. General Boolean/intersections, offset, independent loft, closed sweep bounds and the other acceptance criteria remain open. Earlier `status.md` handoff entries are historical.
