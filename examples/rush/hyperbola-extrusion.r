// @rush/1
// Rational surface, uncapped. Quadratic coefficients use mm coordinates.
show hyperbola_curve(center: [0mm,0mm,0mm], axis_u: [4mm,0mm,0mm], axis_v: [0mm,3mm,0mm], start: -1, end: 1).surface_extrude([0mm,0mm,10mm]).tessellate(segments_u: 16, segments_v: 32)
