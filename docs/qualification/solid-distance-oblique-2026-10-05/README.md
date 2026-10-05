# Oblique sphere distance qualification

`probe.json` records the published WASM before this change: offsets (4,4,4) and (8,1,2) did not converge. It remains historical evidence, not the current native result.

The implementation adds a triangle-inequality lower bound from interval Bernstein radius enclosures about independent model centers. No analytic sphere recognition is used. A bounded coordinate search improves upper witnesses only; proposed parameters must be proved inside the original trim domain and points retain interval enclosures. Each additional evaluation consumes the shared cell budget.

## Native results

Radius 3, tolerance 1e-5 mm:

| Offset | Lower mm | Upper mm | Cells | Domain cells |
|---|---:|---:|---:|---:|
| (4,4,4) | 0.9282032302754816 | 0.9282032303057762 | 312 | 564 |
| (8,1,2) | 2.3066238629180438 | 2.306623862939474 | 624 | 1172 |

Both analytic distances are enclosed, both intervals meet tolerance, and returned points were independently evaluated on the reported original faces. Shell-distance regression: 11/11, including a non-spherical ellipsoid clearance. Trimmed-surface distance: 6/6, including holes, periodic seams and exhausted budgets. Solid-volume distance: 8/8, including nesting and cavities. Native logs are saved alongside this note.

## WASM, worker and current interface

The published build is 10,961,183 bytes, SHA256 `345afdb41c69a9f2090b69c775dcb4c9a538bbff870c8015136b7926ce5fcffd`. `current-wasm.json` records all four new placements at tolerance 1e-5 mm. Product and real-worker regression: 51/51 tests across six files. Both type checks, Vite and the distribution artifact budgets passed.

Current-interface mouse and keyboard runs each cover the seven existing volume-distance fixtures plus four new placements, at the interface's default 0.001 mm tolerance. The reports record 14 actual distance requests per run. They verify original documents remain unchanged, localized invalid selection, injected transport failure followed by Retry, cancellation, and explicit delivery of a captured successful worker reply after cancellation. That late reply leaves the tool closed and measurement markers absent. This is a distance-command P0 qualification, not execution of all 96 commands.

Historical bit-identical witness expectations were replaced with evaluation of returned UVs on the original reported faces, enclosure checks and independent analytic distance checks. Bounds and witness coordinates may improve without reproducing an old subdivision path.

## Remaining scope

General B-rep qualification is still open: these finite fixtures do not prove all surfaces, trims, degeneracies or self-intersections. The ellipsoid regression is native shell-distance evidence; it is not a blanket proof of volume validity for transformed rational surfaces.


## Reproduction

Run the six Vitest files listed in `product-worker-tests.log.gz` against a freshly built kernel. For the browser, build `dist`, set `SOLID_CHROMIUM_EXECUTABLE_PATH` if the default Playwright browser is unavailable, then run:

```sh
node scripts/check-solid-volume-distance-browser.mjs /tmp/volume-distance-mouse --faults --extra-fixtures=docs/qualification/solid-distance-oblique-2026-10-05/oblique-ui-fixtures.json
node scripts/check-solid-volume-distance-browser.mjs /tmp/volume-distance-keyboard --keyboard --faults --extra-fixtures=docs/qualification/solid-distance-oblique-2026-10-05/oblique-ui-fixtures.json
```
