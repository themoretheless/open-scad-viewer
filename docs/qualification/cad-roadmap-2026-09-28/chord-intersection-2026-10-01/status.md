# Proper chord intersection construction

Added outward interval parameters and positional enclosure for a proper intersection of two represented XY chords. Both parameters must be proved strictly inside the segments; endpoint contacts, overlap, parallel or unresolved cases are refused. The retained midpoint is bounded against the exact represented line intersection by error_upper_mm, with an explicit tolerance gate.

Corrected signed interval division for negative determinants, also used by miter intersection construction. Tests include fractional intersections, large translation, reversal, contact/parallel/outside cases, unmet tolerance and reversed miter direction. Full nurbs-core test results are saved in native.txt.

Open: intersection graph splitting, endpoint/overlap graph rules, winding selection, local edge reconstruction errors, region admission and browser exposure. This primitive does not yet trim an offset or certify a region. New native changes are not yet in the browser WASM.
