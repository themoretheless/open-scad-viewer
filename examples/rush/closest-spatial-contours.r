// @rush/1
a = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[0, 0, 0], [2, 0, 0], [2, 2, 0], [0, 2, 0], [0, 0, 0]],weights:[1,1,1,1,1])
b = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[0, 0, 2], [2, 0, 2], [2, 2, 2], [0, 2, 2], [0, 0, 2]],weights:[1,1,1,1,1])
c = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[0, 0, 3], [2, 0, 3], [2, 2, 3], [0, 2, 3], [0, 0, 3]],weights:[1,1,1,1,1])
result = a.closest_spatial_contours(inputs:[b,c],tolerance_mm:0.000001,max_cells:10000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
