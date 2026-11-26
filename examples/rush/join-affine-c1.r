// @rush/1
left = nurbs_curve(degree:1,knots:[0, 0, 1, 1],control_points:[[0mm,0mm,0mm],[3mm,0mm,0mm]],weights:[1, 1])
right = nurbs_curve(degree:1,knots:[10, 10, 13, 13],control_points:[[3mm,0mm,0mm],[9mm,0mm,0mm]],weights:[1, 1])
result = left.join_affine_c1(target_geometry:right,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
