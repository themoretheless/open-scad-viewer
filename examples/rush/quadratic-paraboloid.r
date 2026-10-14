// @rush/1
// Elliptic paraboloid graph z=0.05*x*x+0.08*y*y in mm coordinates.
show quadratic_patch(bounds: [-10mm,10mm,-8mm,8mm], coefficients: [0.05,0,0.08,0,0,0]).tessellate(segments_u: 16, segments_v: 32)
