# Convex G2 transition with calibrated scalar core bending

Added opt-in `build_convex_g2` in Rust. Its exact degree-six transverse profile is `phi(v)=1-2v+(28/3)v^3-18v^4+16v^5-(16/3)v^6`. The prescribed second derivative is `v(1-v)*(56-160v(1-v))`, nonnegative throughout [0,1], zero at both contacts and equal to 4 at the center. Exact half-interval Bernstein hulls certify this sign across the entire width. Original contact positions and two-jets are preserved. This is a variable-radius G2 blend, not a constant-radius rolling-ball fillet.

The first beta-bending candidate also sewed but changed center radius to about 0.533 mm on the R1 control fixture; it was replaced. The final profile preserves scalar center bending of the quadratic core. Actual physical radius remains separately measured: a planar constant-support regression verifies R1 within 0.001 mm over U=[0,1], V=[0.499,0.501], and whole-support curvature below 10 mm^-1. This narrow radius region is not full-width qualification.

The B-rep regression assembles two transitions, splits source end roots, trims/retains original supports, closes against original faces and admits the exact native volume only after global embedding checks. Independent polynomial integration gives mean(phi)=67/105 and exact volume loss 29/(105*18432) for the fixture. All 16 B-rep regressions and the dedicated convex-profile regression pass. Source geometry is unchanged. Normal-aligned material-chord minimum remains 2 mm, with explicit attained source-plane witness; unrestricted thickness is not claimed.

The expanded example is run with `--exact-source-brep-split --convex-g2`. The default construction remains unchanged. General rational NURBS trimming, arbitrary unequal-radius nodes, full-radius law, packaged WASM/UI/STEP and history qualification remain open. This opt-in construction is not application admission.

## Expanded final result

The 20-transition case closes into one 27-face, 82-edge, 57-vertex body with no open edges. All 351 distinct face pairs are proven compatible with embedding, all 19 internal joins are G2. Transition curvature is bounded by 9.28019410495333 mm^-1; the retained-source bound sets the body maximum to 9.867331764661289 mm^-1. The exact volume is 39915174481513799624694542632582210243179415812041101223933159/1173531447851167381620779611245756511993803763898926120304640 mm^3 (approximately 34.01287162316934). Bounds [0,40] are control-fixture bounds, not supplied production tolerances.

The actual R1 core check on U=[0,1], V=[0.499,0.501] passes for central panels 2..17 and rejects longitudinal end panels 0,1,18,19. The broader transverse band and the old constant source radius law still reject this variable-radius construction. All 378 wall products, including 27 self products, qualify at 2 mm under the aligned-material-chord criterion, with an exact attained 2 mm chord. The entire pipeline uses 4,963,071 of one 10,000,000 coefficient budget, without resetting it.
