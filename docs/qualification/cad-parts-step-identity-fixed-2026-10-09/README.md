# Current WASM STEP identity and geometry acceptance — 2026-10-09

Fresh packaged WASM SHA256: `3afcab4a6f41e5db19fecd9ce108245372c1595e1dbfdc6e2d8a14a5a1b4c4b7`. Size: 12784038 bytes.

The Rust STEP v9 importer now restores verified topology metadata for a single occurrence, rather than unconditionally generating occurrence identities. Repeated occurrences retain separate identities. A native regression failed before the fix; all 31 STEP module tests passed afterward, including placement, inverse/refusal and coupled-sense cases.

The current CLI acceptance extracted hashed final documents for bracket, flange and enclosure from the previous mouse and keyboard UI history archive. It verifies archive/document hashes, source immutability, preserved=true/lostCount=0, and identical identity sets for vertices, edges, loops, faces, shells and bodies after import. Index order may change; historical lineage is not claimed as STEP-preserved. Each document is exported, imported and re-exported using the current verified Rust WASM, producing 12 fresh STEP files. See `transport-report.json` and `manifest.json`.

Independent OpenCascade 8.0.1.0.0 imported all 12 files successfully. Each body passed validity, solid count, bounding-dimension and volume checks against the archived numerical part references. Maximum bounding error: 1.000000082740371e-7 mm. Maximum absolute volume error: 3.1529634725302458e-6 mm3, within the declared relative volume tolerance. See `occt-report.json` and `occt-preview.png`.

Run the exporter with a new output directory:

```sh
node --import tsx scripts/check-cad-history-step-current.mts docs/qualification/cad-parts-current-2026-10-08 NEW_OUTPUT
python scripts/verify-cad-step-occt.py NEW_OUTPUT
```

The source archives contain previously verified UI histories; this CLI run does not re-execute current UI commands, Undo/Redo or reload. General fillets, boundary-material/surface certification, the full P0 matrix and P2/P3 remain incomplete.


## Authored selection after STEP topology traversal

Current packaged WASM passed 30 existing service tests across Solid STEP exchange, v6 workbench routing, native face/edge selection and connected-edge selection. A new regression also passes: an offset box is exported/imported via v9, displayed at detail 1 versus 8, and each selected authored face identity is matched by its spatial plane center/normal; each selected straight edge identity is matched by original endpoint geometry. All six face and twelve edge identities retain their geometry association, with zero created identities and unchanged source. The native selection suite now passes seven tests. This checks actual Rust/WASM selection and STEP transport, not a browser command replay.
