# Partial annular arc angles

The native prototype now accepts a finite signed sweep with absolute value
between 1e-10 and pi radians, starting at angle zero. Entry and exit each use
one quarter of that sweep; the constant-radius middle uses one half.
Unrounded remainder arcs use authored shared endpoint controls and at most
quarter-circle spans. No tolerance welding is used to close the topology.

Fifteen circular-blend native tests pass. Six STEP specimens cover 0.31 radians,
pi/3 and pi, both orientations, R=20 mm, inner radius=5 mm, H=6 mm, r=1.25 mm.
All import as one valid OCCT solid and pass independent removed-material volume
and whole-body bounds at unchanged 1e-6 mm³ and 1e-6 mm thresholds.

Reproduce using the partial-annular-matrix example with this output directory
and a second argument `angles`, then run verify-cad-partial-quarter-occt.py.

This is a construction prototype. Arbitrary placement, source topology mapping,
general rolling-ball transition qualification, boundary self-intersection
proof and user command integration remain open. The finite specimens do not
establish qualification for every accepted angle or radius.
