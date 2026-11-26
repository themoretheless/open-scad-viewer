// @rush/1
a = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[0, 0, 0], [2, 0, 0], [2, 2, 0], [0, 2, 0], [0, 0, 0]],weights:[1,1,1,1,1])
b = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[0, 0, 2], [2, 0, 2], [2, 2, 2], [0, 2, 2], [0, 0, 2]],weights:[1,1,1,1,1])
result = a.prove_spatial_clearance_along(inputs:[b],normal:[0,0,1],minimum_mm:2)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
