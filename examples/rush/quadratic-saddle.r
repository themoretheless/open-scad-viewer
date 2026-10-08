// @rush/1
// Rational surface, uncapped. Quadratic coefficients use mm coordinates.
show quadratic_patch(bounds: [-10mm,10mm,-8mm,8mm], coefficients: [0.05,0,-0.05,0,0,0]).tessellate(segments_u: 16, segments_v: 32)
