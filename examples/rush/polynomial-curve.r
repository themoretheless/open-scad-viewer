// @rush/1
// XYZ coefficients by increasing power of t, in the mm coordinate convention.
show polynomial_curve(domain: [-1,1], coefficients: [[0,0,0],[10,0,0],[0,8,0],[0,0,4]]).surface_extrude([0mm,0mm,10mm]).tessellate(segments_u: 16, segments_v: 32)
