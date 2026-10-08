// @rush/1
// Coordinate expressions compile algebraically; output coordinates use mm.
show formula_curve(domain: [-2,2], expressions: ["8*t","3*t^2","t^3"]).surface_extrude([0mm,0mm,10mm]).tessellate(segments_u: 16, segments_v: 32)
