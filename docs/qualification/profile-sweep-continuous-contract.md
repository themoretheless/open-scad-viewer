# Native profile sweep certificates

`surface_profile_sweep` admits a surface only when Rust proves a deviation bound
over every matched profile and path parameter. The fourfold station diagnostic
is retained for comparison and no longer decides admission. Rational positive
weights, authored parameter domains, scale derivatives, retained coefficients,
and rounding are included in the bound.

The adaptive proof refuses when the requested tolerance cannot be proved, a
path tangent may vanish, or `maxCells` is exhausted. No unfinished bound is
reported as continuous. Axis-aligned planar paths use an analytic Bishop frame
and derivative enclosures; general spatial paths currently use a conservative
unit-frame envelope and can therefore refuse tight tolerances.

Closed sweeps with 5, 9, or 17 stations use uniform periodic cubic interpolation.
Rust quantizes the coefficients and checks exact closing strip jets and seam
regularity before reporting G1 or G2. The represented V basis is C2 throughout.
Other station counts retain explicit C0 closure. Quantization and interpolation
deviation are charged to the same continuous error budget.

`continuousBound` certifies matched-parameter deviation, not injectivity or Solid
admission. Surface-wide regularity, global embedding, cap topology, and shell
orientation remain separate obligations. The report explicitly marks these
global claims as uncertified; a successful seam predicate covers only that seam.

G0/G1 semantic qualification concerns program-to-Manifold equivalence. It cannot
establish these NURBS geometry claims or replace an independent STEP check.
