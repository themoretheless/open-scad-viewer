// @rush/1
a = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0, 1, 0], [1, 0, 0]],weights:[1, 1])
b = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0, 0, 0], [1, 0, 0]],weights:[1, 1])
c = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0, 0, 0], [0, 1, 0]],weights:[1, 1])
result = a.assemble_closed_profile(inputs:[b,c],first_segment:1,reverse:false,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
