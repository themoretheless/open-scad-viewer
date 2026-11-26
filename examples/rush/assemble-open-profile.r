// @rush/1
a = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[2, 0, 0], [1, 0, 0]],weights:[1, 1])
b = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0, 0, 0], [1, 0, 0]],weights:[1, 1])
result = a.assemble_open_profile(inputs:[b],start:[0,0,0],max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
