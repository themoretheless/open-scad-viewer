// @rush/1
left = nurbs_curve(degree:2,knots:[0, 0, 0, 1, 1, 1],control_points:[[0mm,0mm,0mm],[0.5mm,0mm,0mm],[1mm,1mm,0mm]],weights:[1, 1, 1])
right = nurbs_curve(degree:2,knots:[10, 10, 10, 12, 12, 12],control_points:[[1mm,1mm,0mm],[1.5mm,2mm,0mm],[2.5mm,5mm,0mm]],weights:[3, 3, 3])
result = left.join_projective_c2(target_geometry:right,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
