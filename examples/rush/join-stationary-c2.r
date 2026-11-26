// @rush/1
left = nurbs_curve(degree:2,knots:[0, 0, 0, 1, 1, 1],control_points:[[1mm,0mm,0mm],[0mm,0mm,0mm],[0mm,0mm,0mm]],weights:[1, 1, 1])
right = nurbs_curve(degree:2,knots:[10, 10, 10, 12, 12, 12],control_points:[[0mm,0mm,0mm],[0mm,0mm,0mm],[1mm,0mm,0mm]],weights:[1, 1, 1])
result = left.join_stationary_c2(target_geometry:right,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
