# Same-host Linux golden reconciliation

[Diagnostic execution](https://github.com/themoretheless/open-scad-viewer/actions/runs/38591445273) captured complete pre-hash payloads from main `2d63a3bec114041daf4cec8a789a882585d18646` and candidate `3c19575792ef5002fe9fb85357bd0b36075caab5` on one Ubuntu host with separate build targets. Main reproduces all 43 historical hashes exactly. Current differs in precisely the six hashes rejected by [Linux CI](https://github.com/themoretheless/open-scad-viewer/actions/runs/38583970477).

Three distance Debug payloads now print typed `DistanceStopReason` variants. Compensated native curve evaluation changes one witness coordinate by one ULP and another by two ULP; both witnesses remain inside their bit-identical outward enclosures. All distance interval limits, parameters, work counts, budgets and convergence values agree. Surface distance has no numerical change.

The two curve/surface intersection payloads contain no numerical changes, including the complete 721 KB unresolved payload. They correctly distinguish `searchComplete` from `certified:false`, `complete:false` and `uncertified-binary64`; a completed numerical search is not certification.

Sphere mesh topology preserves every integer, field, triangle, edge and vertex index. Compensated evaluation propagates into 111 f64 face values, with maximum absolute delta 3.0773994463828558e-15. This is an observed finite comparison, not a universal error bound. The existing native and WASM geometry checks remain separate evidence.

Only the six corresponding lines in `golden_numeric.txt` are updated. The Rust golden test, all 43 cases, platform restriction and exact full-payload SHA-256 comparison are unchanged. No runtime Rust source or measured WASM byte changes in this reconciliation. The new source bundle explicitly includes the golden fixture.

`payloads.tar.gz` retains both original and measured hashes, all complete canonical payloads, and build/test logs. Extract it and run `python3 verify-payloads.py /absolute/extracted/directory` to repeat the structural, numerical and enclosure comparisons. `verified-analysis.json` is the analysis output. The diagnostic run gives no G1 credit.
