# Curved station reconstruction with material proof — 2026-10-03

The synchronized-monotone-coordinate certificate resolves the four curved
longitudinal wall pairs left unproved by the preceding quintic build. Both
Bezier walls must share the same strictly monotone coordinate controls,
independent of U, and have positive weights constant along each V row. Any
contact then has the same V parameter on both walls. Strict opposite control
offsets from their identical shared edge prove that their images meet only
on that edge. Damaged synchronization, varying V weights and extra zero-side
controls refuse. Validation bounds both control dimensions to 33; no fitted
plane, rounded normal, sampling or tolerance enters this certificate.

The curved canonical fixture now has native complete boundary embedding,
consistent shell roles and outward material orientation. The owned-source API
also passes a genuinely curved affine-center-law reconstruction with station
G2 and full error composition. Its wall reconstruction upper is
0.4000000000000013 and its complete boundary upper is 1.1500000000000279;
total tolerance 1 refuses and 2 admits. This is a finite fixture qualification,
not a claim of small error for every frame/guide law.

Final optimized geometry WASM:
`c2b0c324a53fb352df49386e8768fb608baa47712cb6a959ef593b261e96689d`,
10,706,959 bytes, installed after checking the previous package identity.
All 693 brep-core tests passed (two measurement tests ignored), including the
eight shared-boundary regressions and full curved material proof. The frozen
optimized frontend passed 28 tests across seven suites; its final three new
tests also passed against the installed primary artifact. Vue/MCP type checks,
production build and scoped whitespace checks passed. The native NURBS and
predicate sources retain the preceding snapshot's 791/20-test qualification.

Both direct and curved owned-source solids passed independent STEP/OCCT import,
topology, original basis/control-net, full-domain wall and edge preservation,
material orientation and analytic-volume checks. Relative volume error was
2.2617277734851674e-16 for both. Native STEP round-trip then re-proved Solid and
station G2. On the final build the existing station-G2 Rush fixture passed
both 1440-wide and 600-wide UI scenarios, 24 assertions total. This UI check
does not exercise a new Rush reconstruction option, which is not yet exposed.

Compiled at `/private/tmp/open-scad-viewer-sweep-quintic-native-2026-10-03`;
final source/artifact snapshot copied to
`/private/tmp/open-scad-viewer-sweep-monotone-native-2026-10-03`.
`frozen-source-hashes.json` records 2,745 snapshot files. Qualification is about
that frozen graph, not the primary checkout's concurrent native migration.
Older full 33-case STEP and 56-scenario UI results remain associated with their
older artifacts and are not silently transferred to this build.

Remaining: expose bounded reconstruction through Rush and viewport; qualify
moving-frame/curved-guide/affine, hollow and closed combinations; extend shared
boundary proofs beyond the sufficient Cartesian synchronized-coordinate case;
repeat complete STEP/UI matrices; finish scoped publication and new CI.
The overall goal stays active and unpublished.
