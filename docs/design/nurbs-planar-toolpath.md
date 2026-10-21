# Clearance-qualified outside contour paths in Rust

`nurbs_core::planar_toolpath::generate` builds planar cutter-center paths around
one simple authored closed contour. This adds manufacturing admission to an
offset: each complete stored path must have a proved global clearance from
every original boundary piece, and must lie outside the material. It is not an
alias for curve offset construction.

Input is an immutable list of 2..64 bounded XY NURBS pieces in traversal order.
An original trim-region audit proves loop simplicity, exact authored joins and
winding. Clockwise and counterclockwise input are supported. The cutter radius
is positive; stock allowance is nonnegative; stepover and tolerance are
positive. Generate 1..64 passes, ordered from outermost inward.

Every nominal source-normal offset includes `tool_radius + stock_allowance`,
three quarters of the requested tolerance as a proof margin, and the pass's stepover.
Outward arithmetic chooses the actual stored distance, which is reported.
Source-normal portions are fitted with an eighth of the requested tolerance;
represented miter connectors use their endpoint tangent lines and a bounded
extension. If a miter cannot be constructed, a bevel is proposed. A proposed
connector never bypasses the independent clearance proof.

The complete connected closed polyline is retained as unit-weight degree-one
NURBS chunks, with at most 256 controls per curve and exact shared endpoints.
Every chunk is compared against every original contour curve using threshold-driven continuous
outward hull separation over the complete original product domain. Its lower bound must be at least the outward-rounded
sum of cutter radius and stock allowance. A finely converged global minimum is not needed: `curve_distance::prove_clearance`
removes a parameter cell only when its original hull lower bound proves the
required separation. A hull upper bound below the threshold proves unsafe
clearance. Work or parameter precision refusal retains the complete original
product as an explicit unresolved obligation. The threshold is unchanged.

A proved outside winding witness and strictly positive global clearance keep
the entire connected path outside the simple material boundary. Thus the
admitted cutter disks preserve the requested stock. This is a geometric
clearance statement; no sampled display polygon substitutes for the source.
The report separates source-offset fit error, total nominal-offset error and
global clearance bounds. The nominal-offset error includes the safety margin,
stored-distance rounding and fit deviation; it must remain within the original
requested tolerance. A safety margin never silently enlarges that tolerance.
Corner connectors are designed represented miter/bevel paths, not a claim of
constant distance to a smooth source extension.

The plan is all-or-nothing: paths are present only after every requested pass
passes all gates. Failed or exhausted proof retains source-region diagnostics,
the number of previously qualified passes, a specific stop reason and charged
work; no incomplete or unqualified plan is returned as complete. Counters are
shared across all passes. Accepted offset work is the full binary refinement
node count, not just retained segments; an opaque failed fit reserves its
remaining allowance conservatively. Clearance refusal similarly reports a
conservative charge when its underlying resource exception has no receipt.

Limits: at most 65536 offset cells, 1e6 clearance cells, 1e5 clearance pairs,
1e5 source-region pairs/cells and 1e6 classification cells. Each underlying
clearance/classification query is capped at 1e5 cells. Source coordinates,
cutter/stepover and largest offset are bounded by 1e6; inherited NURBS control
and trim-span limits also apply. Nonrepresentable arithmetic and malformed
inputs return errors; inconclusive geometric admission returns no paths.

Holes/pocket clearing, offset-region cleanup, full area-removal coverage,
self-intersection removal from the motion path, rapid links, retracts, plunges,
feed scheduling, dynamics, G-code, WASM/editor and exchange integration remain
separate. Toolpaths are geometric XY curves, not machine instructions.

Independent regressions cover clockwise/counterclockwise rectangles, rational
circular profiles, nested pass spacing, exact stored closure, original-boundary
clearance, shared resource refusal, malformed inputs and a cutter too large
for a concave slot. Separate clearance tests distinguish proof of a global
lower bound, proof of a close pair and a full-domain unresolved budget result.
