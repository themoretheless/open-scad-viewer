// @rush/1
// Homogeneous powers: (20*(1-t^2), 40*t, 0, 1+t^2).
show rational_polynomial_curve(domain: [0,1], coefficients: [[20,0,0,1],[0,40,0,0],[-20,0,0,1]]).surface_extrude([0mm,0mm,10mm]).tessellate(segments_u: 16, segments_v: 32)
