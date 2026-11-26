// @rush/1
left = nurbs_curve(degree:1,knots:[-2,-2,1,1],control_points:[[0,0,0],[3mm,0,0]],weights:[1,2])
right = nurbs_curve(degree:2,knots:[1,1,1,5,5,5],control_points:[[3mm,0,0],[4mm,2mm,0],[5mm,0,0]],weights:[3,5,3])
result = left.join_mixed_degree(target_geometry:right,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
