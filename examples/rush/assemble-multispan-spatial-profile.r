// @rush/1
a = nurbs_curve(degree:1,knots:[0, 0, 1, 2, 3, 3],control_points:[[0, 0, 0], [2, 2, 0], [0, 2, 1], [2, 0, 1]],weights:[1, 1, 1, 1])
b = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[2, 0, 1], [0, 0, 0]],weights:[1, 1])
result = a.assemble_spatial_profile(inputs:[b],first_segment:0,reverse:false,tolerance_mm:0.0000001,max_pairs:100,max_cells:10000,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
