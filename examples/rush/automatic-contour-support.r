// @rush/1
a = nurbs_curve(degree:2,knots:[0, 0, 0, 1, 1, 2, 2, 2],control_points:[[0, 0, 0], [8, -6, 2], [4, -3, 4], [-4, 3, 2], [0, 0, 0]],weights:[1, 2, 1, 3, 1])
b = nurbs_curve(degree:2,knots:[0, 0, 0, 1, 1, 2, 2, 2],control_points:[[3, 4, 0], [11, -2, 2], [7, 1, 4], [-1, 7, 2], [3, 4, 0]],weights:[1, 2, 1, 3, 1])
result = a.inspect_spatial_clearances(inputs:[b],minimum_mm:5,tolerance_mm:0.000001,max_cells:4)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
