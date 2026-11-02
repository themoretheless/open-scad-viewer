# Exact closed graph body: display qualification

2026-11-02. Native Rust → packaged WASM → real worker → read-only CAD inspector.

The nine-face control body restores with 27 edges, no open edges, exact identity links, global embedding, G2 joins and principal curvature upper bound 9.867331764661005 mm⁻¹ (limit 10). The native artifact is retained unchanged. This qualifies the existing supported graph-body construction, not arbitrary rational NURBS trimming or general fillets.

Native edge sampling evaluates exact polynomial curves before bounded conversion to display coordinates. The reported error bounds sample storage only; connecting polyline deviation is not certified.

## Verification

- Native sampler: 2 tests; bridge: 3 tests.
- Restore/display transport: 7 tests.
- Actual worker/display transport/panel: 7 tests, including 3 transport tests already counted above. Total: 16 unique selected tests.
- vue-tsc: success.
- Packaged WASM: 16,444,111 bytes; SHA256 f2fa7abefa0c0486c143766a2a862799c2c1a868d3c284cb003c5026bbf25941.
- Browser at dedicated local test origin: imported JSON/GZ fixture, 27 SVG edge buttons; Enter selected edge 1 and changed its stroke to gold; Escape removed all displayed edges; Retry restored all 27; mouse selected edge index 24. Screenshot attached. Empty-scene tutorial hidden while body is displayed; empty sketch pane collapsed.
- UI test verifies stale reply rejection after cancellation, Retry, selection, inactive clearing and worker disposal.

## Limits

No document application: applicationAdmitted=false, radiusLawCertified=false, thicknessCertified=false. No solid shading, arbitrary NURBS qualification, STEP export, independent part measurements or Undo/Redo/reload acceptance in this milestone. The application background also reported unavailable WebGPU and an unmatched mesh runtime manifest; the CAD SVG wire/geometry worker worked independently. These other runtime paths are not qualified here. Timing values in wasm-report are warm in-process handler measurements, not end-to-end UI latency.
