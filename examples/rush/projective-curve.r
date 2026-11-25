// @rush/1
path = nurbs_curve(degree:1,knots:[2,2,5,5],control_points:[[0,0,0],[1mm,2mm,0]],weights:[1,2])
result = path.projective_curve_exact(coefficients:[[1,0,0,0],[0,1,0,0],[0,0,1,0],[1,0,0,1]],max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
