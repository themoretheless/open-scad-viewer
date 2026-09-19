# Delivery follow-up after Solid STEP

Baseline source: `6f91aaa49737f0c6fb868696be1941552ad4ffef`. The production
distribution remains 5,776,821 bytes against the unchanged 5,600,000-byte gate.
Do not raise the gate or remove third-party notices to report an optimization.

## Reproduction

```sh
node_modules/.bin/vite build --sourcemap --outDir tmp/performance/bundle-current
node scripts/audit-bundle.mjs tmp/performance/bundle-current
```

The local build passed. The report is retained at
`tmp/performance/bundle-current-report.json`. Its 59 JavaScript maps describe
5,779,909 emitted asset bytes excluding the maps themselves. Source-map comments
account for the difference from the normal build; compare like build modes.

Largest emitted assets in this attribution build:

| Asset | Bytes |
| --- | ---: |
| Packed geometry kernel | 2,788,182 |
| Geometry worker | 473,019 |
| Packed language kernel | 438,277 |
| Packed photogrammetry kernel | 243,210 |
| SVG third-party notices | 218,946 |
| Application entry | 211,977 |

## Next optimization boundary

The geometry worker includes the TypeScript OpenSCAD parser, semantic lowerer,
import decoders and semantic-program validator. Their original source lengths
are respectively 157,939, 144,685, 131,467 and 130,231 bytes. These are source
attributions, not removable minified bytes. Establish which runtime routes need
each component before moving more work to the existing Rust language kernel.
Compatibility must cover diagnostics, semantic validation, imported assets,
cancellation and geometry identity, not just successful primitive examples.

Repeated B-rep/kernel/codec source identities occur across worker bundles.
Workers have separate runtime realms; duplicated source text does not prove
duplicated live state can be shared. A packaging trial must measure emitted
bytes and cold loading, preserve independent kernel ownership, and verify
production workers. No new runtime speedup or bundle reduction is claimed by
this inspection, and the delivery gate remains open.

## Worker decoder packaging trial

Two isolated source-map builds tested explicit worker chunks without changing
runtime source or WASM artifacts:

| Variant | Emitted bytes excluding maps | Difference |
| --- | ---: | ---: |
| Baseline | 5,779,909 | - |
| SHA-256, binary codec and DEFLATE grouped | 5,773,993 | -5,916 |
| DEFLATE decoder alone | 5,771,445 | -8,464 |

The combined group produced four differently tree-shaken chunks. The narrower
`wasmPacking.ts` group produced one 2,971-byte mapped chunk shared by four worker
entries. Keep the latter in `vite.config.ts`; no live memory, module state or
kernel ownership is shared across workers. The main-thread grouping is unchanged.

Normal production build: 5,776,821 -> 5,768,300 bytes (-8,521; about 0.15%).
The decoded-WASM/source-identity and individual artifact checks in `verify-dist`
pass before its unchanged total gate rejects the remaining 168,300-byte excess.
One extra module dependency is introduced on affected worker cold-load paths;
this is a measured distribution reduction, not a latency improvement claim.

Verification: all 31 focused decoder and real-worker-boundary tests passed
(`tmp/performance/worker-packing-tests.log`). The production Chrome scenario
passed source builds, exact-solid refusal, cancellation/recovery, App group
replacement and all three mechanical generators. All three source and geometry
hashes match `owned-brep-browser/report.json`; no page errors occurred. Evidence:
`tmp/performance/worker-packing-browser/report.json`. This browser scenario does
not exercise production SVG/G-code UI; their focused boundary tests run outside
the production bundle. A controlled cold-network comparison remains open.

A subsequent production packaging check closes the SVG/G-code worker loading
gap (not their full UI coverage). Run `npm run test:worker-packaging` after a
production build and installation of `tools/browser-qualification` dependencies.
Use `CHROMIUM_EXECUTABLE` for an installed local Chrome or provision Playwright's
Chromium. The script serves `dist` on strict port 4183 and exercises the actual
module-worker entry files from a same-origin inert page. SVG preview dimensions
and generated extrusion source are checked, as are exact G-code moves, extrusion,
print distance, invalid-job refusal and recovery in the same worker. Workers,
browser and preview server close on completion/failure. Local Chrome 156 passed;
evidence is `tmp/performance/worker-packaging/report.json`. This is correctness
coverage, not a network-throttled startup benchmark or full slicer qualification.
