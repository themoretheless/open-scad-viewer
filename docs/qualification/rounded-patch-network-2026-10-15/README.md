# Closed rational patch networks — 2026-10-15

## Implemented

`brep_core::rounded_patch_network::assemble` constructs one connected closed
shell from retained support, blend and end patches. Endpoints only index
candidate edges. Sharing requires either complete copied boundary poles,
weights and compatible normalized Bezier bases, or a native exact curve/lift
identity. No endpoint healing, mesh reconstruction or tolerance snapping is
used. Opposite shell traversal, exactly two owners per edge, and one cyclic
face link at every vertex are mandatory. Missing end patches, duplicate
ownership, mismatched boundary curves, pinched vertices and exhausted work
refuse atomically. Unchanged source face IDs survive sewing.

`Assembly::qualify` recomputes original-face injectivity, complete contact
coverage, boundary agreement and material orientation. Combinatorial closure
never authorizes a solid by itself. The WASM operation
`brep_closed_patch_network` returns a model only after fresh native admission;
full-boundary G2 is additionally required by default. An explicit
`requireFullG2:false` permits admitted solids with intentional sharp seams,
without upgrading their G2 flags.

The G2 annular constructor now uses this same assembly path. Independent
outer-bottom, outer-top, inner-top and inner-bottom transition extents are
available through `cylinder_with_transitions`, the WASM `transitionSizes`
field and the current Tube interface. Adjacent extents must fit their shared
support segment, and the conservative radial wall bound must stay positive.
These values specify G2 setback extents, not constant circular radii.

## Native and independent STEP acceptance

| Case | Parameters, mm | Exact sewing work | Exact smoothness work |
| --- | --- | ---: | ---: |
| Small transition | Outer/inner 10/7, height 20; extents 0.1 × 4 | 1792 | 748696 |
| Independent transitions | Outer/inner 10/7, height 20; extents 0.1 / 0.25 / 0.5 / 0.75 | 1792 | 461056 |
| Noncircular rational edges | Outer/inner 8/4, height 16; extents 0.5; X × 2, Y × 0.5 | 1792 | 130264 |

Each result has 32 rational faces, 64 shared edges, all 64 G2 seams and all
496 face pairs qualified. The sewing ceiling remains 1000000; the smoothness
ceiling remains 2000000. Three native assembly regressions and four native
end-transition regressions pass, including unchanged source face IDs and
refusal of a pinched two-cube vertex.

Independent OpenCascade accepts one valid solid in each STEP. Expected volumes
come from Fraction integration of the designed meridian, including independent
extents and the known affine determinant; native volume estimates are not
used as the reference. Maximum dimension error: 1.0000001537946446e-7 mm.
Maximum volume error: 2.8946942620677873e-8 mm³.
Local generated evidence: output/qualification/rounded-patch-network-2026-10-15/native.

## Runtime and current interface

Sixteen distinct actual WASM cases have passing results for annular bodies, closed patch networks,
open rational bridges, corner scope and existing automatic caps. Two focused
UI scenarios pass cancellation, stale replies, coded native limit errors,
Retry with corrected independent extents, Undo and Redo. Type checking passes.

The real Metal browser creates the 0.1 / 0.25 / 0.5 / 0.75 mm case, exports its
current STEP, performs Undo/Redo and reload, then compares restored body data
exactly. Independent OpenCascade confirms the browser-exported STEP against
the design oracle. Local evidence:
output/qualification/rounded-patch-network-2026-10-15/browser.
The merged kernel is installed in the working checkout:
`21ac2eb50621ea90ae1caaac38ffb62b653d8b2e8d5b72cee6ee31cb9913dbc2`.
The 15-case base matrix passes together on the installed runtime. The additional
noncircular-network case passes in isolation; both timeout cases subsequently
pass separately without changing the 30-second limit. A loaded-host rerun exceeded
the unchanged 30-second limit in two synchronous cases (37–42 seconds); no
assertion or geometric admission failed. Loaded-host latency is not qualified
by the successful functional receipts and remains a P0 concern.

The real worker-coded native size error highlights all four fields, leaves no
body in the document, and Retry succeeds after correction. The browser receipt
has `nativeLimitRetry:true`, exact restored body data, and the installed hash.
Independent OpenCascade also accepts that final browser-exported STEP.

Distribution verification passes all original ceilings: 173 artifacts,
8,060,870 asset bytes against 8,080,000. Nine regressions for merged curve-plane
intersection and orthogonal offset cleanup pass. The earlier published kernel
is preserved at /private/tmp/osv-before-rounded-network-20261015.

## Explicit remaining scope

The generic assembler currently accepts full rectangular positive-weight
rational Bezier patches, degree 1..32, 4..1024 patches. Automatic trimming of
arbitrary existing B-rep faces, general multi-span/periodic source faces,
collapsed pole edges, cavity shells and multiple disconnected bodies are not
implemented by this entry point. Complete support/end patches must be supplied.

The annular and elliptical end transitions are qualified. Arbitrary valence-3
and higher corner construction with prescribed unequal circular edge radii
remains open. Independent G2 setback extents and a manifold vertex audit do
not implement that general corner solver. The bracket/flange/enclosure chain
has not been repeated for this runtime.
