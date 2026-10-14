// @rush/1
// XYZ = (u, v, 0.1*u*v + 0.05*u^2).
show polynomial_surface(domain: [-10,10,-8,8], coefficients: [[[0,0,0],[0,1,0]],[[1,0,0],[0,0,0.1]],[[0,0,0.05],[0,0,0]]]).tessellate(segments_u: 16, segments_v: 32)
