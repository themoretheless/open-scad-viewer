// @rush/1
// XYZ = (20*u, 20*v, 10*u*v)/(1+u), with a positive denominator.
show rational_polynomial_surface(domain: [0,1,0,1], coefficients: [[[0,0,0,1],[0,20,0,0]],[[20,0,0,1],[0,0,10,0]]]).tessellate(segments_u: 16, segments_v: 32)
