# G2 decimal transitions and rational boundary qualification

## Implemented and native-qualified

The annular tube with outer radius 10 mm, inner radius 7 mm, height 20 mm,
and transition extent 0.1 mm now passes exact saved-coefficient G2 admission.
The endpoints and requested extent remain unchanged. Endpoint handles use the
local binary64 lattice to retain zero second endpoint jets. Native scale
proposals retain exact endpoint subtraction and degree multiplication before
forming the rational speed ratio. The complete coefficient/regularity proof
still decides admission; no tolerance or budget is enlarged.

Four control bodies pass fresh global material admission, all 64 seams and
all 496 face pairs each. Exact work is 130264, 136996, 761020 and 748696,
respectively, below 2000000. Independent OpenCascade checks accept all four
STEP files, including the 0.1 mm transition. Maximum dimension error is
1.0000001537946446e-7 mm; maximum volume error is 2.8946942620677873e-8 mm³.
The expected volumes come from independent Fraction polynomial integration.
Local evidence: output/qualification/annular-g2-decimal-2026-10-15/native.

General rational support-boundary bridges now have a qualified native API:
`surface_blend::g2_qualified` and `brep_g2_surface_bridge`. Both support joins
and every internal patch join share one bounded interval ledger. Subrange
checks evaluate the original support surfaces directly; rounded trimmed
copies do not replace the source. A rational curved-edge fixture passes both
complete joins. Zero work remains explicitly incomplete. Reports describe
open surfaces, not a certified solid or a constant-radius fillet.

Owned seam diagnostics also report corner stars: incident edges and faces,
omitted edges, whether the face link is a single cycle, and whether every
incident seam passed. Partial selection and zero work never certify a corner.
This is corner diagnosis, not construction of a new corner patch.

## Runtime checks

The candidate packaged kernel passes 12 actual WASM cases, including the
0.1 mm tube, rational support bridge, corner scope, and original automatic caps.
The UI lifecycle regression passes Retry with 0.1 mm, stale rejection,
parameter change, Escape, late success and Undo. The Metal browser creates
the 0.1 mm tube without page errors; Undo/Redo and reload retain the saved body.
Local browser evidence: output/qualification/annular-g2-decimal-2026-10-15/browser.
The merged runtime is installed in the working checkout:
`562004ee79623ecb2db5bd6aef57360e8a5a5d26c21ed172e01288b23618238a`.
All 12 actual WASM tests pass in the checkout; type checking passes.
The unchanged distribution budgets pass: 173 artifacts, 8,001,242 asset
bytes below the 8,080,000 ceiling. Concurrent watertight correspondence
changes are included; their three regressions pass. Earlier kernels were
preserved under /private/tmp/osv-before-general-g2-20261015.

Ancillary integration corrections preserve the SVG Cow string optimization
on the pinned compiler and reinterpret u32 semantic-edge GPU output bits.
Six GPU edge tests pass, including nonzero readback byte offsets.

## Not complete

- Arbitrary trimmed 3D NURBS edge fillets with end transitions and closed-body
  assembly are not established by the open support-surface bridge.
- Unequal radii and arbitrary valence-3/4+ G2 corner construction remain open.
  The new corner audit does not author those surfaces.
- The bracket/flange/enclosure browser chain has not been repeated on this
  kernel. Earlier acceptance is not evidence for this runtime.
