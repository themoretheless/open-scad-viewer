// @rush/1
a = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0, 1, 0], [1, 0, 0]],weights:[1, 1])
b = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0, 0, 0], [1, 0, 0]],weights:[1, 1])
c = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0, 0, 0], [0, 1, 0]],weights:[1, 1])
result = a.assemble_projected_profile(inputs:[b,c],first_segment:1,reverse:false,dropped_axis:2,tolerance_mm:0.0000001,max_pairs:100,max_cells:10000,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
