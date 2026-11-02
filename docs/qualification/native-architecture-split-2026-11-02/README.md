# Native architecture split, 2026-11-02

Base: `419089d79db7bd3b50c67b297e113d9bfc6694a6` (231 catalogue capabilities).

This change separates implementation responsibilities without adding catalogue entries:

- `minkowski3d/curved/codec.rs`: existing Source and EnclosureCertificate serialization, with the same keys, versions and refusal conditions.
- `nurbs_step_shared/linked_graph.rs`: existing linked STEP graph validation and its private traversal/geometry helpers. Writer, surface conversion and parsing utilities remain in the parent.
- `region_replacement/six_edge_corner/graph_proof.rs`: existing complete saved-boundary graph proof, exact derivatives and polygon integration. Construction and radius requests remain in the parent.
- `source_face_contacts/projective.rs`: existing line/projective-fiber contact entry points. Re-exports retain the public API.
- `curve_surface_composition/exact_trace/extraction.rs`: existing exact extraction, rational interpolation and blossom operations. Work charging and degree limits are unchanged.
- Existing composition, projective shell and authored-sweep test bodies move to test files; test module names are retained. No test is deleted or newly ignored.

`verify_moves.py` compares the complete relocated text against the base commit, permitting only the listed module wrappers/imports and helper visibility needed to retain the original effective scope. This is text preservation evidence, separate from compilation or behavioral tests.

The architecture ratchet removes 69 obsolete size pins and lowers existing pins to observed smaller sizes. It adds no pin, raises no pin and changes no layering rule. The remaining 55 legacy oversized-file pins are still explicit debt; a passing ratchet does not mean all source files are below the default limit.

The codec-disabled B-rep check exposed an existing mismatch: operations refer to `minkowski3d::curved`, which is gated by `codec`. All three conflicting declaration/call-site files are unchanged from the base. This refusal is retained, not hidden by disabling native algorithms. The old qualification drift audit also fails against the integrated and now-refactored checkout: it binds an earlier source bundle, including many files no longer in the active checkout. Current intentional source edits also differ from that bundle. All 920 historical executor archives still match their hashes; historical receipts are not refreshed into new proof.

Focused tests, source hashes, actual command outcomes and preserved initial failures are recorded in `receipt.json` when complete. No complete workspace, WASM, UI, STEP round-trip or global geometry qualification is inferred.
