// @rush/1
part0 = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[2, 0, 0], [2, 1, 0]],weights:[1, 1])
result = part0.repeat_profile_segment(copies:4,sector_index:2)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
