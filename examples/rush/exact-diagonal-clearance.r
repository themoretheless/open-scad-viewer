// @rush/1
a = nurbs_curve(degree:1,knots:[0, 0, 1, 2, 3, 3],control_points:[[0, 0, 0], [-1, 0, 0], [0, -1, 0], [0, 0, 0]],weights:[1, 2, 3, 1])
b = nurbs_curve(degree:1,knots:[0, 0, 1, 2, 3, 3],control_points:[[3, 4, 0], [4, 4, 0], [3, 5, 0], [3, 4, 0]],weights:[1, 2, 3, 1])
result = a.inspect_spatial_clearances(inputs:[b],minimum_mm:5,tolerance_mm:0.0000001,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
