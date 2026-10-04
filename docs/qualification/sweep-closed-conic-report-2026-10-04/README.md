# Closed correction and exact conic station qualification

Circle-corrected closed progressive miter bodies previously requested retained
filled-cap proofs even though their cap scope is empty. Both synchronous and
streaming adapters now omit that open-cap obligation for closed bodies. Retained
wall correspondence, correction displacement, complete boundary budget, actual
wall regularity and independent Solid obligations remain mandatory.

Rust now provides a sufficient exact conic-strip relation inside the projective
jet predicate. For quadratic cross strips with weights `[1,w,1]`, positive common
w and unit normalized transverse scale, every along-seam coefficient must have
the same boundary pole E and satisfy `Q_a+Q_b=2E`. Order two additionally checks
`R_a-R_b=Q_a-Q_b`. These imply constant projective coefficients
lambda=4(w-1), nu=2(2w-1)/w, mu=-2(w-1)nu. No rounded coordinate equality or
division is used to admit a seam. Independent native regularity is still required;
noncanonical strips fall back to the general predicate with the remaining work.

The moving-frame fixture's same complete G2 audit now uses 412,080 operations
instead of 2,446,913. This is a work-accounting measurement, not a latency claim.
The shared profile/station budget returns to 2,000,000; each individual native
predicate remains limited to 1,000,000. Regression refuses a budget one below the
complete measured requirement. Native tests compare the conic result with the
general predicate on the same geometry with uniformly scaled weights, detect
small second-jet damage and varying weights, and refuse insufficient work.

Two new runnable examples cover closed circle correction and closed quintic
reconstruction with authored frames, orientation guide and affine laws:
`closed-miter-frame-guide-affine-hollow-corrected.r` and
`closed-progressive-miter-reconstructed-frame-guide-affine-hollow.r`.
The reconstructed hollow body has a complete boundary bound approximately
6.59169068072 mm within its 11 mm budget, profile G2, and a certified Solid.
Its station audit accounts for all 128 seams, including closure: 96 soft joins
certify G2; the 32 quarter/loop joins at four original sharp vertices retain C0
under the constant-projective criterion. Those refusals are identity differences,
not budget exhaustion. Cap scope and filled-cap bounds are empty.

Installed geometry WASM: 10,712,750 bytes, SHA-256
`2a23e26339d0dd84e4f7da9415996a6e4dbbfa8ea752ec3f1428fd903ce7b97d`.
Language WASM remains
`aac56d2056cc6b24dba96cb1667882797c8ed1a8cbfed080494faf739eb78a82`.
The complete build graph is retained in
`/private/tmp/open-scad-viewer-sweep-closed-conic-native-2026-10-04`.
It uses the previously qualified native graph with the new predicate. A new
locked primary-checkout native run was prevented by concurrent manifest/lock
changes; this report does not claim fresh byte equivalence with all those changes.

Validation on this graph and installed binary:

- Native: 23 predicate, 794 NURBS and 693 B-rep tests passed; two B-rep
  measurement tests remain ignored.
- Installed WASM/Rush/viewport: 37 tests across eight related suites passed;
  Vue and MCP typechecks and frozen production Vite build passed.
- Independent STEP/OCCT: all 33 existing and seven reconstruction cases passed
  geometry/basis preservation, topology, coedge ownership, nesting/orientation
  agreement and volume. The new closed case has two closed shells and no caps.
  Its independent polynomial-generator volume reference is approximately
  11.65561193178 cubic mm; OCCT relative discrepancy is below 1.3e-15.
- Browser: 68 scenarios, 774 assertions passed on widths 1440 and 600. Coverage
  includes both new closed examples, profile G2, successful or explicitly refused
  Solid, build/Solid lifecycle cancellation, source changes and recovery.
  Held dispatch tests do not measure mid-kernel interruption latency; headless
  CPU rendering does not qualify GPU behavior.

This is local qualification, not a new CI or publication. Finite fixture coverage
and these sufficient proofs do not establish universal acceptance of arbitrary
frames, rational weights or singular/intersecting geometry. Unsupported and
exhausted obligations still refuse; the extended sweep/miter goal remains active.
