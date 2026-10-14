// @rush/1
// z = 0.02*x^3 - 0.1*x*y; coefficients use mm coordinates.
show polynomial_graph(bounds: [-10mm,10mm,-8mm,8mm], coefficients: [[0,0],[0,-0.1],[0,0],[0.02,0]]).tessellate(segments_u: 16, segments_v: 32)
