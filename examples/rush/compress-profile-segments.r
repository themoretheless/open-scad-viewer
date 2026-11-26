// @rush/1
a = nurbs_curve(degree:2,knots:[0, 0, 0, 1, 1, 1],control_points:[[0, 0, 0], [1, 0, 0], [2, 0, 0]],weights:[1, 1, 1])
b = nurbs_curve(degree:2,knots:[0, 0, 0, 1, 1, 1],control_points:[[0, 0, 0], [0, 1, 0], [0, 2, 0]],weights:[1, 1, 1])
result = a.compress_profile_segments(inputs:[b],piece:1,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
